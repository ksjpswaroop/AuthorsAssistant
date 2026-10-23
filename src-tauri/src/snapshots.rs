//! Snapshots, backed by a real Git repository inside the project folder.
//!
//! A snapshot is a commit. That choice matters: if AegisDraft is gone, the
//! author's history is still readable with any Git client, which is the same
//! promise the Markdown files make.
//!
//! The repository is the author's safety net, not a collaboration tool — there
//! is no remote, no push, and nothing here ever touches the network.

use crate::error::{err, AppResult};
use crate::model::Snapshot;
use git2::{Repository, Signature, Status, StatusOptions};
use std::path::Path;

const IGNORE: &str = "# AegisDraft keeps snapshots in this repository.\n\
                      # SQLite side files are transient and must not be committed.\n\
                      *.sqlite-wal\n\
                      *.sqlite-shm\n\
                      *.md.tmp\n\
                      trash/\n";

fn signature() -> AppResult<Signature<'static>> {
    Signature::now("AegisDraft", "snapshots@aegisdraft.local")
        .map_err(|e| crate::error::AppError(format!("could not sign the snapshot: {e}")))
}

fn open_or_init(root: &Path) -> AppResult<Repository> {
    match Repository::open(root) {
        Ok(repo) => Ok(repo),
        Err(_) => {
            let repo = Repository::init(root)
                .map_err(|e| crate::error::AppError(format!("could not start the snapshot history: {e}")))?;
            std::fs::write(root.join(".gitignore"), IGNORE)?;
            Ok(repo)
        }
    }
}

/// True when anything has changed since the last snapshot — so the UI can say
/// "nothing to snapshot" instead of making an empty commit.
pub fn is_dirty(root: &Path) -> AppResult<bool> {
    let repo = match Repository::open(root) {
        Ok(r) => r,
        Err(_) => return Ok(true), // no history yet; the first snapshot is worth taking
    };
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo
        .statuses(Some(&mut opts))
        .map_err(|e| crate::error::AppError(format!("could not read the project state: {e}")))?;
    Ok(statuses.iter().any(|s| s.status() != Status::CURRENT))
}

/// Commits everything in the project folder. Returns `None` when there was
/// nothing to commit, which is not an error.
pub fn create(root: &Path, message: &str) -> AppResult<Option<Snapshot>> {
    let repo = open_or_init(root)?;
    let sig = signature()?;

    let mut index = repo
        .index()
        .map_err(|e| crate::error::AppError(format!("could not stage the project: {e}")))?;
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .map_err(|e| crate::error::AppError(format!("could not stage the project: {e}")))?;
    index.write().map_err(|e| crate::error::AppError(format!("could not stage the project: {e}")))?;

    let tree_id = index
        .write_tree()
        .map_err(|e| crate::error::AppError(format!("could not build the snapshot: {e}")))?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|e| crate::error::AppError(format!("could not build the snapshot: {e}")))?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());

    // An identical tree means nothing changed; do not litter the history.
    if let Some(p) = &parent {
        if p.tree_id() == tree_id {
            return Ok(None);
        }
    }

    let text = if message.trim().is_empty() { "Snapshot" } else { message.trim() };
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, text, &tree, &parents)
        .map_err(|e| crate::error::AppError(format!("could not save the snapshot: {e}")))?;

    Ok(Some(Snapshot {
        id: oid.to_string(),
        message: text.to_string(),
        at: sig.when().seconds() * 1000,
        files: Vec::new(),
    }))
}

/// Newest first. `only_path` narrows the list to snapshots that touched one file.
pub fn list(root: &Path, only_path: Option<&str>, limit: usize) -> AppResult<Vec<Snapshot>> {
    let repo = match Repository::open(root) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };
    let mut walk = repo
        .revwalk()
        .map_err(|e| crate::error::AppError(format!("could not read the history: {e}")))?;
    if walk.push_head().is_err() {
        return Ok(Vec::new()); // repository exists but has no commits yet
    }
    walk.set_sorting(git2::Sort::TIME).ok();

    let mut out = Vec::new();
    for oid in walk.flatten() {
        if out.len() >= limit {
            break;
        }
        let Ok(commit) = repo.find_commit(oid) else { continue };
        let Ok(tree) = commit.tree() else { continue };
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let Ok(diff) = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) else { continue };

        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(p) = delta.new_file().path().or_else(|| delta.old_file().path()) {
                    files.push(p.to_string_lossy().to_string());
                }
                true
            },
            None,
            None,
            None,
        )
        .ok();

        if let Some(want) = only_path {
            if !files.iter().any(|f| f == want) {
                continue;
            }
        }

        out.push(Snapshot {
            id: oid.to_string(),
            message: commit.summary().ok().flatten().unwrap_or("Snapshot").to_string(),
            at: commit.time().seconds() * 1000,
            files,
        });
    }
    Ok(out)
}

