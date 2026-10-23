# Implementation

Purpose: describe the code that exists — how it is laid out, what actually works, what is a designed shell, and where the build departs from the plan in [TECH-STACK.md](TECH-STACK.md). Product intent lives in [PRODUCT.md](PRODUCT.md); the visual source of truth is the design canvas under `design/`.

Status: **Phase 1 complete.** Phases 2–5 are not started; their screens exist as marked shells.

## Layout

```
package.json          Vite + React + TypeScript frontend
index.html            Loads the three webfonts and mounts the app
src/
  main.tsx            Entry point; installs the save guards
  App.tsx             Routing, global shortcuts, error toast
  lib/
    tokens.ts         Design tokens lifted from design/*.dc.html
    types.ts          Mirrors src-tauri/src/model.rs
    api.ts            The only path to the outside world
    demoProject.ts    Sample project for the browser preview
    store.ts          Zustand store: project, selection, autosave
    format.ts         Word counts, relative time, ~ paths
  components/         Icon, ui primitives, chrome, binder, editor,
                      corkboard, outliner, inspector, overlay
  screens/            One file per screen
  styles/global.css   Reset, hover states, TipTap typography
src-tauri/
  src/main.rs         Binary entry
  src/lib.rs          Tauri commands — the whole IPC surface
  src/project.rs      Project folders, Markdown I/O, recents
  src/store.rs        SQLite schema and queries
  src/model.rs        Serde types shared with the frontend
  src/error.rs        One error type, serialised as a string
design/               The design canvas and its .dc.html artboards
```

## What a project is on disk

A project is a folder, not a binary file. If AegisDraft disappears, the prose is still there and still readable.

```
MyBook.aegis/
  project.sqlite              structure and metadata only — never prose
  manuscript/
    scenes/0001-opening-scene.md
    characters/
    worldbuilding/
    canon/
  research/
  assets/
  trash/                      deleted documents land here, not in the void
```

File names are `<counter>-<slug>.md`, derived from the title at creation and kept in step when a document is renamed. The slug is cosmetic: identity is the UUID in the database.

### Schema

```sql
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE documents (
    id         TEXT PRIMARY KEY,          -- uuid v4
    parent_id  TEXT REFERENCES documents(id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    kind       TEXT NOT NULL,             -- folder | scene | sheet | lore | research | canon
    path       TEXT,                      -- relative to the project root; NULL for folders
    position   INTEGER NOT NULL DEFAULT 0,
    synopsis   TEXT,
    label      TEXT,
    status     TEXT,
    word_count INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,          -- unix millis
    updated_at INTEGER NOT NULL
);
```

`word_count` is a cache of what is in the file, recomputed on every save by the Rust side. The database is never the only copy of anything a reader would care about.

## The IPC surface

Every command lives in `src-tauri/src/lib.rs`. The webview has no filesystem or database access of its own — this is the whole boundary:

| Command | Does |
| :--- | :--- |
| `create_project` | Creates `<parent>/<name>.aegis`, the folder tree, the schema, and one opening scene |
| `open_project` | Validates the folder and returns totals |
| `list_documents` | The whole binder tree, ordered |
| `read_document` | Markdown body for one document |
| `write_document` | Writes the body, then updates the word count |
| `create_document` | New row plus its file |
| `rename_document` | Retitles the row and renames the file to match |
| `update_document_meta` | Synopsis, label, status |
| `delete_document` | Removes the row; moves the file to `trash/` |
| `recent_projects` | Reads the recents list from the app config directory |

New projects also seed empty **Characters**, **Worldbuilding**, **Canon**, and **Research** folders. Existing projects opened after this change are not migrated; add a Canon folder by hand or wait for Phase 2.

## Saving

Writes go through `project::write_body`, which writes to a temporary file and renames it over the target. A crash mid-save leaves either the old file or the new one, never a truncated one, and the word count in the database is only updated after the rename succeeds.

The frontend saves the open buffer:

- 800 ms after the last keystroke (`store.ts`, `AUTOSAVE_MS`)
- immediately on Cmd/Ctrl-S
- before switching documents
- on window blur, tab hide, and unload (`installSaveGuards`)

