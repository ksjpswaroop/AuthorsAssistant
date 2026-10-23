# The Knowledge-to-Publication layer

**Built.** Sources, claims, evidence, contradiction detection, gap analysis and
deep research — the K2P-OS object model on our own stack, with its cloud service
mapping rejected.

Prerequisite: [BLOCK-IDENTITY.md](BLOCK-IDENTITY.md), which is also built.
Gap analysis against the original specification is in that document's §4.

---

## What the stack is, and is not

Unchanged: **Tauri + SQLite + Markdown + Ollama**, local by default, with the two
narrow egress paths in the [golden rule](ARCHITECTURE.md#golden-rule).

K2P-OS §8–9 maps its layers onto Neo4j, PGVector, Elasticsearch, Redis,
RabbitMQ, an AI gateway and a cloud provider — nine services to install before a
novelist writes a sentence. Every role is already filled here: SQLite join tables
for the graph, the project folder for blobs, `chunks` + cosine for vectors,
shingles for search, direct calls instead of a queue, `ollama.rs` for the model
gateway. A project that is a folder you can copy, back up and open in ten years
is the product.

---

## The layers, as built

| K2P-OS layer | Module | State |
| :--- | :--- | :--- |
| Input & Ingestion | `sources.rs` | Markdown, text, HTML, DOCX in-process; PDF via `pdftotext` |
| Object identity | `blocks.rs` | Stable paragraph ids, content-anchored |
| Provenance & Authorship | `provenance.rs` | Append-only hash-chained ledger |
| Research-to-Book | `knowledge.rs` | Concepts, claims, evidence, contradictions, gaps |
| Deep research | `research.rs` | Decompose → gather → extract → verify → cite |
| Manuscript Workspace | `project.rs`, `store.rs` | Built in Phases 1–2, now block-aware |
| Publishing OS | `compile.rs` | Built in Phase 7 |

---

## Ingestion (K2P-OS §2)

A source gets stable identity, extracted text stored as a plain file inside the
project, metadata read from the document, and deduplication.

**Extraction is honest about its limits.** DOCX is a zip holding
`word/document.xml`, so it needs no new dependency. HTML is stripped, with
`<script>` and `<style>` contents dropped rather than pasted into a research
corpus as prose. **PDF goes through `pdftotext`, and is refused with the exact
install command when poppler is absent** — a silently empty source would look
registered and have nothing to say, which is worse than a refusal.

**Deduplication is content hash, then DOI, then ISBN — and deliberately not
fuzzy title matching.** A wrong merge silently attributes one paper's claims to
another, which costs far more than a duplicate row.

**DOI detection requires a four-digit registrant.** Without that guard, "version
10.2/beta" reads as a DOI, and a fabricated identifier is worse than none: it
makes a citation look verifiable.

Bibliography entries are **generated from the registered source**, never typed —
K2P-OS §5's "no manually fabricated bibliography entries".

---

## Claims and evidence (K2P-OS §4)

A **claim** is one assertion that could turn out to be wrong, pinned to the
source that makes it. RAG over chunks answers "what does the corpus say about
X"; it cannot answer "do my sources disagree", because a chunk has no truth
value. A claim does.

### The verbatim rule

Every extracted claim carries an excerpt, and **the excerpt must appear in the
source or the claim is dropped and named**. A model asked for quotations will
occasionally produce a plausible sentence that is not in the paper, and a
fabricated excerpt attached to a real citation is precisely what makes a research
tool untrustworthy.

The check is deliberately loose — collapsed lowercase alphanumerics — because
models straighten quotes and normalise whitespace. Strict equality would fail
most honest quotations along with the invented ones.

### A prompt bug that a live model found

The first version of the extraction contract was a schema of placeholders:

```
{"claims": [{"text": "one falsifiable assertion", "kind": "assertion|definition|..."}]}
```

`llama3.2:3b` emitted `"text": "one falsifiable assertion"` **as the claim**. A
small model reads a placeholder as content to produce, not as a slot to fill. The
contract is now a *worked example for a different passage* with "copy the
structure, never the content", and extraction went from zero usable claims to
working on the first try.

The validator also gained `is_echoed_instruction`, because the echo reads as a
plausible finding until you look at it. It is checked **before** the length
guard: most echoes are short, and "too short to check" would send someone looking
at their source when the problem is the model.

### Contradictions in two stages

Comparing every claim against every other is O(n²) model calls — 400 claims is
80,000 requests. Instead: word-shingle overlap finds pairs *about the same
thing* (contradictory claims share their subject almost by definition — "X
improves Y" and "X does not improve Y" overlap heavily), and only those go to a
model. Same-source pairs are excluded: a paper restating itself is not a
disagreement.

**The limit is printed in the report**: two claims that disagree in entirely
different words will not be found. A contradiction check that silently misses
things while looking thorough is worse than one that states its scope.

### Gaps need no model at all

Concepts nothing asserts anything about, concepts resting on a single claim,
sources registered but never mined, claims nothing cites. All arithmetic.

### The retraction query

`blocks_depending_on(source)` names every manuscript paragraph that rests on a
source — K2P-OS §7's "a retracted source immediately reveals which blocks depend
on it". This is what block identity was the prerequisite for.

---

## Deep research (K2P-OS §11.1)

```
question → decompose → retrieve → extract → verify → contradict → synthesise → cite
```

**A model is never asked what is true.** It is asked only to break a question
up, to read a passage the author owns and say what it asserts, and to write prose
citing claims already gathered. Every fact traces to a registered source.

**Citations are checked before the brief is shown.** A model will occasionally
cite `[9]` when eight claims were supplied; left alone that renders as a
confident, unverifiable reference. Unresolvable citations are removed, the
sentence is kept, and the removal is listed under "Left out".

**Retrieval works with no model and no network** — shingle containment over the
author's own corpus, capped at two passages per source so one long document
cannot crowd out every other view of the question.

**A brief with no evidence says so.** It does not write around the gap.

Verified live on `llama3.2:3b`: five sub-questions, five claims each verified
word-for-word against the source, a cited synthesis, a generated bibliography.

---

## What leaves the machine

Nothing, unless asked. Retrieval, extraction, contradiction detection, gap
analysis and synthesis all run against local files and a local model.

The web is a **per-run** decision in the research screen, goes through the
author's own wigolo sidecar, writes every query to `research-trace.json`, and the
brief's method line names which sources came from the web. A run that did not use
it says *"Nothing left this machine."*

---

## Storage

```
research/sources/*.txt      extracted text, plain files the author owns
project.sqlite              sources, claims, concepts, links, blocks, events
research-trace.json         every phrase that ever left the machine
```

`project.sqlite` is committed by the snapshot code, so claims, block identity and
the provenance ledger travel with the prose in every Git snapshot.

---

## The interface

| Surface | What it shows |
| :--- | :--- |
| **Research** screen (⌘⇧R, or the database icon) | Sources, claims with their verbatim excerpts, contradictions, gaps, and the Ask tab that runs a full research pass |
| **Inspector → Lineage** | Every paragraph's event chain and derived authorship. A block with no events reads *"No record kept — that is not evidence that anyone in particular wrote it"* |
| **Integrity → Authorship** | Counts by origin, with `no record` on its own line, plus ledger verification |
| **Editor strip** | Appears above the prose only when there is something to say: *"Of 4 paragraphs: 1 model-written."* |
| **Assist → Use this** | Takes a proposal into the manuscript and records which model wrote it |
| **Settings → This project's ledger** | Chain verification, and the plain statement that it is tamper-*evident*, not tamper-proof |

### Why the editor strip is not a per-paragraph gutter

A gutter marker would need TipTap's document positions mapped onto our
content-anchored block ordinals, and those drift apart while the author is
mid-edit. A marker mis-aligned by one paragraph would put *"a model wrote this"*
beside the author's own sentence — precisely the confident-but-wrong signal this
whole feature exists to prevent. A count that is always right beats a position
that is usually right.

### The hole that "Use this" closes, and the one it does not

The Assist panel never inserted generated prose: the author copied it by hand,
which is good for trust and meant **AI-written text entered the manuscript with
no record whatsoever**. The agent loop was traced; the writing loop was not.

**Use this** inserts the proposal and records `create(ai_model)` then
`accept(human)` — "AI-generated text with human review".

**Copying it by hand still leaves no record**, and cannot: pasted text is
indistinguishable from typed text. The panel says so in those words rather than
letting an author assume the app can tell, and a paragraph with no events is
reported as *no record*, never as theirs.

---

## Production concerns that shaped the design

**Ledger volume.** The editor autosaves every 800ms, so a naive one-event-per-save
ledger writes ~4,500 rows for an hour on one paragraph and millions over a book.
A continuous session by one actor is coalesced into one event
([BLOCK-IDENTITY.md §3.1](BLOCK-IDENTITY.md)) — narrowly, so a model's edit or an
external one always starts a new event.

**Anchoring cost.** Reconciliation runs on every save. Pass 0 short-circuits on an
unchanged file; pass 1 removes nearly every block by exact hash; pass 2 compares
only what is left, per document rather than per project. Worst case — a 60-block
scene entirely rewritten — is held under 200ms by `perf03`.

**Extraction cost.** Contradiction detection is two-stage rather than O(n²) model
calls, and capped at 200 candidate pairs so a large corpus degrades into "checked
the most similar pairs" rather than hanging.

**A source that cannot be read is refused**, not registered empty. A silently
blank source would look like a paper with nothing to say.

---

## Agent access

`search_sources`, `list_claims`, `cite_claim` and `knowledge_gaps` are unguarded
gateway tools — they read local files and change nothing. Deep Continuity can
cite claims; World-Builder and Integrity can read the corpus and see where it is
thin. No profile can register or delete a source: the corpus is the author's.

---

## Deliberately not built

- **Publishing channels** (KDP, IngramSpark, Apple Books). A distribution surface,
  not a local-first authoring one.
- **C2PA credentials, Merkle roots, signing.** The ledger is tamper-*evident*;
  tamper-*proof* needs a signing identity and an external timestamp.
- **Cross-document block tracking.** A paragraph moved between chapters is a
  delete and a create. Project-wide matching on every save has a false-positive
  rate bad enough to make the feature dishonest.
- **OCR, Zotero and Crossref connectors.** Real, and not yet.
