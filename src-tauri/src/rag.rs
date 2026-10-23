//! Retrieval (Phase 3): chunking, an embedded vector index, and search.
//!
//! The vector index lives in the project's own SQLite file rather than a
//! separate Qdrant process. That is a deliberate departure from TECH-STACK:
//! a local-first writing app should not require the author to run a database
//! server, and a book's worth of chunks is thousands of vectors, not millions.
//! Exact cosine over that is fast and has no recall cliff to tune.

use crate::error::{err, AppResult};
use crate::model::{Chunk, RetrievedChunk};
use rusqlite::{params, Connection};

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS chunks (
            id          TEXT PRIMARY KEY,
            document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            ordinal     INTEGER NOT NULL,
            text        TEXT NOT NULL,
            embedding   BLOB,
            model       TEXT,
            indexed_at  INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS chunks_doc ON chunks(document_id, ordinal);
        ",
    )?;
    Ok(())
}

/// Splits text into overlapping windows of whole words.
///
/// Two things this must never do: lose text between windows, or loop forever.
/// An overlap greater than or equal to the window size makes the step zero,
/// which is the classic version of the second bug — it is refused, not clamped,
/// because silently changing the caller's parameters hides a configuration
/// mistake that would otherwise be obvious.
pub fn chunk(text: &str, size: usize, overlap: usize) -> AppResult<Vec<String>> {
    if size == 0 {
        return err("chunk size must be greater than zero");
    }
    if overlap >= size {
        return err("overlap must be smaller than the chunk size");
    }
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return Ok(Vec::new());
    }
    let step = size - overlap;
    let mut out = Vec::new();
    let mut start = 0usize;
    while start < words.len() {
        let end = (start + size).min(words.len());
        out.push(words[start..end].join(" "));
        if end == words.len() {
            break;
        }
        start += step;
    }
    Ok(out)
}

/// Cosine similarity. Returns 0 for a zero-length vector rather than NaN, so a
/// degenerate embedding sorts last instead of poisoning the ranking.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    if na <= 0.0 || nb <= 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

fn to_blob(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn from_blob(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// Replaces every chunk for one document.
pub fn index_document(
    conn: &mut Connection,
    document_id: &str,
    chunks: &[(String, Vec<f32>)],
    model: &str,
    now: i64,
) -> AppResult<usize> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM chunks WHERE document_id = ?1", params![document_id])?;
    for (i, (text, embedding)) in chunks.iter().enumerate() {
        tx.execute(
            "INSERT INTO chunks (id, document_id, ordinal, text, embedding, model, indexed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                format!("{document_id}:{i}"),
                document_id,
                i as i64,
                text,
                to_blob(embedding),
                model,
                now
            ],
        )?;
    }
    tx.commit()?;
    Ok(chunks.len())
}

/// Top-k chunks by cosine similarity. `exclude` keeps a document from being its
/// own best match, which is what makes "what else in the book says this" useful.
pub fn search(
    conn: &Connection,
    query: &[f32],
    k: usize,
    exclude_document: Option<&str>,
) -> AppResult<Vec<RetrievedChunk>> {
    let mut stmt =
        conn.prepare("SELECT id, document_id, ordinal, text, embedding FROM chunks WHERE embedding IS NOT NULL")?;
    let rows = stmt.query_map([], |r| {
        Ok(Chunk {
            id: r.get(0)?,
            document_id: r.get(1)?,
            ordinal: r.get(2)?,
            text: r.get(3)?,
            embedding: from_blob(&r.get::<_, Vec<u8>>(4)?),
        })
    })?;

    let mut scored: Vec<RetrievedChunk> = rows
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|c| exclude_document.map_or(true, |x| c.document_id != x))
        .map(|c| RetrievedChunk {
            score: cosine(query, &c.embedding),
            id: c.id,
            document_id: c.document_id,
            ordinal: c.ordinal,
            text: c.text,
        })
        .collect();

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    Ok(scored)
}

