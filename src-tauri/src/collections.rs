//! Collections (Phase 6): virtual sets over binder ids.
//!
//! A collection never copies or moves a file. A *manual* collection is a hand
//! ordered list of ids; a *smart* collection is a stored query re-evaluated on
//! every read, so changing a scene's status moves it in or out without anyone
//! touching the binder.

use crate::error::{err, AppResult};
use crate::model::{Collection, CollectionKind, DocumentRow, SmartFilter};
use crate::store;
use rusqlite::{params, Connection};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS collections (
            id       TEXT PRIMARY KEY,
            name     TEXT NOT NULL,
            kind     TEXT NOT NULL,          -- manual | smart | search
            filters  TEXT,                   -- JSON for smart/search
            position INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS collection_items (
            collection_id TEXT NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
            document_id   TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            position      INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (collection_id, document_id)
        );
        ",
    )?;
    Ok(())
}

pub fn create(conn: &Connection, id: &str, name: &str, kind: CollectionKind, filters: Option<&str>, now: i64) -> AppResult<()> {
    if name.trim().is_empty() {
        return err("a collection needs a name");
    }
    if kind != CollectionKind::Manual && filters.is_none() {
        return err("a smart collection needs a query");
    }
    let pos: i64 = conn.query_row("SELECT COALESCE(MAX(position), -1) + 1 FROM collections", [], |r| r.get(0))?;
    conn.execute(
        "INSERT INTO collections (id, name, kind, filters, position, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, name.trim(), kind.as_str(), filters, pos, now],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM collections WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn add_item(conn: &Connection, collection_id: &str, document_id: &str) -> AppResult<()> {
    let kind = kind_of(conn, collection_id)?;
    if kind != CollectionKind::Manual {
        return err("a smart collection is defined by its query, not by hand");
    }
    let pos: i64 = conn.query_row(
        "SELECT COALESCE(MAX(position), -1) + 1 FROM collection_items WHERE collection_id = ?1",
        params![collection_id],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO collection_items (collection_id, document_id, position) VALUES (?1, ?2, ?3)",
        params![collection_id, document_id, pos],
    )?;
    Ok(())
}

pub fn remove_item(conn: &Connection, collection_id: &str, document_id: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM collection_items WHERE collection_id = ?1 AND document_id = ?2",
        params![collection_id, document_id],
    )?;
    Ok(())
}

