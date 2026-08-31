# Tech stack

Purpose: list the open-source stack for AegisDraft with **why** each piece is chosen and **how** it is used. Locked decisions are at the bottom.

## Summary table

| Component | Open-source technology | Role |
| :--- | :--- | :--- |
| Desktop shell | **Tauri** | Lightweight, secure Rust + webview desktop app |
| Frontend UI | **React**, **TypeScript**, **Tailwind CSS**, **Radix UI** | Dense Scrivener-like chrome with accessible primitives |
| Binder tree | **react-arborist** or **react-complex-tree** | Hierarchical drag-and-drop navigator |
| Rich text editor | **TipTap** (ProseMirror) | Focus modes, markdown, comments, typewriter scrolling |
| Corkboard | **dnd-kit** | Draggable index cards |
| Outliner | **TanStack Table** | Sortable, high-performance scene grid |
| Command bar | Radix dialog / **cmdk**-style palette | Natural-language entry to App Controller |
| Agent model | **Hermes 3** (8B default, 70B optional) via **Ollama** | Function calling, strict JSON, multi-step plans |
| Draft / style models | Llama-3-8B, Mistral-Nemo, Qwen-2.5 via Ollama | Prose generation and tone shifts |
| Agent runtime | **hermes-agent** (Nous Research) | Multi-step tool loops, memory, skills; API on `:18765` |
| RAG / indexing | **LlamaIndex** | Chunk, index, retrieve; tools agents call |
| Vector DB | **Qdrant** (default) or Chroma, local mode | Book-wide memory + internal similarity |
| Embeddings | **nomic-embed-text** (Ollama) | Local embeddings |
| AI detection | **Binoculars** / **DeBERTa-v3** (Hugging Face) | Local AI-writing indicators |
| Web similarity | **SearXNG** + **Trafilatura** + SentenceTransformers | Opt-in external phrase matching |
| Metadata DB | **SQLite** | Binder, sheets, settings, rubrics, style profile refs |
| Manuscript files | **Markdown** / HTML | Human-readable text if the app fails |
| Versioning | **LibGit2** | Background snapshots |
| Tool gateway | **FastAPI** on `:8765` + **Tauri IPC** | hermes skills → binder/editor/integrity mutations |

## Desktop and UI

- **Tauri** over Electron: smaller binary, Rust-backed FS/security, native feel for long writing sessions.
- **Vite + React + TypeScript** (not Next.js): official `create-tauri-app` `react-ts` template; SSR/routing is unnecessary in a local webview.
- **Tailwind + Radix**: consistent styling and accessible menus, dialogs, tabs, and inspector panels.
- **react-resizable-panels**: Scrivener-style binder | editor | inspector chrome (Phase 1).
- **TipTap** (`@tiptap/react`, `@tiptap/markdown`): headless ProseMirror; markdown serialization to on-disk scene files.
- **react-arborist / react-complex-tree**: Scrivener-like binder DnD (Phase 2; Phase 1 may use a stub list).
- **dnd-kit** + **TanStack Table**: corkboard and outliner as first-class views over the same binder IDs.
- **Tauri plugins (npm + Cargo):** `@tauri-apps/plugin-fs`, `plugin-dialog`, `plugin-sql` (sqlite) for project folders, Markdown I/O, and metadata.

Phase 1 package lock-in and install list: [phases/01-core-editor.md](phases/01-core-editor.md#library-choices-npm--tauri).

## Local AI and agents

- **Ollama** hosts Hermes 3 and draft models so inference never leaves the machine.
- **hermes-agent** is the agent runtime: plans, tool loops, memory, and extensible skills over HTTP (`127.0.0.1:18765`). See [HERMES-RUNTIME.md](HERMES-RUNTIME.md).
- **Hermes 3** is the model hermes-agent calls for App Controller, Continuity, Integrity, and World-Builder ([AGENTS.md](AGENTS.md)).
- Draft models (Llama-3, Mistral-Nemo, Qwen-2.5) remain available for continuation and style tasks where a full agent run is unnecessary.
- **Skills** wrap [TOOLS.md](TOOLS.md) endpoints on the FastAPI tool gateway (`:8765`); confirm interrupts for delete, compile, and web search.
- **LlamaIndex** owns indexing and retrieval; agents call `search_binder` rather than reinventing RAG.
- **Style profile + evolving skills** live on disk and deepen with use (voice matching, project-local skill packs).

## Integrity and web

- Internal similarity: embeddings into Qdrant/Chroma against the author’s own drafts.
- External: privacy-respecting **SearXNG** search → **Trafilatura** clean text → cosine similarity—only when the user opts in.
- Detection: **Binoculars** or **DeBERTa** via ONNX Runtime / vLLM on-device.

## Storage and versioning

- **SQLite** for structure and metadata; **Markdown/HTML** for prose (Scrivener folder-project idea).
- Style profile, hermes memory, and skill manifests under project/app data (local only).
- **LibGit2** under the hood for “Save Snapshot” and optional pre-agent commits.

## Bridge

- **hermes-agent** owns the LLM/tool loop.
- **FastAPI tool gateway** implements binder/editor/integrity tools; hermes skills call it.
- **Tauri IPC** carries UI events; Rust owns file I/O authority.
- See [TOOLS.md](TOOLS.md) and [ARCHITECTURE.md](ARCHITECTURE.md).

## Locked decisions

| Decision | Choice |
| :--- | :--- |
| Frontend host | **Vite + React** (not Next.js, not Svelte) |
| Desktop shell | **Tauri 2** via `create-tauri-app` `react-ts` |
| Phase 1 layout | **react-resizable-panels** |
| Rich text editor | **TipTap** + `@tiptap/markdown` |
| Binder tree (Phase 2) | **react-arborist** preferred; `react-complex-tree` acceptable |
| Local I/O | **plugin-fs** + **plugin-dialog** + **plugin-sql** (sqlite) |
| Vector database | **Qdrant** default; Chroma acceptable |
| Agent runtime | **hermes-agent** on `127.0.0.1:18765` (not LangGraph) |
| Agent model | **Hermes 3** via Ollama |
| Tool gateway | **FastAPI** on `127.0.0.1:8765`; Tauri owns file I/O authority |
| Privacy default | Fully offline; no cloud portal; web similarity opt-in only |
