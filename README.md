# AegisDraft

Local-first AI co-author for long-form books: Scrivener-class studio, Turnitin-class integrity, and Hermes 3 agents that *do things* in your project—not a chat box taped onto a document.

## Golden rule

**Local-first and privacy-centric.** Ollama, the vector store, SQLite, manuscript files, and agent thought traces all run on your machine. Unpublished work never touches a cloud server by default. Optional web similarity is explicit, opt-in, and never uploads the full manuscript.

## What it combines

| Pillar | Inspiration | What you get |
| :--- | :--- | :--- |
| **Writing studio** | Scrivener | Binder, corkboard, outliner, inspector, compile |
| **Integrity** | Turnitin | Similarity, AI-writing detection, rubrics, exportable reports |
| **Local AI** | Ollama + RAG | Book-wide memory, continue, rewrite, critique |
| **Agentic workflow** | Hermes 3 | Multi-step tools that mutate binder, editor, and reports |

AegisDraft is not a smart text editor with a side-panel chatbot. Hermes 3 agents create folders, write lore into the binder, highlight plot holes as inline comments, and synthesize integrity reports.

## Why Hermes 3

Nous Research’s **Hermes 3** is fine-tuned for function calling, strict JSON, and multi-step agentic reasoning. Via Ollama it can plan four-to-five-step workflows—create Act 2, add chapters, open a character sheet—without losing the thread, and return reports the UI can chart.

See [docs/AGENTS.md](docs/AGENTS.md) and [docs/TOOLS.md](docs/TOOLS.md).

## Docs

| Doc | Purpose |
| :--- | :--- |
| [docs/PRODUCT.md](docs/PRODUCT.md) | Positioning, personas, UX, privacy as a requirement |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Local-first system design and agent loop |
| [docs/TECH-STACK.md](docs/TECH-STACK.md) | Open-source stack and locked decisions |
| [docs/AGENTS.md](docs/AGENTS.md) | Four Hermes agents (Controller, Continuity, Integrity, World-Builder) |
| [docs/TOOLS.md](docs/TOOLS.md) | Tool catalog and IPC / FastAPI bridge |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Five-phase implementation roadmap |

## Tech stack (snapshot)

| Component | Choice |
| :--- | :--- |
| Desktop shell | Tauri (Rust + webview) |
| UI | React, TypeScript, Tailwind, Radix |
| Editor | TipTap (ProseMirror) |
| Agent brain | Hermes 3 via Ollama |
| Agent framework | LangGraph |
| RAG | LlamaIndex + Qdrant (local) |
| Storage | SQLite + Markdown + LibGit2 snapshots |
| Integrity | Local embeddings, SearXNG + Trafilatura (opt-in), Binoculars / DeBERTa |

Full rationale: [docs/TECH-STACK.md](docs/TECH-STACK.md).

## Roadmap (snapshot)

1. **Core editor** — Tauri + 3-pane + TipTap + save/load  
2. **Scrivener features** — Binder, corkboard, outliner, snapshots (become agent tools)  
3. **Local AI** — Ollama, RAG, inline continue / rewrite / critique  
4. **Integrity engine** — Similarity, web check, AI detection as callable tools  
5. **Hermes agents** — App Controller, Deep Continuity, Integrity, World-Builder  

Details: [docs/ROADMAP.md](docs/ROADMAP.md).

## Project status

**Documentation only.** Architecture and product direction are recorded; application code is not started.

## License

Private / unreleased unless otherwise stated.
