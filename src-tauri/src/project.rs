//! Project folders on disk. A project is a directory, not a binary file:
//! `MyBook.aegis/project.sqlite` plus Markdown under `manuscript/` and
//! `research/`. If this app disappears, the prose is still readable.

use crate::error::{err, AppResult};
use crate::model::{DocKind, DocumentRow, ProjectSummary, RecentProject, SaveReceipt};
use crate::store;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROJECT_EXT: &str = "aegis";
const SUBDIRS: [&str; 5] = [
    "manuscript/scenes",
    "manuscript/characters",
    "manuscript/worldbuilding",
    "research",
    "assets",
];

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn count_words(text: &str) -> i64 {
    text.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count() as i64
}

/// Lowercase, hyphenated, ASCII-only. Used for file names, never for identity.
pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.chars().take(60).collect()
    }
}

fn validate_root(root: &Path) -> AppResult<()> {
    if !root.is_dir() {
        return err(format!("{} is not a folder", root.display()));
    }
    if !root.join("project.sqlite").exists() {
        return err(format!(
            "{} does not look like an AegisDraft project (no project.sqlite)",
            root.display()
        ));
    }
    Ok(())
}

fn project_name(root: &Path) -> String {
    root.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

fn summarise(root: &Path) -> AppResult<ProjectSummary> {
    let conn = store::open(root)?;
    let (word_count, document_count) = store::totals(&conn)?;
    let kind = store::get_meta(&conn, "project_kind")?.unwrap_or_else(|| "novel".to_string());
    Ok(ProjectSummary {
        path: root.to_string_lossy().to_string(),
        name: project_name(root),
        kind,
        word_count,
        document_count,
        opened_at: now_ms(),
    })
}

/// Creates `<parent>/<name>.aegis`, seeds the folder tree, the database and one
/// empty opening scene so the binder is never empty on first run.
pub fn create(parent: &Path, name: &str, kind: &str) -> AppResult<ProjectSummary> {
    if !parent.is_dir() {
        return err(format!("{} is not a folder", parent.display()));
    }
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return err("A project needs a name");
    }
    let root = parent.join(format!("{}.{PROJECT_EXT}", trimmed));
    if root.exists() {
        return err(format!("{} already exists", root.display()));
    }

    fs::create_dir_all(&root)?;
    for dir in SUBDIRS {
        fs::create_dir_all(root.join(dir))?;
    }

    let conn = store::open(&root)?;
    let now = now_ms();
    store::set_meta(&conn, "project_kind", kind)?;
    store::set_meta(&conn, "created_at", &now.to_string())?;

    let manuscript_id = new_id();
    store::insert_document(&conn, &manuscript_id, None, "Manuscript", DocKind::Folder, None, 0, now)?;

    let scene_id = new_id();
    let rel = format!("{}/0001-opening-scene.md", DocKind::Scene.dir());
    fs::write(root.join(&rel), "")?;
    store::insert_document(
        &conn,
        &scene_id,
        Some(&manuscript_id),
        "Opening scene",
        DocKind::Scene,
        Some(&rel),
        0,
        now,
    )?;

    for (title, pos) in [("Characters", 1i64), ("Worldbuilding", 2), ("Research", 3)] {
        store::insert_document(&conn, &new_id(), None, title, DocKind::Folder, None, pos, now)?;
    }

    summarise(&root)
}

pub fn open(root: &Path) -> AppResult<ProjectSummary> {
    validate_root(root)?;
    summarise(root)
}

pub fn documents(root: &Path) -> AppResult<Vec<DocumentRow>> {
    validate_root(root)?;
    let conn = store::open(root)?;
    store::list_documents(&conn)
}

pub fn read_body(root: &Path, id: &str) -> AppResult<String> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let doc = match store::get_document(&conn, id)? {
        Some(d) => d,
        None => return err(format!("no document {id}")),
    };
    let Some(rel) = doc.path else {
        return Ok(String::new()); // folders have no body
    };
    let file = root.join(&rel);
    if !file.exists() {
        // The row outlived its file. Report empty rather than failing the open;
        // the next save re-creates the file.
        return Ok(String::new());
    }
    Ok(fs::read_to_string(file)?)
}

