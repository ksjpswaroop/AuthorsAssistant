//! The provenance ledger (K2P-OS §7).
//!
//! An append-only, hash-chained record of who or what created, changed,
//! accepted, imported or exported every block. Design: `docs/BLOCK-IDENTITY.md`
//! §3. Tests: `docs/TEST-PLAN-BLOCKS.md`, cases `PV-*`.
//!
//! # Why this exists
//!
//! Phase 4 cut AI-writing detection because inferring authorship from prose
//! statistics is unreliable and unfair to the person accused. This is the
//! honest version of the same goal: authorship is **recorded as it happens**
//! rather than guessed afterwards, and where there is no record the report says
//! there is no record — never that a human wrote it.
//!
//! # Three properties, and the limits of each
//!
//! - **Append-only.** There is no update and no delete function. A correction
//!   is a new event superseding an older one.
//! - **Hash-chained.** Each entry hashes the previous one, so a retroactive
//!   edit or a deleted row is detectable. This is tamper-*evident*, not
//!   tamper-*proof*: anyone who can write `project.sqlite` can rebuild the whole
//!   chain. Signing is deliberately out of scope.
//! - **Honest about confidence.** An event AegisDraft watched happen is
//!   `Observed`. One reconstructed by matching text after an external edit is
//!   `Inferred`. Treating those as the same fact would make the ledger
//!   confidently wrong, which is worse than incomplete.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::AppResult;

/// Separator for the hash preimage. A control byte that cannot occur in a hex
/// hash, a UUID, or any of our enum spellings, so no combination of field
/// values can be arranged to collide with a different combination.
const SEP: u8 = 0x1f;

pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// A continuous editing session by one actor is one event, not one per save.
///
/// The editor autosaves every 800ms. Without this, an hour of work on a single
/// paragraph writes about 4,500 `rewrite` rows, and a book-length project runs
/// to millions — an unreadable lineage and a database that grows without bound
/// for ordinary writing.
///
/// Coalescing does **not** rewrite history: nothing is updated or deleted, so
/// the hash chain is untouched. Some appends simply do not happen. The block row
/// still counts every save in its `revision`, and the recorded event carries the
/// revision it started from, so "rewritten by the author, revisions 3 to 47" is
/// still answerable — it is just one row instead of forty-four.
pub const COALESCE_WINDOW_MS: i64 = 120_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorType {
    Human,
    AiModel,
    Tool,
    External,
}

