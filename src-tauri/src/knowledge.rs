//! The knowledge layer: concepts, claims, evidence (K2P-OS §4).
//!
//! A source is a document. A **claim** is one atomic assertion that could turn
//! out to be wrong, pinned to the source that makes it. **Evidence** links a
//! claim to the passage supporting it. A **concept** is a node in the knowledge
//! graph that exists independently of any chapter.
//!
//! # Why claims rather than chunks
//!
//! RAG over chunks answers "what does the corpus say about X". It cannot answer
//! "do my sources disagree", because a chunk has no truth value. A claim does,
//! so contradictions become findable, gaps become nameable, and a paragraph can
//! cite the assertion it rests on instead of a page number.
//!
//! # The two-stage contradiction search
//!
//! Comparing every claim against every other is O(n²) model calls — at 400
//! claims that is 80,000 requests and hours of compute. Instead:
//!
//! 1. **Retrieve candidates cheaply.** Word-shingle overlap finds pairs that are
//!    *about the same thing*. Contradictory claims share their subject almost by
//!    definition — "X improves Y" and "X does not improve Y" overlap heavily.
//! 2. **Ask the model only about those.** Typically a few dozen pairs.
//!
//! This is a real trade: two claims that contradict each other in entirely
//! different words will not be surfaced. That limit is stated in the report
//! rather than hidden, because a contradiction check that silently misses things
//! while looking thorough is worse than one that says what it examined.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::{err, AppResult};

/// Shingle overlap above which two claims are worth asking a model about.
/// Low on purpose: this is a recall filter, and the model does the judging.
pub const CANDIDATE_THRESHOLD: f32 = 0.12;
/// Never send more than this many pairs to a model in one pass, so a large
/// corpus degrades into "checked the most similar pairs" rather than hanging.
pub const MAX_PAIRS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    Assertion,
    Definition,
    Method,
    Result,
}