If a keystroke lands while a write is in flight, the dirty flag survives the receipt rather than being cleared — the next tick saves the newer text.

## What is real, and what is a shell

Being explicit about this is the point. Every screen below exists and is reachable; the middle column says whether it does anything.

| Screen | State | Notes |
| :--- | :--- | :--- |
| Launch (`Welcome`) | **Live** | Real recents, real create, real open |
| Studio — binder | **Live** | Create, rename (double-click), delete, select, nest |
| Studio — editor | **Live** | TipTap over Markdown, autosave, dirty indicator |
| Studio — corkboard | **Live** | Real documents, real synopses, click to select |
| Studio — outliner | **Live** | Real documents, sortable by title, words, edited |
| Studio — inspector | **Partly live** | Synopsis, label and status write to SQLite; notes and the agent log are Phase 2 and 5 |
| Compile dialog | **Counts are real** | It totals the documents it would include. Compiling is Phase 2 |
| Command bar | Shell | Phase 5. Steps through the designed states and says plainly that nothing ran |
| Integrity | Shell | Phase 4. Sample findings, marked as such |
| Settings | Shell | Phase 3. Network toggles are real UI but not persisted |
| Deep Research | Shell | Phase 5. Real papers, invented metrics |
| Library | Shell | Phase 4 |
| Research paper | Shell | Phase 4 |
| Technical book | Shell | Phase 4 |

Shell screens carry a `PhaseBanner` naming the phase and saying what is not implemented. That component is not decoration — it is the difference between a prototype and a lie.

### Browser preview

Run `npm run dev` without the desktop shell and `api.ts` falls back to an in-memory backend seeded with the sample manuscript from the design canvas. Useful for reviewing the UI without a Rust toolchain. It never touches disk, and the launch screen says so.

## Departures from TECH-STACK.md

Each of these is a real decision, not an oversight. They need confirming or reverting.

1. **SQLite through `rusqlite` in the Rust core, not `@tauri-apps/plugin-sql`.** TECH-STACK locks the plugin; ARCHITECTURE says the Rust core owns SQLite access. They conflict, and the architecture won: the webview gets no database handle, no SQL is written in TypeScript, and the manuscript has exactly one writer. **Decision needed** — if the plugin is preferred, the store module is the only thing that changes.
2. **No `@tauri-apps/plugin-fs`.** Same reasoning: file I/O is Rust-side, behind typed commands. `plugin-dialog` is used for folder pickers.
3. **`react-resizable-panels` v4** exports `Group` / `Panel` / `Separator`, not the `PanelGroup` / `PanelResizeHandle` names in older docs. Sizes are strings for percentages.
4. **`@tiptap/extension-character-count` dropped.** Word counts come from Rust, which is the authority. Re-add it in Phase 2 when session statistics need per-keystroke counts.
5. **Native window controls.** The design canvas draws its own traffic lights. The app uses `titleBarStyle: "Overlay"` so macOS draws real ones over our title bar — a painted dot that does not close the window is worse than no dot. The title bar reserves 78 px on the left for them.
6. **Corkboard and outliner are views inside the studio**, switched from the title bar, exactly as the design canvas shows. `dnd-kit` and TanStack Table are not yet needed; they arrive in Phase 2 with drag-reordering and column configuration.

## Running it

```bash
npm install
npm run app        # Tauri desktop app (needs Rust)
npm run dev        # browser preview only, sample project
npm run typecheck  # tsc --noEmit
cd src-tauri && cargo test   # Rust unit tests, including the save round-trip
```

### Testing a screen without clicking through the launcher

```bash
# Build a project folder with a few scenes, synopses, labels and statuses
cd src-tauri && cargo run --example seed_project -- /tmp/AegisDemo "The Salt Road"

# Open it directly instead of stopping at the launch screen
AEGISDRAFT_OPEN="/tmp/AegisDemo/The Salt Road.aegis" npm run app
```

`AEGISDRAFT_OPEN` is read by the `startup_project` command and is a development
affordance only — the app never opens a project on its own without it.

## Branding, menus and the splash window