impl ActorType {
    pub fn as_str(self) -> &'static str {
        match self {
            ActorType::Human => "human",
            ActorType::AiModel => "ai_model",
            ActorType::Tool => "tool",
            ActorType::External => "external",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "ai_model" => ActorType::AiModel,
            "tool" => ActorType::Tool,
            "external" => ActorType::External,
            _ => ActorType::Human,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub kind: ActorType,
    pub id: String,
    pub model: Option<String>,
    pub params: Option<Value>,
}

impl Actor {
    pub fn human() -> Self {
        Actor { kind: ActorType::Human, id: "author".into(), model: None, params: None }
    }
    pub fn model(name: &str, params: Value) -> Self {
        Actor {
            kind: ActorType::AiModel,
            id: name.to_string(),
            model: Some(name.to_string()),
            params: Some(params),
        }
    }
    pub fn tool(id: &str) -> Self {
        Actor { kind: ActorType::Tool, id: id.to_string(), model: None, params: None }
    }
    /// An edit made outside AegisDraft. Never observed, only reconstructed.
    pub fn external() -> Self {
        Actor { kind: ActorType::External, id: "external_edit".into(), model: None, params: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// AegisDraft watched this happen.
    Observed,
    /// Reconstructed by matching text after the fact.
    Inferred,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Observed => "observed",
            Confidence::Inferred => "inferred",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "inferred" { Confidence::Inferred } else { Confidence::Observed }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Create,
    Rewrite,
    Delete,
    Accept,
    Reject,
    Import,
    Transcribe,
    Export,
    Restore,
    Verify,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Rewrite => "rewrite",
            Action::Delete => "delete",
            Action::Accept => "accept",
            Action::Reject => "reject",
            Action::Import => "import",
            Action::Transcribe => "transcribe",
            Action::Export => "export",
            Action::Restore => "restore",
            Action::Verify => "verify",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "rewrite" => Action::Rewrite,
            "delete" => Action::Delete,
            "accept" => Action::Accept,
            "reject" => Action::Reject,
            "import" => Action::Import,
            "transcribe" => Action::Transcribe,
            "export" => Action::Export,
            "restore" => Action::Restore,
            "verify" => Action::Verify,
            _ => Action::Create,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disclosure {
    Private,
    Collaborator,
    Institution,
    Public,
}

impl Disclosure {
    pub fn as_str(self) -> &'static str {
        match self {
            Disclosure::Private => "private",
            Disclosure::Collaborator => "collaborator",
            Disclosure::Institution => "institution",
            Disclosure::Public => "public",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "collaborator" => Disclosure::Collaborator,
            "institution" => Disclosure::Institution,
            "public" => Disclosure::Public,
            _ => Disclosure::Private,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub id: String,
    pub seq: i64,
    pub object_type: String,
    pub object_id: String,
    pub actor_type: ActorType,
    pub actor_id: String,
    pub action: Action,
    pub confidence: Confidence,
    pub parent_revision: Option<i64>,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
    pub model: Option<String>,
    pub model_params: Option<String>,
    pub tool_id: Option<String>,
    pub source_refs: Vec<String>,
    pub note: Option<String>,
    pub at: i64,
    pub disclosure: Disclosure,
    pub prev_hash: String,
    pub entry_hash: String,
}

#[derive(Debug, Clone)]
pub struct NewEvent {
    pub object_type: String,
    pub object_id: String,
    pub actor: Actor,
    pub action: Action,
    pub confidence: Confidence,
    pub parent_revision: Option<i64>,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
    pub source_refs: Vec<String>,
    pub note: Option<String>,
    pub disclosure: Disclosure,
}

impl NewEvent {
    /// The common case: something happened to a block.
    pub fn block(id: &str, actor: Actor, action: Action, confidence: Confidence) -> Self {
        NewEvent {
            object_type: "block".into(),
            object_id: id.to_string(),
            actor,
            action,
            confidence,
            parent_revision: None,
            input_hash: None,
            output_hash: None,
            source_refs: Vec::new(),
            note: None,
            disclosure: Disclosure::Private,
        }
    }
    pub fn about(mut self, object_type: &str, id: &str) -> Self {
        self.object_type = object_type.to_string();
        self.object_id = id.to_string();
        self
    }
    pub fn hashes(mut self, input: Option<String>, output: Option<String>) -> Self {
        self.input_hash = input;
        self.output_hash = output;
        self
    }
    pub fn revision(mut self, revision: i64) -> Self {
        self.parent_revision = Some(revision);
        self
    }
    pub fn note(mut self, note: &str) -> Self {
        self.note = Some(note.to_string());
        self
    }
    pub fn sources(mut self, refs: Vec<String>) -> Self {
        self.source_refs = refs;
        self
    }
}

// ---------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------

pub fn migrate(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS blocks (
            id           TEXT PRIMARY KEY,
            document_id  TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            ordinal      INTEGER NOT NULL,
            kind         TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            text         TEXT NOT NULL,
            revision     INTEGER NOT NULL,
            first_seen   INTEGER NOT NULL,
            updated_at   INTEGER NOT NULL,
            removed_at   INTEGER
        );
        CREATE INDEX IF NOT EXISTS blocks_doc ON blocks(document_id, ordinal);
        CREATE INDEX IF NOT EXISTS blocks_hash ON blocks(content_hash);

        CREATE TABLE IF NOT EXISTS events (
            id              TEXT PRIMARY KEY,
            seq             INTEGER NOT NULL UNIQUE,
            object_type     TEXT NOT NULL,
            object_id       TEXT NOT NULL,
            actor_type      TEXT NOT NULL,
            actor_id        TEXT NOT NULL,
            action          TEXT NOT NULL,
            confidence      TEXT NOT NULL,
            parent_revision INTEGER,
            input_hash      TEXT,
            output_hash     TEXT,
            model           TEXT,
            model_params    TEXT,
            tool_id         TEXT,
            source_refs     TEXT NOT NULL DEFAULT '[]',
            note            TEXT,
            at              INTEGER NOT NULL,
            disclosure      TEXT NOT NULL,
            prev_hash       TEXT NOT NULL,
            entry_hash      TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS events_object ON events(object_type, object_id, seq);
        ",
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Appending
// ---------------------------------------------------------------------------

/// The hash preimage. `seq` participates so two events with otherwise identical
/// fields still differ, and the separator cannot appear inside any field.
fn compute_hash(prev_hash: &str, seq: i64, e: &NewEvent, at: i64) -> String {
    let mut hasher = Sha256::new();
    for part in [
        prev_hash,
        &seq.to_string(),
        &e.object_type,
        &e.object_id,
        e.actor.kind.as_str(),
        &e.actor.id,
        e.action.as_str(),
        e.confidence.as_str(),
        e.output_hash.as_deref().unwrap_or(""),
        &at.to_string(),
    ] {
        hasher.update(part.as_bytes());
        hasher.update([SEP]);
    }
    format!("{:x}", hasher.finalize())
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<Event> {
    let refs: String = row.get("source_refs")?;
    Ok(Event {
        id: row.get("id")?,
        seq: row.get("seq")?,
        object_type: row.get("object_type")?,
        object_id: row.get("object_id")?,
        actor_type: ActorType::parse(&row.get::<_, String>("actor_type")?),
        actor_id: row.get("actor_id")?,
        action: Action::parse(&row.get::<_, String>("action")?),
        confidence: Confidence::parse(&row.get::<_, String>("confidence")?),
        parent_revision: row.get("parent_revision")?,
        input_hash: row.get("input_hash")?,
        output_hash: row.get("output_hash")?,
        model: row.get("model")?,
        model_params: row.get("model_params")?,
        tool_id: row.get("tool_id")?,
        source_refs: serde_json::from_str(&refs).unwrap_or_default(),
        note: row.get("note")?,
        at: row.get("at")?,
        disclosure: Disclosure::parse(&row.get::<_, String>("disclosure")?),
        prev_hash: row.get("prev_hash")?,
        entry_hash: row.get("entry_hash")?,
    })
}

/// Would appending this merely restate an edit already recorded?
///
/// True only for a `rewrite` continuing the same actor's session on the same
/// block, inside the window. Everything else — a different actor, a different
/// confidence, a create, an accept, a delete — is always its own event. A
/// model's edit following the author's must never disappear into the author's
/// session, which is the whole reason this check is this narrow.
fn continues_a_session(conn: &Connection, event: &NewEvent, now: i64) -> AppResult<bool> {
    if event.action != Action::Rewrite {
        return Ok(false);
    }
    let last: Option<(String, String, String, String, i64)> = conn
        .query_row(
            "SELECT action, actor_type, actor_id, confidence, at FROM events
             WHERE object_id = ?1 ORDER BY seq DESC LIMIT 1",
            params![event.object_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .ok();
    let Some((action, actor_type, actor_id, confidence, at)) = last else {
        return Ok(false);
    };
    Ok(action == Action::Rewrite.as_str()
        && actor_type == event.actor.kind.as_str()
        && actor_id == event.actor.id
        && confidence == event.confidence.as_str()
        && now.saturating_sub(at) < COALESCE_WINDOW_MS)
}

/// Append one event, chaining it to the last. The only write path there is.
///
/// Returns the existing event when this one merely continues a session already
/// recorded — see [`COALESCE_WINDOW_MS`].
pub fn append(conn: &Connection, event: NewEvent, now: i64) -> AppResult<Event> {
    if continues_a_session(conn, &event, now)? {
        if let Some(existing) = history(conn, &event.object_id)?.pop() {
            return Ok(existing);
        }
    }
    append_always(conn, event, now)
}

/// Append unconditionally, without the session check. For callers that have
/// already decided this is a distinct act.
pub fn append_always(conn: &Connection, event: NewEvent, now: i64) -> AppResult<Event> {
    let (last_seq, prev_hash): (i64, String) = conn
        .query_row(
            "SELECT seq, entry_hash FROM events ORDER BY seq DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, GENESIS.to_string()));

    let seq = last_seq + 1;
    let entry_hash = compute_hash(&prev_hash, seq, &event, now);
    let id = format!("evt_{}", uuid::Uuid::new_v4());
    let refs = serde_json::to_string(&event.source_refs)?;
    let params_json = event.actor.params.as_ref().map(|p| p.to_string());
    let tool_id = matches!(event.actor.kind, ActorType::Tool).then(|| event.actor.id.clone());

    conn.execute(
        "INSERT INTO events (
            id, seq, object_type, object_id, actor_type, actor_id, action, confidence,
            parent_revision, input_hash, output_hash, model, model_params, tool_id,
            source_refs, note, at, disclosure, prev_hash, entry_hash
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        params![
            id,
            seq,
            event.object_type,
            event.object_id,
            event.actor.kind.as_str(),
            event.actor.id,
            event.action.as_str(),
            event.confidence.as_str(),
            event.parent_revision,
            event.input_hash,
            event.output_hash,
            event.actor.model,
            params_json,
            tool_id,
            refs,
            event.note,
            now,
            event.disclosure.as_str(),
            prev_hash,
            entry_hash,
        ],
    )?;

    Ok(Event {
        id,
        seq,
        object_type: event.object_type,
        object_id: event.object_id,
        actor_type: event.actor.kind,
        actor_id: event.actor.id,
        action: event.action,
        confidence: event.confidence,
        parent_revision: event.parent_revision,
        input_hash: event.input_hash,
        output_hash: event.output_hash,
        model: event.actor.model,
        model_params: params_json,
        tool_id,
        source_refs: event.source_refs,
        note: event.note,
        at: now,
        disclosure: event.disclosure,
        prev_hash,
        entry_hash,
    })
}

// ---------------------------------------------------------------------------
// Verification and reading
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainBreak {
    pub seq: i64,
    pub expected: String,
    pub found: String,
    /// What a person should understand from this, in words.
    pub detail: String,
}

/// Walk the chain. `None` means intact.
///
/// Streams row by row rather than loading the ledger, so a project with a
/// hundred thousand events verifies in constant memory.
pub fn verify_chain(conn: &Connection) -> AppResult<Option<ChainBreak>> {
    let mut stmt = conn.prepare(
        "SELECT id, seq, object_type, object_id, actor_type, actor_id, action, confidence,
                parent_revision, input_hash, output_hash, model, model_params, tool_id,
                source_refs, note, at, disclosure, prev_hash, entry_hash
         FROM events ORDER BY seq",
    )?;
    let rows = stmt.query_map([], row_to_event)?;

    let mut expected_prev = GENESIS.to_string();

    for (expected_seq, row) in (1i64..).zip(rows) {
        let e = row?;
        if e.seq != expected_seq {
            return Ok(Some(ChainBreak {
                seq: e.seq,
                expected: expected_seq.to_string(),
                found: e.seq.to_string(),
                detail: format!(
                    "The ledger jumps from event {} to {}. An entry has been removed.",
                    expected_seq - 1,
                    e.seq
                ),
            }));
        }
        if e.prev_hash != expected_prev {
            return Ok(Some(ChainBreak {
                seq: e.seq,
                expected: expected_prev.clone(),
                found: e.prev_hash.clone(),
                detail: format!("Event {} does not follow the one before it.", e.seq),
            }));
        }
        // Recompute from the stored fields: this is what catches an edit.
        let recomputed = compute_hash(
            &e.prev_hash,
            e.seq,
            &NewEvent {
                object_type: e.object_type.clone(),
                object_id: e.object_id.clone(),
                actor: Actor {
                    kind: e.actor_type,
                    id: e.actor_id.clone(),
                    model: e.model.clone(),
                    params: None,
                },
                action: e.action,
                confidence: e.confidence,
                parent_revision: e.parent_revision,
                input_hash: e.input_hash.clone(),
                output_hash: e.output_hash.clone(),
                source_refs: e.source_refs.clone(),
                note: e.note.clone(),
                disclosure: e.disclosure,
            },
            e.at,
        );
        if recomputed != e.entry_hash {
            return Ok(Some(ChainBreak {
                seq: e.seq,
                expected: e.entry_hash.clone(),
                found: recomputed,
                detail: format!("Event {} has been altered since it was recorded.", e.seq),
            }));
        }
        expected_prev = e.entry_hash;
    }
    Ok(None)
}

pub fn history(conn: &Connection, object_id: &str) -> AppResult<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, seq, object_type, object_id, actor_type, actor_id, action, confidence,
                parent_revision, input_hash, output_hash, model, model_params, tool_id,
                source_refs, note, at, disclosure, prev_hash, entry_hash
         FROM events WHERE object_id = ?1 ORDER BY seq",
    )?;
    let rows = stmt.query_map(params![object_id], row_to_event)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn since(conn: &Connection, seq: i64, limit: usize) -> AppResult<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT id, seq, object_type, object_id, actor_type, actor_id, action, confidence,
                parent_revision, input_hash, output_hash, model, model_params, tool_id,
                source_refs, note, at, disclosure, prev_hash, entry_hash
         FROM events WHERE seq > ?1 ORDER BY seq LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![seq, limit as i64], row_to_event)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// ---------------------------------------------------------------------------
// Authorship, derived
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Human,
    Ai,
    External,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Authorship {
    pub origin: Origin,
    pub transformation: Option<String>,
    /// The sentence a report prints.
    pub disclosure_line: String,
    /// True when any event in the chain was inferred rather than observed.
    pub uncertain: bool,
    /// True when there is no record at all. Not the same as human-written, and
    /// never presented as if it were.
    pub unrecorded: bool,
    pub events: usize,
    /// Every model that touched this block.
    pub models: Vec<String>,
}

/// K2P-OS §7.2 as a pure function over an event chain.
///
/// Sorts by `seq` first, so the answer does not depend on the order the caller
/// happened to read the rows in.
pub fn classify(events: &[Event]) -> Authorship {
    if events.is_empty() {
        return Authorship {
            origin: Origin::Unknown,
            transformation: None,
            disclosure_line: "No record. This text predates the ledger or was inserted by a means \
                              AegisDraft cannot observe — which is not evidence of who wrote it."
                .into(),
            uncertain: false,
            unrecorded: true,
            events: 0,
            models: Vec::new(),
        };
    }

    let mut sorted: Vec<&Event> = events.iter().collect();
    sorted.sort_by_key(|e| e.seq);

    let uncertain = sorted.iter().any(|e| e.confidence == Confidence::Inferred);
    let mut models: Vec<String> = sorted.iter().filter_map(|e| e.model.clone()).collect();
    models.sort();
    models.dedup();

    let creation = sorted
        .iter()
        .find(|e| matches!(e.action, Action::Create | Action::Import))
        .copied();

    let origin = match creation.map(|e| (e.actor_type, e.action)) {
        Some((ActorType::AiModel, _)) => Origin::Ai,
        Some((ActorType::Tool, Action::Import)) | Some((ActorType::Tool, _)) => Origin::External,
        Some((ActorType::External, _)) => Origin::Unknown,
        Some((ActorType::Human, _)) => Origin::Human,
        None => Origin::Unknown,
    };

    // A human rewriting AI-originated text substantially makes it mixed. The
    // threshold lives with the anchoring thresholds, not here, so "substantial"
    // means the same thing everywhere.
    let human_rewrote = sorted
        .iter()
        .any(|e| e.actor_type == ActorType::Human && e.action == Action::Rewrite);
    let ai_touched = sorted.iter().any(|e| e.actor_type == ActorType::AiModel);
    let human_accepted = sorted
        .iter()
        .any(|e| e.actor_type == ActorType::Human && e.action == Action::Accept);

    let origin = if origin == Origin::Ai && human_rewrote { Origin::Mixed } else { origin };

    let transformation = match (origin, ai_touched, human_rewrote, human_accepted) {
        (Origin::Human, true, _, _) => Some("AI-assisted editing".to_string()),
        (Origin::Ai, _, _, true) => Some("Human-reviewed".to_string()),
        (Origin::Mixed, _, true, _) => Some("Human substantial rewrite".to_string()),
        (Origin::External, _, true, _) => Some("Human-edited".to_string()),
        _ => None,
    };

    let disclosure_line = match (origin, &transformation) {
        (Origin::Human, Some(t)) => format!("Human-originated; {}", t.to_lowercase()),
        (Origin::Human, None) => "Human-originated".to_string(),
        (Origin::Ai, _) => "AI-generated text with human review".to_string(),
        (Origin::Mixed, _) => {
            "Mixed: AI-originated, substantially rewritten by the author".to_string()
        }
        (Origin::External, _) => "Imported material, selected by the author".to_string(),
        (Origin::Unknown, _) => "Origin not recorded".to_string(),
    };

    let disclosure_line = if uncertain {
        format!("{disclosure_line} (lineage partly reconstructed, not observed)")
    } else {
        disclosure_line
    };

    Authorship {
        origin,
        transformation,
        disclosure_line,
        uncertain,
        unrecorded: false,
        events: sorted.len(),
        models,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorshipSummary {
    pub blocks: i64,
    pub human: i64,
    pub ai: i64,
    pub external: i64,
    pub mixed: i64,
    pub unknown: i64,
    /// Blocks with no events at all. Its own number, never folded into `human`.
    pub unrecorded: i64,
    pub uncertain: i64,
    pub chain_intact: bool,
    pub models: Vec<String>,
}

pub fn summarise(conn: &Connection, document_id: Option<&str>) -> AppResult<AuthorshipSummary> {
    let ids: Vec<String> = match document_id {
        Some(doc) => {
            let mut stmt = conn.prepare(
                "SELECT id FROM blocks WHERE document_id = ?1 AND removed_at IS NULL ORDER BY ordinal",
            )?;
            let rows = stmt.query_map(params![doc], |r| r.get(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
        None => {
            let mut stmt =
                conn.prepare("SELECT id FROM blocks WHERE removed_at IS NULL ORDER BY document_id, ordinal")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        }
    };

    let mut out = AuthorshipSummary {
        chain_intact: verify_chain(conn)?.is_none(),
        ..Default::default()
    };
    let mut models: Vec<String> = Vec::new();

    for id in &ids {
        out.blocks += 1;
        let who = classify(&history(conn, id)?);
        if who.unrecorded {
            out.unrecorded += 1;
        }
        if who.uncertain {
            out.uncertain += 1;
        }
        match who.origin {
            Origin::Human => out.human += 1,
            Origin::Ai => out.ai += 1,
            Origin::External => out.external += 1,
            Origin::Mixed => out.mixed += 1,
            Origin::Unknown => out.unknown += 1,
        }
        models.extend(who.models);
    }
    models.sort();
    models.dedup();
    out.models = models;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE documents (id TEXT PRIMARY KEY);
             INSERT INTO documents (id) VALUES ('doc');",
        )
        .unwrap();
        migrate(&conn).unwrap();
        conn
    }

    fn block(conn: &Connection, id: &str, ordinal: i64) {
        conn.execute(
            "INSERT INTO blocks (id, document_id, ordinal, kind, content_hash, text,
                                 revision, first_seen, updated_at)
             VALUES (?1,'doc',?2,'paragraph','h','t',1,0,0)",
            params![id, ordinal],
        )
        .unwrap();
    }

    fn ev(conn: &Connection, id: &str, actor: Actor, action: Action, c: Confidence, at: i64) -> Event {
        append(conn, NewEvent::block(id, actor, action, c), at).unwrap()
    }

    // --- PV-01 .. PV-05: appending -----------------------------------------

    #[test]
    fn pv01_the_first_event_starts_from_genesis() {
        let conn = db();
        let e = ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 100);
        assert_eq!(e.seq, 1);
        assert_eq!(e.prev_hash, GENESIS);
        assert_eq!(e.entry_hash.len(), 64);
    }

    #[test]
    fn pv02_each_event_chains_to_the_one_before_it() {
        let conn = db();
        // Spaced beyond COALESCE_WINDOW_MS: this test is about the chain, not
        // about sessions, so each edit must be its own event.
        let a = ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 1);
        let b = ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 200_000);
        let c = ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 400_000);
        assert_eq!((a.seq, b.seq, c.seq), (1, 2, 3));
        assert_eq!(b.prev_hash, a.entry_hash);
        assert_eq!(c.prev_hash, b.entry_hash);
    }

    #[test]
    fn pv03_identical_events_still_hash_differently() {
        let conn = db();
        let a = ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 7);
        let b = ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 7);
        assert_ne!(a.entry_hash, b.entry_hash, "seq participates in the hash");
    }

    #[test]
    fn pv04_a_model_event_records_which_model_and_how() {
        let conn = db();
        let actor = Actor::model("llama3.2:3b", json!({"temperature": 0.7}));
        ev(&conn, "b1", actor, Action::Create, Confidence::Observed, 1);
        let back = history(&conn, "b1").unwrap();
        assert_eq!(back[0].actor_type, ActorType::AiModel);
        assert_eq!(back[0].model.as_deref(), Some("llama3.2:3b"));
        assert!(back[0].model_params.as_ref().unwrap().contains("0.7"));
    }

    #[test]
    fn pv05_source_references_round_trip_as_a_list() {
        let conn = db();
        let e = NewEvent::block("b1", Actor::human(), Action::Create, Confidence::Observed)
            .sources(vec!["src-1".into(), "src-2".into()]);
        append(&conn, e, 1).unwrap();
        assert_eq!(history(&conn, "b1").unwrap()[0].source_refs, vec!["src-1", "src-2"]);
    }

    // --- PV-06 .. PV-09: verification --------------------------------------

    #[test]
    fn pv06_an_untouched_chain_verifies_clean() {
        let conn = db();
        for i in 0..50 {
            ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, i * COALESCE_WINDOW_MS * 2);
        }
        assert!(verify_chain(&conn).unwrap().is_none());
    }

    #[test]
    fn pv07_an_altered_row_is_caught_at_its_own_seq() {
        let conn = db();
        for i in 0..50 {
            ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, i * COALESCE_WINDOW_MS * 2);
        }
        // Rewrite history: make event 25 look like the author's work when it
        // was not. This is exactly the attack the chain exists to expose.
        conn.execute("UPDATE events SET actor_type = 'human' WHERE seq = 25", []).unwrap();
        conn.execute("UPDATE events SET action = 'create' WHERE seq = 25", []).unwrap();
        let broken = verify_chain(&conn).unwrap().expect("tampering must be detected");
        assert_eq!(broken.seq, 25);
        assert!(broken.detail.contains("altered"), "{}", broken.detail);
    }

    #[test]
    fn pv08_a_deleted_row_is_caught_at_the_gap() {
        let conn = db();
        for i in 0..10 {
            ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, i * COALESCE_WINDOW_MS * 2);
        }
        conn.execute("DELETE FROM events WHERE seq = 5", []).unwrap();
        let broken = verify_chain(&conn).unwrap().expect("a removed entry must be detected");
        assert_eq!(broken.seq, 6);
        assert!(broken.detail.contains("removed"), "{}", broken.detail);
    }

    #[test]
    fn pv09_an_empty_ledger_is_intact_not_broken() {
        assert!(verify_chain(&db()).unwrap().is_none());
    }

    // --- PV-10 .. PV-17: classification ------------------------------------

    #[test]
    fn pv10_human_written_prose_is_human_originated() {
        let conn = db();
        ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 1);
        let who = classify(&history(&conn, "b1").unwrap());
        assert_eq!(who.origin, Origin::Human);
        assert_eq!(who.transformation, None);
        assert_eq!(who.disclosure_line, "Human-originated");
        assert!(!who.unrecorded && !who.uncertain);
    }