fn kind_of(conn: &Connection, id: &str) -> AppResult<CollectionKind> {
    let s: String = conn
        .query_row("SELECT kind FROM collections WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|_| crate::error::AppError(format!("no collection {id}")))?;
    Ok(CollectionKind::parse(&s))
}

/// Does one document satisfy a smart filter? Every set field must match, so
/// adding a criterion always narrows — an author's mental model of a filter.
pub fn matches(doc: &DocumentRow, f: &SmartFilter) -> bool {
    if doc.kind == crate::model::DocKind::Folder {
        return false; // collections gather documents, not containers
    }
    if let Some(want) = &f.status {
        if doc.status.as_deref() != Some(want.as_str()) {
            return false;
        }
    }
    if let Some(want) = &f.label {
        if doc.label.as_deref() != Some(want.as_str()) {
            return false;
        }
    }
    if let Some(want) = &f.pov {
        if doc.pov.as_deref() != Some(want.as_str()) {
            return false;
        }
    }
    if let Some(want) = &f.section_type {
        if doc.section_type.as_deref() != Some(want.as_str()) {
            return false;
        }
    }
    if let Some(want) = &f.keyword {
        if !doc.keywords.iter().any(|k| k.eq_ignore_ascii_case(want)) {
            return false;
        }
    }
    if let Some(min) = f.min_words {
        if doc.word_count < min {
            return false;
        }
    }
    if let Some(max) = f.max_words {
        if doc.word_count > max {
            return false;
        }
    }
    if let Some(text) = &f.text {
        let needle = text.to_lowercase();
        let hay = format!(
            "{} {} {}",
            doc.title.to_lowercase(),
            doc.synopsis.clone().unwrap_or_default().to_lowercase(),
            doc.notes.clone().unwrap_or_default().to_lowercase()
        );
        if !hay.contains(&needle) {
            return false;
        }
    }
    true
}

/// Documents in a collection, in the collection's own order for a manual one
/// and binder order for a smart one.
pub fn members(conn: &Connection, id: &str) -> AppResult<Vec<DocumentRow>> {
    let all = store::list_documents(conn)?;
    let (kind, filters): (String, Option<String>) = conn
        .query_row("SELECT kind, filters FROM collections WHERE id = ?1", params![id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .map_err(|_| crate::error::AppError(format!("no collection {id}")))?;

    match CollectionKind::parse(&kind) {
        CollectionKind::Manual => {
            let mut stmt = conn.prepare(
                "SELECT document_id FROM collection_items WHERE collection_id = ?1 ORDER BY position",
            )?;
            let ids: Vec<String> = stmt
                .query_map(params![id], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            Ok(ids
                .into_iter()
                .filter_map(|did| all.iter().find(|d| d.id == did).cloned())
                .collect())
        }
        _ => {
            let filter: SmartFilter = match filters {
                Some(json) => serde_json::from_str(&json)?,
                None => SmartFilter::default(),
            };
            Ok(all.into_iter().filter(|d| matches(d, &filter)).collect())
        }
    }
}

pub fn list(conn: &Connection) -> AppResult<Vec<Collection>> {
    let mut stmt =
        conn.prepare("SELECT id, name, kind, filters, position FROM collections ORDER BY position, name")?;
    let rows = stmt.query_map([], |r| {
        Ok(Collection {
            id: r.get(0)?,
            name: r.get(1)?,
            kind: CollectionKind::parse(&r.get::<_, String>(2)?),
            filters: r.get(3)?,
            position: r.get(4)?,
            count: 0,
        })
    })?;
    let mut out: Vec<Collection> = rows.collect::<Result<_, _>>()?;
    for c in &mut out {
        c.count = members(conn, &c.id)?.len() as i64;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DocKind, SmartFilter};
    use crate::project;
    use std::path::PathBuf;

    fn temp_project(tag: &str) -> PathBuf {
        let parent = std::env::temp_dir().join(format!(
            "aegis-coll-{tag}-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&parent).unwrap();
        let summary = project::create(&parent, "Coll Book", "novel").unwrap();
        PathBuf::from(&summary.path)
    }

    #[test]
    fn manual_collection_keeps_its_own_order_and_does_not_move_files() {
        let root = temp_project("manual");
        let a = project::create_document(&root, None, "A", DocKind::Scene).unwrap();
        let b = project::create_document(&root, None, "B", DocKind::Scene).unwrap();
        let path_before = a.path.clone();

        let conn = store::open(&root).unwrap();
        create(&conn, "c1", "Reading order", CollectionKind::Manual, None, 0).unwrap();
        // Added out of binder order on purpose.
        add_item(&conn, "c1", &b.id).unwrap();
        add_item(&conn, "c1", &a.id).unwrap();

        let names: Vec<_> = members(&conn, "c1").unwrap().into_iter().map(|d| d.title).collect();
        assert_eq!(names, vec!["B", "A"], "a manual collection keeps the order it was given");

        // The binder is untouched: same parent, same file.
        let fresh = store::get_document(&conn, &a.id).unwrap().unwrap();
        assert_eq!(fresh.path, path_before);
        assert_eq!(fresh.parent_id, None);
        assert!(root.join(fresh.path.unwrap()).exists());

        remove_item(&conn, "c1", &b.id).unwrap();
        assert_eq!(members(&conn, "c1").unwrap().len(), 1);
        std::fs::remove_dir_all(root.parent().unwrap()).ok();
    }

    #[test]
    fn smart_collection_reflects_metadata_changes_without_touching_the_binder() {
        let root = temp_project("smart");
        let a = project::create_document(&root, None, "A", DocKind::Scene).unwrap();
        let b = project::create_document(&root, None, "B", DocKind::Scene).unwrap();
        project::update_meta(&root, &a.id, None, None, Some("Draft")).unwrap();
        project::update_meta(&root, &b.id, None, None, Some("Done")).unwrap();

        let filter = serde_json::to_string(&SmartFilter {
            status: Some("Draft".into()),
            ..Default::default()
        })
        .unwrap();

        let conn = store::open(&root).unwrap();
        create(&conn, "c2", "Needs revision", CollectionKind::Smart, Some(&filter), 0).unwrap();
        let names: Vec<_> = members(&conn, "c2").unwrap().into_iter().map(|d| d.title).collect();
        assert_eq!(names, vec!["A"]);
        drop(conn);

        // Flip B to Draft; it joins with no binder change.
        project::update_meta(&root, &b.id, None, None, Some("Draft")).unwrap();
        let conn = store::open(&root).unwrap();
        assert_eq!(members(&conn, "c2").unwrap().len(), 2, "smart membership follows metadata");

        // Hand-adding to a smart collection is refused: its query defines it.
        assert!(add_item(&conn, "c2", &a.id).is_err());
        std::fs::remove_dir_all(root.parent().unwrap()).ok();
    }

    #[test]
    fn smart_filters_narrow_and_never_match_folders() {
        let root = temp_project("filters");
        let folder = project::create_document(&root, None, "Act I", DocKind::Folder).unwrap();
        let scene = project::create_document(&root, None, "Rimfall", DocKind::Scene).unwrap();
        project::update_meta(&root, &scene.id, None, Some("Plot"), Some("Draft")).unwrap();
        project::write_body(&root, &scene.id, "one two three four five").unwrap();
        project::set_keywords(&root, &scene.id, &["tamsin".to_string()]).unwrap();

        let conn = store::open(&root).unwrap();
        let all = store::list_documents(&conn).unwrap();
        let got = |f: SmartFilter| all.iter().filter(|d| matches(d, &f)).count();

        // A folder is never a member, even when every field is unset.
        assert!(!matches(all.iter().find(|d| d.id == folder.id).unwrap(), &SmartFilter::default()));

        assert_eq!(got(SmartFilter { status: Some("Draft".into()), ..Default::default() }), 1);
        assert_eq!(got(SmartFilter { label: Some("Plot".into()), ..Default::default() }), 1);
        assert_eq!(got(SmartFilter { keyword: Some("TAMSIN".into()), ..Default::default() }), 1, "keywords match case-insensitively");
        assert_eq!(got(SmartFilter { text: Some("rimfall".into()), ..Default::default() }), 1, "text matches the title, case-insensitively");
        assert_eq!(got(SmartFilter { min_words: Some(5), ..Default::default() }), 1);
        assert_eq!(got(SmartFilter { min_words: Some(6), ..Default::default() }), 0);

        // Adding a criterion narrows; it never widens.
        assert_eq!(
            got(SmartFilter { status: Some("Draft".into()), label: Some("Character".into()), ..Default::default() }),
            0,
            "every set field must match"
        );
        std::fs::remove_dir_all(root.parent().unwrap()).ok();
    }

    #[test]
    fn a_smart_collection_requires_a_query_and_names_are_required() {
        let root = temp_project("guards");
        let conn = store::open(&root).unwrap();
        assert!(create(&conn, "x", "No query", CollectionKind::Smart, None, 0).is_err());
        assert!(create(&conn, "y", "   ", CollectionKind::Manual, None, 0).is_err());
        assert!(create(&conn, "z", "Fine", CollectionKind::Manual, None, 0).is_ok());
        std::fs::remove_dir_all(root.parent().unwrap()).ok();
    }

    #[test]
    fn deleting_a_collection_leaves_its_documents_alone() {
        let root = temp_project("delete");
        let a = project::create_document(&root, None, "A", DocKind::Scene).unwrap();
        let conn = store::open(&root).unwrap();
        create(&conn, "c3", "Temp", CollectionKind::Manual, None, 0).unwrap();
        add_item(&conn, "c3", &a.id).unwrap();
        delete(&conn, "c3").unwrap();

        assert!(store::get_document(&conn, &a.id).unwrap().is_some(), "the document survives");
        assert!(root.join(a.path.unwrap()).exists(), "and so does its file");
        assert_eq!(list(&conn).unwrap().len(), 0);
        std::fs::remove_dir_all(root.parent().unwrap()).ok();
    }
}
