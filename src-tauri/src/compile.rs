//! Compile and export (Phase 7).
//!
//! Assembles a set of binder documents into one manuscript and renders it.
//! Section type is what makes this more than concatenation: a Chapter Heading
//! and a Scene are laid out differently, and Front Matter is not a chapter.
//!
//! Source files are never modified. Compile only ever reads.

use crate::error::{err, AppResult};
use crate::model::{CompileFilters, CompilePreset, DocKind, DocumentRow, ExportFormat};

/// One assembled piece: the document plus the body read from disk.
#[derive(Debug, Clone)]
pub struct Item {
    pub doc: DocumentRow,
    pub body: String,
}

/// Does this document belong in the output?
pub fn included(doc: &DocumentRow, f: &CompileFilters) -> bool {
    if doc.kind == DocKind::Folder {
        return false; // folders provide structure, not text
    }
    // Research and canon are working material, not manuscript, unless asked for.
    if !f.include_research && matches!(doc.kind, DocKind::Research | DocKind::Canon) {
        return false;
    }
    if !f.include_sheets && matches!(doc.kind, DocKind::Sheet | DocKind::Location | DocKind::Lore) {
        return false;
    }
    if let Some(want) = &f.statuses {
        match &doc.status {
            Some(s) if want.iter().any(|w| w == s) => {}
            _ => return false,
        }
    }
    if let Some(skip) = &f.exclude_statuses {
        if let Some(s) = &doc.status {
            if skip.iter().any(|w| w == s) {
                return false;
            }
        }
    }
    if let Some(skip) = &f.exclude_section_types {
        if let Some(s) = &doc.section_type {
            if skip.iter().any(|w| w == s) {
                return false;
            }
        }
    }
    if let Some(skip) = &f.exclude_labels {
        if let Some(l) = &doc.label {
            if skip.iter().any(|w| w == l) {
                return false;
            }
        }
    }
    if f.skip_empty && doc.word_count == 0 {
        return false;
    }
    true
}

fn heading_level(item: &Item) -> Option<u8> {
    match item.doc.section_type.as_deref() {
        Some("Part Heading") => Some(1),
        Some("Chapter Heading") => Some(2),
        Some("Front Matter") | Some("Back Matter") => Some(2),
        Some("Scene") => None,
        // With no section type, a document that owns children reads as a
        // chapter; a leaf reads as a scene.
        _ => None,
    }
}

/// Renders Markdown. This is the reference rendering — every other format is
/// derived from the same assembled items, never from this string.
pub fn to_markdown(items: &[Item], preset: &CompilePreset) -> String {
    let mut out = String::new();
    if let Some(title) = &preset.title_page {
        out.push_str(&format!("# {title}\n\n"));
        if let Some(author) = &preset.author {
            out.push_str(&format!("*{author}*\n\n"));
        }
        out.push_str("---\n\n");
    }

    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            match heading_level(item) {
                Some(_) => out.push_str("\n\n"),
                None => {
                    out.push_str("\n\n");
                    if !preset.scene_separator.is_empty() {
                        out.push_str(&preset.scene_separator);
                        out.push_str("\n\n");
                    }
                }
            }
        }
        if let Some(level) = heading_level(item) {
            out.push_str(&"#".repeat(level as usize));
            out.push(' ');
            out.push_str(&item.doc.title);
            out.push_str("\n\n");
        } else if preset.scene_titles {
            out.push_str(&format!("### {}\n\n", item.doc.title));
        }
        if preset.include_synopsis {
            if let Some(s) = &item.doc.synopsis {
                out.push_str(&format!("<!-- synopsis: {s} -->\n\n"));
            }
        }
        out.push_str(item.body.trim());
        out.push('\n');
    }
    out
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Minimal Markdown-to-HTML for the subset the editor produces: paragraphs,
/// ATX headings, emphasis and horizontal rules. Deliberately small — anything
/// richer belongs in a real renderer, not a half-finished one here.
fn md_to_html(md: &str) -> String {
    let mut out = String::new();
    for block in md.split("\n\n") {
        let b = block.trim();
        if b.is_empty() {
            continue;
        }
        if b == "---" {
            out.push_str("<hr/>\n");
            continue;
        }
        if let Some(rest) = b.strip_prefix("### ") {
            out.push_str(&format!("<h3>{}</h3>\n", inline(rest)));
        } else if let Some(rest) = b.strip_prefix("## ") {
            out.push_str(&format!("<h2>{}</h2>\n", inline(rest)));
        } else if let Some(rest) = b.strip_prefix("# ") {
            out.push_str(&format!("<h1>{}</h1>\n", inline(rest)));
        } else if b.starts_with("<!--") {
            out.push_str(b);
            out.push('\n');
        } else {
            out.push_str(&format!("<p>{}</p>\n", inline(b)));
        }
    }
    out
}