/// The contents of one file as of a snapshot, for side-by-side compare.
pub fn read_file_at(root: &Path, commit_id: &str, rel_path: &str) -> AppResult<String> {
    let repo = Repository::open(root)
        .map_err(|e| crate::error::AppError(format!("no snapshot history yet: {e}")))?;
    let oid = git2::Oid::from_str(commit_id)
        .map_err(|_| crate::error::AppError("that snapshot id is not valid".into()))?;
    let commit = repo
        .find_commit(oid)
        .map_err(|e| crate::error::AppError(format!("no such snapshot: {e}")))?;
    let tree = commit
        .tree()
        .map_err(|e| crate::error::AppError(format!("could not read the snapshot: {e}")))?;
    let entry = match tree.get_path(Path::new(rel_path)) {
        Ok(e) => e,
        Err(_) => return Ok(String::new()), // the file did not exist yet
    };
    let obj = entry
        .to_object(&repo)
        .map_err(|e| crate::error::AppError(format!("could not read the snapshot: {e}")))?;
    let blob = match obj.as_blob() {
        Some(b) => b,
        None => return err("that path is a folder in the snapshot"),
    };
    Ok(String::from_utf8_lossy(blob.content()).to_string())
}

/// Restores one file from a snapshot, after committing whatever is on disk so
/// the restore itself can be undone. Never a silent overwrite.
pub fn restore_file(root: &Path, commit_id: &str, rel_path: &str) -> AppResult<()> {
    let before = read_file_at(root, commit_id, rel_path)?;
    let target = root.join(rel_path);
    if !target.starts_with(root) {
        return err("refusing to write outside the project folder");
    }
    // Preserve the current text as its own snapshot first.
    create(root, &format!("Before restoring {rel_path}"))?;
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = target.with_extension("md.tmp");
    std::fs::write(&tmp, before)?;
    std::fs::rename(&tmp, &target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshots_round_trip_and_skip_no_op_commits() {
        let root = std::env::temp_dir().join(format!(
            "aegis-snap-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("manuscript/scenes")).unwrap();
        let rel = "manuscript/scenes/0001-a.md";
        std::fs::write(root.join(rel), "first draft").unwrap();

        let first = create(&root, "First").unwrap().expect("a first snapshot");
        assert!(!first.id.is_empty());

        // Nothing changed, so no second commit.
        assert!(create(&root, "Again").unwrap().is_none());
        assert!(!is_dirty(&root).unwrap());

        std::fs::write(root.join(rel), "second draft").unwrap();
        assert!(is_dirty(&root).unwrap());
        create(&root, "Second").unwrap().expect("a second snapshot");

        assert_eq!(read_file_at(&root, &first.id, rel).unwrap(), "first draft");
        assert_eq!(list(&root, Some(rel), 10).unwrap().len(), 2);

        let second = list(&root, Some(rel), 10).unwrap()[0].id.clone();
        restore_file(&root, &first.id, rel).unwrap();
        assert_eq!(std::fs::read_to_string(root.join(rel)).unwrap(), "first draft");
        // Restoring did not destroy the newer text — it is still in history.
        assert_eq!(read_file_at(&root, &second, rel).unwrap(), "second draft");

        std::fs::remove_dir_all(&root).ok();
    }

    /// Snapshotting a real project: the SQLite side files must stay out of the
    /// commit, or every snapshot carries transient bytes and diffs are noise.
    #[test]
    fn snapshotting_a_real_project_ignores_sqlite_side_files() {
        let parent = std::env::temp_dir().join(format!(
            "aegis-snap-proj-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&parent).unwrap();
        let summary = crate::project::create(&parent, "Snap Book", "novel").unwrap();
        let root = std::path::PathBuf::from(&summary.path);

        let snap = create(&root, "First").unwrap().expect("a first snapshot");
        let listed = list(&root, None, 10).unwrap();
        assert_eq!(listed.len(), 1);

        let files = &listed[0].files;
        assert!(
            files.iter().any(|f| f == "project.sqlite"),
            "structure must be captured, got {files:?}"
        );
        assert!(
            !files.iter().any(|f| f.ends_with("-wal") || f.ends_with("-shm")),
            "SQLite side files must be ignored, got {files:?}"
        );
        assert!(files.iter().any(|f| f.starts_with("manuscript/scenes/")));
        assert!(!snap.id.is_empty());

        std::fs::remove_dir_all(&parent).ok();
    }

    /// The case restore has to get right: text typed but never snapshotted must
    /// survive being restored over, or an undo silently eats the last hour.
    #[test]
    fn restoring_preserves_unsnapshotted_work() {
        let root = std::env::temp_dir().join(format!(
            "aegis-snap-dirty-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("manuscript/scenes")).unwrap();
        let rel = "manuscript/scenes/0001-a.md";

        std::fs::write(root.join(rel), "committed text").unwrap();
        let base = create(&root, "Base").unwrap().expect("a base snapshot");

        // Typed, never snapshotted.
        std::fs::write(root.join(rel), "unsaved afternoon of work").unwrap();

        restore_file(&root, &base.id, rel).unwrap();
        assert_eq!(std::fs::read_to_string(root.join(rel)).unwrap(), "committed text");

        // The unsnapshotted text was committed before being overwritten.
        let history = list(&root, Some(rel), 10).unwrap();
        let recovered = history
            .iter()
            .any(|snap| read_file_at(&root, &snap.id, rel).unwrap() == "unsaved afternoon of work");
        assert!(recovered, "restore must snapshot uncommitted work before overwriting it");

        std::fs::remove_dir_all(&root).ok();
    }
}