/// Writes the Markdown body, then records the new word count. The file is
/// written first and atomically, so a crash mid-save can never leave the prose
/// shorter than what the database claims.
pub fn write_body(root: &Path, id: &str, body: &str) -> AppResult<SaveReceipt> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let doc = match store::get_document(&conn, id)? {
        Some(d) => d,
        None => return err(format!("no document {id}")),
    };
    let Some(rel) = doc.path else {
        return err("folders have no body to save");
    };

    let file = root.join(&rel);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("md.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, &file)?;

    let words = count_words(body);
    let now = now_ms();
    store::touch_body(&conn, id, words, now)?;
    Ok(SaveReceipt { id: id.to_string(), word_count: words, updated_at: now })
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn create_document(
    root: &Path,
    parent_id: Option<&str>,
    title: &str,
    kind: DocKind,
) -> AppResult<DocumentRow> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let now = now_ms();
    let position = store::next_position(&conn, parent_id)?;
    let id = new_id();

    let path = if kind == DocKind::Folder {
        None
    } else {
        let rel = unique_path(root, kind, title)?;
        fs::create_dir_all(root.join(kind.dir()))?;
        fs::write(root.join(&rel), "")?;
        Some(rel)
    };

    store::insert_document(&conn, &id, parent_id, title, kind, path.as_deref(), position, now)?;
    match store::get_document(&conn, &id)? {
        Some(d) => Ok(d),
        None => err("document vanished immediately after being created"),
    }
}

/// `manuscript/scenes/0007-nightfall-at-the-rim.md`.
///
/// The counter is the next free sequence number in that directory, not just a
/// collision breaker — it is what makes the folder read in creation order when
/// someone opens it in Finder or a text editor, which is the whole point of
/// keeping the prose as files.
fn unique_path(root: &Path, kind: DocKind, title: &str) -> AppResult<String> {
    let dir = kind.dir();
    let slug = slugify(title);
    let dir_path = root.join(dir);

    let existing = fs::read_dir(&dir_path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
                .count()
        })
        .unwrap_or(0);

    for seq in (existing + 1..).take(10_000) {
        let rel = format!("{dir}/{seq:04}-{slug}.md");
        if !root.join(&rel).exists() {
            return Ok(rel);
        }
    }
    err("could not find a free file name")
}

pub fn rename_document(root: &Path, id: &str, title: &str) -> AppResult<DocumentRow> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let doc = match store::get_document(&conn, id)? {
        Some(d) => d,
        None => return err(format!("no document {id}")),
    };
    let now = now_ms();

    // Keep the file name in step with the title so the folder stays readable.
    // If the rename fails for any reason, the old path is kept — the title is
    // what the author sees, and losing the file would be worse.
    let mut new_rel: Option<String> = None;
    if let Some(old_rel) = doc.path.as_deref() {
        if let Ok(candidate) = unique_path(root, doc.kind, title) {
            if fs::rename(root.join(old_rel), root.join(&candidate)).is_ok() {
                new_rel = Some(candidate);
            }
        }
    }

    store::rename(&conn, id, title, new_rel.as_deref(), now)?;
    match store::get_document(&conn, id)? {
        Some(d) => Ok(d),
        None => err("document vanished during rename"),
    }
}

pub fn update_meta(
    root: &Path,
    id: &str,
    synopsis: Option<&str>,
    label: Option<&str>,
    status: Option<&str>,
) -> AppResult<DocumentRow> {
    validate_root(root)?;
    let conn = store::open(root)?;
    store::update_meta_fields(&conn, id, synopsis, label, status, now_ms())?;
    match store::get_document(&conn, id)? {
        Some(d) => Ok(d),
        None => err(format!("no document {id}")),
    }
}

/// Removes the row and moves its files to `trash/` rather than unlinking them.
/// Phase 1 has no snapshots yet, so deletion must not be final — and deleting a
/// folder must not orphan the files of everything inside it.
pub fn delete_document(root: &Path, id: &str) -> AppResult<()> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let doomed = store::subtree(&conn, id)?;

    let trash = root.join("trash");
    let stamp = now_ms();
    for doc in &doomed {
        let Some(rel) = doc.path.as_deref() else { continue };
        let src = root.join(rel);
        if !src.exists() {
            continue;
        }
        fs::create_dir_all(&trash)?;
        let name = Path::new(rel).file_name().unwrap_or_default();
        let dest = trash.join(format!("{}-{}", stamp, name.to_string_lossy()));
        let _ = fs::rename(&src, &dest);
    }

    // The SQL cascade removes the descendant rows.
    store::delete(&conn, id)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Recent projects — a small JSON file in the app config directory.
// ---------------------------------------------------------------------------

