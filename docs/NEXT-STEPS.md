# Next steps

Purpose: what to do next, in order, now that Phase 1 is built. Phase scope lives in [ROADMAP.md](ROADMAP.md); what exists today is in [IMPLEMENTATION.md](IMPLEMENTATION.md).

## Decisions needed before more code

These are yours, not the compiler's. Each one is cheap now and expensive later.

1. **SQLite ownership.** [TECH-STACK.md](TECH-STACK.md) locks `@tauri-apps/plugin-sql`; [ARCHITECTURE.md](ARCHITECTURE.md) says the Rust core owns SQLite. The build followed the architecture and uses `rusqlite` behind Tauri commands. Confirm that and correct the TECH-STACK row, or say the word and the store module moves to the plugin.
2. **Deep Research and the offline promise.** The research screens assume a network path the golden rule does not currently allow. The narrow version is designed — only generated questions leave, never the manuscript, with a visible domain allowlist and a request log. It needs writing into ARCHITECTURE.md and PRODUCT.md before Phase 5, or dropping.
3. **Project types.** Papers and technical books are designed as project types with their own binder shapes, inspectors and compile targets. That changes the `documents.kind` vocabulary and the compile pipeline. Decide whether they are Phase 4 or a separate track.
4. **A fifth agent.** Deep Research is not one of the four in [AGENTS.md](AGENTS.md). If it ships, AGENTS.md and TOOLS.md need `plan_research`, `fetch_source` and `extract_claims`.

## Blocking a Mac App Store build

- [ ] **Security-scoped bookmarks for recent projects.** `recent-projects.json` stores plain paths. Under the App Sandbox a plain path does not grant access on the next launch, so every recent project would fail to reopen. The bookmark must be created when the folder is picked and resolved on open — `project::read_recents` and `project::remember`. This is the single largest gap between the current build and a submittable one. Full context: [APP-STORE.md](APP-STORE.md).
- [ ] Test a signed, sandboxed build rather than `tauri dev` — the sandbox is not active in development, so this class of bug is invisible until packaged.

## Finishing Phase 1 properly

Small, worth doing before moving on:

- [ ] **Native menus.** New, Open, Save, Close Project exist only as in-app buttons. A desktop app without a File menu feels unfinished.
- [ ] **Window state.** Remember size and position between launches.
- [ ] **Reordering.** `documents.position` only reflects creation order — there is no UI to change it. Until Phase 2's drag-and-drop, even a "move up / move down" would help.
- [ ] **Move between folders.** A document cannot change parent today.
- [ ] **External-change detection.** If the project folder is edited outside the app while it is open, nothing notices. A file watcher, or at least an mtime check before saving.
- [ ] **Frontend tests.** The Rust side has four; the TypeScript side has none. The autosave race in `store.ts` (a keystroke landing while a write is in flight) is exactly the kind of logic that deserves a Vitest case.
- [ ] **Accessibility pass.** The binder is a tree without tree semantics or arrow-key navigation; overlays do not trap focus.
- [ ] **Bundle size.** One 796 kB chunk. Code-split the later-phase screens — they are the bulk and none of them are needed at launch.

## Phase 2 — Scrivener features

- Binder drag-and-drop (`react-arborist`), corkboard drag (`dnd-kit`), outliner columns (TanStack Table)
- Document notes and keywords (the inspector tabs are built and waiting)
- **Canon** folder and `kind: canon` (seeded on new projects; inspector/editing in this phase)
- Snapshots via LibGit2, and the snapshot list already sketched in the inspector
- Compile for real, Markdown first — the dialog already counts what it would include
- Import Markdown and existing folders

## Phase 3 — Local AI

- An Ollama client, and the model list the Settings screen is shaped for
- Persist model role assignments
- `nomic-embed-text` + Qdrant, indexing on save
- Continue, rewrite, critique — the editor's empty state already has the affordance, disabled; rewrite uses a KEEP list when replacing more than a sentence

## Phase 4 — Integrity, papers, technical books

- Internal similarity against the author's own drafts
- On-device AI detection through ONNX Runtime
- Rubric editor (including mechanical slop) and the exportable packet
- The paper and technical-book project types, the library, and LaTeX compile

## Phase 5 — hermes-agent

- hermes-agent on `:18765`, FastAPI tool gateway on `:8765`, the four agent profiles
- Skills wrapping TOOLS.md; style profile + evolving project skills; **canon, slop, revision briefs** ([WRITING-LOOP.md](WRITING-LOOP.md))
- Agent traces / memory written into the project folder, surfaced in the inspector tab that already exists
- Deep Research, if decision 2 above says yes
- Details: [HERMES-RUNTIME.md](HERMES-RUNTIME.md), [phases/05-hermes-agents.md](phases/05-hermes-agents.md)

## Fixed while building Phase 1

Recorded because each was found by using the thing rather than by reading it, and the same classes of bug will recur:

- **Deleting a folder orphaned its children's files.** The SQL cascade removed the rows; the files stayed on disk with nothing pointing at them. Now the whole subtree moves to `trash/`. Covered by a test.
- **Scene file names were always numbered `0001`.** The counter only incremented on collision, so `0001-a.md`, `0001-b.md` — the number conveyed nothing and the folder did not read in order. It is now the next free sequence number in that directory. Covered by a test.
- **Paragraphs in the editor had no spacing.** `.tiptap p { margin: 0 }` out-specified `.tiptap > * + * { margin-top: 22px }`, so prose ran together. Found by looking at the running app, not by typechecking it.

## Beyond Phase 5

[ROADMAP.md](ROADMAP.md) now carries four v2 phases — Scrivener completeness, compile and export, AI story visualization, and advanced visualization. None of their screens exist in the design canvas or the app yet. When they are ready to design, the canvas at `design/aegisdraft-tauri-app.html` is where they should be drawn first; the app's shell, tokens and primitives already carry them.

## Debt to watch

- **Sample data is scattered.** Each shell screen carries its own fixtures. When two of them need to agree, move them to one module.
- **`types.ts` mirrors `model.rs` by hand.** It is small enough today. If it grows, generate it.
- **The demo backend duplicates command semantics.** It is genuinely useful for design review without Rust, but it is a second implementation that can drift.
