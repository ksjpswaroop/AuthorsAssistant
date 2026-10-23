//! Block-level identity: a stable id for every paragraph (K2P-OS §5, §7).
//!
//! Design and rationale: `docs/BLOCK-IDENTITY.md`. Test plan:
//! `docs/TEST-PLAN-BLOCKS.md`, cases `BI-*`.
//!
//! # The tension this resolves
//!
//! AegisDraft promises that a manuscript is a folder of readable Markdown the
//! author owns and can edit anywhere. Provenance needs `P-184` to survive 47
//! revisions. Most systems resolve that by taking custody of the text; that
//! option is not available here.
//!
//! So identity lives in SQLite and is **re-anchored to the text on every load**
//! by matching content. The prose stays clean, and an edit made in vim does not
//! destroy history.
//!
//! # Observation versus inference
//!
//! Re-anchoring after an external edit is *inference* — the app did not watch it
//! happen and is reconstructing what probably did. An edit made inside the app
//! is *observation*. Conflating them would make the ledger confidently wrong, so
//! every anchor carries whether it was certain, and the ledger records
//! `observed` or `inferred` accordingly.

use std::collections::HashSet;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppResult;
use crate::provenance::{self, Action, Actor, Confidence, NewEvent};

/// Below this similarity, two blocks are different blocks — not one revised.
pub const ANCHOR_THRESHOLD: f32 = 0.35;
/// Above this, an anchor is certain enough not to flag.
pub const CONFIDENT_THRESHOLD: f32 = 0.60;
/// A runner-up this close to the winner makes the match ambiguous.
pub const AMBIGUITY_MARGIN: f32 = 0.10;
/// Shorter than this, fuzzy matching produces confident nonsense, so it is not
/// attempted at all. A three-word heading shingles to almost anything.
pub const MIN_FUZZY_WORDS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Heading,
    Paragraph,
    Code,
    Quote,
    List,
    Table,
    FrontMatter,
    Rule,
}

impl BlockKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BlockKind::Heading => "heading",
            BlockKind::Paragraph => "paragraph",
            BlockKind::Code => "code",
            BlockKind::Quote => "quote",
            BlockKind::List => "list",
            BlockKind::Table => "table",
            BlockKind::FrontMatter => "frontmatter",
            BlockKind::Rule => "rule",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "heading" => BlockKind::Heading,
            "code" => BlockKind::Code,
            "quote" => BlockKind::Quote,
            "list" => BlockKind::List,
            "table" => BlockKind::Table,
            "frontmatter" => BlockKind::FrontMatter,
            "rule" => BlockKind::Rule,
            _ => BlockKind::Paragraph,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedBlock {
    pub kind: BlockKind,
    pub text: String,
    pub ordinal: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub id: String,
    pub document_id: String,
    pub ordinal: i64,
    pub kind: BlockKind,
    pub content_hash: String,
    pub text: String,
    pub revision: i64,
    pub first_seen: i64,
    pub updated_at: i64,
    /// Soft delete. A block is never dropped and its id is never reused: a
    /// provenance chain that ends in a deletion is still worth reading.
    pub removed_at: Option<i64>,
}

// ---------------------------------------------------------------------------
// Normalisation and hashing
// ---------------------------------------------------------------------------

/// Collapse whitespace runs to one space and trim. What identity compares.
///
/// Hard-wrapping a paragraph at 80 columns is therefore **not** a change. That
/// matters more than it sounds: a large share of external edits to a Markdown
/// file are reflows, and recording those as rewrites would bury the real events
/// under noise. Punctuation and capitalisation are content and are left alone.
pub fn normalise(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// sha256 of the normalised text, lowercase hex.
pub fn content_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalise(text).as_bytes());
    format!("{:x}", hasher.finalize())
}