fn inline(s: &str) -> String {
    let escaped = escape_html(s);
    // Emphasis, innermost first so ** is not eaten by *.
    let mut out = String::new();
    let mut rest = escaped.as_str();
    while let Some(start) = rest.find("**") {
        if let Some(end) = rest[start + 2..].find("**") {
            out.push_str(&rest[..start]);
            out.push_str(&format!("<strong>{}</strong>", &rest[start + 2..start + 2 + end]));
            rest = &rest[start + 4 + end..];
        } else {
            break;
        }
    }
    out.push_str(rest);
    out
}

pub fn to_html(items: &[Item], preset: &CompilePreset) -> String {
    let title = preset.title_page.clone().unwrap_or_else(|| "Manuscript".into());
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\"/>\n\
         <title>{}</title>\n<style>\n{}\n</style>\n</head>\n<body>\n{}</body>\n</html>\n",
        escape_html(&title),
        preset.stylesheet(),
        md_to_html(&to_markdown(items, preset))
    )
}

/// Fountain: scene headings from binder titles, action as plain paragraphs.
pub fn to_fountain(items: &[Item], preset: &CompilePreset) -> String {
    let mut out = String::new();
    if let Some(title) = &preset.title_page {
        out.push_str(&format!("Title: {title}\n"));
        if let Some(a) = &preset.author {
            out.push_str(&format!("Author: {a}\n"));
        }
        out.push('\n');
    }
    for item in items {
        // Fountain scene headings are upper case and start with a slug word.
        let heading = item.doc.title.to_uppercase();
        let heading = if heading.starts_with("INT") || heading.starts_with("EXT") {
            heading
        } else {
            format!("INT. {heading}")
        };
        out.push_str(&heading);
        out.push_str("\n\n");
        out.push_str(item.body.trim());
        out.push_str("\n\n");
    }
    out
}

/// EPUB 3: a zip with mimetype first and uncompressed, then the container,
/// the package document and one XHTML file per document.
pub fn to_epub(items: &[Item], preset: &CompilePreset) -> AppResult<Vec<u8>> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    if items.is_empty() {
        return err("nothing to compile");
    }
    let title = preset.title_page.clone().unwrap_or_else(|| "Manuscript".into());
    let author = preset.author.clone().unwrap_or_else(|| "Unknown".into());

    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);
    let stored = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let deflated = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // The spec requires this entry first and stored, not deflated.
    zip.start_file("mimetype", stored)
        .map_err(|e| crate::error::AppError(format!("epub: {e}")))?;
    zip.write_all(b"application/epub+zip")?;

    zip.start_file("META-INF/container.xml", deflated)
        .map_err(|e| crate::error::AppError(format!("epub: {e}")))?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#,
    )?;

    let mut manifest = String::new();
    let mut spine = String::new();
    let mut nav = String::new();
    for (i, item) in items.iter().enumerate() {
        let name = format!("ch{i:04}.xhtml");
        manifest.push_str(&format!("<item id=\"ch{i}\" href=\"{name}\" media-type=\"application/xhtml+xml\"/>"));
        spine.push_str(&format!("<itemref idref=\"ch{i}\"/>"));
        nav.push_str(&format!("<li><a href=\"{name}\">{}</a></li>", escape_html(&item.doc.title)));

        let body = md_to_html(&to_markdown(std::slice::from_ref(item), &CompilePreset {
            title_page: None,
            ..preset.clone()
        }));
        zip.start_file(format!("OEBPS/{name}"), deflated)
            .map_err(|e| crate::error::AppError(format!("epub: {e}")))?;
        zip.write_all(
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <html xmlns=\"http://www.w3.org/1999/xhtml\"><head><title>{}</title></head>\
                 <body>{}</body></html>",
                escape_html(&item.doc.title),
                body
            )
            .as_bytes(),
        )?;
    }

    zip.start_file("OEBPS/nav.xhtml", deflated)
        .map_err(|e| crate::error::AppError(format!("epub: {e}")))?;
    zip.write_all(
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\">\
             <head><title>Contents</title></head><body><nav epub:type=\"toc\"><ol>{nav}</ol></nav></body></html>"
        )
        .as_bytes(),
    )?;

    zip.start_file("OEBPS/content.opf", deflated)
        .map_err(|e| crate::error::AppError(format!("epub: {e}")))?;
    zip.write_all(
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\">\
             <metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\
             <dc:identifier id=\"bookid\">aegisdraft-{}</dc:identifier>\
             <dc:title>{}</dc:title><dc:creator>{}</dc:creator><dc:language>en</dc:language>\
             </metadata><manifest><item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>{}</manifest>\
             <spine>{}</spine></package>",
            items.len(),
            escape_html(&title),
            escape_html(&author),
            manifest,
            spine
        )
        .as_bytes(),
    )?;

    let cursor = zip.finish().map_err(|e| crate::error::AppError(format!("epub: {e}")))?;
    Ok(cursor.into_inner())
}

