# Phase 4: Integrity engine

Purpose: ship Turnitin-class checks as **callable tools**—internal similarity, opt-in web similarity, local AI detection, and rubric feedback—so Phase 5’s Integrity agent can orchestrate them. Status: **not started**. Months 7–8. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 3.

## Goals

- Internal self-plagiarism / draft similarity via local embeddings
- Opt-in web pipeline: phrases → SearXNG → Trafilatura → cosine similarity
- Local AI-writing detection (Binoculars / DeBERTa)
- Rubric engine + LLM evaluator → scores and comments
- Each capability exposed with a stable tool name for Phase 5

## Checklist

### Internal similarity

- [ ] Chunk chapter text; query Qdrant against same project and optional other local projects
- [ ] Thresholds and exclude quotes / small matches
- [ ] Side-by-side UI for matches
- [ ] Tool: `check_internal_similarity`

### Web similarity (opt-in)

- [ ] User confirmation gate before any network call
- [ ] Extract key phrases from selection (not full manuscript upload)
- [ ] SearXNG query → top N URLs → Trafilatura extract → SentenceTransformers similarity
- [ ] Color-coded matches with source links
- [ ] Tool: `search_web`

### AI detection

- [ ] Run Binoculars and/or DeBERTa via ONNX Runtime / local host
- [ ] Paragraph- and document-level scores
- [ ] Tool: `run_ai_detection`

### Rubrics and feedback

- [ ] Author/teacher-defined rubrics (pacing, show-don’t-tell, dialogue tags, custom)
- [ ] LLM evaluator against rubric criteria (local model)
- [ ] Inline comments + percentage / scale scores
- [ ] Exportable integrity packet (JSON + human-readable)

### Tool readiness

- [ ] Document JSON result shapes for charts (match %, AI score, rubric dimensions)
- [ ] `highlight_text_in_editor` integration points for flagged spans

## Acceptance criteria

- Offline-only integrity run produces internal similarity + AI detection without network
- Web check requires explicit confirm and only sends phrases / short excerpts
- Rubric run attaches comments to a chapter and exports a packet
- Tools are invocable from a future sidecar without UI clicks

## Out of scope

- Hermes multi-step synthesis of “why flagged” prose reports (Phase 5 Integrity agent)
- Institutional closed corpora (non-goal)

## Privacy reminders

Default offline. Never upload the full manuscript. Agent traces stay local when Phase 5 lands.
