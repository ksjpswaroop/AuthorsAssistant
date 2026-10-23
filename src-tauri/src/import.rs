//! Import a folder of Markdown into the binder.
//!
//! The promise AegisDraft makes is that a manuscript is a folder of readable
//! files you own. That promise is worth little in one direction only, so this
//! is the way in: point at a folder of `.md` files and the binder mirrors it —
//! sub-folders become binder folders, files become documents, in the order the
//! folder already reads in.
//!
//! What it deliberately will not do:
//!
//! - **Overwrite anything.** Every imported file is created through
//!   `create_document`, which allocates its own sequence-numbered filename, so
//!   an import can never land on top of prose already in the project.
//! - **Swallow another project.** A folder holding `project.sqlite` is someone
//!   else's AegisDraft project; importing it would copy the prose out of its own
//!   history and leave two divergent copies. It is refused, with the reason.
//! - **Run away.** A mis-picked folder (a home directory, a repository) is the
//!   expensive mistake here, so depth and file count are capped and the import
//!   refuses up front rather than filling the binder with thousands of rows.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{err, AppResult};
use crate::model::DocKind;
use crate::project;

/// Deep enough for any real manuscript, shallow enough that a stray symlink or
/// a source tree cannot spiral.
const MAX_DEPTH: usize = 12;
/// A long novel is a few hundred files. Anything past this is a wrong folder.
const MAX_FILES: usize = 2_000;

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    /// Binder folders created, not counting the top-level one.
    pub folders: i64,
    pub documents: i64,
    pub words: i64,
    /// Everything passed over, each with the reason, so an import never
    /// silently loses a file. Capped so a huge folder cannot flood the UI.
    pub skipped: Vec<String>,
    /// The binder id of the folder the import landed in.
    pub root_id: String,
}

impl ImportReport {
    fn skip(&mut self, path: &Path, why: &str) {
        if self.skipped.len() < 200 {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            self.skipped.push(format!("{name} — {why}"));
        }
    }
}

/// A readable title for an imported file.
///
/// A Markdown file usually names itself in its first heading, and that is a
/// title a person wrote. The filename is the fallback, cleaned of the sequence
/// prefix and separators a shell needs but a binder does not.
pub fn title_for(stem: &str, body: &str) -> String {
    let mut lines = body.lines().peekable();
    // Step over YAML front matter if the file opens with it.
    if lines.peek().map(|l| l.trim()) == Some("---") {
        lines.next();
        for line in lines.by_ref() {
            if line.trim() == "---" {
                break;
            }
        }
    }
    for line in lines.take(20) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("# ") {
            let heading = rest.trim();
            if !heading.is_empty() {
                return heading.chars().take(120).collect();
            }
        }
        // The first non-blank line was prose, not a heading; stop looking.
        break;
    }
    humanise(stem)
}

/// `0007-nightfall-at-the-rim` → `Nightfall At The Rim`.
fn humanise(stem: &str) -> String {
    let without_sequence = stem.trim_start_matches(|c: char| c.is_ascii_digit());
    let core = without_sequence
        .trim_start_matches(['-', '_', '.', ' '])
        .trim();
    let core = if core.is_empty() { stem } else { core };

    let mut out = String::new();
    for word in core.split(['-', '_', ' ']).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push(' ');
        }
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    if out.is_empty() {
        "Untitled".to_string()
    } else {
        out
    }
}

/// Count what an import would bring in, and refuse a folder that should not be
/// imported at all. Separate from `run` so the UI can show the size of the
/// thing before the author commits to it.
pub fn survey(source: &Path) -> AppResult<(usize, usize)> {
    if !source.is_dir() {
        return err(format!("{} is not a folder", source.display()));
    }
    if source.join("project.sqlite").exists() {
        return err(format!(
            "{} is already an AegisDraft project — open it instead of importing it",
            source.display()
        ));
    }
    let mut files = 0usize;
    let mut folders = 0usize;
    walk(source, 0, &mut |_path, is_dir| {
        if is_dir {
            folders += 1;
        } else {
            files += 1;
        }
    })?;
    if files == 0 {
        return err(format!("{} holds no .md files", source.display()));
    }
    if files > MAX_FILES {
        return err(format!(
            "{files} Markdown files is more than an import should carry (limit {MAX_FILES}) — pick a narrower folder"
        ));
    }
    Ok((folders, files))
}

/// Visit every importable entry under `dir`, deepest-last, sorted by name.
fn walk(dir: &Path, depth: usize, visit: &mut impl FnMut(&Path, bool)) -> AppResult<()> {
    if depth >= MAX_DEPTH {
        return Ok(());
    }
    for entry in sorted_children(dir)? {
        let path = entry.as_path();
        // `symlink_metadata` rather than `is_dir`, so a symlink pointing back up
        // the tree is a skipped entry rather than an infinite descent.
        let Ok(meta) = fs::symlink_metadata(path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            visit(path, true);
            walk(path, depth + 1, visit)?;
        } else if is_markdown(path) {
            visit(path, false);
        }
    }
    Ok(())
}