    #[test]
    fn pv11_ai_copyediting_does_not_make_prose_ai_originated() {
        let conn = db();
        ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 1);
        ev(&conn, "b1", Actor::model("llama3.2:3b", json!({})), Action::Rewrite, Confidence::Observed, 2);
        ev(&conn, "b1", Actor::human(), Action::Accept, Confidence::Observed, 3);
        let who = classify(&history(&conn, "b1").unwrap());
        assert_eq!(who.origin, Origin::Human, "the author wrote it; a model tidied it");
        assert_eq!(who.transformation.as_deref(), Some("AI-assisted editing"));
        assert!(who.disclosure_line.contains("ai-assisted editing"));
        assert_eq!(who.models, vec!["llama3.2:3b"]);
    }

    #[test]
    fn pv12_accepted_generation_is_ai_originated_with_review() {
        let conn = db();
        ev(&conn, "b1", Actor::model("qwen3.5:cloud", json!({})), Action::Create, Confidence::Observed, 1);
        ev(&conn, "b1", Actor::human(), Action::Accept, Confidence::Observed, 2);
        let who = classify(&history(&conn, "b1").unwrap());
        assert_eq!(who.origin, Origin::Ai);
        assert_eq!(who.disclosure_line, "AI-generated text with human review");
    }

    #[test]
    fn pv13_a_substantial_human_rewrite_of_ai_prose_is_mixed() {
        let conn = db();
        ev(&conn, "b1", Actor::model("llama3.2:3b", json!({})), Action::Create, Confidence::Observed, 1);
        ev(&conn, "b1", Actor::human(), Action::Accept, Confidence::Observed, 2);
        ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 3);
        let who = classify(&history(&conn, "b1").unwrap());
        assert_eq!(who.origin, Origin::Mixed);
        assert_eq!(who.transformation.as_deref(), Some("Human substantial rewrite"));
    }

    #[test]
    fn pv14_imported_prose_is_external() {
        let conn = db();
        ev(&conn, "b1", Actor::tool("import"), Action::Import, Confidence::Observed, 1);
        let who = classify(&history(&conn, "b1").unwrap());
        assert_eq!(who.origin, Origin::External);
        assert!(who.disclosure_line.contains("Imported"));
    }

    #[test]
    fn pv15_any_inferred_event_makes_the_whole_chain_uncertain() {
        let conn = db();
        ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 1);
        ev(&conn, "b1", Actor::external(), Action::Rewrite, Confidence::Inferred, 2);
        let who = classify(&history(&conn, "b1").unwrap());
        assert!(who.uncertain);
        assert!(
            who.disclosure_line.contains("reconstructed"),
            "the report must say the lineage is inferred: {}",
            who.disclosure_line
        );
    }

    #[test]
    fn pv16_no_record_is_never_reported_as_human_written() {
        let who = classify(&[]);
        assert_eq!(who.origin, Origin::Unknown);
        assert!(who.unrecorded);
        assert_ne!(who.origin, Origin::Human);
        assert!(
            who.disclosure_line.contains("not evidence of who wrote it"),
            "silence must not be read as authorship: {}",
            who.disclosure_line
        );
    }

    #[test]
    fn pv17_a_rejection_is_kept() {
        let conn = db();
        ev(&conn, "b1", Actor::model("llama3.2:3b", json!({})), Action::Create, Confidence::Observed, 1);
        ev(&conn, "b1", Actor::human(), Action::Reject, Confidence::Observed, 2);
        let chain = history(&conn, "b1").unwrap();
        assert!(chain.iter().any(|e| e.action == Action::Reject));
    }

    #[test]
    fn neg09_classification_does_not_depend_on_input_order() {
        let conn = db();
        ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 1);
        ev(&conn, "b1", Actor::model("m", json!({})), Action::Rewrite, Confidence::Observed, 2);
        let mut chain = history(&conn, "b1").unwrap();
        let forwards = classify(&chain);
        chain.reverse();
        let backwards = classify(&chain);
        assert_eq!(forwards.origin, backwards.origin);
        assert_eq!(forwards.disclosure_line, backwards.disclosure_line);
    }

    // --- Coalescing --------------------------------------------------------

    #[test]
    fn a_continuous_editing_session_is_one_event_not_one_per_autosave() {
        let conn = db();
        block(&conn, "b1", 0);
        ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 0);
        // The editor autosaves every 800ms. Simulate four minutes of typing.
        for i in 1..=300 {
            ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, i * 800);
        }
        let chain = history(&conn, "b1").unwrap();
        assert!(
            chain.len() <= 4,
            "300 autosaves must not be 300 events: got {}",
            chain.len()
        );
        assert_eq!(chain[0].action, Action::Create, "the creation is never folded away");
        assert!(chain[1..].iter().all(|e| e.action == Action::Rewrite));
        assert!(verify_chain(&conn).unwrap().is_none(), "coalescing must not break the chain");
    }

    #[test]
    fn a_model_edit_is_never_folded_into_the_authors_session() {
        let conn = db();
        block(&conn, "b1", 0);
        ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 0);
        ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 1000);
        // One second later — well inside the window — a model edits it.
        ev(&conn, "b1", Actor::model("llama3.2:3b", json!({})), Action::Rewrite, Confidence::Observed, 2000);
        ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 3000);

        let chain = history(&conn, "b1").unwrap();
        assert_eq!(chain.len(), 4, "a change of hand always starts a new event: {chain:?}");
        assert_eq!(chain[2].actor_type, ActorType::AiModel);
        assert_eq!(
            classify(&chain).transformation.as_deref(),
            Some("AI-assisted editing"),
            "the model's contribution must survive coalescing"
        );
    }

    #[test]
    fn an_inferred_edit_is_never_folded_into_an_observed_session() {
        let conn = db();
        block(&conn, "b1", 0);
        ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 1000);
        ev(&conn, "b1", Actor::external(), Action::Rewrite, Confidence::Inferred, 2000);
        let chain = history(&conn, "b1").unwrap();
        assert_eq!(chain.len(), 2);
        assert!(classify(&chain).uncertain, "an inferred edit must still make the chain uncertain");
    }

    #[test]
    fn returning_to_a_paragraph_later_is_a_new_event() {
        let conn = db();
        block(&conn, "b1", 0);
        ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 0);
        ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 1000);
        // The next day.
        ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 86_400_000);
        let chain = history(&conn, "b1").unwrap();
        assert_eq!(chain.len(), 2, "two sittings are two events: {chain:?}");
    }

    #[test]
    fn discrete_acts_are_never_coalesced() {
        let conn = db();
        block(&conn, "b1", 0);
        // Accept, delete, import and export are decisions, not typing.
        for action in [Action::Accept, Action::Accept, Action::Export, Action::Export] {
            ev(&conn, "b1", Actor::human(), action, Confidence::Observed, 1000);
        }
        assert_eq!(history(&conn, "b1").unwrap().len(), 4);
    }

    #[test]
    fn editing_two_paragraphs_in_turn_keeps_both_sessions() {
        let conn = db();
        block(&conn, "b1", 0);
        block(&conn, "b2", 1);
        // Coalescing is per block: the last event overall is not the test.
        for i in 1..=10 {
            ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, i * 800);
            ev(&conn, "b2", Actor::human(), Action::Rewrite, Confidence::Observed, i * 800);
        }
        assert_eq!(history(&conn, "b1").unwrap().len(), 1);
        assert_eq!(history(&conn, "b2").unwrap().len(), 1);
    }

    // --- PV-18 .. PV-20: summary -------------------------------------------

    #[test]
    fn pv18_unrecorded_blocks_are_counted_separately_from_human_ones() {
        let conn = db();
        for i in 0..10 {
            let id = format!("h{i}");
            block(&conn, &id, i);
            ev(&conn, &id, Actor::human(), Action::Create, Confidence::Observed, i);
        }
        for i in 0..3 {
            let id = format!("a{i}");
            block(&conn, &id, 100 + i);
            ev(&conn, &id, Actor::model("m", json!({})), Action::Create, Confidence::Observed, i);
            ev(&conn, &id, Actor::human(), Action::Accept, Confidence::Observed, i);
        }
        for i in 0..2 {
            block(&conn, &format!("u{i}"), 200 + i); // no events at all
        }

        let s = summarise(&conn, None).unwrap();
        assert_eq!(s.blocks, 15);
        assert_eq!(s.human, 10);
        assert_eq!(s.ai, 3);
        assert_eq!(s.unrecorded, 2);
        assert_eq!(s.unknown, 2, "unrecorded blocks are Unknown, never Human");
        assert!(s.chain_intact);
        assert_eq!(s.models, vec!["m"]);
    }

    #[test]
    fn pv20_a_broken_chain_is_reported_and_the_summary_still_returns() {
        let conn = db();
        block(&conn, "b1", 0);
        ev(&conn, "b1", Actor::human(), Action::Create, Confidence::Observed, 1);
        ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, 2);
        conn.execute("UPDATE events SET actor_id = 'someone else' WHERE seq = 1", []).unwrap();
        let s = summarise(&conn, None).unwrap();
        assert!(!s.chain_intact);
        assert_eq!(s.blocks, 1, "a broken chain must not stop the report");
    }

    // --- NEG ---------------------------------------------------------------

    #[test]
    fn neg01_an_event_about_something_since_deleted_is_still_recorded() {
        let conn = db();
        // The ledger records what happened, including to objects that no longer
        // exist. There is no foreign key on events for exactly this reason.
        ev(&conn, "never-existed", Actor::human(), Action::Delete, Confidence::Observed, 1);
        assert_eq!(history(&conn, "never-existed").unwrap().len(), 1);
    }

    #[test]
    fn neg07_field_contents_cannot_be_rearranged_into_a_collision() {
        let conn = db();
        // Two events whose fields concatenate to the same string without a
        // separator: "ab" + "c" versus "a" + "bc".
        let a = append(
            &conn,
            NewEvent::block("ab", Actor::human(), Action::Create, Confidence::Observed).about("x", "ab"),
            1,
        )
        .unwrap();
        let b = append(
            &conn,
            NewEvent::block("bc", Actor::human(), Action::Create, Confidence::Observed).about("xa", "bc"),
            1,
        )
        .unwrap();
        assert_ne!(a.entry_hash, b.entry_hash);
    }

    #[test]
    fn perf04_a_large_ledger_verifies_quickly() {
        let conn = db();
        conn.execute_batch("BEGIN").unwrap();
        for i in 0..20_000 {
            ev(&conn, "b1", Actor::human(), Action::Rewrite, Confidence::Observed, i * COALESCE_WINDOW_MS * 2);
        }
        conn.execute_batch("COMMIT").unwrap();
        let started = std::time::Instant::now();
        assert!(verify_chain(&conn).unwrap().is_none());
        let took = started.elapsed();
        assert!(took.as_secs() < 2, "verifying 20k events took {took:?}");
    }
}
