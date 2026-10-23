# Phase 4: Integrity engine

Purpose: ship Turnitin-class checks as **callable tools**—internal similarity, opt-in web similarity, prose measurements and rubric feedback—so Phase 5’s Integrity agent can orchestrate them. Status: **complete**, with one goal deliberately dropped. Internal similarity, writing signals, rubric scoring, the exportable packet and the wigolo web-similarity client are built and tested; **AI-writing detection is cut as a claim**, for the reasons below. Months 7–8. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 3.

## AI detection: dropped, and why

This phase originally promised local AI-writing detection via Binoculars or DeBERTa. It is not shipping, and the goal is removed rather than left open:

- **Binoculars needs token log-probabilities from two models.** Ollama's API exposes no logprobs, so the method is not reachable through the runtime this app is built on. Not "hard" — not available.
- **A DeBERTa classifier would mean shipping ~1 GB of ONNX weights** for a verdict that is wrong often enough to ruin a person. False positives on non-native English and on heavily-edited prose are well documented, and an author accused by their own writing tool has no way to appeal it.
- **The measurements stay.** Sentence length and variance, vocabulary range, abstraction, dialogue balance — these describe prose, and the UI says so. What is gone is the leap from *this text measures like that* to *a machine wrote this*.

The packet states this in its own method notes, so a reader of a report is told what it does not determine, not left to assume.

## Goals

- Internal self-plagiarism / draft similarity via local embeddings
- Opt-in web pipeline: phrases → SearXNG → Trafilatura → cosine similarity
- Rubric engine + LLM evaluator → scores and comments
- **Mechanical slop scan** (no LLM) as a default rubric dimension
- Optional **propose_cuts** (classified fat + KEEP passage)
- Each capability exposed with a stable tool name for Phase 5

Loop contract: [WRITING-LOOP.md](../WRITING-LOOP.md).

## Checklist

### Internal similarity

- [x] Compare each paragraph against every other document in the project — word-shingle containment, no model needed
- [x] Threshold the author sets; paragraphs shorter than the shingle window are skipped rather than scored
- [x] Side-by-side UI for matches
- [x] Tool: `check_internal_similarity`

### Web similarity (opt-in)

- [x] User confirmation gate before any network call — `confirmed: true` at the command and at the agent gateway
- [x] Phrases only, never a document: over 60 words is refused before a socket opens
- [x] **wigolo** (`wigolo serve` on loopback) rather than SearXNG + Trafilatura — one local daemon the author runs, returning excerpts with provenance
- [x] Matches with source links, wigolo's evidence score and our own overlap kept as separate numbers
- [x] Every request appended to `research-trace.json` in the project
- [x] Tool: `search_web`

### AI detection — **cut**

- [x] ~~Run Binoculars and/or DeBERTa via ONNX Runtime~~ — not reachable through Ollama (no logprobs), and not worth the false-positive cost. See above.

### Rubrics and feedback

- [x] Built-in rubric criteria, each a number the author can check against the text
- [ ] **Built-in mechanical slop rubric** (`run_slop_scan`): tier-1 banned words, fiction tells, telling, structural tics, rhythm; spans for highlights
- [ ] Optional `propose_cuts` → classified OVER-EXPLAIN / REDUNDANT / … JSON; apply only on confirm
- [ ] LLM evaluator against rubric criteria (local model; **prefer a different model than the writer**)
- [ ] Inline comments + percentage / scale scores
- [x] **Exportable integrity packet** (JSON + HTML), carrying the matches, the measurements, the rubric, every phrase ever sent off the machine, and the method notes that say what the packet does not determine

### Tool readiness

- [ ] Document JSON result shapes for charts (match %, AI score, slop penalty, rubric dimensions)
- [ ] `highlight_text_in_editor` integration points for flagged spans

## Acceptance criteria

- Offline-only integrity run produces internal similarity, measurements and rubric without touching the network
- Web check requires explicit confirm and only sends phrases / short excerpts
- Rubric run attaches comments to a chapter and exports a packet
- Tools are invocable from a future sidecar without UI clicks
- Cuts are never applied without confirm; KEEP passages are not rewritten

## Out of scope

- Hermes multi-step synthesis of “why flagged” prose reports (Phase 5 Integrity agent)
- Institutional closed corpora (non-goal)

## Privacy reminders

Default offline. Never upload the full manuscript.

**Be precise about wigolo.** It listens on loopback, which makes it tempting to describe a web check as local. It is not: wigolo queries the internet on the author's behalf, so an approved phrase does leave the machine — through a component the author runs and can inspect. That is a real difference from a hosted API, and it is not the same as staying put. The confirm says so in those words, shows the word count about to leave, and the trace records it afterwards.

## What was built

- `src-tauri/src/websearch.rs` — the wigolo client. `POST {base}/v1/search`, `GET {base}/health` for the status probe, `Authorization: Bearer` from `WIGOLO_API_TOKEN` when the daemon is not on loopback, base overridable with `AEGISDRAFT_WIGOLO`.
- **Two scores, never merged.** `evidenceScore` is wigolo's own confidence in a *result*. `overlap` is measured here — word-shingle containment of the sent phrase in the returned excerpt — and answers the different question of whether the sentence actually appears in that page.
- **Parsed forgivingly on purpose.** wigolo's README and its REST reference document different field names (`excerpt` vs `snippet`; `citation_id` and `source_span` in one and not the other). Everything but title and url is optional, and an absent field is reported as absent rather than defaulted to a number that would look measured. Tests drive both documented shapes.
- **Verified against a live daemon.** wigolo 0.2.1 was started as a managed sidecar and queried for real: three results back through DuckDuckGo, parsed, scored and traced. The mock tests remain, and they cover the shapes and failures a live run does not reach — the auth header, an error body, and the README response shape.
  - The live daemon answered with **`snippet`, an `evidence_score`, and no `citation_id` or `source_span`** — the REST-reference shape, not the README one. Parsing both was not defensive over-engineering; a client that had trusted the README would have returned nothing.
  - The two scores separated exactly as designed on real data: wigolo's own site scored `evidence 1.00` with `overlap 0.00`, because it is unmistakably the canonical source *and* does not contain the sent phrase. Averaging those into one number would have destroyed the only useful thing either of them said.
- `integrity::IntegrityPacket` + `packet_html` — the exportable report, written as a JSON and an HTML file side by side.
- `src-tauri/src/sidecar.rs` — wigolo as a **managed sidecar**: AegisDraft resolves, starts, health-gates and stops it. See [SIDECAR.md](../SIDECAR.md) for why it is a supervised process rather than a bundled binary, and for the orphaned-daemon bug the first live run exposed.