pub fn read_recents(config_dir: &Path) -> Vec<RecentProject> {
    let file = config_dir.join("recent-projects.json");
    let Ok(raw) = fs::read_to_string(file) else {
        return Vec::new();
    };
    let mut list: Vec<RecentProject> = serde_json::from_str(&raw).unwrap_or_default();
    for item in &mut list {
        item.exists = Path::new(&item.path).join("project.sqlite").exists();
    }
    list
}

pub fn remember(config_dir: &Path, summary: &ProjectSummary) -> AppResult<Vec<RecentProject>> {
    fs::create_dir_all(config_dir)?;
    let mut list = read_recents(config_dir);
    list.retain(|r| r.path != summary.path);
    list.insert(
        0,
        RecentProject {
            path: summary.path.clone(),
            name: summary.name.clone(),
            kind: summary.kind.clone(),
            word_count: summary.word_count,
            opened_at: now_ms(),
            exists: true,
        },
    );
    list.truncate(12);
    fs::write(
        config_dir.join("recent-projects.json"),
        serde_json::to_string_pretty(&list)?,
    )?;
    Ok(list)
}

pub fn to_path(raw: &str) -> PathBuf {
    PathBuf::from(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_is_filename_safe() {
        assert_eq!(slugify("4. Nightfall at the Rim"), "4-nightfall-at-the-rim");
        assert_eq!(slugify("  ---  "), "untitled");
        assert_eq!(slugify("Salt & Debt"), "salt-debt");
    }

    #[test]
    fn word_count_ignores_punctuation_runs() {
        assert_eq!(count_words("one two three"), 3);
        assert_eq!(count_words("one — two"), 2);
        assert_eq!(count_words("   "), 0);
    }

    #[test]
    fn scene_files_are_numbered_in_creation_order() {
        let tmp = std::env::temp_dir().join(format!("aegis-seq-{}", now_ms()));
        fs::create_dir_all(&tmp).unwrap();
        let summary = create(&tmp, "Sequence Test", "novel").unwrap();
        let root = PathBuf::from(&summary.path);

        // create() already seeded 0001-opening-scene.md.
        let a = create_document(&root, None, "Second scene", DocKind::Scene).unwrap();
        let b = create_document(&root, None, "Third scene", DocKind::Scene).unwrap();

        assert_eq!(a.path.as_deref(), Some("manuscript/scenes/0002-second-scene.md"));
        assert_eq!(b.path.as_deref(), Some("manuscript/scenes/0003-third-scene.md"));

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn deleting_a_folder_trashes_the_files_inside_it() {
        let tmp = std::env::temp_dir().join(format!("aegis-del-{}", now_ms()));
        fs::create_dir_all(&tmp).unwrap();
        let summary = create(&tmp, "Delete Test", "novel").unwrap();
        let root = PathBuf::from(&summary.path);

        let folder = create_document(&root, None, "Act I", DocKind::Folder).unwrap();
        let child = create_document(&root, Some(&folder.id), "A scene", DocKind::Scene).unwrap();
        write_body(&root, &child.id, "words on a page").unwrap();

        let child_file = root.join(child.path.as_ref().unwrap());
        assert!(child_file.exists());

        delete_document(&root, &folder.id).unwrap();

        // Row gone, file moved rather than destroyed.
        assert!(documents(&root).unwrap().iter().all(|d| d.id != child.id));
        assert!(!child_file.exists(), "the child file should have left its original path");
        let trashed: Vec<_> = fs::read_dir(root.join("trash")).unwrap().filter_map(Result::ok).collect();
        assert_eq!(trashed.len(), 1, "the child file should be in trash");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn create_open_write_read_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("aegis-test-{}", now_ms()));
        fs::create_dir_all(&tmp).unwrap();

        let summary = create(&tmp, "Test Book", "novel").unwrap();
        assert_eq!(summary.name, "Test Book");

        let root = PathBuf::from(&summary.path);
        let docs = documents(&root).unwrap();
        let scene = docs.iter().find(|d| d.kind == DocKind::Scene).unwrap();

        let receipt = write_body(&root, &scene.id, "The rim of the basin held the heat.").unwrap();
        assert_eq!(receipt.word_count, 8);
        assert_eq!(read_body(&root, &scene.id).unwrap(), "The rim of the basin held the heat.");

        // The prose must be readable without going through the database.
        let on_disk = fs::read_to_string(root.join(scene.path.as_ref().unwrap())).unwrap();
        assert_eq!(on_disk, "The rim of the basin held the heat.");

        fs::remove_dir_all(&tmp).ok();
    }
}
