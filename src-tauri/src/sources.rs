//! The source registry (K2P-OS §2).
//!
//! Turns heterogeneous external material — a PDF, a DOCX, a web page, a note —
//! into normalised source objects with stable identity, so that a claim can
//! point at *something* rather than at a filename someone may rename tomorrow.
//!
//! This layer deliberately does not "understand the book". Its whole job is to
//! make every incoming item dependable, uniquely identifiable and searchable.
//!
//! # Extraction, and being honest about its limits
//!
//! Markdown, plain text, HTML and DOCX are extracted in-process with no new
//! dependency: DOCX is a zip holding `word/document.xml`, and we already carry
//! `zip` for EPUB export.
//!
//! **PDF is not.** There is no pure-Rust extractor in this dependency set worth
//! trusting with a research corpus, so a PDF is passed to `pdftotext` when the
//! author has poppler installed, and **refused with the exact install command**
//! when they do not. A silently empty source is far worse than a refusal: it
//! would look like a registered paper that simply had nothing to say.

use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{err, AppResult};

/// Sources longer than this are truncated for extraction, with a note. A
/// 900-page corpus dump helps nobody and would blow out every later prompt.
const MAX_EXTRACT_CHARS: usize = 2_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Pdf,
    Docx,
    Markdown,
    Text,
    Html,
    Web,
    Manual,
}

impl SourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SourceKind::Pdf => "pdf",
            SourceKind::Docx => "docx",
            SourceKind::Markdown => "markdown",
            SourceKind::Text => "text",
            SourceKind::Html => "html",
            SourceKind::Web => "web",
            SourceKind::Manual => "manual",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "pdf" => SourceKind::Pdf,
            "docx" => SourceKind::Docx,
            "markdown" => SourceKind::Markdown,
            "html" => SourceKind::Html,
            "web" => SourceKind::Web,
            "manual" => SourceKind::Manual,
            _ => SourceKind::Text,
        }
    }
    pub fn of_path(path: &Path) -> Self {
        match path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default()
            .as_str()
        {
            "pdf" => SourceKind::Pdf,
            "docx" => SourceKind::Docx,
            "md" | "markdown" => SourceKind::Markdown,
            "html" | "htm" => SourceKind::Html,
            _ => SourceKind::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub kind: SourceKind,
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<i64>,
    pub doi: Option<String>,
    pub isbn: Option<String>,
    pub url: Option<String>,
    /// Where it came from on disk, if it was a file.
    pub origin: Option<String>,
    /// Where the extracted text lives inside the project.
    pub text_path: Option<String>,
    pub content_hash: String,
    pub word_count: i64,
    pub added_at: i64,
    /// What the author is allowed to do with it. Never guessed.
    pub rights: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub year: Option<i64>,
    pub doi: Option<String>,
    pub isbn: Option<String>,
    /// How much of this was actually found rather than defaulted, 0–1.
    pub confidence: f32,
}

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sources (
            id           TEXT PRIMARY KEY,
            kind         TEXT NOT NULL,
            title        TEXT NOT NULL,
            authors      TEXT NOT NULL DEFAULT '[]',
            year         INTEGER,
            doi          TEXT,
            isbn         TEXT,
            url          TEXT,
            origin       TEXT,
            text_path    TEXT,
            content_hash TEXT NOT NULL,
            word_count   INTEGER NOT NULL DEFAULT 0,
            added_at     INTEGER NOT NULL,
            rights       TEXT,
            note         TEXT
        );
        CREATE INDEX IF NOT EXISTS sources_hash ON sources(content_hash);
        CREATE INDEX IF NOT EXISTS sources_doi ON sources(doi);
        ",
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

/// Strip HTML to readable text. Deliberately small: script and style contents
/// are dropped, tags removed, entities decoded, whitespace collapsed.
pub fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len() / 2);
    let bytes: Vec<char> = html.chars().collect();
    let lower = html.to_lowercase();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] == '<' {
            // Skip the whole element for script and style: their contents are
            // code, and pasting minified JavaScript into a research corpus is
            // worse than dropping it.
            let rest = &lower[lower.char_indices().nth(i).map(|(b, _)| b).unwrap_or(lower.len())..];
            for tag in ["script", "style"] {
                if rest.starts_with(&format!("<{tag}")) {
                    if let Some(end) = rest.find(&format!("</{tag}>")) {
                        i += rest[..end].chars().count() + format!("</{tag}>").len();
                    } else {
                        i = bytes.len();
                    }
                }
            }
            if i >= bytes.len() {
                break;
            }
            // A block-level tag becomes a paragraph break so structure survives.
            let is_break = ["<p", "<br", "<div", "<h1", "<h2", "<h3", "<h4", "<li", "</p", "</div"]
                .iter()
                .any(|t| lower[lower.char_indices().nth(i).map(|(b, _)| b).unwrap_or(0)..].starts_with(t));
            while i < bytes.len() && bytes[i] != '>' {
                i += 1;
            }
            i += 1;
            if is_break {
                out.push('\n');
            }
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }

    let decoded = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    decoded
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Pull the text out of a DOCX. It is a zip holding `word/document.xml`;
/// paragraph and break elements become newlines before the tags are stripped.
pub fn extract_docx(bytes: &[u8]) -> AppResult<String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| crate::error::AppError(format!("that .docx is not readable as a zip: {e}")))?;
    let mut xml = String::new();
    zip.by_name("word/document.xml")
        .map_err(|_| {
            crate::error::AppError(
                "that .docx has no word/document.xml — it may be a .doc renamed, which is a \
                 different format this build cannot read"
                    .into(),
            )
        })?
        .read_to_string(&mut xml)?;

    let broken = xml
        .replace("</w:p>", "\n\n")
        .replace("<w:br/>", "\n")
        .replace("<w:tab/>", "\t")
        .replace("</w:tr>", "\n")
        .replace("</w:tc>", " | ");
    Ok(strip_html(&broken))
}