/// DOCX: the minimum Word will open — a document part, content types and one
/// relationship. Headings use Word's built-in Heading styles.
pub fn to_docx(items: &[Item], preset: &CompilePreset) -> AppResult<Vec<u8>> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    if items.is_empty() {
        return err("nothing to compile");
    }
    let mut body = String::new();
    let para = |style: &str, text: &str| -> String {
        format!(
            "<w:p><w:pPr><w:pStyle w:val=\"{style}\"/></w:pPr><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            escape_html(text)
        )
    };

    if let Some(title) = &preset.title_page {
        body.push_str(&para("Title", title));
        if let Some(a) = &preset.author {
            body.push_str(&para("Subtitle", a));
        }
    }
    for item in items {
        match heading_level(item) {
            Some(1) => body.push_str(&para("Heading1", &item.doc.title)),
            Some(_) => body.push_str(&para("Heading2", &item.doc.title)),
            None => {
                if preset.scene_titles {
                    body.push_str(&para("Heading3", &item.doc.title));
                }
            }
        }
        for block in item.body.split("\n\n") {
            let t = block.trim();
            if !t.is_empty() {
                body.push_str(&para("Normal", t));
            }
        }
    }

    let buf = std::io::Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(buf);
    let o = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", o).map_err(|e| crate::error::AppError(format!("docx: {e}")))?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#,
    )?;

    zip.start_file("_rels/.rels", o).map_err(|e| crate::error::AppError(format!("docx: {e}")))?;
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#,
    )?;

    zip.start_file("word/document.xml", o).map_err(|e| crate::error::AppError(format!("docx: {e}")))?;
    zip.write_all(
        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <w:document xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\">\
             <w:body>{body}</w:body></w:document>"
        )
        .as_bytes(),
    )?;

    let cursor = zip.finish().map_err(|e| crate::error::AppError(format!("docx: {e}")))?;
    Ok(cursor.into_inner())
}

