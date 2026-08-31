# AegisDraft

Local-first AI co-author for long-form books: Scrivener-class studio, Turnitin-class integrity, and **hermes-agent** that *does things* in your project—learns your voice, grows skills with use, and never chat-boxes around your manuscript.

## Golden rule

**Local-first and privacy-centric.** Ollama, hermes-agent, the vector store, SQLite, manuscript files, style profile, skills, and agent traces all run on your machine. Unpublished work never touches a cloud server by default. Optional web similarity is explicit, opt-in, and never uploads the full manuscript.

## What it combines

| Pillar | Inspiration | What you get |
| :--- | :--- | :--- |
| **Writing studio** | Scrivener | Binder, corkboard, outliner, inspector, compile |
| **Integrity** | Turnitin | Similarity, AI-writing detection, rubrics, exportable reports |
| **Local AI** | Ollama + RAG | Book-wide memory, continue, rewrite, critique |
| **Agentic workflow** | hermes-agent + Hermes 3 | Multi-step tools, voice learning, evolving skills |

AegisDraft is not a smart text editor with a side-panel chatbot. hermes-agent creates folders, writes lore into the binder, highlights plot holes as inline comments, synthesizes integrity reports, and gets richer the more you write.

## Why hermes-agent + Hermes 3

- **Hermes 3** (Ollama): function calling, strict JSON, multi-step plans
- **hermes-agent** (Nous Research): local runtime with memory and extensible skills—API on `127.0.0.1:18765`
- Learns your **writing style** from accepted prose; proposes new **skills** after repeated workflows

See [docs/HERMES-RUNTIME.md](docs/HERMES-RUNTIME.md), [docs/AGENTS.md](docs/AGENTS.md), and [docs/TOOLS.md](docs/TOOLS.md).

## Docs

| Doc | Purpose |
| :--- | :--- |
| [docs/PRODUCT.md](docs/PRODUCT.md) | Positioning, personas, UX, privacy as a requirement |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Local-first system design and agent loop |
| [docs/TECH-STACK.md](docs/TECH-STACK.md) | Open-source stack and locked decisions |
| [docs/HERMES-RUNTIME.md](docs/HERMES-RUNTIME.md) | hermes-agent ports, profiles, style, skills |
| [docs/AGENTS.md](docs/AGENTS.md) | Four agents (Controller, Continuity, Integrity, World-Builder) |
| [docs/TOOLS.md](docs/TOOLS.md) | Tool catalog and skills → FastAPI bridge |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Core + v2 implementation roadmap |
| [docs/IMPLEMENTATION.md](docs/IMPLEMENTATION.md) | The code that exists: layout, what works, departures from the plan |
| [docs/DESIGN-SYSTEM.md](docs/DESIGN-SYSTEM.md) | Palette, type, components, and the honesty rules |
| [docs/NEXT-STEPS.md](docs/NEXT-STEPS.md) | Decisions needed and the ordered backlog |

## Tech stack (snapshot)

| Component | Choice |
| :--- | :--- |
| Desktop shell | Tauri (Rust + webview) |
| UI | React, TypeScript, Tailwind, Radix |
| Editor | TipTap (ProseMirror) |
| Agent model | Hermes 3 via Ollama |
| Agent runtime | hermes-agent (`:18765`) |
| Tool gateway | FastAPI (`:8765`) + Tauri IPC |
| RAG | LlamaIndex + Qdrant (local) |
| Storage | SQLite + Markdown + LibGit2 snapshots |
| Integrity | Local embeddings, SearXNG + Trafilatura (opt-in), Binoculars / DeBERTa |

Full rationale: [docs/TECH-STACK.md](docs/TECH-STACK.md).

## Roadmap (snapshot)

1. **Core editor** — Tauri + 3-pane + TipTap + save/load — **done**  
2. **Scrivener features** — Binder, corkboard, outliner, snapshots (become agent tools)  
3. **Local AI** — Ollama, RAG, inline continue / rewrite / critique  
4. **Integrity engine** — Similarity, web check, AI detection as callable tools  
5. **hermes-agent** — Profiles, skills, voice learning, App Controller / Continuity / Integrity / World-Builder  

Details: [docs/ROADMAP.md](docs/ROADMAP.md).

## Project status

**Phase 1 complete.** The Tauri writing studio runs: create or open a project, write in TipTap, and everything saves as Markdown beside a SQLite index. Later phases are not started — their screens exist, each marked with the phase that will build it.

```bash
npm install
npm run app        # desktop app (needs Rust)
npm run dev        # browser preview with a sample project, no Rust needed
```

What exists, what is a shell, and where the build departs from the plan: [docs/IMPLEMENTATION.md](docs/IMPLEMENTATION.md). What to do next: [docs/NEXT-STEPS.md](docs/NEXT-STEPS.md).

## License

Private / unreleased unless otherwise stated.