fn sorted_children(dir: &Path) -> AppResult<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            // Hidden entries are tooling, not manuscript: `.git`, `.DS_Store`,
            // `.obsidian`. None of them belong in a binder.
            !p.file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(true)
        })
        .collect();
    out.sort();
    Ok(out)
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .map(|e| {
            let e = e.to_string_lossy().to_lowercase();
            e == "md" || e == "markdown" || e == "txt"
        })
        .unwrap_or(false)
}

/// Import `source` into `root` as a folder under `parent_id`.
pub fn run(
    root: &Path,
    source: &Path,
    parent_id: Option<&str>,
    kind: DocKind,
) -> AppResult<ImportReport> {
    survey(source)?;

    let root_real = fs::canonicalize(root)?;
    let source_real = fs::canonicalize(source)?;
    if source_real.starts_with(&root_real) || root_real.starts_with(&source_real) {
        return err(
            "that folder is inside the project (or holds it) — importing it would copy the project into itself"
                .to_string(),
        );
    }

    let mut report = ImportReport::default();
    let name = source_real
        .file_name()
        .map(|n| humanise(&n.to_string_lossy()))
        .unwrap_or_else(|| "Imported".to_string());
    let holder = project::create_document(root, parent_id, &name, DocKind::Folder)?;
    report.root_id = holder.id.clone();

    descend(root, &source_real, &holder.id, kind, 0, &mut report)?;
    Ok(report)
}