/// Bytes for any format. PDF is produced from the print-ready HTML by the
/// system print pipeline rather than a bundled typesetter — see the phase doc.
pub fn render(items: &[Item], preset: &CompilePreset, format: ExportFormat) -> AppResult<Vec<u8>> {
    if items.is_empty() {
        return err("nothing to compile — every document was filtered out");
    }
    Ok(match format {
        ExportFormat::Markdown => to_markdown(items, preset).into_bytes(),
        ExportFormat::Html | ExportFormat::Pdf => to_html(items, preset).into_bytes(),
        ExportFormat::Fountain => to_fountain(items, preset).into_bytes(),
        ExportFormat::Epub => to_epub(items, preset)?,
        ExportFormat::Docx => to_docx(items, preset)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DocKind;

    fn doc(title: &str, section: Option<&str>, status: Option<&str>, words: i64) -> DocumentRow {
        DocumentRow {
            id: title.to_lowercase().replace(' ', "-"),
            parent_id: None,
            title: title.into(),
            kind: DocKind::Scene,
            path: Some(format!("manuscript/scenes/{title}.md")),
            position: 0,
            synopsis: Some(format!("Synopsis of {title}")),
            label: None,
            status: status.map(String::from),
            word_count: words,
            updated_at: 0,
            notes: None,
            target: None,
            pov: None,
            plants: None,
            payoffs: None,
            keywords: vec![],
            links: vec![],
            section_type: section.map(String::from),
            icon: None,
            x: None,
            y: None,
        }
    }

    fn item(title: &str, section: Option<&str>, body: &str) -> Item {
        Item { doc: doc(title, section, Some("Done"), 10), body: body.into() }
    }

    fn sample() -> Vec<Item> {
        vec![
            item("Act I", Some("Part Heading"), ""),
            item("Chapter One", Some("Chapter Heading"), ""),
            item("The Well", Some("Scene"), "The well at Kharsa was going dry."),
            item("Salt and Debt", Some("Scene"), "Nine days, counted by someone else."),
        ]
    }

    // --- filters -----------------------------------------------------------

    #[test]
    fn folders_are_never_compiled() {
        let mut d = doc("Act I", None, Some("Done"), 0);
        d.kind = DocKind::Folder;
        assert!(!included(&d, &CompileFilters::default()));
    }

    #[test]
    fn research_and_sheets_are_excluded_unless_asked_for() {
        for kind in [DocKind::Research, DocKind::Canon, DocKind::Sheet, DocKind::Location, DocKind::Lore] {
            let mut d = doc("X", None, Some("Done"), 10);
            d.kind = kind;
            assert!(!included(&d, &CompileFilters::default()), "{kind:?} must stay out by default");
        }
        let mut research = doc("X", None, Some("Done"), 10);
        research.kind = DocKind::Research;
        let f = CompileFilters { include_research: true, ..Default::default() };
        assert!(included(&research, &f));
    }

    #[test]
    fn status_and_label_filters_narrow_the_set() {
        let done = doc("A", None, Some("Done"), 10);
        let draft = doc("B", None, Some("Draft"), 10);

        let only_done = CompileFilters { statuses: Some(vec!["Done".into()]), ..Default::default() };
        assert!(included(&done, &only_done));
        assert!(!included(&draft, &only_done));

        let not_draft = CompileFilters { exclude_statuses: Some(vec!["Draft".into()]), ..Default::default() };
        assert!(included(&done, &not_draft));
        assert!(!included(&draft, &not_draft));

        let mut labelled = doc("C", None, Some("Done"), 10);
        labelled.label = Some("Notes".into());
        let no_notes = CompileFilters { exclude_labels: Some(vec!["Notes".into()]), ..Default::default() };
        assert!(!included(&labelled, &no_notes));
    }

    #[test]
    fn a_document_with_no_status_is_dropped_by_an_allow_list() {
        // An allow-list is a positive assertion; "unset" does not satisfy it.
        let unset = doc("A", None, None, 10);
        let f = CompileFilters { statuses: Some(vec!["Done".into()]), ..Default::default() };
        assert!(!included(&unset, &f));
    }

    #[test]
    fn empty_documents_can_be_skipped() {
        let empty = doc("A", None, Some("Done"), 0);
        assert!(included(&empty, &CompileFilters::default()));
        assert!(!included(&empty, &CompileFilters { skip_empty: true, ..Default::default() }));
    }

    // --- markdown ----------------------------------------------------------

    #[test]
    fn section_types_drive_heading_levels() {
        let md = to_markdown(&sample(), &CompilePreset::default());
        assert!(md.contains("# Act I"), "a part heading is level 1:\n{md}");
        assert!(md.contains("## Chapter One"), "a chapter heading is level 2:\n{md}");
        assert!(!md.contains("# The Well"), "a scene contributes no heading:\n{md}");
        assert!(md.contains("The well at Kharsa was going dry."));
    }

    #[test]
    fn scenes_are_joined_by_the_presets_separator() {
        let md = to_markdown(&sample(), &CompilePreset::default());
        assert!(md.contains("\n#\n"), "the default manuscript separator sits between scenes:\n{md}");

        let paperback = to_markdown(&sample(), &CompilePreset::named("Paperback"));
        assert!(paperback.contains("* * *"));
        assert!(!paperback.contains("\n#\n"));
    }

    #[test]
    fn a_preset_can_print_scene_titles_and_synopses() {
        let working = to_markdown(&sample(), &CompilePreset::named("Working Draft"));
        assert!(working.contains("### The Well"), "working drafts label their scenes");
        assert!(working.contains("<!-- synopsis: Synopsis of The Well -->"));

        let standard = to_markdown(&sample(), &CompilePreset::default());
        assert!(!standard.contains("### The Well"), "a submission manuscript does not");
        assert!(!standard.contains("synopsis:"));
    }

    #[test]
    fn a_title_page_is_emitted_only_when_asked_for() {
        let preset = CompilePreset { title_page: Some("The Salt Road".into()), author: Some("T. Reyes".into()), ..Default::default() };
        let md = to_markdown(&sample(), &preset);
        assert!(md.starts_with("# The Salt Road"));
        assert!(md.contains("*T. Reyes*"));
        assert!(!to_markdown(&sample(), &CompilePreset::default()).starts_with("# The Salt Road"));
    }

    // --- other formats -----------------------------------------------------

    #[test]
    fn html_escapes_prose_and_carries_a_stylesheet() {
        let items = vec![item("Danger", Some("Scene"), "5 < 6 & \"quoted\" <script>alert(1)</script>")];
        let html = to_html(&items, &CompilePreset::default());
        assert!(html.contains("5 &lt; 6 &amp;"), "angle brackets and ampersands are escaped:\n{html}");
        assert!(!html.contains("<script>"), "raw markup from the manuscript must not survive:\n{html}");
        assert!(html.contains("@page"), "print rules are present for the PDF path");
    }

    #[test]
    fn fountain_upper_cases_scene_headings() {
        let f = to_fountain(&sample(), &CompilePreset::default());
        assert!(f.contains("INT. THE WELL"));
        assert!(f.contains("The well at Kharsa was going dry."));
    }

    #[test]
    fn epub_is_a_valid_archive_with_mimetype_first_and_stored() {
        let bytes = to_epub(&sample(), &CompilePreset { title_page: Some("Book".into()), ..Default::default() }).unwrap();
        // The EPUB spec requires the mimetype entry first, uncompressed.
        assert_eq!(&bytes[30..38], b"mimetype", "mimetype must be the first entry");

        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
        for required in ["mimetype", "META-INF/container.xml", "OEBPS/content.opf", "OEBPS/nav.xhtml"] {
            assert!(names.iter().any(|n| n == required), "missing {required} in {names:?}");
        }
        assert_eq!(names.iter().filter(|n| n.ends_with(".xhtml")).count(), 5, "one per document plus nav");

        let mut opf = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("OEBPS/content.opf").unwrap(), &mut opf).unwrap();
        assert!(opf.contains("<dc:title>Book</dc:title>"));
    }

    #[test]
    fn docx_contains_the_parts_word_requires_and_the_prose() {
        let bytes = to_docx(&sample(), &CompilePreset::default()).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
        for required in ["[Content_Types].xml", "_rels/.rels", "word/document.xml"] {
            assert!(names.iter().any(|n| n == required), "missing {required} in {names:?}");
        }
        let mut xml = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("word/document.xml").unwrap(), &mut xml).unwrap();
        assert!(xml.contains("The well at Kharsa was going dry."));
        assert!(xml.contains("Heading1"), "a part heading maps to a Word heading style");
    }

    #[test]
    fn compiling_an_empty_set_is_an_error_not_an_empty_file() {
        for fmt in [ExportFormat::Markdown, ExportFormat::Epub, ExportFormat::Docx, ExportFormat::Html] {
            assert!(render(&[], &CompilePreset::default(), fmt).is_err(), "{fmt:?} must refuse an empty set");
        }
    }

    #[test]
    fn every_format_renders_and_contains_the_prose() {
        for fmt in [
            ExportFormat::Markdown,
            ExportFormat::Html,
            ExportFormat::Pdf,
            ExportFormat::Fountain,
            ExportFormat::Epub,
            ExportFormat::Docx,
        ] {
            let out = render(&sample(), &CompilePreset::default(), fmt).unwrap();
            assert!(!out.is_empty(), "{fmt:?} produced nothing");
            if matches!(fmt, ExportFormat::Epub | ExportFormat::Docx) {
                assert_eq!(&out[0..2], b"PK", "{fmt:?} must be a zip archive");
            } else {
                let text = String::from_utf8(out).unwrap();
                assert!(text.contains("Kharsa"), "{fmt:?} lost the prose");
            }
        }
    }
}
