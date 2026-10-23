# Roadmap

Purpose: implementation plan for AegisDraft. **Core phases 1–5** deliver the studio, local AI, integrity, and Hermes agents. **v2 phases 6–9** add Scrivener completeness, compile/export, and AI story visualization. All phases are **not started**.

| Phase | Months | Status | Doc |
| :--- | :--- | :--- | :--- |
| 1. Core editor | 1–2 | **Complete** | [phases/01-core-editor.md](phases/01-core-editor.md) |
| 2. Scrivener features | 3–4 | **Complete** | [phases/02-scrivener-features.md](phases/02-scrivener-features.md) |
| 3. Local AI + RAG | 5–6 | **Mostly complete** | [phases/03-ai-integration.md](phases/03-ai-integration.md) |
| 4. Integrity engine | 7–8 | Partly complete | [phases/04-integrity-engine.md](phases/04-integrity-engine.md) |
| 5. Hermes agents | 9–10 | Partly complete | [phases/05-hermes-agents.md](phases/05-hermes-agents.md) |
| 6. Scrivener completeness (v2) | 11–12 | Partly complete | [phases/06-scrivener-completeness.md](phases/06-scrivener-completeness.md) |
| 7. Compile and export (v2) | 13–14 | **Complete** | [phases/07-compile-export.md](phases/07-compile-export.md) |
| 8. AI story visualization (v2) | 15–16 | Partly complete | [phases/08-ai-story-visualization.md](phases/08-ai-story-visualization.md) |
| 9. AI viz advanced (v2) | 17–18 | Partly complete | [phases/09-ai-viz-advanced.md](phases/09-ai-viz-advanced.md) |

## Phase 1 — Core editor (Months 1–2)

**Outcomes:** Tauri + React + Tailwind shell; three-pane chrome; TipTap center editor; SQLite + Markdown save/load; “if the app dies, text remains.”

**Depends on:** Nothing.

## Phase 2 — Scrivener features (Months 3–4)

**Outcomes:** Full binder DnD; corkboard; outliner; inspector sheets; LibGit2 snapshots. Every binder mutation is designed so it can become a tool in [TOOLS.md](TOOLS.md).

**Depends on:** Phase 1.

## Phase 3 — Local AI + RAG (Months 5–6)

**Outcomes:** Ollama connection; Hermes 3 in the model catalog (for later agents) plus draft models; LlamaIndex + Qdrant book-wide RAG; single-shot continue / rewrite / tone / devil’s advocate. `search_binder` exists as a retrieval path even before full agents.

**Depends on:** Phase 2 (enough project text and structure to index).

## Phase 4 — Integrity engine (Months 7–8)

**Outcomes:** Internal similarity; opt-in web similarity (SearXNG + Trafilatura); local AI detection; rubric evaluator **including mechanical slop**; optional classified cuts. Each check is a **callable tool** for Phase 5.

**Depends on:** Phase 3 (embeddings / local model host).

## Phase 5 — hermes-agent workflows (Months 9–10)

**Outcomes:** hermes-agent on `:18765` + FastAPI tool gateway on `:8765` + Tauri IPC; profiles for App Controller, Deep Continuity, Integrity, World-Builder; command bar; integrity report UI; **writing-style profile**, **canon + debts**, **revision briefs**, and **evolving skills**. See [HERMES-RUNTIME.md](HERMES-RUNTIME.md), [WRITING-LOOP.md](WRITING-LOOP.md).

**Depends on:** Phases 2–4 (binder, RAG, integrity as tools). Agents only matter once those capabilities exist.

**Acceptance highlight:** One natural-language command creates Act 2 + three chapters + antagonist sheet and the UI updates without a menu click; “rewrite in my voice” uses the local style profile.

v2 work continues after this phase (Months 11–18).

---

## v2 — Phases 6–9

Enhancements layered on core. Phase docs 01–05 are not rewritten; v2 files stand alone.

### Phase 6 — Scrivener Completeness (Months 11–12)

**Outcomes:** Collections (manual, smart, saved search); Scrivenings mode; section types; multi-pane editor; bulk metadata; snapshot compare; writing goals; composition mode; templates; bookmarks; project keywords; custom icons; freeform corkboard as core.

**Depends on:** Phase 2.

**Doc:** [phases/06-scrivener-completeness.md](phases/06-scrivener-completeness.md)

### Phase 7 — Compile and Export (Months 13–14)

**Outcomes:** Compile pipeline with section-type rules; presets for DOCX, PDF, EPUB, Markdown, Fountain; filters by label/status/collection; `compile_manuscript` tool.

**Depends on:** Phase 6 (section types and collections).

**Doc:** [phases/07-compile-export.md](phases/07-compile-export.md)

### Phase 8 — AI Story Visualization Core (Months 15–16)

**Outcomes:** Plot diagrams (AI beat classification); dual-order timeline; character relationship graph; infinite canvas with commit-to-binder. Visualizations as projections over binder metadata.

**Depends on:** Phase 2 metadata + Phase 3 Ollama/RAG.

**Doc:** [phases/08-ai-story-visualization.md](phases/08-ai-story-visualization.md)

### Phase 9 — AI Visualization Advanced and Analytics (Months 17–18)

**Outcomes:** Mind maps; flowcharts; story-bible infographics; scene clustering → collections; pacing / redundancy / gap analytics; Hermes-callable v2 tools.

**Depends on:** Phase 8 + Phase 3 embeddings + Phase 4 similarity concepts.

**Doc:** [phases/09-ai-viz-advanced.md](phases/09-ai-viz-advanced.md)

## Related docs

- [PRODUCT.md](PRODUCT.md) — what authors experience
- [ARCHITECTURE.md](ARCHITECTURE.md) — how processes fit
- [AGENTS.md](AGENTS.md) / [TOOLS.md](TOOLS.md) — Phase 5 contracts (+ [v2 tools](TOOLS.md#v2-tools))
- [WRITING-LOOP.md](WRITING-LOOP.md) — canon, slop rubric, revision briefs, context pack
- [HERMES-RUNTIME.md](HERMES-RUNTIME.md) — hermes-agent ports, style, skills
- [TECH-STACK.md](TECH-STACK.md) — locked choices
