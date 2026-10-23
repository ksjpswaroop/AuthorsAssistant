//! SQLite access. The database holds structure and metadata only — never the
//! prose, which stays in Markdown files so the manuscript survives this app.

use crate::error::{err, AppResult};
use crate::model::{DocKind, DocumentLink, DocumentRow};
use rusqlite::{params, Connection};
use std::path::Path;

pub const SCHEMA_VERSION: i64 = 3;

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
    // Phase 2 columns. Added one at a time rather than in a versioned rewrite,
    // so a project made by an older build opens without a migration dance.
    for (col, decl) in [
        ("notes", "TEXT"),
        ("target", "INTEGER"),
        ("pov", "TEXT"),
        ("plants", "TEXT"),
        ("payoffs", "TEXT"),
        ("section_type", "TEXT"),
        ("icon", "TEXT"),
        // Phase 6 freeform corkboard. NULL means "not placed yet", which is
        // what every existing card is — a card without a position falls back to
        // the grid rather than piling up at the origin.
        ("x", "REAL"),
        ("y", "REAL"),
        // Block identity: the last reconciled whole-file hash, so an unchanged
        // document costs one comparison rather than a full re-anchor.
        ("body_hash", "TEXT"),
    ] {
        add_column_if_missing(conn, "documents", col, decl)?;
    }

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS keywords (
            id   INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE COLLATE NOCASE
        );

        CREATE TABLE IF NOT EXISTS document_keywords (
            document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            keyword_id  INTEGER NOT NULL REFERENCES keywords(id) ON DELETE CASCADE,
            PRIMARY KEY (document_id, keyword_id)
        );

        -- Sheet-to-scene links: which character or location appears where.
        CREATE TABLE IF NOT EXISTS document_links (
            from_id  TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            to_id    TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            relation TEXT NOT NULL DEFAULT 'appears_in',
            PRIMARY KEY (from_id, to_id, relation)
        );

        CREATE INDEX IF NOT EXISTS links_to ON document_links(to_id);
        ",
    )?;

    crate::collections::migrate(conn)?;
    crate::rag::migrate(conn)?;
    crate::provenance::migrate(conn)?;

    conn.execute(
        "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![SCHEMA_VERSION.to_string()],
    )?;
    Ok(())
}

/// SQLite has no `ADD COLUMN IF NOT EXISTS`, so ask first.
fn add_column_if_missing(conn: &Connection, table: &str, column: &str, decl: &str) -> AppResult<()> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let existing: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<Result<_, _>>()?;
    if !existing.iter().any(|c| c == column) {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"), [])?;
    }
    Ok(())
}

/// Every table a project is expected to have. Kept here so a module whose
/// migration is never called fails a test instead of failing at runtime, in
/// front of an author, the first time the feature is used.
pub const EXPECTED_TABLES: &[&str] = &[
    "meta",
    "documents",
    "keywords",
    "document_keywords",
    "document_links",
    "collections",
    "collection_items",
    "chunks",
    "blocks",
    "events",
];

pub fn table_names(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
        section_type: row.get("section_type")?,
        icon: row.get("icon")?,
        x: row.get("x")?,
        y: row.get("y")?,
        notes: row.get("notes")?,
        target: row.get("target")?,
        pov: row.get("pov")?,
        plants: row.get("plants")?,
        payoffs: row.get("payoffs")?,
        keywords: Vec::new(), // filled by list_documents
        links: Vec::new(),
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
                word_count, updated_at, notes, target, pov, plants, payoffs,
                section_type, icon, x, y
         FROM documents
         ORDER BY position, title",
    )?;
    let rows = stmt.query_map([], row_to_doc)?;
    let mut out: Vec<DocumentRow> = Vec::new();
    for r in rows {
        out.push(r?);
    }

    // Attach keywords and links in two sweeps rather than a query per row.
    let mut kw = conn.prepare(
        "SELECT dk.document_id, k.name FROM document_keywords dk
         JOIN keywords k ON k.id = dk.keyword_id
         ORDER BY k.name COLLATE NOCASE",
    )?;
    for pair in kw.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (doc_id, name) = pair?;
        if let Some(d) = out.iter_mut().find(|d| d.id == doc_id) {
            d.keywords.push(name);
        }
    }

    let mut ln = conn.prepare("SELECT from_id, to_id, relation FROM document_links")?;
    for row in ln.query_map([], |r| {
        Ok(DocumentLink {
            from_id: r.get(0)?,
            to_id: r.get(1)?,
            relation: r.get(2)?,
        })
    })? {
        let link = row?;
        if let Some(d) = out.iter_mut().find(|d| d.id == link.from_id) {
            d.links.push(link.clone());
        }
        if let Some(d) = out.iter_mut().find(|d| d.id == link.to_id) {
            d.links.push(link);
        }
    }

    Ok(out)
}