/// sha256 of a whole file, used to skip reconciliation when nothing changed.
pub fn body_hash(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Splitting
// ---------------------------------------------------------------------------

fn is_fence(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("```") || t.starts_with("~~~")
}

fn is_rule(line: &str) -> bool {
    let t = line.trim();
    (t.len() >= 3 && t.chars().all(|c| c == '-'))
        || (t.len() >= 3 && t.chars().all(|c| c == '*'))
        || (t.len() >= 3 && t.chars().all(|c| c == '_'))
}

fn is_list_item(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- ")
        || t.starts_with("* ")
        || t.starts_with("+ ")
        || t.split_once(". ").is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
}

fn is_table_row(line: &str) -> bool {
    let t = line.trim();
    t.starts_with('|') && t.len() > 1
}

/// Split Markdown into typed blocks. Pure: no I/O, no database.
pub fn split(markdown: &str) -> Vec<ParsedBlock> {
    // Normalise line endings first so a CRLF file parses identically to LF.
    let text = markdown.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<ParsedBlock> = Vec::new();
    let mut i = 0usize;

    // YAML front matter, but only when the file opens with it. A `---` further
    // down is a horizontal rule, and treating it as front matter would swallow
    // the chapter after it.
    if lines.first().map(|l| l.trim()) == Some("---") {
        let mut j = 1;
        while j < lines.len() && lines[j].trim() != "---" {
            j += 1;
        }
        if j < lines.len() {
            out.push(ParsedBlock {
                kind: BlockKind::FrontMatter,
                text: lines[..=j].join("\n"),
                ordinal: 0,
            });
            i = j + 1;
        }
    }

    let push = |out: &mut Vec<ParsedBlock>, kind: BlockKind, body: Vec<&str>| {
        let text = body.join("\n");
        if text.trim().is_empty() {
            return;
        }
        let ordinal = out.len();
        out.push(ParsedBlock { kind, text, ordinal });
    };

    while i < lines.len() {
        let line = lines[i];

        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        // A fenced code block is one block even though it contains blank lines.
        if is_fence(line) {
            let mut body = vec![line];
            i += 1;
            while i < lines.len() {
                body.push(lines[i]);
                let closed = is_fence(lines[i]);
                i += 1;
                if closed {
                    break;
                }
            }
            // An unterminated fence runs to the end of the file rather than
            // hanging or swallowing the rest as prose.
            push(&mut out, BlockKind::Code, body);
            continue;
        }

        // An indented code block: four spaces or a tab, the other half of
        // Markdown's code syntax. Without this a listing reads as a paragraph,
        // and `kind` is part of anchoring — a block that changes kind between
        // parses cannot anchor to itself.
        if line.starts_with("    ") || line.starts_with('\t') {
            let mut body = Vec::new();
            while i < lines.len()
                && (lines[i].starts_with("    ")
                    || lines[i].starts_with('\t')
                    || lines[i].trim().is_empty())
            {
                // A trailing blank line belongs to whatever comes next.
                if lines[i].trim().is_empty()
                    && lines
                        .get(i + 1)
                        .map(|n| !n.starts_with("    ") && !n.starts_with('\t'))
                        .unwrap_or(true)
                {
                    break;
                }
                body.push(lines[i]);
                i += 1;
            }
            push(&mut out, BlockKind::Code, body);
            continue;
        }

        if line.trim_start().starts_with('#') {
            push(&mut out, BlockKind::Heading, vec![line]);
            i += 1;
            continue;
        }

        if is_rule(line) {
            push(&mut out, BlockKind::Rule, vec![line]);
            i += 1;
            continue;
        }

        // Runs: quote, list and table each continue until a blank line or a
        // line of a different sort.
        let run_kind = if line.trim_start().starts_with('>') {
            Some(BlockKind::Quote)
        } else if is_list_item(line) {
            Some(BlockKind::List)
        } else if is_table_row(line) {
            Some(BlockKind::Table)
        } else {
            None
        };

        if let Some(kind) = run_kind {
            let mut body = Vec::new();
            while i < lines.len() && !lines[i].trim().is_empty() {
                let matches = match kind {
                    BlockKind::Quote => lines[i].trim_start().starts_with('>'),
                    BlockKind::List => is_list_item(lines[i]) || lines[i].starts_with("  "),
                    BlockKind::Table => is_table_row(lines[i]),
                    _ => false,
                };
                if !matches {
                    break;
                }
                body.push(lines[i]);
                i += 1;
            }
            push(&mut out, kind, body);
            continue;
        }

        // Plain paragraph: to the next blank line or the start of another kind.
        let mut body = Vec::new();
        while i < lines.len() && !lines[i].trim().is_empty() {
            let starts_other = is_fence(lines[i])
                || lines[i].trim_start().starts_with('#')
                || lines[i].trim_start().starts_with('>')
                || is_rule(lines[i])
                || is_table_row(lines[i]);
            if starts_other && !body.is_empty() {
                break;
            }
            body.push(lines[i]);
            i += 1;
        }
        push(&mut out, BlockKind::Paragraph, body);
    }

    out
}

// ---------------------------------------------------------------------------
// Anchoring
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "outcome", rename_all = "camelCase")]
pub enum AnchorOutcome {
    /// Byte-identical after normalisation. Nothing happened to this block.
    Same { block_id: String },
    /// Recognisably the same block, changed. `uncertain` means the match was
    /// close enough to act on but not close enough to assert.
    Revised { block_id: String, similarity: f32, uncertain: bool },
    /// Nothing recognisable. A new block, and a new id.
    New,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    /// Index into the parsed slice.
    pub parsed: usize,
    pub outcome: AnchorOutcome,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorPlan {
    pub anchors: Vec<Anchor>,
    /// Ids of known blocks that no longer appear.
    pub removed: Vec<String>,
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

fn similarity(a: &str, b: &str) -> f32 {
    let sa: HashSet<String> = crate::integrity::shingles(a, 3);
    let sb: HashSet<String> = crate::integrity::shingles(b, 3);
    crate::integrity::jaccard(&sa, &sb)
}

/// Match parsed blocks to known ones.
///
/// Pure on purpose: it takes the knowns, returns a plan, and touches nothing.
/// Every anchoring test drives this function directly, without a database.
pub fn anchor(parsed: &[ParsedBlock], known: &[Block]) -> AnchorPlan {
    let live: Vec<&Block> = known.iter().filter(|b| b.removed_at.is_none()).collect();
    let mut outcome: Vec<Option<AnchorOutcome>> = vec![None; parsed.len()];
    let mut claimed: Vec<bool> = vec![false; live.len()];

    // --- Pass 1: exact content hash, nearest ordinal first.
    //
    // Ordinal proximity is not a nicety. Two identical paragraphs matched
    // first-come would swap identities on every save, and their histories with
    // them.
    for (pi, p) in parsed.iter().enumerate() {
        let hash = content_hash(&p.text);
        let best = live
            .iter()
            .enumerate()
            .filter(|(ki, k)| !claimed[*ki] && k.content_hash == hash && k.kind == p.kind)
            .min_by_key(|(_, k)| (k.ordinal - p.ordinal as i64).abs());
        if let Some((ki, k)) = best {
            claimed[ki] = true;
            outcome[pi] = Some(AnchorOutcome::Same { block_id: k.id.clone() });
        }
    }

    // --- Pass 2: similarity, greedily, best pair first.
    //
    // Collect every candidate pair above the threshold, then take them in
    // descending order so the strongest evidence wins regardless of document
    // order. Ties break on ordinal distance.
    let mut candidates: Vec<(f32, usize, usize, f32)> = Vec::new(); // (score, pi, ki, runner_up)
    for (pi, p) in parsed.iter().enumerate() {
        if outcome[pi].is_some() || word_count(&p.text) < MIN_FUZZY_WORDS {
            continue;
        }
        let mut scored: Vec<(f32, usize)> = live
            .iter()
            .enumerate()
            .filter(|(ki, k)| !claimed[*ki] && k.kind == p.kind && word_count(&k.text) >= MIN_FUZZY_WORDS)
            .map(|(ki, k)| (similarity(&p.text, &k.text), ki))
            .filter(|(s, _)| *s >= ANCHOR_THRESHOLD)
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        if let Some(&(best, ki)) = scored.first() {
            let runner_up = scored.get(1).map(|(s, _)| *s).unwrap_or(0.0);
            candidates.push((best, pi, ki, runner_up));
        }
    }
    candidates.sort_by(|a, b| {
        b.0.total_cmp(&a.0).then_with(|| {
            let da = (live[a.2].ordinal - parsed[a.1].ordinal as i64).abs();
            let db = (live[b.2].ordinal - parsed[b.1].ordinal as i64).abs();
            da.cmp(&db)
        })
    });

    for (score, pi, ki, runner_up) in candidates {
        if outcome[pi].is_some() || claimed[ki] {
            continue; // a stronger pair already took one of these
        }
        claimed[ki] = true;
        // Uncertain when the score is merely adequate, or when something else
        // was nearly as good. The margin matters as much as the score: a 0.72
        // match with a 0.66 rival is a coin toss wearing a confident number.
        let uncertain = score < CONFIDENT_THRESHOLD || (score - runner_up) < AMBIGUITY_MARGIN;
        outcome[pi] = Some(AnchorOutcome::Revised {
            block_id: live[ki].id.clone(),
            similarity: score,
            uncertain,
        });
    }

    // --- Pass 3: leftovers.
    let anchors = outcome
        .into_iter()
        .enumerate()
        .map(|(parsed, o)| Anchor { parsed, outcome: o.unwrap_or(AnchorOutcome::New) })
        .collect();
    let removed = live
        .iter()
        .enumerate()
        .filter(|(ki, _)| !claimed[*ki])
        .map(|(_, k)| k.id.clone())
        .collect();

    AnchorPlan { anchors, removed }
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

fn row_to_block(row: &rusqlite::Row<'_>) -> rusqlite::Result<Block> {
    Ok(Block {
        id: row.get("id")?,
        document_id: row.get("document_id")?,
        ordinal: row.get("ordinal")?,
        kind: BlockKind::parse(&row.get::<_, String>("kind")?),
        content_hash: row.get("content_hash")?,
        text: row.get("text")?,
        revision: row.get("revision")?,
        first_seen: row.get("first_seen")?,
        updated_at: row.get("updated_at")?,
        removed_at: row.get("removed_at")?,
    })
}

/// Live blocks for a document, in reading order.
pub fn list(conn: &Connection, document_id: &str) -> AppResult<Vec<Block>> {
    let mut stmt = conn.prepare(
        "SELECT id, document_id, ordinal, kind, content_hash, text, revision,
                first_seen, updated_at, removed_at
         FROM blocks WHERE document_id = ?1 AND removed_at IS NULL ORDER BY ordinal",
    )?;
    let rows = stmt.query_map(params![document_id], row_to_block)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

/// Every block including removed ones — what a lineage view needs.
pub fn list_all(conn: &Connection, document_id: &str) -> AppResult<Vec<Block>> {
    let mut stmt = conn.prepare(
        "SELECT id, document_id, ordinal, kind, content_hash, text, revision,
                first_seen, updated_at, removed_at
         FROM blocks WHERE document_id = ?1 ORDER BY ordinal",
    )?;
    let rows = stmt.query_map(params![document_id], row_to_block)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn get(conn: &Connection, block_id: &str) -> AppResult<Option<Block>> {
    let mut stmt = conn.prepare(
        "SELECT id, document_id, ordinal, kind, content_hash, text, revision,
                first_seen, updated_at, removed_at
         FROM blocks WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![block_id], row_to_block)?;
    Ok(match rows.next() {
        Some(r) => Some(r?),
        None => None,
    })
}

/// Reconcile a document's text with its known blocks, writing the difference to
/// the ledger.
///
/// `actor` and `confidence` say who did this and whether AegisDraft watched it
/// happen. Getting those wrong is the one failure that would make the whole
/// feature dishonest — an agent's edit recorded as `Actor::human()` launders AI
/// writing as the author's own — so every caller passes them explicitly and
/// there is no default.
pub fn reconcile(
    conn: &Connection,
    document_id: &str,
    markdown: &str,
    actor: Actor,
    confidence: Confidence,
    now: i64,
) -> AppResult<Vec<Block>> {
    // Pass 0: nothing to do if the file is byte-identical to last time. This is
    // what keeps reconciliation free on the common path.
    let file_hash = body_hash(markdown);
    let seen: Option<String> = conn
        .query_row("SELECT body_hash FROM documents WHERE id = ?1", params![document_id], |r| r.get(0))
        .unwrap_or(None);
    if seen.as_deref() == Some(file_hash.as_str()) {
        return list(conn, document_id);
    }

    let parsed = split(markdown);
    let known = list(conn, document_id)?;
    let plan = anchor(&parsed, &known);
    let by_id: std::collections::HashMap<&str, &Block> =
        known.iter().map(|b| (b.id.as_str(), b)).collect();

    for a in &plan.anchors {
        let p = &parsed[a.parsed];
        let hash = content_hash(&p.text);
        match &a.outcome {
            AnchorOutcome::Same { block_id } => {
                // Ordinal may still have moved: reordering is not rewriting, so
                // it updates position without an event or a revision bump.
                conn.execute(
                    "UPDATE blocks SET ordinal = ?2 WHERE id = ?1 AND ordinal != ?2",
                    params![block_id, p.ordinal as i64],
                )?;
            }
            AnchorOutcome::Revised { block_id, similarity, uncertain } => {
                let before = by_id.get(block_id.as_str());
                let revision = before.map(|b| b.revision).unwrap_or(0) + 1;
                conn.execute(
                    "UPDATE blocks SET ordinal = ?2, content_hash = ?3, text = ?4,
                            revision = ?5, updated_at = ?6 WHERE id = ?1",
                    params![block_id, p.ordinal as i64, hash, p.text, revision, now],
                )?;
                let note = if *uncertain {
                    format!(
                        "Matched at {:.0}% similarity, which is not certain enough to assert.",
                        similarity * 100.0
                    )
                } else {
                    format!("Matched at {:.0}% similarity.", similarity * 100.0)
                };
                // An uncertain anchor is inferred whatever the caller claimed:
                // if we had to guess which block this is, we did not observe it.
                let c = if *uncertain { Confidence::Inferred } else { confidence };
                provenance::append(
                    conn,
                    NewEvent::block(block_id, actor.clone(), Action::Rewrite, c)
                        .hashes(before.map(|b| b.content_hash.clone()), Some(hash))
                        .revision(revision)
                        .note(&note),
                    now,
                )?;
            }
            AnchorOutcome::New => {
                let id = format!("blk_{}", crate::project::new_id());
                conn.execute(
                    "INSERT INTO blocks (id, document_id, ordinal, kind, content_hash, text,
                                         revision, first_seen, updated_at)
                     VALUES (?1,?2,?3,?4,?5,?6,1,?7,?7)",
                    params![id, document_id, p.ordinal as i64, p.kind.as_str(), hash, p.text, now],
                )?;
                let action =
                    if actor.kind == provenance::ActorType::Tool { Action::Import } else { Action::Create };
                provenance::append(
                    conn,
                    NewEvent::block(&id, actor.clone(), action, confidence)
                        .hashes(None, Some(hash))
                        .revision(1),
                    now,
                )?;
            }
        }
    }

    for id in &plan.removed {
        conn.execute("UPDATE blocks SET removed_at = ?2 WHERE id = ?1", params![id, now])?;
        provenance::append(
            conn,
            NewEvent::block(id, actor.clone(), Action::Delete, confidence),
            now,
        )?;
    }

    conn.execute(
        "UPDATE documents SET body_hash = ?2 WHERE id = ?1",
        params![document_id, file_hash],
    )?;

    list(conn, document_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(id: &str, ordinal: i64, kind: BlockKind, text: &str) -> Block {
        Block {
            id: id.into(),
            document_id: "doc".into(),
            ordinal,
            kind,
            content_hash: content_hash(text),
            text: text.into(),
            revision: 1,
            first_seen: 0,
            updated_at: 0,
            removed_at: None,
        }
    }

    // --- BI-01 .. BI-04: normalisation and hashing -------------------------

    #[test]
    fn bi01_normalise_collapses_whitespace_and_trims() {
        assert_eq!(normalise("The well  was\n  dry."), "The well was dry.");
        assert_eq!(normalise("  padded  "), "padded");
        assert_eq!(normalise(""), "");
    }

    #[test]
    fn bi02_a_reflow_is_not_an_edit() {
        let one_line = "The well at Kharsa was the only honest thing in the town and it was going dry that season.";
        let wrapped = "The well at Kharsa was the only honest thing in the\ntown and it was going dry that season.";
        assert_eq!(
            content_hash(one_line),
            content_hash(wrapped),
            "hard-wrapping in an external editor must not read as a rewrite"
        );
    }

    #[test]
    fn bi03_surrounding_blank_lines_do_not_change_identity() {
        assert_eq!(content_hash("\n\n  Salt on the rim.  \n\n"), content_hash("Salt on the rim."));
    }

    #[test]
    fn bi04_punctuation_is_content() {
        assert_ne!(
            content_hash("She went back, without a word."),
            content_hash("She went back without a word."),
            "a comma changes the prose, so it changes the block"
        );
    }

    // --- BI-05 .. BI-13: splitting -----------------------------------------

    #[test]
    fn bi05_paragraphs_split_on_blank_lines() {
        let b = split("One.\n\nTwo.\n\nThree.");
        assert_eq!(b.len(), 3);
        assert!(b.iter().all(|x| x.kind == BlockKind::Paragraph));
        assert_eq!(b.iter().map(|x| x.ordinal).collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn bi06_a_heading_is_its_own_block() {
        let b = split("## Chapter Two\n\nThe road ran north.");
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].kind, BlockKind::Heading);
        assert_eq!(b[1].kind, BlockKind::Paragraph);
    }

    #[test]
    fn bi07_a_code_fence_containing_a_blank_line_is_one_block() {
        let b = split("Before.\n\n```rust\nlet a = 1;\n\nlet b = 2;\n```\n\nAfter.");
        assert_eq!(b.len(), 3, "{b:?}");
        assert_eq!(b[1].kind, BlockKind::Code);
        assert!(b[1].text.contains("let a = 1;") && b[1].text.contains("let b = 2;"));
    }

    #[test]
    fn bi08_a_run_of_quote_lines_is_one_block() {
        let b = split("> one\n> two\n> three\n> four");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].kind, BlockKind::Quote);
    }

    #[test]
    fn bi09_a_list_is_one_block_not_one_per_item() {
        let b = split("- a\n- b\n- c\n- d\n- e");
        assert_eq!(b.len(), 1, "an author moves and rewrites a list as a unit");
        assert_eq!(b[0].kind, BlockKind::List);
    }

    #[test]
    fn bi10_a_pipe_table_is_one_block() {
        let b = split("| a | b |\n| --- | --- |\n| 1 | 2 |");
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].kind, BlockKind::Table);
    }

    #[test]
    fn bi11_front_matter_only_counts_at_the_top() {
        let b = split("---\ntitle: x\n---\n\nProse.\n\n---\n\nMore.");
        assert_eq!(b[0].kind, BlockKind::FrontMatter);
        assert!(
            b.iter().skip(1).any(|x| x.kind == BlockKind::Rule),
            "a later --- is a rule, not a second front matter: {b:?}"
        );
        assert_eq!(b.iter().filter(|x| x.kind == BlockKind::FrontMatter).count(), 1);
    }

    #[test]
    fn bi07b_an_indented_code_block_is_code_not_prose() {
        // Markdown's other code syntax. Found because seed data with stray
        // indentation rendered as a listing in the editor while this parser
        // called it a paragraph — and `kind` is part of anchoring, so a block
        // that changes kind between parses cannot anchor to itself.
        let b = split("Before.\n\n    let a = 1;\n    let b = 2;\n\nAfter.");
        assert_eq!(b.len(), 3, "{b:?}");
        assert_eq!(b[1].kind, BlockKind::Code);
        assert!(b[1].text.contains("let a = 1;") && b[1].text.contains("let b = 2;"));
        assert_eq!(b[2].kind, BlockKind::Paragraph);
        assert_eq!(b[2].text.trim(), "After.");

        // A tab indent is the same construct.
        assert_eq!(split("\tcode here").first().map(|x| x.kind), Some(BlockKind::Code));

        // And a blank line inside an indented run keeps it together.
        let joined = split("    one\n\n    two\n\nProse.");
        assert_eq!(joined.len(), 2, "{joined:?}");
        assert_eq!(joined[0].kind, BlockKind::Code);
    }

    #[test]
    fn bi12_empty_input_gives_no_blocks_and_does_not_panic() {
        assert!(split("").is_empty());
        assert!(split("   \n\n  \n").is_empty());
    }

    #[test]
    fn bi13_crlf_parses_the_same_as_lf() {
        let lf = split("One.\n\nTwo.");
        let crlf = split("One.\r\n\r\nTwo.");
        assert_eq!(lf.len(), crlf.len());
        assert_eq!(content_hash(&lf[0].text), content_hash(&crlf[0].text));
        assert_eq!(content_hash(&lf[1].text), content_hash(&crlf[1].text));
    }

    // --- BI-14 .. BI-17: exact anchoring -----------------------------------

    const A: &str = "The well at Kharsa was the only honest thing in the town and it was going dry.";
    const B: &str = "She counted what she had left, and then she counted it again more slowly.";
    const C: &str = "Below them the salt went on being white in the dark, mile after mile of it.";

    #[test]
    fn bi14_an_unchanged_document_anchors_everything_and_changes_nothing() {
        let parsed = split(&format!("{A}\n\n{B}\n\n{C}"));
        let live = vec![
            known("b1", 0, BlockKind::Paragraph, A),
            known("b2", 1, BlockKind::Paragraph, B),
            known("b3", 2, BlockKind::Paragraph, C),
        ];
        let plan = anchor(&parsed, &live);
        assert!(plan.removed.is_empty());
        assert!(
            plan.anchors.iter().all(|a| matches!(a.outcome, AnchorOutcome::Same { .. })),
            "{:?}",
            plan.anchors
        );
    }

    #[test]
    fn bi15_inserting_a_paragraph_leaves_every_other_id_intact() {
        let parsed = split(&format!("{A}\n\nA newly written paragraph that did not exist before now.\n\n{B}"));
        let live = vec![
            known("b1", 0, BlockKind::Paragraph, A),
            known("b2", 1, BlockKind::Paragraph, B),
        ];
        let plan = anchor(&parsed, &live);
        assert_eq!(plan.anchors[0].outcome, AnchorOutcome::Same { block_id: "b1".into() });
        assert_eq!(plan.anchors[1].outcome, AnchorOutcome::New);
        assert_eq!(plan.anchors[2].outcome, AnchorOutcome::Same { block_id: "b2".into() });
        assert!(plan.removed.is_empty());
    }

    #[test]
    fn bi16_reordering_is_not_rewriting() {
        let parsed = split(&format!("{B}\n\n{A}"));
        let live = vec![
            known("b1", 0, BlockKind::Paragraph, A),
            known("b2", 1, BlockKind::Paragraph, B),
        ];
        let plan = anchor(&parsed, &live);
        assert_eq!(plan.anchors[0].outcome, AnchorOutcome::Same { block_id: "b2".into() });
        assert_eq!(plan.anchors[1].outcome, AnchorOutcome::Same { block_id: "b1".into() });
        assert!(plan.removed.is_empty());
    }

    #[test]
    fn bi17_identical_paragraphs_anchor_to_the_nearer_one() {
        // Two identical paragraphs; the second is edited. Without ordinal
        // proximity the edit would steal the first one's identity, and the two
        // would swap histories on every save.
        let dup = "They went over at first light and the river took two of the mules with it.";
        let edited = "They went over at first light and the river took three of the mules with it.";
        let parsed = split(&format!("{dup}\n\n{edited}"));
        let live = vec![
            known("first", 0, BlockKind::Paragraph, dup),
            known("second", 1, BlockKind::Paragraph, dup),
        ];
        let plan = anchor(&parsed, &live);
        assert_eq!(
            plan.anchors[0].outcome,
            AnchorOutcome::Same { block_id: "first".into() },
            "the untouched copy keeps the nearer id"
        );
        match &plan.anchors[1].outcome {
            AnchorOutcome::Revised { block_id, .. } => assert_eq!(block_id, "second"),
            other => panic!("the edited copy must revise the nearer block, got {other:?}"),
        }
    }

    // --- BI-18 .. BI-23: fuzzy anchoring -----------------------------------

    #[test]
    fn bi18_a_typo_fix_keeps_the_identity() {
        let before = "The rim of the basin held the last of the heat the way a kiln holds it after the fire is out.";
        let after = "The rim of the basin held the last of the heat the way a kiln holds it after the fire goes out.";
        let plan = anchor(&split(after), &[known("b1", 0, BlockKind::Paragraph, before)]);
        match &plan.anchors[0].outcome {
            AnchorOutcome::Revised { block_id, similarity, uncertain } => {
                assert_eq!(block_id, "b1");
                assert!(*similarity > CONFIDENT_THRESHOLD, "similarity {similarity}");
                assert!(!uncertain, "a typo fix is not an ambiguous match");
            }
            other => panic!("{other:?}"),
        }
        assert!(plan.removed.is_empty());
    }

    #[test]
    fn bi20_unrelated_prose_does_not_inherit_an_identity() {
        let before = "The rim of the basin held the last of the heat the way a kiln holds it.";
        let after = "Forty years later the tribunal published its findings in three sealed volumes.";
        let plan = anchor(&split(after), &[known("b1", 0, BlockKind::Paragraph, before)]);
        assert_eq!(
            plan.anchors[0].outcome,
            AnchorOutcome::New,
            "identity must not be forced onto text that shares nothing"
        );
        assert_eq!(plan.removed, vec!["b1"]);
    }

    #[test]
    fn bi23_short_blocks_are_never_fuzzy_matched() {
        // Two short headings share a word. Fuzzy matching would confidently
        // declare them the same block.
        let plan = anchor(
            &split("## The Descent"),
            &[known("h1", 0, BlockKind::Heading, "## The Crossing")],
        );
        assert_eq!(plan.anchors[0].outcome, AnchorOutcome::New);
        assert_eq!(plan.removed, vec!["h1"]);
    }

    #[test]
    fn bi_kinds_never_anchor_across_each_other() {
        // The same words as a paragraph and as a quote are different blocks.
        let text = "The well at Kharsa was the only honest thing in the town and it was going dry.";
        let plan = anchor(
            &split(&format!("> {text}")),
            &[known("p1", 0, BlockKind::Paragraph, text)],
        );
        assert_eq!(plan.anchors[0].outcome, AnchorOutcome::New);
    }

    // --- BI-24 .. BI-27: uncertainty ---------------------------------------

    #[test]
    fn bi25_a_close_runner_up_makes_a_strong_match_uncertain() {
        // Two known blocks that share a long preamble and differ only in their
        // final word. Anything resembling one resembles the other almost
        // exactly, so the winner is a coin toss wearing a confident number.
        let shared = "She counted the water in her head and got a different answer each time, \
                      and the road ran north out of the town and then it stopped pretending to";
        let one = format!("{shared} anyway.");
        let two = format!("{shared} entirely.");
        let now = format!("{shared} regardless.");

        let plan = anchor(
            &split(&now),
            &[
                known("b1", 0, BlockKind::Paragraph, &one),
                known("b2", 1, BlockKind::Paragraph, &two),
            ],
        );
        match &plan.anchors[0].outcome {
            AnchorOutcome::Revised { uncertain, similarity, .. } => assert!(
                *uncertain,
                "two candidates within the margin is a coin toss, not a match ({similarity})"
            ),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn bi26_a_merely_adequate_match_is_uncertain_even_with_no_rival() {
        let before = "The road ran north out of the town and then it stopped pretending to. Ruts and grit and a wheel rim someone had left where it fell.";
        let after = "The road ran north out of the town. She walked it anyway, counting the water in her head and getting a different answer each time.";
        let plan = anchor(&split(after), &[known("b1", 0, BlockKind::Paragraph, before)]);
        match &plan.anchors[0].outcome {
            AnchorOutcome::Revised { similarity, uncertain, .. } => {
                assert!(*similarity < CONFIDENT_THRESHOLD, "similarity {similarity}");
                assert!(*uncertain);
            }
            // Falling below the anchor threshold entirely is also acceptable
            // here; what must never happen is a confident claim.
            AnchorOutcome::New => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn anchoring_is_deterministic() {
        let doc = format!("{A}\n\n{B}\n\n{C}");
        let live = vec![
            known("b1", 0, BlockKind::Paragraph, A),
            known("b3", 1, BlockKind::Paragraph, C),
        ];
        let first = anchor(&split(&doc), &live);
        for _ in 0..20 {
            let again = anchor(&split(&doc), &live);
            assert_eq!(
                format!("{:?}", first.anchors),
                format!("{:?}", again.anchors),
                "the same inputs must always produce the same plan"
            );
        }
    }

    #[test]
    fn a_removed_block_is_not_a_candidate_for_anchoring() {
        let mut gone = known("dead", 0, BlockKind::Paragraph, A);
        gone.removed_at = Some(1);
        let plan = anchor(&split(A), &[gone]);
        assert_eq!(
            plan.anchors[0].outcome,
            AnchorOutcome::New,
            "retyping deleted text must not resurrect its id"
        );
        assert!(plan.removed.is_empty());
    }

    // --- NEG cases ---------------------------------------------------------

    #[test]
    fn neg05_an_unterminated_code_fence_does_not_hang() {
        let b = split("Before.\n\n```rust\nlet a = 1;\nno closing fence");
        assert_eq!(b.len(), 2);
        assert_eq!(b[1].kind, BlockKind::Code);
        assert!(b[1].text.contains("no closing fence"));
    }

    #[test]
    fn neg06_a_very_large_paragraph_completes() {
        let huge = "word ".repeat(200_000);
        let started = std::time::Instant::now();
        let b = split(&huge);
        let _ = content_hash(&b[0].text);
        assert_eq!(b.len(), 1);
        assert!(started.elapsed().as_secs() < 5, "splitting must not be quadratic");
    }

    #[test]
    fn neg10_a_line_of_markdown_syntax_is_a_rule_not_a_paragraph() {
        let b = split("Before.\n\n***\n\nAfter.");
        assert_eq!(b[1].kind, BlockKind::Rule);
        // And a rule is short, so it is never fuzzy-anchored.
        let plan = anchor(&split("***"), &[known("r1", 0, BlockKind::Rule, "---")]);
        assert_eq!(plan.anchors[0].outcome, AnchorOutcome::New);
    }

    // --- PERF --------------------------------------------------------------

    #[test]
    fn perf03_anchoring_does_not_grow_worse_than_quadratically() {
        // A guard against an algorithmic regression, not a benchmark.
        //
        // This was a 200ms wall-clock bound and it went flaky: the suite runs in
        // parallel alongside tests that drive a real model, so the machine is
        // loaded and a stopwatch measures the load as much as the code. The
        // failure it exists to catch — someone making anchoring project-wide, so
        // a 2,000-block book compares every pair — is a thousandfold change, not
        // a twofold one. So it is measured against doubled input rather than
        // against the clock, and the bound is generous enough to survive noise
        // while still catching anything superquadratic.
        let time_it = |n: usize| -> std::time::Duration {
            let known: Vec<Block> = (0..n)
                .map(|i| {
                    known(
                        &format!("b{i}"),
                        i as i64,
                        BlockKind::Paragraph,
                        &format!("Chapter {i}. The road ran north out of the town and then it stopped pretending to."),
                    )
                })
                .collect();
            let after: String = (0..n)
                .map(|i| format!("Section {i}. A completely different sentence about turbines and grain.\n\n"))
                .collect();
            let parsed = split(&after);
            let started = std::time::Instant::now();
            let plan = anchor(&parsed, &known);
            assert_eq!(plan.anchors.len(), n);
            started.elapsed().max(std::time::Duration::from_micros(1))
        };

        // Warm the caches so the first call does not carry the cost of both.
        time_it(30);
        let small = time_it(30);
        let large = time_it(60);

        let ratio = large.as_secs_f64() / small.as_secs_f64();
        assert!(
            ratio < 12.0,
            "doubling the blocks multiplied the work by {ratio:.1}× ({small:?} → {large:?}). \
             Quadratic predicts about 4×; anything near this is a different algorithm."
        );
        // And a sanity ceiling, loose enough to be about the code rather than
        // the machine: sixty blocks is a scene, not a book.
        assert!(large.as_secs() < 5, "sixty blocks took {large:?}");
    }
}
