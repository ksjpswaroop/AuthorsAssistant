//! SQLite access. The database holds structure and metadata only — never the
//! prose, which stays in Markdown files so the manuscript survives this app.

use crate::error::AppResult;
use crate::model::{DocKind, DocumentRow};
use rusqlite::{params, Connection};
use std::path::Path;

pub const SCHEMA_VERSION: i64 = 1;

pub fn open(project_root: &Path) -> AppResult<Connection> {
    let conn = Connection::open(project_root.join("project.sqlite"))?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS meta (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS documents (
            id         TEXT PRIMARY KEY,
            parent_id  TEXT REFERENCES documents(id) ON DELETE CASCADE,
            title      TEXT NOT NULL,
            kind       TEXT NOT NULL,
            path       TEXT,
            position   INTEGER NOT NULL DEFAULT 0,
            synopsis   TEXT,
            label      TEXT,
            status     TEXT,
            word_count INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS documents_parent ON documents(parent_id, position);
        ",
    )?;
    conn.execute(
        "INSERT OR IGNORE INTO meta (key, value) VALUES ('schema_version', ?1)",
        params![SCHEMA_VERSION.to_string()],
    )?;
    Ok(())
}

pub fn get_meta(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM meta WHERE key = ?1")?;
    let mut rows = stmt.query(params![key])?;
    Ok(match rows.next()? {
        Some(r) => Some(r.get(0)?),
        None => None,
    })
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn row_to_doc(row: &rusqlite::Row<'_>) -> rusqlite::Result<DocumentRow> {
    let kind: String = row.get("kind")?;
    Ok(DocumentRow {
        id: row.get("id")?,
        parent_id: row.get("parent_id")?,
        title: row.get("title")?,
        kind: DocKind::parse(&kind),
        path: row.get("path")?,
        position: row.get("position")?,
        synopsis: row.get("synopsis")?,
        label: row.get("label")?,
        status: row.get("status")?,
        word_count: row.get("word_count")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list_documents(conn: &Connection) -> AppResult<Vec<DocumentRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, title, kind, path, position, synopsis, label, status,
                word_count, updated_at
         FROM documents
         ORDER BY position, title",
    )?;
    let rows = stmt.query_map([], row_to_doc)?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn get_document(conn: &Connection, id: &str) -> AppResult<Option<DocumentRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, title, kind, path, position, synopsis, label, status,
                word_count, updated_at
         FROM documents WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id], row_to_doc)?;
    Ok(match rows.next() {
        Some(r) => Some(r?),
        None => None,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn insert_document(
    conn: &Connection,
    id: &str,
    parent_id: Option<&str>,
    title: &str,
    kind: DocKind,
    path: Option<&str>,
    position: i64,
    now: i64,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO documents
            (id, parent_id, title, kind, path, position, word_count, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?7)",
        params![id, parent_id, title, kind.as_str(), path, position, now],
    )?;
    Ok(())
}

pub fn next_position(conn: &Connection, parent_id: Option<&str>) -> AppResult<i64> {
    let mut stmt = match parent_id {
        Some(_) => conn.prepare(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM documents WHERE parent_id = ?1",
        )?,
        None => conn
            .prepare("SELECT COALESCE(MAX(position), -1) + 1 FROM documents WHERE parent_id IS NULL")?,
    };
    let mut rows = match parent_id {
        Some(p) => stmt.query(params![p])?,
        None => stmt.query([])?,
    };
    Ok(match rows.next()? {
        Some(r) => r.get(0)?,
        None => 0,
    })
}

pub fn touch_body(conn: &Connection, id: &str, word_count: i64, now: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE documents SET word_count = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, word_count, now],
    )?;
    Ok(())
}

pub fn rename(conn: &Connection, id: &str, title: &str, path: Option<&str>, now: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE documents SET title = ?2, path = COALESCE(?3, path), updated_at = ?4 WHERE id = ?1",
        params![id, title, path, now],
    )?;
    Ok(())
}

pub fn update_meta_fields(
    conn: &Connection,
    id: &str,
    synopsis: Option<&str>,
    label: Option<&str>,
    status: Option<&str>,
    now: i64,
) -> AppResult<()> {
    conn.execute(
        "UPDATE documents
         SET synopsis = COALESCE(?2, synopsis),
             label    = COALESCE(?3, label),
             status   = COALESCE(?4, status),
             updated_at = ?5
         WHERE id = ?1",
        params![id, synopsis, label, status, now],
    )?;
    Ok(())
}

/// A document and every descendant, deepest last. Used so deleting a folder
/// can move all of its children's files to trash — the SQL cascade removes the
/// rows, but files are our responsibility.
pub fn subtree(conn: &Connection, root_id: &str) -> AppResult<Vec<DocumentRow>> {
    let all = list_documents(conn)?;
    let mut out = Vec::new();
    let mut frontier = vec![root_id.to_string()];
    while let Some(id) = frontier.pop() {
        if let Some(doc) = all.iter().find(|d| d.id == id) {
            out.push(doc.clone());
        }
        for child in all.iter().filter(|d| d.parent_id.as_deref() == Some(id.as_str())) {
            frontier.push(child.id.clone());
        }
    }
    Ok(out)
}

pub fn delete(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn totals(conn: &Connection) -> AppResult<(i64, i64)> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(SUM(word_count), 0), COUNT(*) FROM documents WHERE kind != 'folder'",
    )?;
    let mut rows = stmt.query([])?;
    Ok(match rows.next()? {
        Some(r) => (r.get(0)?, r.get(1)?),
        None => (0, 0),
    })
}
