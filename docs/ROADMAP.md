# Roadmap

Purpose: five-phase implementation plan for AegisDraft. All phases are **not started**. Agents land last because they need binder, RAG, and integrity as real tools.

| Phase | Months | Status | Doc |
| :--- | :--- | :--- | :--- |
| 1. Core editor | 1–2 | Not started | [phases/01-core-editor.md](phases/01-core-editor.md) |
| 2. Scrivener features | 3–4 | Not started | [phases/02-scrivener-features.md](phases/02-scrivener-features.md) |
| 3. Local AI + RAG | 5–6 | Not started | [phases/03-ai-integration.md](phases/03-ai-integration.md) |
| 4. Integrity engine | 7–8 | Not started | [phases/04-integrity-engine.md](phases/04-integrity-engine.md) |
| 5. Hermes agents | 9–10 | Not started | [phases/05-hermes-agents.md](phases/05-hermes-agents.md) |

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

**Outcomes:** Internal similarity; opt-in web similarity (SearXNG + Trafilatura); local AI detection; rubric evaluator. Each check is a **callable tool** for Phase 5.

**Depends on:** Phase 3 (embeddings / local model host).

## Phase 5 — Hermes 3 agentic workflows (Months 9–10)

**Outcomes:** FastAPI sidecar + Tauri IPC; LangGraph graphs for App Controller, Deep Continuity, Integrity, World-Builder; command bar; integrity report UI driven by agent JSON.

**Depends on:** Phases 2–4 (binder, RAG, integrity as tools). Agents only matter once those capabilities exist.

**Acceptance highlight:** One natural-language command creates Act 2 + three chapters + antagonist sheet and the UI updates without a menu click.

## Related docs

- [PRODUCT.md](PRODUCT.md) — what authors experience
- [ARCHITECTURE.md](ARCHITECTURE.md) — how processes fit
- [AGENTS.md](AGENTS.md) / [TOOLS.md](TOOLS.md) — Phase 5 contracts
- [TECH-STACK.md](TECH-STACK.md) — locked choices