/// Is a PDF extractor available on this machine?
pub fn pdftotext() -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|d| d.join("pdftotext"))
            .find(|p| p.is_file())
    })
}

pub fn extract_pdf(path: &Path) -> AppResult<String> {
    let Some(tool) = pdftotext() else {
        return err(
            "Reading PDFs needs `pdftotext`, which is not installed. Install poppler \
             (`brew install poppler` on macOS, `apt install poppler-utils` on Debian) and try \
             again — AegisDraft will not register a PDF it cannot actually read.",
        );
    };
    let out = std::process::Command::new(tool)
        .arg("-layout")
        .arg(path)
        .arg("-")
        .output()
        .map_err(|e| crate::error::AppError(format!("could not run pdftotext: {e}")))?;
    if !out.status.success() {
        return err(format!(
            "pdftotext could not read {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Extract readable text from a file, by kind.
pub fn extract(path: &Path) -> AppResult<(SourceKind, String)> {
    if !path.is_file() {
        return err(format!("{} is not a file", path.display()));
    }
    let kind = SourceKind::of_path(path);
    let text = match kind {
        SourceKind::Pdf => extract_pdf(path)?,
        SourceKind::Docx => extract_docx(&std::fs::read(path)?)?,
        SourceKind::Html => strip_html(&std::fs::read_to_string(path)?),
        _ => std::fs::read_to_string(path).map_err(|e| {
            crate::error::AppError(format!(
                "{} is not readable as text ({e}). If it is a binary format, it needs a \
                 converter this build does not have.",
                path.display()
            ))
        })?,
    };
    if text.trim().is_empty() {
        return err(format!(
            "{} produced no text. A source with nothing in it would look registered and be \
             useless, so it is refused instead.",
            path.display()
        ));
    }
    let text = if text.chars().count() > MAX_EXTRACT_CHARS {
        let cut: String = text.chars().take(MAX_EXTRACT_CHARS).collect();
        format!("{cut}\n\n[Truncated by AegisDraft at {MAX_EXTRACT_CHARS} characters.]")
    } else {
        text
    };
    Ok((kind, text))
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

fn find_doi(text: &str) -> Option<String> {
    // `10.<registrant>/<suffix>`. Hand-rolled rather than pulling in a regex
    // crate for one fixed shape. Searched in the original text, not a
    // lowercased copy: `to_lowercase` can change byte lengths on non-ASCII and
    // the offsets would then slice mid-character.
    //
    // The registrant is at least four digits. Without that guard "version
    // 10.2/beta" reads as a DOI, and a fabricated identifier on a source is
    // worse than no identifier — it makes a citation look verifiable.
    let mut from = 0usize;
    while let Some(offset) = text[from..].find("10.") {
        let at = from + offset;
        from = at + 3;
        let rest = &text[at..];
        let Some(slash) = rest.find('/') else { continue };
        let registrant = &rest[3..slash];
        if registrant.len() < 4 || !registrant.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let end = rest[slash..]
            .find(|c: char| c.is_whitespace() || c == ')' || c == ']' || c == ',' || c == ';')
            .map(|i| slash + i)
            .unwrap_or(rest.len());
        let doi = rest[..end].trim_end_matches(['.', ',']).to_string();
        if doi.len() > 8 {
            return Some(doi);
        }
    }
    None
}

fn find_isbn(text: &str) -> Option<String> {
    let idx = text.to_lowercase().find("isbn")?;
    let tail: String = text[idx..].chars().skip(4).take(24).collect();
    let digits: String = tail.chars().filter(|c| c.is_ascii_digit() || *c == 'X').collect();
    (digits.len() == 10 || digits.len() == 13).then_some(digits)
}

fn find_year(text: &str) -> Option<i64> {
    // The first plausible publication year in the opening of the document.
    let head: String = text.chars().take(3000).collect();
    let chars: Vec<char> = head.chars().collect();
    for w in chars.windows(4) {
        if w.iter().all(|c| c.is_ascii_digit()) {
            let year: i64 = w.iter().collect::<String>().parse().ok()?;
            if (1500..=2100).contains(&year) {
                return Some(year);
            }
        }
    }
    None
}

/// Read what the document says about itself.
///
/// `confidence` is the share of fields actually found, so a caller can tell a
/// well-described paper from a bare text file rather than treating every source
/// as equally well known.
pub fn read_metadata(text: &str, fallback_title: &str) -> Metadata {
    let head: String = text.chars().take(4000).collect();

    let title = head
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|l| l.trim_start_matches('#').trim().to_string())
        .filter(|l| l.len() > 3 && l.len() < 300);

    // A line of comma- or "and"-separated capitalised names near the top.
    let authors = head
        .lines()
        .skip(1)
        .take(12)
        .map(str::trim)
        .find(|l| {
            !l.is_empty()
                && l.len() < 200
                && (l.contains(',') || l.contains(" and "))
                && l.split([',', '&'])
                    .filter(|p| !p.trim().is_empty())
                    .all(|p| p.trim().chars().next().is_some_and(|c| c.is_uppercase()))
        })
        .map(|l| {
            l.split([',', '&'])
                .flat_map(|p| p.split(" and "))
                .map(|p| p.trim().to_string())
                .filter(|p| p.len() > 2)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let doi = find_doi(&head);
    let isbn = find_isbn(&head);
    let year = find_year(text);

    let found = [title.is_some(), !authors.is_empty(), year.is_some(), doi.is_some() || isbn.is_some()]
        .iter()
        .filter(|x| **x)
        .count();

    Metadata {
        title: title.or_else(|| Some(fallback_title.to_string())),
        authors,
        year,
        doi,
        isbn,
        confidence: found as f32 / 4.0,
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

fn row_to_source(row: &rusqlite::Row<'_>) -> rusqlite::Result<Source> {
    let authors: String = row.get("authors")?;
    Ok(Source {
        id: row.get("id")?,
        kind: SourceKind::parse(&row.get::<_, String>("kind")?),
        title: row.get("title")?,
        authors: serde_json::from_str(&authors).unwrap_or_default(),
        year: row.get("year")?,
        doi: row.get("doi")?,
        isbn: row.get("isbn")?,
        url: row.get("url")?,
        origin: row.get("origin")?,
        text_path: row.get("text_path")?,
        content_hash: row.get("content_hash")?,
        word_count: row.get("word_count")?,
        added_at: row.get("added_at")?,
        rights: row.get("rights")?,
        note: row.get("note")?,
    })
}

const SELECT: &str = "SELECT id, kind, title, authors, year, doi, isbn, url, origin, text_path,
                             content_hash, word_count, added_at, rights, note FROM sources";

pub fn list(conn: &Connection) -> AppResult<Vec<Source>> {
    let mut stmt = conn.prepare(&format!("{SELECT} ORDER BY added_at DESC"))?;
    let rows = stmt.query_map([], row_to_source)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get(conn: &Connection, id: &str) -> AppResult<Option<Source>> {
    let mut stmt = conn.prepare(&format!("{SELECT} WHERE id = ?1"))?;
    let mut rows = stmt.query_map(params![id], row_to_source)?;
    Ok(match rows.next() {
        Some(r) => Some(r?),
        None => None,
    })
}

/// An existing source that is the same material, if there is one.
///
/// Content hash first, then DOI, then ISBN. Deliberately not fuzzy on title: a
/// wrong merge is much more expensive than a duplicate, because it silently
/// attributes one paper's claims to another.
pub fn find_duplicate(
    conn: &Connection,
    content_hash: &str,
    doi: Option<&str>,
    isbn: Option<&str>,
) -> AppResult<Option<Source>> {
    let mut stmt = conn.prepare(&format!("{SELECT} WHERE content_hash = ?1"))?;
    let mut rows = stmt.query_map(params![content_hash], row_to_source)?;
    if let Some(r) = rows.next() {
        return Ok(Some(r?));
    }
    for (column, value) in [("doi", doi), ("isbn", isbn)] {
        let Some(value) = value.filter(|v| !v.is_empty()) else { continue };
        let mut stmt = conn.prepare(&format!("{SELECT} WHERE {column} = ?1"))?;
        let mut rows = stmt.query_map(params![value], row_to_source)?;
        if let Some(r) = rows.next() {
            return Ok(Some(r?));
        }
    }
    Ok(None)
}

pub const TEXT_DIR: &str = "research/sources";

/// Register a file as a source: extract, read its metadata, deduplicate, and
/// keep the extracted text inside the project as a readable file.
pub fn register_file(root: &Path, path: &Path, now: i64) -> AppResult<(Source, bool)> {
    let (kind, text) = extract(path)?;
    let content_hash = crate::blocks::body_hash(&text);
    let conn = crate::store::open(root)?;

    let fallback = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Untitled source".into());
    let meta = read_metadata(&text, &fallback);

    if let Some(existing) =
        find_duplicate(&conn, &content_hash, meta.doi.as_deref(), meta.isbn.as_deref())?
    {
        // Already here. Returning the existing source rather than erroring lets
        // a caller re-import a folder without fighting it.
        return Ok((existing, true));
    }

    let id = format!("src_{}", crate::project::new_id());
    let title = meta.title.clone().unwrap_or(fallback);
    let rel = format!("{TEXT_DIR}/{}.txt", crate::project::slugify(&title));
    let rel = unique_rel(root, &rel);
    std::fs::create_dir_all(root.join(TEXT_DIR))?;
    std::fs::write(root.join(&rel), &text)?;

    let source = Source {
        id: id.clone(),
        kind,
        title,
        authors: meta.authors,
        year: meta.year,
        doi: meta.doi,
        isbn: meta.isbn,
        url: None,
        origin: Some(path.display().to_string()),
        text_path: Some(rel),
        content_hash,
        word_count: crate::project::count_words(&text),
        added_at: now,
        rights: None,
        note: None,
    };
    insert(&conn, &source)?;

    // A source is an object with a lineage like any other.
    crate::provenance::append(
        &conn,
        crate::provenance::NewEvent::block(
            &id,
            crate::provenance::Actor::tool("source_registry"),
            crate::provenance::Action::Import,
            crate::provenance::Confidence::Observed,
        )
        .about("source", &id)
        .hashes(None, Some(source.content_hash.clone()))
        .note(&format!("Registered from {}", path.display())),
        now,
    )?;

    Ok((source, false))
}

/// Register text that did not come from a file — a web page, a note, a quote.
#[allow(clippy::too_many_arguments)]
pub fn register_text(
    root: &Path,
    title: &str,
    text: &str,
    kind: SourceKind,
    url: Option<&str>,
    note: Option<&str>,
    now: i64,
) -> AppResult<(Source, bool)> {
    if text.trim().is_empty() {
        return err("a source with no text is not a source");
    }
    let conn = crate::store::open(root)?;
    let content_hash = crate::blocks::body_hash(text);
    let meta = read_metadata(text, title);

    if let Some(existing) =
        find_duplicate(&conn, &content_hash, meta.doi.as_deref(), meta.isbn.as_deref())?
    {
        return Ok((existing, true));
    }

    let id = format!("src_{}", crate::project::new_id());
    let rel = unique_rel(root, &format!("{TEXT_DIR}/{}.txt", crate::project::slugify(title)));
    std::fs::create_dir_all(root.join(TEXT_DIR))?;
    std::fs::write(root.join(&rel), text)?;

    let source = Source {
        id: id.clone(),
        kind,
        title: title.to_string(),
        authors: meta.authors,
        year: meta.year,
        doi: meta.doi,
        isbn: meta.isbn,
        url: url.map(str::to_string),
        origin: url.map(str::to_string),
        text_path: Some(rel),
        content_hash,
        word_count: crate::project::count_words(text),
        added_at: now,
        rights: None,
        note: note.map(str::to_string),
    };
    insert(&conn, &source)?;
    crate::provenance::append(
        &conn,
        crate::provenance::NewEvent::block(
            &id,
            crate::provenance::Actor::tool("source_registry"),
            crate::provenance::Action::Import,
            crate::provenance::Confidence::Observed,
        )
        .about("source", &id)
        .hashes(None, Some(source.content_hash.clone())),
        now,
    )?;
    Ok((source, false))
}

fn unique_rel(root: &Path, rel: &str) -> String {
    if !root.join(rel).exists() {
        return rel.to_string();
    }
    let (stem, ext) = rel.rsplit_once('.').unwrap_or((rel, "txt"));
    for n in 2..10_000 {
        let candidate = format!("{stem}-{n}.{ext}");
        if !root.join(&candidate).exists() {
            return candidate;
        }
    }
    rel.to_string()
}

fn insert(conn: &Connection, s: &Source) -> AppResult<()> {
    conn.execute(
        "INSERT INTO sources (id, kind, title, authors, year, doi, isbn, url, origin,
                              text_path, content_hash, word_count, added_at, rights, note)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![
            s.id,
            s.kind.as_str(),
            s.title,
            serde_json::to_string(&s.authors)?,
            s.year,
            s.doi,
            s.isbn,
            s.url,
            s.origin,
            s.text_path,
            s.content_hash,
            s.word_count,
            s.added_at,
            s.rights,
            s.note,
        ],
    )?;
    Ok(())
}

pub fn read_text(root: &Path, source: &Source) -> AppResult<String> {
    let Some(rel) = source.text_path.as_deref() else {
        return err(format!("source {} has no extracted text", source.id));
    };
    Ok(std::fs::read_to_string(root.join(rel))?)
}

pub fn forget(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM sources WHERE id = ?1", params![id])?;
    Ok(())
}

/// A citation line in the author's bibliography.
///
/// Generated from the registered source, never typed by hand — K2P-OS §5's
/// "no manually fabricated bibliography entries".
pub fn citation(s: &Source) -> String {
    let authors = match s.authors.len() {
        0 => String::new(),
        1 => format!("{}. ", s.authors[0]),
        2 => format!("{} and {}. ", s.authors[0], s.authors[1]),
        _ => format!("{} et al. ", s.authors[0]),
    };
    let year = s.year.map(|y| format!("({y}). ")).unwrap_or_default();
    let locator = s
        .doi
        .as_ref()
        .map(|d| format!(" https://doi.org/{}", d.trim_start_matches("https://doi.org/")))
        .or_else(|| s.isbn.as_ref().map(|i| format!(" ISBN {i}")))
        .or_else(|| s.url.clone().map(|u| format!(" {u}")))
        .unwrap_or_default();
    format!("{authors}{year}{}.{locator}", s.title.trim_end_matches('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> (PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!(
            "aegis-src-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&base).ok();
        std::fs::create_dir_all(&base).unwrap();
        let summary = crate::project::create(&base, "Sources", "nonfiction").unwrap();
        (base, PathBuf::from(summary.path))
    }

    #[test]
    fn html_loses_its_tags_and_its_scripts() {
        let html = "<html><head><style>p{color:red}</style></head><body>\
                    <h1>The Title</h1><p>First para.</p><script>alert(1)</script>\
                    <p>Second &amp; last.</p></body></html>";
        let text = strip_html(html);
        assert!(text.contains("The Title"));
        assert!(text.contains("First para."));
        assert!(text.contains("Second & last."));
        assert!(!text.contains("alert"), "script contents are code, not prose: {text}");
        assert!(!text.contains("color:red"), "style contents too: {text}");
        assert!(!text.contains('<'));
    }

    #[test]
    fn a_doi_is_found_and_a_version_number_is_not() {
        assert_eq!(
            find_doi("Published in 2025. doi:10.1234/abcd.5678 accessed today"),
            Some("10.1234/abcd.5678".to_string())
        );
        assert_eq!(find_doi("we used version 10.2/beta"), None, "a version is not a DOI");
        assert_eq!(find_doi("see 10.99/x"), None, "too short a registrant is not a DOI");
        // A real DOI later in a line that also contains a version number.
        assert_eq!(
            find_doi("version 10.2/beta, see doi 10.5555/abcd-1234 for details"),
            Some("10.5555/abcd-1234".to_string())
        );
        assert_eq!(find_doi("no identifier here at all"), None);
    }

    #[test]
    fn an_isbn_and_a_year_are_read_from_the_front_matter() {
        assert_eq!(find_isbn("ISBN 978-0-13-235088-4"), Some("9780132350884".into()));
        assert_eq!(find_isbn("no isbn at all here"), None);
        assert_eq!(find_year("Smith, J. (2019). A paper about things."), Some(2019));
        assert_eq!(find_year("no year here"), None);
    }

    #[test]
    fn metadata_confidence_separates_a_described_paper_from_a_bare_file() {
        let paper = "Constraint Satisfaction in Practice\nSmith, J. and Okafor, A.\n2025\n\
                     doi:10.1234/abcd\n\nAbstract. We show that...";
        let rich = read_metadata(paper, "fallback");
        assert_eq!(rich.title.as_deref(), Some("Constraint Satisfaction in Practice"));
        // Surnames, which is what survives splitting an author line. Good
        // enough to identify a paper and honest about what it is.
        assert_eq!(rich.authors, vec!["Smith", "Okafor"]);
        assert_eq!(rich.year, Some(2025));
        assert_eq!(rich.doi.as_deref(), Some("10.1234/abcd"));
        assert!(rich.confidence > 0.5);

        let bare = read_metadata("just some words with nothing to identify them", "fallback");
        assert!(bare.confidence < 0.5, "a bare file must not look well described");
    }

    #[test]
    fn a_source_is_registered_once_however_often_it_is_offered() {
        let (base, root) = scratch("dedupe");
        let file = base.join("paper.md");
        std::fs::write(&file, "# A Paper\n\nSmith, J. and Okafor, A.\n\ndoi:10.1234/abcd\n\nBody text here.").unwrap();

        let (first, was_dup) = register_file(&root, &file, 1).unwrap();
        assert!(!was_dup);
        assert_eq!(first.title, "A Paper");

        let (again, was_dup) = register_file(&root, &file, 2).unwrap();
        assert!(was_dup, "the same file must not become a second source");
        assert_eq!(again.id, first.id);

        // A copy at a different path with the same content is still the same
        // material.
        let copy = base.join("paper-copy.md");
        std::fs::copy(&file, &copy).unwrap();
        let (third, was_dup) = register_file(&root, &copy, 3).unwrap();
        assert!(was_dup);
        assert_eq!(third.id, first.id);

        let conn = crate::store::open(&root).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 1);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_registered_source_keeps_readable_text_inside_the_project() {
        let (base, root) = scratch("text");
        let file = base.join("notes.txt");
        std::fs::write(&file, "The salt flats were a sea once, and the maps still say so.").unwrap();

        let (s, _) = register_file(&root, &file, 1).unwrap();
        let rel = s.text_path.clone().unwrap();
        assert!(root.join(&rel).exists(), "the extracted text is a file the author owns");
        assert!(read_text(&root, &s).unwrap().contains("still say so"));
        assert!(s.word_count > 0);

        // And it has a lineage.
        let conn = crate::store::open(&root).unwrap();
        let chain = crate::provenance::history(&conn, &s.id).unwrap();
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].object_type, "source");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn an_empty_file_is_refused_rather_than_registered_as_a_silent_blank() {
        let (base, root) = scratch("empty");
        let file = base.join("blank.txt");
        std::fs::write(&file, "   \n\n  ").unwrap();
        let message = register_file(&root, &file, 1).unwrap_err().to_string();
        assert!(message.contains("no text"), "{message}");
        assert!(message.contains("refused"), "{message}");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_pdf_without_a_reader_says_exactly_what_to_install() {
        // Only meaningful when poppler is absent; when it is present this
        // asserts the happy path is at least reachable.
        let (base, root) = scratch("pdf");
        let file = base.join("paper.pdf");
        std::fs::write(&file, b"%PDF-1.4 not really a pdf").unwrap();
        match register_file(&root, &file, 1) {
            Err(e) => {
                let m = e.to_string();
                assert!(
                    m.contains("poppler") || m.contains("pdftotext"),
                    "a refusal must name the fix: {m}"
                );
            }
            Ok(_) => assert!(pdftotext().is_some(), "it can only succeed if a reader exists"),
        }
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_citation_is_generated_not_typed() {
        let s = Source {
            id: "s".into(),
            kind: SourceKind::Pdf,
            title: "Constraint Satisfaction in Practice.".into(),
            authors: vec!["Smith, J.".into(), "Okafor, A.".into(), "Lin, R.".into()],
            year: Some(2025),
            doi: Some("10.1234/abcd".into()),
            isbn: None,
            url: None,
            origin: None,
            text_path: None,
            content_hash: "h".into(),
            word_count: 10,
            added_at: 0,
            rights: None,
            note: None,
        };
        let line = citation(&s);
        assert!(line.starts_with("Smith, J. et al. (2025)."), "{line}");
        assert!(line.contains("Constraint Satisfaction in Practice."));
        assert!(line.contains("https://doi.org/10.1234/abcd"));
        assert!(!line.contains(".."), "the trailing period is not doubled: {line}");
    }

    #[test]
    fn a_docx_that_is_not_a_docx_says_so() {
        let message = extract_docx(b"not a zip at all").unwrap_err().to_string();
        assert!(message.contains("not readable as a zip"), "{message}");
    }
}