pub fn get_document(conn: &Connection, id: &str) -> AppResult<Option<DocumentRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, parent_id, title, kind, path, position, synopsis, label, status,
                word_count, updated_at, notes, target, pov, plants, payoffs,
                section_type, icon, x, y
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

// ---------------------------------------------------------------------------
// Phase 2: structure, metadata, keywords, links
// ---------------------------------------------------------------------------

/// True when `candidate` is `root` or sits somewhere beneath it. Used to refuse
/// a drag that would make a folder its own ancestor and detach the subtree.
pub fn is_descendant(conn: &Connection, root: &str, candidate: &str) -> AppResult<bool> {
    if root == candidate {
        return Ok(true);
    }
    let all = list_documents(conn)?;
    let mut cursor = Some(candidate.to_string());
    // Walk up from the candidate; the tree is shallow and the guard must be exact.
    let mut hops = 0;
    while let Some(id) = cursor {
        if id == root {
            return Ok(true);
        }
        hops += 1;
        if hops > 1000 {
            return err("binder tree is malformed (cycle while checking a move)");
        }
        cursor = all.iter().find(|d| d.id == id).and_then(|d| d.parent_id.clone());
    }
    Ok(false)
}

/// Moves a document under `new_parent` at `index`, renumbering both the old and
/// new sibling lists so positions stay dense and ordering is total.
pub fn move_document(
    conn: &mut Connection,
    id: &str,
    new_parent: Option<&str>,
    index: i64,
    now: i64,
) -> AppResult<()> {
    let doc = match get_document(conn, id)? {
        Some(d) => d,
        None => return err(format!("no document {id}")),
    };
    if let Some(parent) = new_parent {
        if is_descendant(conn, id, parent)? {
            return err("a folder cannot be moved inside itself");
        }
        match get_document(conn, parent)? {
            Some(p) if p.kind == DocKind::Folder => {}
            Some(_) => return err("documents can only be nested inside folders"),
            None => return err(format!("no folder {parent}")),
        }
    }

    let tx = conn.transaction()?;
    let siblings: Vec<String> = {
        let mut stmt = match new_parent {
            Some(_) => tx.prepare("SELECT id FROM documents WHERE parent_id = ?1 AND id != ?2 ORDER BY position, title")?,
            None => tx.prepare("SELECT id FROM documents WHERE parent_id IS NULL AND id != ?2 ORDER BY position, title")?,
        };
        let rows = match new_parent {
            Some(p) => stmt.query_map(params![p, id], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?,
            None => stmt.query_map(params![Option::<String>::None, id], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?,
        };
        rows
    };

    let at = index.clamp(0, siblings.len() as i64) as usize;
    let mut ordered = siblings;
    ordered.insert(at, id.to_string());

    tx.execute(
        "UPDATE documents SET parent_id = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, new_parent, now],
    )?;
    for (i, sib) in ordered.iter().enumerate() {
        tx.execute("UPDATE documents SET position = ?2 WHERE id = ?1", params![sib, i as i64])?;
    }

    // Close the gap the document left behind.
    let old_parent = doc.parent_id.clone();
    if old_parent.as_deref() != new_parent {
        let leftovers: Vec<String> = {
            let mut stmt = match old_parent.as_deref() {
                Some(_) => tx.prepare("SELECT id FROM documents WHERE parent_id = ?1 ORDER BY position, title")?,
                None => tx.prepare("SELECT id FROM documents WHERE parent_id IS NULL ORDER BY position, title")?,
            };
            match old_parent.as_deref() {
                Some(p) => stmt.query_map(params![p], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?,
                None => stmt.query_map([], |r| r.get::<_, String>(0))?.collect::<Result<Vec<_>, _>>()?,
            }
        };
        for (i, sib) in leftovers.iter().enumerate() {
            tx.execute("UPDATE documents SET position = ?2 WHERE id = ?1", params![sib, i as i64])?;
        }
    }

    tx.commit()?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_extended(
    conn: &Connection,
    id: &str,
    notes: Option<&str>,
    target: Option<i64>,
    pov: Option<&str>,
    plants: Option<&str>,
    payoffs: Option<&str>,
    now: i64,
) -> AppResult<()> {
    conn.execute(
        "UPDATE documents
         SET notes   = COALESCE(?2, notes),
             target  = COALESCE(?3, target),
             pov     = COALESCE(?4, pov),
             plants  = COALESCE(?5, plants),
             payoffs = COALESCE(?6, payoffs),
             updated_at = ?7
         WHERE id = ?1",
        params![id, notes, target, pov, plants, payoffs, now],
    )?;
    Ok(())
}

/// Replaces a document's keywords, creating any that are new to the project.
/// The keyword table doubles as the project vocabulary Phase 6 draws from.
pub fn set_keywords(conn: &mut Connection, id: &str, names: &[String], now: i64) -> AppResult<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM document_keywords WHERE document_id = ?1", params![id])?;
    for raw in names {
        let name = raw.trim();
        if name.is_empty() {
            continue;
        }
        tx.execute("INSERT OR IGNORE INTO keywords (name) VALUES (?1)", params![name])?;
        let kid: i64 = tx.query_row(
            "SELECT id FROM keywords WHERE name = ?1 COLLATE NOCASE",
            params![name],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO document_keywords (document_id, keyword_id) VALUES (?1, ?2)",
            params![id, kid],
        )?;
    }
    tx.execute("UPDATE documents SET updated_at = ?2 WHERE id = ?1", params![id, now])?;
    tx.commit()?;
    Ok(())
}

/// Every keyword used anywhere in the project, with how many documents use it.
pub fn project_keywords(conn: &Connection) -> AppResult<Vec<(String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT k.name, COUNT(dk.document_id)
         FROM keywords k LEFT JOIN document_keywords dk ON dk.keyword_id = k.id
         GROUP BY k.id ORDER BY k.name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn link(conn: &Connection, from_id: &str, to_id: &str, relation: &str) -> AppResult<()> {
    if from_id == to_id {
        return err("a document cannot be linked to itself");
    }
    conn.execute(
        "INSERT OR IGNORE INTO document_links (from_id, to_id, relation) VALUES (?1, ?2, ?3)",
        params![from_id, to_id, relation],
    )?;
    Ok(())
}

pub fn unlink(conn: &Connection, from_id: &str, to_id: &str, relation: &str) -> AppResult<()> {
    conn.execute(
        "DELETE FROM document_links WHERE from_id = ?1 AND to_id = ?2 AND relation = ?3",
        params![from_id, to_id, relation],
    )?;
    Ok(())
}

/// Sets fields across many documents at once (Phase 6 bulk metadata).
/// Returns how many rows changed, so the UI can report the blast radius.
pub fn bulk_set(
    conn: &mut Connection,
    ids: &[String],
    status: Option<&str>,
    label: Option<&str>,
    section_type: Option<&str>,
    pov: Option<&str>,
    now: i64,
) -> AppResult<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    if status.is_none() && label.is_none() && section_type.is_none() && pov.is_none() {
        return err("nothing to set");
    }
    let tx = conn.transaction()?;
    let mut changed = 0usize;
    for id in ids {
        changed += tx.execute(
            "UPDATE documents
             SET status       = COALESCE(?2, status),
                 label        = COALESCE(?3, label),
                 section_type = COALESCE(?4, section_type),
                 pov          = COALESCE(?5, pov),
                 updated_at   = ?6
             WHERE id = ?1",
            params![id, status, label, section_type, pov, now],
        )?;
    }
    tx.commit()?;
    Ok(changed)
}

pub fn set_icon(conn: &Connection, id: &str, icon: Option<&str>, now: i64) -> AppResult<()> {
    conn.execute(
        "UPDATE documents SET icon = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, icon, now],
    )?;
    Ok(())
}

/// Set or clear a document's freeform corkboard position (Phase 6).
///
/// Passing `None` unplaces the card, which is how "revert to the grid" works —
/// distinct from placing it at (0, 0), which is a real position someone chose.
pub fn set_position(
    conn: &Connection,
    id: &str,
    x: Option<f32>,
    y: Option<f32>,
    now: i64,
) -> AppResult<()> {
    conn.execute(
        "UPDATE documents SET x = ?2, y = ?3, updated_at = ?4 WHERE id = ?1",
        params![id, x, y, now],
    )?;
    Ok(())
}
