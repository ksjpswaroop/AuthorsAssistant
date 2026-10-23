# Block-level identity and provenance

**Status: design. Nothing here is built yet.** The functions, schema and tests
below are the specification; [TEST-PLAN-BLOCKS.md](TEST-PLAN-BLOCKS.md) is the
suite that must go red before any of it goes green, and
[jira/block-identity-stories.csv](jira/block-identity-stories.csv) is the same
work as trackable stories.

Parent: [ARCHITECTURE.md](ARCHITECTURE.md) · Source of the idea:
`Knowledge_to_Publication_OS_Architecture_Specification.docx` (referred to below
as **K2P-OS**).

---

## 0. The decision this document starts from

**The stack does not change.** Tauri + SQLite + Markdown + Ollama, local by
default, with the two narrow egress paths in the
[golden rule](ARCHITECTURE.md#golden-rule).

K2P-OS §16 maps its capabilities onto Next.js, FastAPI, PostgreSQL, Neo4j,
Celery, S3 and Kafka. That is a multi-tenant SaaS, and it is not this product.
What *is* portable from K2P-OS is its **object model and its invariant**, which
are runtime-independent:

> No generated paragraph, citation, figure, or publication package should exist
> without a stable object identity. Every meaningful transformation should be
> observable as an event.

K2P-OS §15.1 already names our topology — "Local-first author: Tauri Desktop,
local SQLite subset, local blob directory, local model gateway" — so adopting
the object model is not a fork from that document. It is the deployment it
anticipates.

---

## 1. The tension, stated honestly before it is solved

AegisDraft's central promise is that **a manuscript is a folder of readable
Markdown the author owns**, editable in any editor, surviving this app being
uninstalled. K2P-OS wants `P-184` to be traceable across 47 revisions and 3
editions.

Those two are in genuine tension, and most systems resolve it by breaking the
first: they take custody of the text, and the file on disk becomes an export.
That resolution is not available here.

Three ways out, and why the third wins:

| | Approach | Why not / why |
| :--- | :--- | :--- |
| **A** | **Inline IDs in the Markdown** — `^blk_7f3a` at paragraph end, or an HTML comment | Survives everything and needs no cleverness. Rejected: it puts machine noise into the author's prose. Obsidian does this and people tolerate it, but our promise is a file a person *reads*, not one they tolerate. |
| **B** | **Sidecar map, position-keyed** — block 3 of chapter 4 | Prose stays clean. Rejected: the first external edit that inserts a paragraph shifts every subsequent id, silently reassigning history to the wrong text. Worse than no history. |
| **C** | **Content-anchored identity** — ids in SQLite, re-anchored to the text on every load by matching content | **Chosen.** Prose stays pristine; identity survives external editing; and when it cannot, it says so rather than guessing quietly. |

### 1.1 The consequence that makes C honest

Anchoring after an external edit is **inference**. The app did not see the edit
happen; it is reconstructing what probably happened by comparing text. An edit
made *inside* the app is **observation** — the app knows exactly what changed
and who changed it.

A provenance ledger that records both as the same kind of fact is lying about
its own reliability. So every event carries:

```
confidence: observed | inferred
```

`observed` means AegisDraft watched it happen. `inferred` means AegisDraft
matched text after the fact and believes this is the same block. A provenance
report separates them, and an authorship claim built on `inferred` events says
so on its face.

This is the single most important idea in this document. K2P-OS assumes every
event is captured at source, because in a SaaS the editor is the only way in.
Ours is not, and pretending otherwise would make the ledger worse than useless —
it would make it confidently wrong.

---

## 2. Block model

### 2.1 What a block is

The smallest unit an author addresses and a reader perceives: a paragraph. Not a
sentence (too volatile), not a scene (too coarse — that is `documents`, which we
already have).

| Kind | Recognised by | One block? |
| :--- | :--- | :--- |
| `heading` | line starting `#`–`######` | Yes, alone |
| `paragraph` | default | Blank-line separated |
| `code` | ```` ``` ```` fence | Whole fence, blank lines inside included |
| `quote` | contiguous lines starting `>` | Whole run |
| `list` | contiguous `-`/`*`/`1.` items | Whole run, **not** per item |
| `table` | contiguous lines containing `\|` with a `---\|` separator row | Whole run |
| `frontmatter` | `---` fence at position 0 | Whole fence |
| `rule` | `---`, `***` on its own line, not at position 0 | Yes, alone |

A list is one block rather than one per item because an author moves and rewrites
a list as a unit; per-item ids would churn on every reorder.

### 2.2 Normalisation

Identity compares **normalised** text, never raw:

- Collapse runs of whitespace (including newlines) to one space
- Trim
- Leave punctuation, capitalisation and Markdown markers alone

Hard-wrapping a paragraph at 80 columns in vim is therefore **not** a change.
This matters: half of all external edits to a Markdown file are reflows, and a
system that records those as rewrites would fill the ledger with noise and make
the real events unfindable.

The raw text is kept for display and diffing; the normalised form is what is
hashed.

### 2.3 Schema

```sql
CREATE TABLE IF NOT EXISTS blocks (
  id           TEXT PRIMARY KEY,          -- blk_<uuid>, stable for life
  document_id  TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  ordinal      INTEGER NOT NULL,          -- position in the document, 0-based
  kind         TEXT NOT NULL,             -- see 2.1
  content_hash TEXT NOT NULL,             -- sha256 of normalised text
  text         TEXT NOT NULL,             -- raw text, for anchoring and display
  revision     INTEGER NOT NULL,          -- increments on content change
  first_seen   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  removed_at   INTEGER                    -- soft delete; a block is never dropped
);
CREATE INDEX IF NOT EXISTS blocks_doc ON blocks(document_id, ordinal);
CREATE INDEX IF NOT EXISTS blocks_hash ON blocks(content_hash);
```

A deleted block is **soft-deleted**. Its id must never be reused, and a
provenance chain that ends in a deletion is still a chain worth reading.

`project.sqlite` is already committed by the snapshot code, so block identity
and history travel with the prose in every Git snapshot. Nothing new is needed
for durability.

### 2.4 Anchoring algorithm

Runs on document load and after every save. Given the parsed blocks `P` and the
known live blocks `K` for that document:

**Pass 0 — skip.** If the file's whole-content hash equals the last one seen,
nothing changed. Return immediately. This is what keeps anchoring free in the
common case.

**Pass 1 — exact.** Match `p ∈ P` to `k ∈ K` where `content_hash` is equal and
`kind` matches. Duplicated text is matched by **nearest ordinal**, not
first-come: two identical paragraphs must anchor to the nearer one or they will
swap identities on every save.

**Pass 2 — fuzzy.** For each still-unmatched pair of the same `kind`, compute
Jaccard over 3-word shingles (`integrity::shingles`, already built). Take pairs
greedily in descending similarity, above `ANCHOR_THRESHOLD = 0.35`. Tie-break on
ordinal distance.

Blocks shorter than `MIN_FUZZY_WORDS = 8` are **exempt** from pass 2. A 3-word
heading shingles to almost anything; fuzzy-matching short text produces
confident nonsense.

**Pass 3 — leftovers.** Unmatched `p` → new block, `create` event. Unmatched
`k` → `removed_at` set, `delete` event.

**Uncertainty.** A pass-2 match is flagged `uncertain` when either:

- similarity is below `CONFIDENT_THRESHOLD = 0.60`, **or**
- a runner-up candidate scored within `AMBIGUITY_MARGIN = 0.10` of the winner

An uncertain anchor still anchors — losing history to caution is also a loss —
but the event records it, and provenance reports show it.

**Complexity.** Anchoring is per-document. A scene is 10–60 blocks, and pass 1
removes nearly all of them, so pass 2 typically compares a handful. The
pathological case (60 × 60) is 3,600 shingle comparisons — microseconds. This is
deliberately *not* a project-wide match: cross-document block movement is out of
scope (§7).

---

## 3. Provenance ledger

### 3.1 Schema

```sql
CREATE TABLE IF NOT EXISTS events (
  id             TEXT PRIMARY KEY,        -- evt_<uuid>
  seq            INTEGER NOT NULL,        -- monotonic, gapless, per project
  object_type    TEXT NOT NULL,           -- block | document | asset | project
  object_id      TEXT NOT NULL,
  actor_type     TEXT NOT NULL,           -- human | ai_model | tool | external
  actor_id       TEXT NOT NULL,           -- "author" | "llama3.2:3b" | "import" | "external_edit"
  action         TEXT NOT NULL,           -- see 3.2
  confidence     TEXT NOT NULL,           -- observed | inferred
  parent_revision INTEGER,
  input_hash     TEXT,
  output_hash    TEXT,
  model          TEXT,                    -- when actor_type = ai_model
  model_params   TEXT,                    -- json: temperature, max_tokens
  tool_id        TEXT,                    -- when actor_type = tool
  source_refs    TEXT,                    -- json array of ids
  note           TEXT,
  at             INTEGER NOT NULL,
  disclosure     TEXT NOT NULL,           -- private | collaborator | institution | public
  prev_hash      TEXT NOT NULL,           -- previous entry_hash, or 64 zeros
  entry_hash     TEXT NOT NULL            -- sha256 over prev_hash + canonical fields
);
CREATE INDEX IF NOT EXISTS events_object ON events(object_type, object_id, seq);
CREATE UNIQUE INDEX IF NOT EXISTS events_seq ON events(seq);
```

**Append-only.** There is no update path and no delete path. A correction is a
new event that supersedes an earlier one (K2P-OS §7: "E-104 never changes; E-105
can supersede it").

**Coalesced by session.** The editor autosaves every 800ms. One event per save
means an hour on a single paragraph writes ~4,500 rows, and a book runs to
millions — an unreadable lineage and a database that grows without bound for
ordinary writing. So a `rewrite` that continues the same actor's work on the same
block within `COALESCE_WINDOW_MS` (two minutes) is **not appended**.

This is not a violation of append-only: nothing is updated or deleted, so the
hash chain is untouched. Some appends simply do not happen.

What survives: who wrote it, when they started, the revision they started from,
and what the block says now (the block row is always current). What is lost:
the intermediate states inside a sitting, which were never the point.

The check is deliberately narrow. **A different actor, a different confidence,
or any action other than `rewrite` always starts a new event.** A model's edit
must never disappear into the author's session — that would be the laundering
failure by another route, and it has its own test.

**Hash chain.** `entry_hash = sha256(prev_hash ‖ seq ‖ object_id ‖ action ‖
actor_id ‖ output_hash ‖ at)`, with fields joined by a byte that cannot occur in
any of them. Walking the chain detects any retroactive edit or deletion. This is
tamper-**evident**, not tamper-proof: someone with write access to the file can
rewrite the whole chain. Signing (K2P-OS's `SignedCheckpoint`) is out of scope
and stated as such in §7.

### 3.2 Actions

| Action | Actor | When |
| :--- | :--- | :--- |
| `create` | human / ai_model / tool | A block first appears |
| `rewrite` | human / ai_model | Content changed, identity anchored |
| `delete` | human | Block gone from the document |
| `accept` | human | The author took a model's proposal into the manuscript |
| `reject` | human | The author declined one — a real fact worth keeping |
| `import` | tool | Brought in from outside (Phase 2 import) |
| `transcribe` | tool | Dictation, if it is ever built |
| `export` | tool | Included in a compile |
| `restore` | human | Reverted from a snapshot |
| `verify` | tool | A check ran against it (integrity, canon) |

### 3.3 The gap this closes

Today the Assist panel deliberately never inserts generated prose — the author
copies it across by hand. Good for trust, and it means **AI-originated text
enters the manuscript with no record whatsoever**. The agent loop is traced in
`traces/`; the writing loop is not traced at all.

The fix is an explicit **"Use this"** action that inserts the proposal and
records `accept` with `actor_type = ai_model`, the model name and parameters, and
the input/output hashes.

**And the honest limit:** if the author selects the text and copies it manually,
AegisDraft cannot know. That path must therefore be *visible in the report*, not
papered over. A document whose blocks carry no `accept` event is not thereby
proven human-written; it is a document with no record. The report says exactly
that, in those words.

### 3.4 Authorship classification is derived, never typed

Implementing K2P-OS §7.2 as a pure function over a block's event chain:

| Event chain | Origin | Transformation | Public disclosure |
| :--- | :--- | :--- | :--- |
| `create(human)` | Human-originated | — | Human-originated |
| `create(human)` → `rewrite(ai_model)` → `accept(human)` | Human-originated | AI-assisted editing | Human-originated; AI copyediting |
| `create(ai_model)` → `accept(human)` | AI-originated | Human-reviewed | AI-generated text with human review |
| `create(ai_model)` → `accept(human)` → `rewrite(human)` with similarity < 0.5 | Mixed | Human substantial rewrite | Policy-specific; ledger preserved |
| `create(tool: import)` | External | Human selected | Imported material |
| any chain containing an `inferred` event | **Uncertain** | — | Lineage partly reconstructed, not observed |

The last row is ours, not K2P-OS's, and it is not optional. A classification
derived partly from inference must never be presented with the same confidence
as one derived from observation.

**This is the honest replacement for the AI detection we cut in
[Phase 4](phases/04-integrity-engine.md).** We do not infer authorship from
prose statistics, because that is unreliable and unfair. We record it as it
happens, and where we have no record we say we have no record.

---

## 4. Gap analysis: K2P-OS against AegisDraft

Verdicts: **Adopt** (build it, as specified) · **Adapt** (build the idea, our
way) · **Defer** (real, not now) · **Reject** (wrong for this product, with a
reason).

### 4.1 Provenance & Authorship Engine (K2P-OS §7)

| Component | Today | Verdict | Notes |
| :--- | :--- | :--- | :--- |
| Event Capture | Agent runs in `traces/`; web egress in `research-trace.json`. Writing loop: **nothing** | **Adopt** | §3.3. The writing loop is the hole. |
| Provenance Ledger (append-only) | Git snapshots — real history, but coarse and not an event log | **Adopt** | §3.1 |
| Object Identity | Documents have stable UUIDs. Blocks: none | **Adopt** | §2 — the core of this document |
| Relationship Graph | `document_links` (sheet ↔ scene), keywords, plants/payoffs | **Adapt** | Extend to `block → claim`, `block → source` when sources exist. Not a graph database; join tables. |
| Tamper-Evident Storage | Git commit hashes (incidental) | **Adapt** | Hash chain (§3.1). Merkle roots and signing deferred. |
| C2PA Content Credentials | None | **Defer** | Needs an asset pipeline and a signing identity we do not have. Real, later. |
| Provenance Reports | Integrity packet (measurements, no lineage) | **Adopt** | Extend the existing packet — §5.5 |

### 4.2 Manuscript Workspace (K2P-OS §5)

| Component | Today | Verdict |
| :--- | :--- | :--- |
| Part / Chapter with stable IDs surviving rename and reorder | **Built** — `documents.id` is a UUID; move and rename preserve it | — |
| Paragraph Block with stable BlockID and revision lineage | Not built | **Adopt** |
| Front Matter as typed semantic nodes | `section_type` has Front/Back Matter | **Adapt** — the type exists; the typed *fields* (title, copyright, dedication) do not |
| Figure / Table / Equation / Code / Callout blocks | Parsed as blocks by kind (§2.1); no Asset object behind them | **Adapt** for identity now, **Defer** the Asset lifecycle |
| References/Bibliography generated from Citation objects | None | **Defer** — needs §4.3 |
| Index/Glossary derived from Concept objects | None | **Defer** — needs §4.3 |

### 4.3 Research-to-Book Engine (K2P-OS §4)

Sources, concepts, claims, evidence, contradiction detection, citation
management.

**Defer, entirely.** It is the largest single piece of K2P-OS and it is a
different product surface from the one AegisDraft ships. It also *depends* on
block identity: an evidence link is `claim → block`, and there are no blocks to
link to yet. Building identity first is the prerequisite, which is why it is
this document and not that one.

One thing to get right now so it is not painful later: `events.source_refs`
already exists as a JSON array of ids, so an evidence link has somewhere to
land the day sources exist.

### 4.4 Ingestion (K2P-OS §2)

| Component | Today | Verdict |
| :--- | :--- | :--- |
| Connectors, Parse & OCR, Normalize | Markdown folder import (Phase 2) | **Defer** the rest — PDF/OCR is a research-engine feature |
| De-duplicate | Internal similarity (Phase 4) does this for prose | **Adapt** — same shingle machinery, different objects |
| Source Registry | None | **Defer** with §4.3 |

### 4.5 Publishing OS (K2P-OS §6)

| Component | Today | Verdict |
| :--- | :--- | :--- |
| Format Compiler (EPUB/PDF/DOCX/HTML) | **Built** (Phase 7), already "semantic AST → adapters, formatting separate from content" exactly as §16 asks | — |
| Editorial workflow, AI editorial team | Rubric + integrity | **Adapt** — the review *record* is what is missing, and it is an event type |
| Asset & Rights Management, ISBN/BISAC metadata | None | **Defer** |
| Publishing packages, preflight, channels (KDP etc.) | None | **Reject for now.** Distribution to retail channels is a business surface, not a local-first authoring one. Revisit only if the product's scope changes. |

### 4.6 Data & Services (K2P-OS §8) and Infrastructure (§9)

**Reject the mapping, keep the roles.** Neo4j, PGVector, Elasticsearch, Redis,
RabbitMQ, an AI Model Gateway and a cloud provider are nine services to install
before a novelist can write a sentence. Every role they fill is already filled:

| K2P-OS service | Ours |
| :--- | :--- |
| Graph Database | SQLite join tables (`document_links`, and blocks' edges to come) |
| Document Store (S3/Blob) | The project folder |
| Vector Store (PGVector) | `chunks` in the project's own SQLite, cosine in Rust |
| Relational DB (PostgreSQL) | SQLite |
| Search Index (Elasticsearch) | Shingle search + embeddings |
| Message Queue (RabbitMQ/Kafka) | Direct calls. One user, one machine, no queue. |
| AI Model Gateway | `ollama.rs` |
| Multi-tenant, CDN, K8s | Not applicable to a desktop app |

This is not a shortcut. A project that is a folder you can copy, back up and
open in ten years is the product; a project that is rows in nine services is a
different one.

---

## 5. Functions to build

Signatures are the contract the tests in
[TEST-PLAN-BLOCKS.md](TEST-PLAN-BLOCKS.md) hold them to.

### 5.1 `src-tauri/src/blocks.rs` — parsing and identity

```rust
pub enum BlockKind { Heading, Paragraph, Code, Quote, List, Table, FrontMatter, Rule }
impl BlockKind { pub fn as_str(self) -> &'static str; pub fn parse(s: &str) -> Self; }

pub struct ParsedBlock { pub kind: BlockKind, pub text: String, pub ordinal: usize }

pub struct Block {
    pub id: String, pub document_id: String, pub ordinal: i64,
    pub kind: BlockKind, pub content_hash: String, pub text: String,
    pub revision: i64, pub first_seen: i64, pub updated_at: i64,
    pub removed_at: Option<i64>,
}

/// Split Markdown into blocks. Pure, no I/O, no database.
pub fn split(markdown: &str) -> Vec<ParsedBlock>;

/// Collapse whitespace and trim. What identity compares.
pub fn normalise(text: &str) -> String;

/// sha256 of `normalise(text)`, lowercase hex.
pub fn content_hash(text: &str) -> String;

pub enum AnchorOutcome {
    Same { block_id: String },
    Revised { block_id: String, similarity: f32, uncertain: bool },
    New,
}
pub struct Anchor { pub parsed: usize, pub outcome: AnchorOutcome }
pub struct AnchorPlan { pub anchors: Vec<Anchor>, pub removed: Vec<String> }

/// Match parsed blocks to known ones. Pure: takes the knowns, returns a plan,
/// touches nothing. Every test in §2.4 drives this function directly.
pub fn anchor(parsed: &[ParsedBlock], known: &[Block]) -> AnchorPlan;

/// Apply a plan: insert, bump revisions, soft-delete, and emit events.
pub fn reconcile(
    conn: &Connection, document_id: &str, markdown: &str,
    actor: Actor, confidence: Confidence, now: i64,
) -> AppResult<Vec<Block>>;

pub fn list(conn: &Connection, document_id: &str) -> AppResult<Vec<Block>>;
pub fn get(conn: &Connection, block_id: &str) -> AppResult<Option<Block>>;
```

### 5.2 `src-tauri/src/provenance.rs` — the ledger

```rust
pub enum ActorType { Human, AiModel, Tool, External }
pub struct Actor { pub kind: ActorType, pub id: String,
                   pub model: Option<String>, pub params: Option<Value> }
impl Actor {
    pub fn human() -> Self;
    pub fn model(name: &str, params: Value) -> Self;
    pub fn tool(id: &str) -> Self;
    pub fn external() -> Self;
}

pub enum Confidence { Observed, Inferred }
pub enum Action { Create, Rewrite, Delete, Accept, Reject, Import,
                  Transcribe, Export, Restore, Verify }
pub enum Disclosure { Private, Collaborator, Institution, Public }

pub struct Event { /* the §3.1 columns */ }

pub struct NewEvent {
    pub object_type: String, pub object_id: String,
    pub actor: Actor, pub action: Action, pub confidence: Confidence,
    pub parent_revision: Option<i64>,
    pub input_hash: Option<String>, pub output_hash: Option<String>,
    pub source_refs: Vec<String>, pub note: Option<String>,
    pub disclosure: Disclosure,
}

/// Append one event, chaining its hash to the previous. The only write path.
pub fn append(conn: &Connection, event: NewEvent, now: i64) -> AppResult<Event>;

/// Walk the chain. Returns the first break, or None if intact.
pub struct ChainBreak { pub seq: i64, pub expected: String, pub found: String }
pub fn verify_chain(conn: &Connection) -> AppResult<Option<ChainBreak>>;

pub fn history(conn: &Connection, object_id: &str) -> AppResult<Vec<Event>>;
pub fn since(conn: &Connection, seq: i64, limit: usize) -> AppResult<Vec<Event>>;

pub enum Origin { Human, Ai, External, Mixed, Unknown }
pub struct Authorship {
    pub origin: Origin,
    pub transformation: Option<String>,
    pub disclosure_line: String,
    /// True when any event in the chain was inferred rather than observed.
    pub uncertain: bool,
    /// True when the block has no events at all — no record is not proof.
    pub unrecorded: bool,
}
/// K2P-OS §7.2 as a pure function over an event chain.
pub fn classify(events: &[Event]) -> Authorship;

pub struct AuthorshipSummary {
    pub blocks: i64, pub human: i64, pub ai: i64, pub external: i64,
    pub mixed: i64, pub unknown: i64, pub unrecorded: i64,
    pub uncertain: i64, pub chain_intact: bool,
}
pub fn summarise(conn: &Connection, document_id: Option<&str>)
    -> AppResult<AuthorshipSummary>;
```

### 5.3 Store changes — `src-tauri/src/store.rs`

```rust
// Added to the existing add-column / CREATE TABLE IF NOT EXISTS migration.
// No versioned rewrite: a project made by an older build must open untouched.
pub const EXPECTED_TABLES: &[&str] = &[ /* … */ "blocks", "events" ];
fn migrate_blocks(conn: &Connection) -> AppResult<()>;
```

`documents` gains one column:

```
("body_hash", "TEXT")   -- last reconciled whole-file hash; pass 0 of §2.4
```

### 5.4 Wiring into what exists

| Call site | Change |
| :--- | :--- |
| `project::write_body` | After the atomic write, `blocks::reconcile(actor = human, confidence = Observed)` |
| `project::open` | For each document whose `body_hash` differs from the file, reconcile with `actor = External, confidence = Inferred` — this is the vim case |
| `import::run` | Reconcile each imported document with `actor = Tool("import")`, `Observed` |
| `snapshots::restore_file` | Reconcile, then `Action::Restore` |
| `compile::render` | One `Action::Export` event per document included |
| **New** `assist_accept` | Insert a proposal at a block and record `accept` with the model — §3.3 |
| `agent_loop::execute` | Tool calls that write prose already run through `project::write_body`, so they inherit reconciliation; the actor must be threaded through as `AiModel` rather than defaulting to human |

The last row is a real trap: without threading the actor, every agent edit would
be recorded as the author's own work. That would be the worst possible failure of
this feature — a ledger that launders AI writing as human. It has its own test.

### 5.5 Tauri commands

```rust
blocks_list(path, document_id) -> Vec<Block>
block_history(path, block_id) -> Vec<Event>
block_authorship(path, block_id) -> Authorship
provenance_summary(path, document_id: Option<String>) -> AuthorshipSummary
provenance_verify(path) -> Option<ChainBreak>
provenance_export(path, document_id, out_path) -> (String, String)  // json + html
assist_accept(path, document_id, block_id: Option<String>, text, model, params) -> Block
```

`integrity_export` gains an `authorship` section, so the existing packet becomes
K2P-OS's Provenance Report rather than a second artefact.

### 5.6 UI

| Surface | Change |
| :--- | :--- |
| Inspector → new **Lineage** tab | The selected block's event chain, its derived authorship, and whether any of it was inferred |
| Editor gutter | A quiet marker on blocks with an `ai_model` event in their chain. Not a scarlet letter — a fact the author can see |
| Assist panel | **Use this** button beside the copy affordance, which is what creates the record |
| Integrity screen | Authorship summary: counts by origin, with `unrecorded` shown as its own category and *not* folded into "human" |
| Settings | Chain verification with a plain-language result |

---

## 6. Migration and compatibility

- Every existing project has zero blocks and zero events. On first open after
  the update, each document reconciles once with `actor = External, confidence =
  Inferred`, `action = Create`. That is the truthful record: this text existed
  before there was a ledger, and we do not know who wrote it.
- The ledger therefore **starts honest and empty**, and `unrecorded` will
  dominate a pre-existing manuscript. The UI must present that as normal
  history-before-history, not as a warning.
- No file on disk changes. A project opened by an older build still works: the
  new tables are additive and the Markdown is untouched.

---

## 7. What this explicitly does not do

Stated so nobody reads the sections above as promising it:

- **No signing, no Merkle roots, no C2PA.** The hash chain is tamper-*evident*
  against casual edits. Anyone with write access to `project.sqlite` can rebuild
  it. Real tamper-*proofing* needs a signing identity and an external timestamp,
  which is a separate piece of work.
- **No cross-document block tracking.** A paragraph moved from chapter 3 to
  chapter 9 is a delete and a create. Tracking it would need project-wide
  matching on every save, and the false-positive rate across a whole book is
  bad enough to make the feature dishonest.
- **No sentence-level identity.** Too volatile to be useful.
- **Manual copy-paste from the Assist panel remains invisible.** This is a real
  hole and it is disclosed in the report rather than hidden.
- **No claims, evidence, sources or citations.** K2P-OS §4 is deferred whole
  (§4.3), and this work is its prerequisite, not its beginning.