**Brand assets** are authored as SVG in `brand/src/` and rendered to `brand/export/` with `rsvg-convert`; the wordmark is rendered through headless Chrome so it uses the real font. The application icon set in `src-tauri/icons/` is generated from `brand/export/app-icon-1024.png` by `npx tauri icon`. Never edit anything in `export/` or `icons/` by hand. Full guidance: [BRAND.md](BRAND.md).

**The native menu bar** is built in `src-tauri/src/menu.rs` and installed in the Tauri `setup` hook. Menu items do not act on the project: they emit a `menu` event carrying a string id, which `App.tsx` maps onto the same store actions the in-app controls call. A menu command and its button therefore run identical code, and neither can drift from the other. Items belonging to unbuilt phases are `.enabled(false)` rather than silently inert.

**The splash window** is a second Tauri window (`splash.html`, a separate Vite entry) shown while the app boots. The main window starts `visible: false`; the frontend calls the `frontend_ready` command after its first effect runs, which closes the splash and reveals the main window. A 10-second watchdog thread in `run()` reveals the main window regardless — a webview that fails to boot must not leave someone staring at a splash screen with no way forward.

## Screens added for phases 6–9

| Screen | State | Notes |
| :--- | :--- | :--- |
| Studio — Scrivenings view | Shell | Reads real documents; stitches a folder's children with visible seams. Editing lands in Phase 6 |
| Studio — collections rail | Shell | Manual, smart and saved-search collections with their queries shown |
| Studio — composition mode | **Live** | Real chrome-free writing over the real editor; Escape leaves |
| Studio — bulk metadata bar | Shell | Appears on multi-select and states the blast radius |
| Inspector — section type, bookmarks, keywords | Shell | Phase 6 |
| Templates / Goals / Snapshot compare | Shell | Goals show the real project word total against a placeholder target |
| Story | Shell | Plot diagram, dual-order timeline, character graph, infinite canvas |
| Insights | Shell | Pacing and gaps, scene clusters, mind map, flowchart, story bible |

Composition mode is the only genuinely live addition — it wraps the real editor, so text typed there saves through the same path as everywhere else.

## App Store

Listing copy, privacy answers, entitlements, signing commands and the screenshot shot list are in [APP-STORE.md](APP-STORE.md). `tauri.conf.json` and `src-tauri/entitlements.plist` are configured; the one blocking code change is security-scoped bookmarks for recent projects, without which reopening a project fails on the second launch under the sandbox.

## Phase 2 — what changed

**Schema v2.** Columns are added one at a time by `add_column_if_missing`, so a project made by a Phase 1 build opens without a migration step. New: `notes`, `target`, `pov`, `plants`, `payoffs` on `documents`, plus three tables — `keywords` and `document_keywords` (the project vocabulary), and `document_links` (a sheet and the scenes it appears in, stored once and surfaced on both ends).

**New Rust modules and commands.** `snapshots.rs` wraps LibGit2. Fourteen commands were added: `move_document`, `create_sheet`, `append_canon_entry`, `update_document_extended`, `set_keywords`, `project_keywords`, `link_documents`, `unlink_documents`, and five `snapshot_*`.

**Structure moves are validated in Rust, not the UI.** `move_document` refuses a folder dropped inside itself or a descendant, and refuses nesting a document inside a non-folder. It renumbers both the old and new sibling lists inside one transaction, so positions stay dense and ordering is total. Getting this wrong detaches a subtree, which is why the guard lives below the UI where every caller — including a future agent tool — hits it.

**Snapshots are Git commits.** The repository lives in the project folder with no remote. Design decisions worth keeping:

- An identical tree makes no commit, so the history has no empty entries.
- `restore_file` commits whatever is on disk *before* overwriting, so restoring cannot eat text that was typed but never snapshotted. There is a test for exactly this.
- `.gitignore` excludes `*.sqlite-wal`, `*.sqlite-shm`, `*.md.tmp` and `trash/`; `project.sqlite` is committed, so structure restores alongside prose.
- `take_snapshot` in the store flushes the editor buffer to disk first — a snapshot that misses the last sentence would be worse than none.

