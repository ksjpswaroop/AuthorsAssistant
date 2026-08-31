# Phase 3: AI integration

Purpose: connect Ollama, index the book with LlamaIndex + Qdrant, and ship single-shot writing aids. Hermes 3 is in the catalog for Phase 5; multi-agent orchestration is not required yet. Status: **not started**. Months 5–6. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 2.

## Goals

- Detect / configure Ollama; list and pull models
- Model catalog includes **Hermes 3** (agent brain later) plus draft models (Llama-3-8B, Mistral-Nemo, Qwen-2.5)
- Book-wide RAG: chunk → embed (`nomic-embed-text`) → Qdrant
- Expose retrieval as a path that becomes `search_binder`
- Inline tools: continue (`Ctrl+Space`), rewrite, tone shift, devil’s advocate (single-shot)

## Checklist

- [ ] Settings UI: Ollama base URL, default draft model, default embed model
- [ ] Health check for Ollama; clear errors when offline
- [ ] Index pipeline on save / on demand for manuscript + research
- [ ] LlamaIndex (or equivalent) chunking strategy documented
- [ ] Local Qdrant (default) collection per project
- [ ] Retrieve top-k chunks for current scene context (characters, plot, world-bible—not only open chapter)
- [ ] Continue selection / last paragraph → generate next ~3 paragraphs
- [ ] Rewrite / tone shifter prompts with style instructions
- [ ] Devil’s advocate critique → markdown or simple comment list in inspector
- [ ] Privacy: all inference local; no cloud LLM fallback
- [ ] Document `search_binder` contract for Phase 5 ([TOOLS.md](../TOOLS.md))

## Acceptance criteria

- With Ollama running, continue and rewrite work offline
- Asking about a character introduced only in chapter 1 returns relevant context when editing a later chapter
- Hermes 3 appears as an available model even if agents are not wired yet

## Out of scope

- LangGraph multi-step agents, command bar App Controller (Phase 5)
- Web similarity and Binoculars reports (Phase 4)
- Mutating binder structure via natural language (Phase 5)

## Notes

Keep generation APIs (continue, rewrite, retrieve) as callable services so World-Builder and Continuity can reuse them as tools.
