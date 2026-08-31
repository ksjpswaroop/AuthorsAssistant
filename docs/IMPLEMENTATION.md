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
    kind       TEXT NOT NULL,             -- folder | scene | sheet | lore | research
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

Folder picking uses `tauri-plugin-dialog`. Nothing else touches a plugin.

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
