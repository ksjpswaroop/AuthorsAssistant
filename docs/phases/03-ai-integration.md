# Phase 3: AI integration

Purpose: connect Ollama, index the book with LlamaIndex + Qdrant, and ship single-shot writing aids. Hermes 3 is in the catalog for Phase 5; multi-agent orchestration is not required yet. Status: **complete** — Ollama client, generation, chunking, embedding and retrieval are built and verified live against a real model, and the style profile now makes voice-matching a measurement rather than a label. Months 5–6. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 2.

## Goals

- Detect / configure Ollama; list and pull models
- Model catalog includes **Hermes 3** (agent brain later) plus draft models (Llama-3-8B, Mistral-Nemo, Qwen-2.5)
- Book-wide RAG: chunk → embed (`nomic-embed-text`) → Qdrant
- Expose retrieval as a path that becomes `search_binder`
- Continue, rewrite, tone shift, devil’s advocate (single-shot). Rewrite that replaces more than a sentence takes a [revision brief](../WRITING-LOOP.md#revision-brief) (`keep` / `change` / `voice_rules`).
- `draft_scene` loads the [context pack](../WRITING-LOOP.md#drafting-context-pack) (voice, sheets, lore, adjacent synopses, canon hits)—not only the open chapter

## Checklist

- [x] Settings UI: Ollama base URL, default draft model, default embed model
- [x] Health check for Ollama; clear errors when offline
- [x] Index pipeline on demand for manuscript + research
- [x] Chunking strategy documented — a plain overlapping word-window chunker in `rag.rs`, not LlamaIndex
- [x] Per-project vector store — SQLite with cosine similarity, not Qdrant (see below)
- [x] Retrieve top-k chunks for current scene context (characters, plot, world-bible, **canon**—not only open chapter)
- [x] Continue selection / last paragraph → generate next paragraphs
- [x] Rewrite / tone shifter prompts with style instructions from the measured profile
- [x] Devil’s advocate critique → prose in the Assist panel, never inserted automatically
- [x] **Style profile**: measure the author's own prose and inject it into rewrite and continue
- [x] Privacy: local by default. Remote models exist but require a per-session opt-in that names what leaves the machine — see the golden rule in [ARCHITECTURE.md](../ARCHITECTURE.md)
- [x] Document `search_binder` and `draft_scene` contracts for Phase 5 ([TOOLS.md](../TOOLS.md), [WRITING-LOOP.md](../WRITING-LOOP.md))

## Acceptance criteria

- With Ollama running, continue and rewrite work offline
- Asking about a character introduced only in chapter 1 returns relevant context when editing a later chapter
- Hermes 3 appears as an available model even if agents are not wired yet

## Out of scope

- hermes-agent multi-step agents, command bar App Controller (Phase 5 — see [HERMES-RUNTIME.md](../HERMES-RUNTIME.md))
- Web similarity and Binoculars reports (Phase 4)
- Mutating binder structure via natural language (Phase 5)

## What was built

- **Ollama client** (`ollama.rs`) on loopback, with generation and embedding model pools kept **disjoint by capability**. They have to be: asking a chat model to embed makes Ollama answer `This server does not support embeddings`, which blames the server for the caller's mistake and cost this project a wrong diagnosis (recorded in [evidence/README.md](../evidence/README.md)).
- **Reasoning models** are sent `think: false`, because otherwise the prose comes back in a `thinking` field and `response` is empty — a silent blank answer rather than an error.
- **RAG** (`rag.rs`): an overlapping word-window chunker, embeddings stored in the project's own SQLite, cosine similarity in Rust. Qdrant and LlamaIndex were both dropped: a per-project vector store of a few thousand chunks does not need a server, and adding one would break the promise that a project is a folder you can copy.
- **Style profile** (`style.rs`): the measured half — sentence length and variance, vocabulary range, dialogue balance, abstraction — computed over the prose the author marked **Done**, plus an optional model-written description of the voice that is labelled as the model's reading. Built without a model it is still useful; the measurements are the part that has to be right.
  - Vocabulary range is a type/token ratio averaged over fixed 100-word windows, and it is **not reported at all** below one window. A raw ratio falls as text gets longer, so comparing a 400-word scene to a 40,000-word profile would report a length difference as a style difference. The first version of this had that bug and a test caught it.
  - The profile says whether it came from finished work. With nothing marked Done it falls back to drafts and says so, rather than presenting a draft as a standard.
  - `assist` injects it into the system prompt for **continue** and **rewrite**, in plain language ("sentences average about 8 words, and lengths vary sharply") rather than as raw fields. Critique is deliberately left alone: a critic should react to the page, not to a target.
- **Inspector → Voice** shows how far the open document sits from the profile, worst dimension first, each with a note saying which way it is off.

## Notes

Keep generation APIs (continue, rewrite, retrieve) as callable services so World-Builder and Continuity can reuse them as tools. Prefer a different local model for critique than for draft when both are installed.