pub fn index_stats(conn: &Connection) -> AppResult<(i64, i64)> {
    let chunks: i64 = conn.query_row("SELECT COUNT(*) FROM chunks", [], |r| r.get(0))?;
    let docs: i64 = conn.query_row("SELECT COUNT(DISTINCT document_id) FROM chunks", [], |r| r.get(0))?;
    Ok((chunks, docs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunking_covers_every_word_with_no_gaps() {
        let text: String = (1..=25).map(|n| n.to_string()).collect::<Vec<_>>().join(" ");
        let chunks = chunk(&text, 10, 2).unwrap();

        // Every word appears somewhere.
        for n in 1..=25 {
            assert!(chunks.iter().any(|c| c.split_whitespace().any(|w| w == n.to_string())), "lost word {n}");
        }
        // Consecutive windows share exactly the overlap.
        for pair in chunks.windows(2) {
            let a: Vec<&str> = pair[0].split_whitespace().collect();
            let b: Vec<&str> = pair[1].split_whitespace().collect();
            assert_eq!(&a[a.len() - 2..], &b[..2], "windows must overlap by 2 words");
        }
    }

    #[test]
    fn overlap_not_smaller_than_size_is_refused_rather_than_looping() {
        assert!(chunk("a b c", 4, 4).is_err());
        assert!(chunk("a b c", 4, 9).is_err());
        assert!(chunk("a b c", 0, 0).is_err());
        assert!(chunk("a b c", 4, 3).is_ok());
    }

    #[test]
    fn short_and_empty_texts_behave() {
        assert_eq!(chunk("", 10, 2).unwrap().len(), 0);
        assert_eq!(chunk("   \n  ", 10, 2).unwrap().len(), 0);
        assert_eq!(chunk("three little words", 10, 2).unwrap(), vec!["three little words"]);
    }

    #[test]
    fn cosine_is_one_for_identical_and_zero_for_orthogonal() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 1.0, 0.0];
        assert!((cosine(&a, &a) - 1.0).abs() < 1e-6);
        assert!(cosine(&a, &b).abs() < 1e-6);
        // Direction, not magnitude.
        assert!((cosine(&a, &[5.0, 0.0, 0.0]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_degrades_safely_instead_of_returning_nan() {
        assert_eq!(cosine(&[0.0, 0.0], &[1.0, 1.0]), 0.0, "a zero vector must not produce NaN");
        assert_eq!(cosine(&[1.0], &[1.0, 2.0]), 0.0, "mismatched lengths score zero");
        assert_eq!(cosine(&[], &[]), 0.0);
    }

    #[test]
    fn blobs_round_trip_exactly() {
        let v = vec![0.5f32, -1.25, 0.0, 3.75];
        assert_eq!(from_blob(&to_blob(&v)), v);
    }

    fn mem_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE documents (id TEXT PRIMARY KEY);").unwrap();
        conn.execute("INSERT INTO documents (id) VALUES ('d1'), ('d2')", []).unwrap();
        migrate(&conn).unwrap();
        conn
    }

    #[test]
    fn search_ranks_by_similarity_and_respects_k() {
        let mut conn = mem_db();
        index_document(
            &mut conn,
            "d1",
            &[
                ("east".into(), vec![1.0, 0.0]),
                ("north".into(), vec![0.0, 1.0]),
                ("north-east".into(), vec![0.7, 0.7]),
            ],
            "test",
            0,
        )
        .unwrap();

        let hits = search(&conn, &[1.0, 0.0], 3, None).unwrap();
        assert_eq!(hits[0].text, "east", "the closest chunk ranks first");
        assert_eq!(hits[1].text, "north-east");
        assert!(hits[0].score > hits[1].score && hits[1].score > hits[2].score);

        assert_eq!(search(&conn, &[1.0, 0.0], 2, None).unwrap().len(), 2, "k limits the result");
    }

    #[test]
    fn search_can_exclude_the_document_that_asked() {
        let mut conn = mem_db();
        index_document(&mut conn, "d1", &[("mine".into(), vec![1.0, 0.0])], "test", 0).unwrap();
        index_document(&mut conn, "d2", &[("theirs".into(), vec![0.9, 0.1])], "test", 0).unwrap();

        let all = search(&conn, &[1.0, 0.0], 5, None).unwrap();
        assert_eq!(all.len(), 2);

        let others = search(&conn, &[1.0, 0.0], 5, Some("d1")).unwrap();
        assert_eq!(others.len(), 1);
        assert_eq!(others[0].text, "theirs", "a document is not its own best match");
    }

    #[test]
    fn reindexing_a_document_replaces_its_chunks_rather_than_duplicating() {
        let mut conn = mem_db();
        index_document(&mut conn, "d1", &[("v1 a".into(), vec![1.0, 0.0]), ("v1 b".into(), vec![0.0, 1.0])], "m", 0)
            .unwrap();
        assert_eq!(index_stats(&conn).unwrap(), (2, 1));

        index_document(&mut conn, "d1", &[("v2".into(), vec![1.0, 0.0])], "m", 1).unwrap();
        let (chunks, docs) = index_stats(&conn).unwrap();
        assert_eq!((chunks, docs), (1, 1), "stale chunks must not survive a reindex");
        assert_eq!(search(&conn, &[1.0, 0.0], 5, None).unwrap()[0].text, "v2");
    }

    #[test]
    fn deleting_a_document_removes_its_chunks() {
        let mut conn = mem_db();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        index_document(&mut conn, "d1", &[("x".into(), vec![1.0, 0.0])], "m", 0).unwrap();
        conn.execute("DELETE FROM documents WHERE id = 'd1'", []).unwrap();
        assert_eq!(index_stats(&conn).unwrap().0, 0, "the index must not outlive the document");
    }
}