fn descend(
    root: &Path,
    dir: &Path,
    parent_id: &str,
    kind: DocKind,
    depth: usize,
    report: &mut ImportReport,
) -> AppResult<()> {
    if depth >= MAX_DEPTH {
        report.skip(dir, "nested deeper than an import will follow");
        return Ok(());
    }

    for path in sorted_children(dir)? {
        let Ok(meta) = fs::symlink_metadata(&path) else {
            report.skip(&path, "unreadable");
            continue;
        };
        if meta.file_type().is_symlink() {
            report.skip(&path, "a link, not a file");
            continue;
        }

        if meta.is_dir() {
            let title = humanise(&path.file_name().unwrap_or_default().to_string_lossy());
            let folder = project::create_document(root, Some(parent_id), &title, DocKind::Folder)?;
            report.folders += 1;
            descend(root, &path, &folder.id, kind, depth + 1, report)?;
            continue;
        }

        if !is_markdown(&path) {
            report.skip(&path, "not Markdown");
            continue;
        }
        let body = match fs::read_to_string(&path) {
            Ok(text) => text,
            // A binary or non-UTF-8 file named `.md` is a real thing to hit;
            // it is skipped by name rather than crashing the whole import.
            Err(_) => {
                report.skip(&path, "not readable as text");
                continue;
            }
        };
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let title = title_for(&stem, &body);

        // `create_document` allocates a fresh sequence-numbered file, so this
        // never lands on prose that is already in the project.
        let doc = project::create_document(root, Some(parent_id), &title, kind)?;
        // Imported prose is external material the author selected, not their
        // own writing. Recording it as theirs would misattribute a quotation.
        let receipt = project::write_body_as(
            root,
            &doc.id,
            &body,
            crate::provenance::Actor::tool("import"),
        )?;
        report.documents += 1;
        report.words += receipt.word_count;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "aegis-import-{label}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn source_tree(base: &Path) -> PathBuf {
        let src = base.join("Old Manuscript");
        fs::create_dir_all(src.join("02-part-two")).unwrap();
        fs::create_dir_all(src.join(".git")).unwrap();
        fs::write(
            src.join("01-opening.md"),
            "# The Rim at Dusk\n\nSalt on the wind, and the pumps still running.\n",
        )
        .unwrap();
        fs::write(
            src.join("02-part-two/first-light.md"),
            "No heading here, just prose that runs on for a few words.\n",
        )
        .unwrap();
        fs::write(src.join("notes.txt"), "A stray note.\n").unwrap();
        fs::write(src.join("cover.png"), [0x89, 0x50, 0x4e, 0x47]).unwrap();
        fs::write(src.join(".git/config"), "[core]\n").unwrap();
        src
    }

    #[test]
    fn a_folder_of_markdown_becomes_a_binder_that_mirrors_it() {
        let base = scratch("mirror");
        let src = source_tree(&base);
        let projects = base.join("projects");
        fs::create_dir_all(&projects).unwrap();
        let summary = project::create(&projects, "Receiver", "novel").unwrap();
        let root = Path::new(&summary.path);

        let report = run(root, &src, None, DocKind::Scene).unwrap();

        assert_eq!(report.folders, 1, "one sub-folder, mirrored");
        assert_eq!(report.documents, 3, "two .md files and one .txt");
        assert!(report.words > 0);
        assert!(
            report.skipped.iter().any(|s| s.contains("cover.png")),
            "a skipped file must be named, not silently dropped: {:?}",
            report.skipped
        );
        assert!(
            !report.skipped.iter().any(|s| s.contains(".git")),
            "hidden tooling is not manuscript and is not reported as a loss"
        );

        let docs = project::documents(root).unwrap();
        // The heading inside the file wins over the filename.
        assert!(
            docs.iter().any(|d| d.title == "The Rim at Dusk"),
            "titles: {:?}",
            docs.iter().map(|d| &d.title).collect::<Vec<_>>()
        );
        // No heading, so the filename is cleaned up instead.
        assert!(docs.iter().any(|d| d.title == "First Light"));
        assert!(docs.iter().any(|d| d.title == "Old Manuscript"));
        assert!(docs.iter().any(|d| d.title == "Part Two"));

        // The prose actually arrived, not just the row.
        let opening = docs.iter().find(|d| d.title == "The Rim at Dusk").unwrap();
        let body = project::read_body(root, &opening.id).unwrap();
        assert!(body.contains("the pumps still running"));

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn importing_twice_adds_a_second_copy_and_overwrites_nothing() {
        let base = scratch("twice");
        let src = source_tree(&base);
        let projects = base.join("projects");
        fs::create_dir_all(&projects).unwrap();
        let summary = project::create(&projects, "Receiver", "novel").unwrap();
        let root = Path::new(&summary.path);

        let files = |root: &Path| -> Vec<String> {
            project::documents(root)
                .unwrap()
                .iter()
                .filter_map(|d| d.path.clone())
                .collect()
        };

        // The template seeds its own empty scene, so compare against what the
        // first import actually added rather than everything on disk.
        let seeded = files(root);
        run(root, &src, None, DocKind::Scene).unwrap();
        let first = files(root);
        let imported: Vec<String> = first
            .iter()
            .filter(|p| !seeded.contains(p))
            .cloned()
            .collect();
        assert_eq!(imported.len(), 3);
        let before: Vec<String> = imported
            .iter()
            .map(|p| fs::read_to_string(root.join(p)).unwrap())
            .collect();

        run(root, &src, None, DocKind::Scene).unwrap();
        let after = files(root);

        for (path, text) in imported.iter().zip(&before) {
            assert!(after.contains(path), "an existing file was displaced: {path}");
            assert_eq!(
                &fs::read_to_string(root.join(path)).unwrap(),
                text,
                "{path} was rewritten by the second import"
            );
        }
        assert_eq!(
            after.len(),
            first.len() + imported.len(),
            "the second import is additive"
        );

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn another_project_is_refused_rather_than_swallowed() {
        let base = scratch("nested");
        let projects = base.join("projects");
        fs::create_dir_all(&projects).unwrap();
        let one = project::create(&projects, "One", "novel").unwrap();
        let two = project::create(&projects, "Two", "novel").unwrap();

        let refused = run(
            Path::new(&one.path),
            Path::new(&two.path),
            None,
            DocKind::Scene,
        );
        let message = refused.unwrap_err().to_string();
        assert!(
            message.contains("already an AegisDraft project"),
            "a project must be opened, not imported: {message}"
        );

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn the_project_cannot_import_itself() {
        let base = scratch("self");
        let projects = base.join("projects");
        fs::create_dir_all(&projects).unwrap();
        let summary = project::create(&projects, "Ouroboros", "novel").unwrap();
        let root = Path::new(&summary.path);
        fs::write(root.join("manuscript/scenes/loose.md"), "# Loose\n\nWords.\n").unwrap();

        let refused = run(root, &root.join("manuscript"), None, DocKind::Scene);
        let message = refused.unwrap_err().to_string();
        assert!(
            message.contains("inside the project"),
            "importing a project into itself must be refused: {message}"
        );

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn an_empty_folder_says_so_instead_of_making_an_empty_binder_folder() {
        let base = scratch("empty");
        let src = base.join("nothing");
        fs::create_dir_all(&src).unwrap();
        let message = survey(&src).unwrap_err().to_string();
        assert!(message.contains("no .md files"), "{message}");
        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn titles_come_from_the_heading_then_the_filename() {
        assert_eq!(title_for("0007-x", "# Nightfall\n\nprose"), "Nightfall");
        assert_eq!(
            title_for("0007-x", "---\ntitle: y\n---\n\n# Nightfall\n"),
            "Nightfall"
        );
        // Prose before any heading: the heading is the file's, not a later one.
        assert_eq!(
            title_for("0007-nightfall-at-the-rim", "Prose first.\n\n# Later\n"),
            "Nightfall At The Rim"
        );
        assert_eq!(title_for("0007", ""), "0007");
        assert_eq!(title_for("---", ""), "Untitled");
    }
}