impl ClaimKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ClaimKind::Assertion => "assertion",
            ClaimKind::Definition => "definition",
            ClaimKind::Method => "method",
            ClaimKind::Result => "result",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "definition" => ClaimKind::Definition,
            "method" => ClaimKind::Method,
            "result" => ClaimKind::Result,
            _ => ClaimKind::Assertion,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Concept {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub description: Option<String>,
    pub mentions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Claim {
    pub id: String,
    pub text: String,
    pub kind: ClaimKind,
    pub source_id: String,
    /// Verbatim excerpt from the source that carries this claim.
    pub excerpt: String,
    /// Concept ids this claim is about.
    pub concepts: Vec<String>,
    /// The model that extracted it. A claim is a model's reading of a source,
    /// and the report says so.
    pub extracted_by: String,
    pub at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    Supports,
    Contradicts,
    Refines,
}

impl Relation {
    pub fn as_str(self) -> &'static str {
        match self {
            Relation::Supports => "supports",
            Relation::Contradicts => "contradicts",
            Relation::Refines => "refines",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "supports" => Some(Relation::Supports),
            "contradicts" => Some(Relation::Contradicts),
            "refines" => Some(Relation::Refines),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimLink {
    pub from_claim: String,
    pub to_claim: String,
    pub relation: Relation,
    /// Why, in the model's words. A bare "contradicts" is not actionable.
    pub reason: String,
    pub found_by: String,
    pub at: i64,
}

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS concepts (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE COLLATE NOCASE,
            aliases     TEXT NOT NULL DEFAULT '[]',
            description TEXT
        );

        CREATE TABLE IF NOT EXISTS claims (
            id           TEXT PRIMARY KEY,
            text         TEXT NOT NULL,
            kind         TEXT NOT NULL,
            source_id    TEXT NOT NULL,
            excerpt      TEXT NOT NULL,
            extracted_by TEXT NOT NULL,
            at           INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS claims_source ON claims(source_id);

        CREATE TABLE IF NOT EXISTS claim_concepts (
            claim_id   TEXT NOT NULL,
            concept_id TEXT NOT NULL,
            PRIMARY KEY (claim_id, concept_id)
        );

        CREATE TABLE IF NOT EXISTS claim_links (
            from_claim TEXT NOT NULL,
            to_claim   TEXT NOT NULL,
            relation   TEXT NOT NULL,
            reason     TEXT NOT NULL,
            found_by   TEXT NOT NULL,
            at         INTEGER NOT NULL,
            PRIMARY KEY (from_claim, to_claim, relation)
        );

        -- A manuscript block resting on a claim. This is what makes a retracted
        -- source able to name the paragraphs that depend on it.
        CREATE TABLE IF NOT EXISTS block_claims (
            block_id TEXT NOT NULL,
            claim_id TEXT NOT NULL,
            PRIMARY KEY (block_id, claim_id)
        );
        ",
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Extraction
// ---------------------------------------------------------------------------

/// The extraction contract.
///
/// A **worked example** rather than a template of placeholders. `llama3.2:3b`
/// copied `"text": "one falsifiable assertion"` straight through when this was
/// phrased as a schema — a small model reads a placeholder as content to emit,
/// not as a slot to fill. Showing it a filled-in example and telling it not to
/// copy that example is what made extraction work at all.
const EXTRACT_SHAPE: &str = r#"Return ONE JSON object and nothing else. No prose, no code fence.

Here is the shape, filled in for a DIFFERENT passage. Copy the STRUCTURE, never the content:
{"concepts":[{"name":"Write-ahead logging","description":"recording changes before applying them"}],
 "claims":[{"text":"Write-ahead logging records a change before the change is applied","kind":"definition","excerpt":"every change is written to the log before it touches the table","concepts":["Write-ahead logging"]}]}

Now do the same for the passage above. Rules:
- "text" is one specific assertion the passage makes, written in your own words. It must name a real thing from the passage.
- "kind" is exactly one word: assertion, definition, method, or result.
- "excerpt" must be copied VERBATIM from the passage, word for word.
- Do not infer, do not summarise the whole passage, do not judge quality.
- Between 0 and 12 claims. If the passage asserts nothing checkable, return {"concepts":[],"claims":[]}."#;

/// What one extraction pass produced, before it is written down.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Extraction {
    pub concepts: Vec<Concept>,
    pub claims: Vec<Claim>,
    /// Claims the model produced that were thrown away, with the reason.
    pub dropped: Vec<String>,
}

#[derive(Deserialize)]
struct RawExtraction {
    #[serde(default)]
    concepts: Vec<RawConcept>,
    #[serde(default)]
    claims: Vec<RawClaim>,
}

#[derive(Deserialize)]
struct RawConcept {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Deserialize)]
struct RawClaim {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    excerpt: Option<String>,
    #[serde(default)]
    concepts: Vec<String>,
}

/// Loose containment check for "did this excerpt really come from the passage".
///
/// Not equality: models normalise whitespace, straighten quotes and drop
/// hyphens. Comparing on collapsed lowercase alphanumerics catches a fabricated
/// quotation without failing an honest one that lost a curly apostrophe.
/// Did the model hand back the prompt instead of an answer?
fn is_echoed_instruction(text: &str) -> bool {
    let lower = text.to_lowercase();
    [
        "falsifiable assertion",
        "one specific assertion",
        "assertion|definition",
        "verbatim from the passage",
        "in your own words",
        "concept name",
        "write-ahead logging records a change",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn appears_in(excerpt: &str, passage: &str) -> bool {
    let squash = |s: &str| -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
    };
    let needle = squash(excerpt);
    if needle.len() < 12 {
        return false; // too short to verify; treat as unverified
    }
    squash(passage).contains(&needle)
}

/// Turn a model's reply into concepts and claims, dropping anything it made up.
///
/// The verbatim check is the load-bearing part. A model asked for quotations
/// will occasionally produce a plausible sentence that is not in the source, and
/// a fabricated excerpt attached to a real paper is exactly the failure that
/// makes a research tool untrustworthy.
pub fn validate_extraction(
    reply: &str,
    passage: &str,
    source_id: &str,
    model: &str,
    now: i64,
) -> AppResult<Extraction> {
    let Some(json_text) = crate::diagrams::extract_json(reply) else {
        return err(format!(
            "{model} did not return claims. It replied: {}",
            reply.chars().take(200).collect::<String>()
        ));
    };
    let raw: RawExtraction = serde_json::from_str(json_text).map_err(|e| {
        crate::error::AppError(format!("{model} returned something that is not an extraction: {e}"))
    })?;

    let mut out = Extraction::default();
    let mut by_name: HashMap<String, String> = HashMap::new();

    for c in raw.concepts {
        let Some(name) = c.name.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()) else {
            continue;
        };
        let id = format!("cpt_{}", crate::project::slugify(&name));
        by_name.insert(name.to_lowercase(), id.clone());
        out.concepts.push(Concept {
            id,
            name,
            aliases: Vec::new(),
            description: c.description,
            mentions: 0,
        });
    }

    for c in raw.claims {
        let text = c.text.unwrap_or_default().trim().to_string();
        // Echo is checked before length. Most echoed placeholders are short, so
        // the length guard would catch them first and report the wrong reason —
        // and "too short" sends someone looking at the passage when the problem
        // is the model.
        if is_echoed_instruction(&text) {
            out.dropped.push(format!(
                "{text:?} — the model repeated the instructions instead of reading the passage"
            ));
            continue;
        }
        if text.split_whitespace().count() < 4 {
            out.dropped.push(format!("a claim too short to check: {text:?}"));
            continue;
        }
        let excerpt = c.excerpt.unwrap_or_default().trim().to_string();
        if !appears_in(&excerpt, passage) {
            out.dropped.push(format!(
                "\"{}\" — its quotation is not in the source",
                text.chars().take(70).collect::<String>()
            ));
            continue;
        }
        let concepts = c
            .concepts
            .iter()
            .filter_map(|n| by_name.get(&n.trim().to_lowercase()).cloned())
            .collect();
        out.claims.push(Claim {
            id: format!("clm_{}", crate::project::new_id()),
            text,
            kind: ClaimKind::parse(c.kind.as_deref().unwrap_or("assertion")),
            source_id: source_id.to_string(),
            excerpt,
            concepts,
            extracted_by: model.to_string(),
            at: now,
        });
    }
    Ok(out)
}

pub fn extraction_prompt(passage: &str) -> String {
    format!(
        "Extract the checkable claims from this passage, and the concepts they are about.\n\n\
         Passage:\n{passage}\n\n{EXTRACT_SHAPE}"
    )
}

/// Extract claims from one source, a window at a time.
pub fn extract_from_source(
    root: &std::path::Path,
    source_id: &str,
    model: &str,
    allow_cloud: bool,
    max_windows: usize,
) -> AppResult<Extraction> {
    let conn = crate::store::open(root)?;
    let Some(source) = crate::sources::get(&conn, source_id)? else {
        return err(format!("no source {source_id}"));
    };
    let text = crate::sources::read_text(root, &source)?;

    // Windows rather than the whole document: a claim needs its context nearby,
    // and a 40,000-word paper does not fit in a small model's attention.
    let windows = crate::rag::chunk(&text, 700, 80)?;
    let mut out = Extraction::default();
    let now = crate::project::now_ms();

    for window in windows.iter().take(max_windows) {
        let reply = crate::ollama::generate_guarded(
            model,
            &extraction_prompt(window),
            Some("You extract checkable claims as JSON. Never explain, never use a code fence."),
            1200,
            0.2,
            allow_cloud,
        )?;
        match validate_extraction(&reply, window, source_id, model, now) {
            Ok(part) => {
                out.concepts.extend(part.concepts);
                out.claims.extend(part.claims);
                out.dropped.extend(part.dropped);
            }
            // One bad window must not lose the whole document.
            Err(e) => out.dropped.push(format!("a window produced nothing usable: {e}")),
        }
    }

    // Concepts are shared across windows; the same name is the same node.
    out.concepts.sort_by(|a, b| a.id.cmp(&b.id));
    out.concepts.dedup_by(|a, b| a.id == b.id);

    save_extraction(&conn, &out)?;
    Ok(out)
}

pub fn save_extraction(conn: &Connection, e: &Extraction) -> AppResult<()> {
    for c in &e.concepts {
        conn.execute(
            "INSERT INTO concepts (id, name, aliases, description) VALUES (?1,?2,?3,?4)
             ON CONFLICT(id) DO UPDATE SET description = COALESCE(excluded.description, description)",
            params![c.id, c.name, serde_json::to_string(&c.aliases)?, c.description],
        )?;
    }
    for c in &e.claims {
        conn.execute(
            "INSERT OR REPLACE INTO claims (id, text, kind, source_id, excerpt, extracted_by, at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![c.id, c.text, c.kind.as_str(), c.source_id, c.excerpt, c.extracted_by, c.at],
        )?;
        for concept in &c.concepts {
            conn.execute(
                "INSERT OR IGNORE INTO claim_concepts (claim_id, concept_id) VALUES (?1,?2)",
                params![c.id, concept],
            )?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

fn row_to_claim(conn: &Connection, row: &rusqlite::Row<'_>) -> rusqlite::Result<Claim> {
    let id: String = row.get("id")?;
    let mut stmt = conn.prepare("SELECT concept_id FROM claim_concepts WHERE claim_id = ?1")?;
    let concepts = stmt
        .query_map(params![id], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Claim {
        id,
        text: row.get("text")?,
        kind: ClaimKind::parse(&row.get::<_, String>("kind")?),
        source_id: row.get("source_id")?,
        excerpt: row.get("excerpt")?,
        concepts,
        extracted_by: row.get("extracted_by")?,
        at: row.get("at")?,
    })
}

pub fn claims(conn: &Connection, source_id: Option<&str>) -> AppResult<Vec<Claim>> {
    let sql = match source_id {
        Some(_) => "SELECT id, text, kind, source_id, excerpt, extracted_by, at FROM claims WHERE source_id = ?1 ORDER BY at",
        None => "SELECT id, text, kind, source_id, excerpt, extracted_by, at FROM claims ORDER BY at",
    };
    let mut stmt = conn.prepare(sql)?;
    let mapped: Vec<Claim> = match source_id {
        Some(id) => stmt
            .query_map(params![id], |r| row_to_claim(conn, r))?
            .collect::<Result<Vec<_>, _>>()?,
        None => stmt
            .query_map([], |r| row_to_claim(conn, r))?
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(mapped)
}

pub fn concepts(conn: &Connection) -> AppResult<Vec<Concept>> {
    let mut stmt = conn.prepare(
        "SELECT c.id, c.name, c.aliases, c.description,
                (SELECT COUNT(*) FROM claim_concepts cc WHERE cc.concept_id = c.id) AS mentions
         FROM concepts c ORDER BY mentions DESC, name",
    )?;
    let rows = stmt.query_map([], |r| {
        let aliases: String = r.get("aliases")?;
        Ok(Concept {
            id: r.get("id")?,
            name: r.get("name")?,
            aliases: serde_json::from_str(&aliases).unwrap_or_default(),
            description: r.get("description")?,
            mentions: r.get("mentions")?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn links(conn: &Connection) -> AppResult<Vec<ClaimLink>> {
    let mut stmt = conn.prepare(
        "SELECT from_claim, to_claim, relation, reason, found_by, at FROM claim_links ORDER BY at",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(ClaimLink {
            from_claim: r.get(0)?,
            to_claim: r.get(1)?,
            relation: Relation::parse(&r.get::<_, String>(2)?).unwrap_or(Relation::Supports),
            reason: r.get(3)?,
            found_by: r.get(4)?,
            at: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// ---------------------------------------------------------------------------
// Contradictions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidatePair {
    pub a: String,
    pub b: String,
    pub overlap: f32,
}

/// Claim pairs worth asking a model about: about the same thing, and from
/// different sources.
///
/// Same-source pairs are excluded because a paper restating itself is not a
/// disagreement, and including them would fill the candidate budget with noise.
pub fn candidate_pairs(claims: &[Claim]) -> Vec<CandidatePair> {
    let shingled: Vec<HashSet<String>> =
        claims.iter().map(|c| crate::integrity::shingles(&c.text, 2)).collect();
    let mut out = Vec::new();
    for i in 0..claims.len() {
        for j in (i + 1)..claims.len() {
            if claims[i].source_id == claims[j].source_id {
                continue;
            }
            let overlap = crate::integrity::jaccard(&shingled[i], &shingled[j]);
            if overlap >= CANDIDATE_THRESHOLD {
                out.push(CandidatePair {
                    a: claims[i].id.clone(),
                    b: claims[j].id.clone(),
                    overlap,
                });
            }
        }
    }
    out.sort_by(|x, y| y.overlap.total_cmp(&x.overlap));
    out.truncate(MAX_PAIRS);
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContradictionReport {
    pub links: Vec<ClaimLink>,
    /// How many pairs were actually examined, so a reader knows the scope.
    pub examined: usize,
    /// Total claims in the corpus.
    pub claims: usize,
    /// The method's own limit, stated in the report rather than left implicit.
    pub method: String,
}

fn judge_prompt(a: &Claim, b: &Claim) -> String {
    format!(
        "Two claims from different sources. Decide their relationship.\n\n\
         A: {}\nB: {}\n\n\
         Answer with ONE JSON object and nothing else:\n\
         {{\"relation\": \"contradicts|supports|refines|unrelated\", \"reason\": \"one sentence\"}}\n\
         Use \"contradicts\" only when both cannot be true of the same thing at the same time. \
         Differing scope, differing conditions, or differing wording is \"refines\" or \
         \"unrelated\", not a contradiction.",
        a.text, b.text
    )
}

/// Find contradictions between claims from different sources.
pub fn find_contradictions(
    root: &std::path::Path,
    model: &str,
    allow_cloud: bool,
) -> AppResult<ContradictionReport> {
    let conn = crate::store::open(root)?;
    let all = claims(&conn, None)?;
    let by_id: HashMap<&str, &Claim> = all.iter().map(|c| (c.id.as_str(), c)).collect();
    let pairs = candidate_pairs(&all);
    let now = crate::project::now_ms();
    let mut found = Vec::new();

    for pair in &pairs {
        let (Some(a), Some(b)) = (by_id.get(pair.a.as_str()), by_id.get(pair.b.as_str())) else {
            continue;
        };
        let Ok(reply) = crate::ollama::generate_guarded(
            model,
            &judge_prompt(a, b),
            Some("You judge whether two claims conflict. Return only JSON."),
            300,
            0.1,
            allow_cloud,
        ) else {
            continue;
        };
        let Some(text) = crate::diagrams::extract_json(&reply) else { continue };
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) else { continue };
        let Some(relation) = parsed["relation"].as_str().and_then(Relation::parse) else {
            continue; // "unrelated", or nonsense — either way, nothing to record
        };
        let reason = parsed["reason"].as_str().unwrap_or("").trim().to_string();
        if reason.is_empty() {
            continue; // a verdict with no reason is not actionable
        }
        let link = ClaimLink {
            from_claim: a.id.clone(),
            to_claim: b.id.clone(),
            relation,
            reason,
            found_by: model.to_string(),
            at: now,
        };
        conn.execute(
            "INSERT OR REPLACE INTO claim_links (from_claim, to_claim, relation, reason, found_by, at)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![link.from_claim, link.to_claim, link.relation.as_str(), link.reason, link.found_by, link.at],
        )?;
        found.push(link);
    }

    Ok(ContradictionReport {
        links: found,
        examined: pairs.len(),
        claims: all.len(),
        method: format!(
            "Claims from different sources that share wording were compared by {model}. \
             Pairs below {:.0}% overlap were not examined, so two claims that disagree in \
             entirely different words would not be found here.",
            CANDIDATE_THRESHOLD * 100.0
        ),
    })
}

// ---------------------------------------------------------------------------
// Gaps
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gap {
    pub kind: String,
    pub subject: String,
    pub detail: String,
}

/// Where the knowledge base is thin. All arithmetic, no model.
pub fn gaps(conn: &Connection) -> AppResult<Vec<Gap>> {
    let mut out = Vec::new();

    // A concept nothing asserts anything about is a heading with no content.
    for c in concepts(conn)? {
        if c.mentions == 0 {
            out.push(Gap {
                kind: "unsupported_concept".into(),
                subject: c.name.clone(),
                detail: format!("`{}` is named but no claim is about it.", c.name),
            });
        } else if c.mentions == 1 {
            out.push(Gap {
                kind: "single_source_concept".into(),
                subject: c.name.clone(),
                detail: format!(
                    "`{}` rests on one claim only. A second source would make it defensible.",
                    c.name
                ),
            });
        }
    }

    // A source registered but never mined is work the author paid for and did
    // not get.
    let mut stmt = conn.prepare(
        "SELECT s.id, s.title FROM sources s
         WHERE NOT EXISTS (SELECT 1 FROM claims c WHERE c.source_id = s.id)",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (_, title) = row?;
        out.push(Gap {
            kind: "unmined_source".into(),
            subject: title.clone(),
            detail: format!("`{title}` is registered but no claims have been extracted from it."),
        });
    }

    // A claim nothing in the manuscript uses.
    let unused: i64 = conn.query_row(
        "SELECT COUNT(*) FROM claims c WHERE NOT EXISTS
            (SELECT 1 FROM block_claims b WHERE b.claim_id = c.id)",
        [],
        |r| r.get(0),
    )?;
    if unused > 0 {
        out.push(Gap {
            kind: "unused_claims".into(),
            subject: format!("{unused} claims"),
            detail: format!("{unused} extracted claims are not cited by any paragraph yet."),
        });
    }

    Ok(out)
}

/// Record that a manuscript block rests on a claim.
pub fn cite(conn: &Connection, block_id: &str, claim_id: &str, now: i64) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO block_claims (block_id, claim_id) VALUES (?1,?2)",
        params![block_id, claim_id],
    )?;
    crate::provenance::append(
        conn,
        crate::provenance::NewEvent::block(
            block_id,
            crate::provenance::Actor::human(),
            crate::provenance::Action::Verify,
            crate::provenance::Confidence::Observed,
        )
        .sources(vec![claim_id.to_string()])
        .note("Cited a claim"),
        now,
    )?;
    Ok(())
}

/// Which manuscript blocks depend on a source. The retraction query: if a paper
/// is withdrawn, this names every paragraph that rests on it.
pub fn blocks_depending_on(conn: &Connection, source_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT bc.block_id FROM block_claims bc
         JOIN claims c ON c.id = bc.claim_id WHERE c.source_id = ?1",
    )?;
    let rows = stmt.query_map(params![source_id], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PASSAGE: &str = "Logical replication is a method of replicating data objects and their \
        changes, based upon their replication identity. We found that symbolic constraints \
        improve consistency by a wide margin in every configuration we tested.";

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE documents (id TEXT PRIMARY KEY);").unwrap();
        crate::provenance::migrate(&conn).unwrap();
        crate::sources::migrate(&conn).unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn claim(id: &str, text: &str, source: &str) -> Claim {
        Claim {
            id: id.into(),
            text: text.into(),
            kind: ClaimKind::Assertion,
            source_id: source.into(),
            excerpt: text.into(),
            concepts: Vec::new(),
            extracted_by: "test".into(),
            at: 0,
        }
    }

    #[test]
    fn a_fabricated_quotation_is_dropped_and_named() {
        let reply = r#"{"concepts":[{"name":"Logical replication"}],
          "claims":[
            {"text":"Symbolic constraints improve consistency","kind":"result",
             "excerpt":"symbolic constraints improve consistency by a wide margin","concepts":["Logical replication"]},
            {"text":"The authors recommend abandoning the approach entirely","kind":"assertion",
             "excerpt":"we recommend abandoning the approach entirely","concepts":[]}
          ]}"#;
        let out = validate_extraction(reply, PASSAGE, "src-1", "test-model", 1).unwrap();

        assert_eq!(out.claims.len(), 1, "only the real quotation survives");
        assert_eq!(out.claims[0].source_id, "src-1");
        assert_eq!(out.claims[0].kind, ClaimKind::Result);
        assert_eq!(out.claims[0].concepts, vec!["cpt_logical-replication"]);
        assert_eq!(out.dropped.len(), 1);
        assert!(
            out.dropped[0].contains("not in the source"),
            "an invented quotation must be named, not silently kept: {:?}",
            out.dropped
        );
    }

    #[test]
    fn an_excerpt_that_differs_only_in_punctuation_still_verifies() {
        // Models straighten quotes and normalise whitespace. Failing an honest
        // quotation for that would drop most of what they correctly extract.
        let reply = r#"{"claims":[{"text":"Logical replication uses replication identity",
          "excerpt":"Logical  replication is a method of replicating data objects — and their changes","concepts":[]}]}"#;
        let out = validate_extraction(reply, PASSAGE, "s", "m", 1).unwrap();
        assert_eq!(out.claims.len(), 1, "{:?}", out.dropped);
    }

    #[test]
    fn prose_where_claims_should_be_is_refused() {
        let message = validate_extraction("I'd be happy to help you analyse this passage!", PASSAGE, "s", "m", 1)
            .unwrap_err()
            .to_string();
        assert!(message.contains("did not return claims"), "{message}");
    }

    #[test]
    fn an_echoed_instruction_is_caught_rather_than_stored_as_a_claim() {
        // Verbatim from a live `llama3.2:3b` run: it emitted the placeholder
        // text from the prompt as if it were a finding. It reads like a claim
        // until you look at it.
        let reply = r#"{"claims":[
            {"text":"one falsifiable assertion","kind":"assertion",
             "excerpt":"Logical replication is a method of replicating data objects","concepts":[]},
            {"text":"Logical replication is based on replication identity","kind":"definition",
             "excerpt":"based upon their replication identity","concepts":[]}
        ]}"#;
        let out = validate_extraction(reply, PASSAGE, "s", "m", 1).unwrap();
        assert_eq!(out.claims.len(), 1, "only the real claim survives: {:?}", out.dropped);
        assert!(out.claims[0].text.contains("replication identity"));
        assert!(
            out.dropped.iter().any(|d| d.contains("repeated the instructions")),
            "{:?}",
            out.dropped
        );
    }

    #[test]
    fn a_passage_with_nothing_checkable_yields_nothing_rather_than_padding() {
        let out = validate_extraction(r#"{"concepts":[],"claims":[]}"#, PASSAGE, "s", "m", 1).unwrap();
        assert!(out.claims.is_empty() && out.concepts.is_empty() && out.dropped.is_empty());
    }

    #[test]
    fn candidate_pairs_look_across_sources_and_not_within_one() {
        let claims = vec![
            claim("c1", "Symbolic constraints improve consistency in every configuration", "src-a"),
            claim("c2", "Symbolic constraints do not improve consistency in any configuration", "src-b"),
            claim("c3", "Symbolic constraints improve consistency in every configuration tested", "src-a"),
            claim("c4", "The wheat harvest failed in the third year of the drought", "src-c"),
        ];
        let pairs = candidate_pairs(&claims);

        assert!(
            pairs.iter().any(|p| (p.a == "c1" && p.b == "c2") || (p.a == "c2" && p.b == "c1")),
            "the disagreeing pair must be a candidate: {pairs:?}"
        );
        assert!(
            !pairs.iter().any(|p| p.a == "c1" && p.b == "c3"),
            "a paper restating itself is not a disagreement"
        );
        assert!(
            !pairs.iter().any(|p| p.a.contains("c4") || p.b.contains("c4")),
            "unrelated claims are not worth a model call: {pairs:?}"
        );
    }

    #[test]
    fn the_candidate_budget_is_bounded() {
        let many: Vec<Claim> = (0..300)
            .map(|i| {
                claim(
                    &format!("c{i}"),
                    "Symbolic constraints improve consistency in every configuration we tested",
                    &format!("src-{i}"),
                )
            })
            .collect();
        let pairs = candidate_pairs(&many);
        assert!(pairs.len() <= MAX_PAIRS, "a large corpus must degrade, not hang: {}", pairs.len());
        // And the strongest overlaps are the ones kept.
        assert!(pairs.windows(2).all(|w| w[0].overlap >= w[1].overlap));
    }

    #[test]
    fn gaps_name_what_is_thin_without_asking_a_model() {
        let conn = db();
        conn.execute(
            "INSERT INTO sources (id, kind, title, content_hash, added_at)
             VALUES ('s1','pdf','An Unmined Paper','h',1)",
            [],
        )
        .unwrap();
        save_extraction(
            &conn,
            &Extraction {
                concepts: vec![
                    Concept { id: "cpt_a".into(), name: "Alpha".into(), aliases: vec![], description: None, mentions: 0 },
                    Concept { id: "cpt_b".into(), name: "Beta".into(), aliases: vec![], description: None, mentions: 0 },
                ],
                claims: vec![Claim { concepts: vec!["cpt_a".into()], ..claim("c1", "Alpha does a thing", "s2") }],
                dropped: vec![],
            },
        )
        .unwrap();

        let found = gaps(&conn).unwrap();
        let kinds: Vec<&str> = found.iter().map(|g| g.kind.as_str()).collect();
        assert!(kinds.contains(&"unsupported_concept"), "{found:?}");
        assert!(kinds.contains(&"single_source_concept"), "{found:?}");
        assert!(kinds.contains(&"unmined_source"), "{found:?}");
        assert!(kinds.contains(&"unused_claims"), "{found:?}");
        assert!(found.iter().any(|g| g.subject == "Beta" && g.kind == "unsupported_concept"));
    }

    #[test]
    fn a_retracted_source_can_name_the_paragraphs_that_rest_on_it() {
        let conn = db();
        save_extraction(
            &conn,
            &Extraction {
                concepts: vec![],
                claims: vec![claim("c1", "A thing is true", "src-doomed"), claim("c2", "Another thing", "src-fine")],
                dropped: vec![],
            },
        )
        .unwrap();
        cite(&conn, "blk_1", "c1", 1).unwrap();
        cite(&conn, "blk_2", "c1", 2).unwrap();
        cite(&conn, "blk_3", "c2", 3).unwrap();

        let affected = blocks_depending_on(&conn, "src-doomed").unwrap();
        assert_eq!(affected.len(), 2);
        assert!(affected.contains(&"blk_1".to_string()) && affected.contains(&"blk_2".to_string()));
        assert!(!affected.contains(&"blk_3".to_string()));

        // And citing a claim is itself an event.
        assert_eq!(crate::provenance::history(&conn, "blk_1").unwrap()[0].source_refs, vec!["c1"]);
    }

    #[test]
    fn extraction_survives_the_round_trip_to_sqlite() {
        let conn = db();
        let out = validate_extraction(
            r#"{"concepts":[{"name":"Replication","description":"copying data"}],
                "claims":[{"text":"Logical replication uses replication identity","kind":"definition",
                           "excerpt":"based upon their replication identity","concepts":["Replication"]}]}"#,
            PASSAGE,
            "src-1",
            "test-model",
            5,
        )
        .unwrap();
        save_extraction(&conn, &out).unwrap();

        let back = claims(&conn, Some("src-1")).unwrap();
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].kind, ClaimKind::Definition);
        assert_eq!(back[0].concepts, vec!["cpt_replication"]);
        assert_eq!(back[0].extracted_by, "test-model");

        let cpts = concepts(&conn).unwrap();
        assert_eq!(cpts.len(), 1);
        assert_eq!(cpts[0].mentions, 1);
        assert_eq!(cpts[0].description.as_deref(), Some("copying data"));
    }
}