**Frontend libraries**, as locked in the phase doc: `react-arborist` for the binder tree, `@dnd-kit` for corkboard cards, `@tanstack/react-table` for the outliner. Table was pinned to **v8**: v9 is published but has a different, young API (`createCoreRowModel`, a `tableFeatures` object) with far less documentation, and nothing in the phase requires it.

**Split view** is a read-only reference pane beside the editor (⌘⌥S), which is the "draft + research" case the phase asks for. Editing two documents at once is Phase 6's multi-pane work.

The screen table above is now out of date in one respect: the binder, corkboard, outliner, inspector metadata and snapshots are all **live**, not shells. Scrivenings, collections, bulk metadata and the Story/Insights screens remain shells for their own phases.

## Phases 3–9 — what was built

Nine new Rust modules, 52 Tauri commands, 102 Rust tests and 12 frontend tests.
Full evidence and reproduction: [evidence/README.md](evidence/README.md).

| Module | Phase | What it does |
| :--- | :--- | :--- |
| `collections.rs` | 6 | Manual and smart collections over binder ids — virtual sets, never file copies |
| `scrivenings.rs` | 6 | Stitch a folder's children into one buffer and split it safely back |
| `compile.rs` | 7 | Assemble and render Markdown, HTML, DOCX, EPUB, Fountain |
| `ollama.rs` | 3 | Loopback-only client: status, embeddings, generation |
| `rag.rs` | 3 | Chunking, an embedded vector index in the project's own SQLite, cosine search |
| `integrity.rs` | 4 | Shingle-based reuse detection, prose signals, rubric scoring |
| `analysis.rs` | 8, 9 | Beats, dual-order timeline, character graph, pacing, clustering |
| `agent.rs` | 5 | The tool gateway and its guardrails |

### Decisions worth revisiting

1. **The vector index lives in the project's SQLite, not Qdrant.** TECH-STACK
   locks Qdrant. A local-first writing app should not require the author to run a
   database server, and a book is thousands of vectors, not millions — exact
   cosine over that is fast and has no recall cliff to tune. **Confirm or revert.**
2. **Similarity uses word shingles, not embeddings.** It works with no model, no
   daemon and no network, and it catches what authors actually hit: a paragraph
   they wrote twice. Embedding-based similarity can be added alongside.
3. **There is no AI detector.** `integrity.rs` computes *signals* — sentence-length
   variance, vocabulary range, abstraction density. Every name, type and label
   says "signal" so the UI cannot present a heuristic as a verdict. A real
   detector needs Binoculars or DeBERTa weights this build does not ship.
4. **PDF exports print-ready HTML** rather than bundling a typesetting engine.
   The stylesheet carries `@page` rules; the OS print pipeline makes the PDF.
5. **Agent guardrails live in the gateway, not the runtime.** The runtime is a
   separate process that could be swapped; `GUARDED` tools require
   `confirmed: true`, which only the UI sets after asking a person. A model
   cannot argue its way past it.
6. **Generation is never inserted automatically.** The assist panel shows output
   and the author copies it. A tool that silently edits the manuscript is one the
   author stops trusting.

### Honest gaps

- **Embeddings are verified.** An earlier note here claimed the daemon lacked
  embedding support; that was a misreading of Ollama's error, which blames the
  server when the real problem is that a chat model cannot embed. The full loop
  — chunk, embed, store, search — runs against a real embedding model and a real
  project. See [evidence/README.md](evidence/README.md#a-wrong-diagnosis-corrected).
- **Embedding and generation models are separate pools** (`ollama::embedding_models`
  and `generation_models`). Mixing them fails in a way that looks like a broken
  feature rather than a wrong choice, so the split is enforced in Rust, mirrored
  in the UI, and asserted on both sides.
- **No hermes-agent runtime is installed**, so the gateway has been driven by
  tests, not by an agent.
- Phase 6: multi-pane editing (only a read-only reference pane), project and
  document templates, writing goals and freeform corkboard remain unbuilt.
- Phase 8: the canvas is read-only — `canvas_to_binder` does not exist.
- Phase 9: mind maps, flowcharts and story-bible infographics remain designs.
- Web similarity (SearXNG + Trafilatura) is not built, so nothing in the app makes
  a network request except the loopback calls to Ollama.
