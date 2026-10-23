# Phase 2: Scrivener features

Purpose: deliver binder, corkboard, outliner, inspector metadata, and LibGit2 snapshots so AegisDraft feels like a Scrivener-class studio. Status: **complete**, except the freeform corkboard stretch goal (moved to Phase 6, where it is a core view). Months 3–4. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 1.

## Goals

- Hierarchical binder with drag-and-drop
- Corkboard and outliner bound to the same binder items
- Inspector for synopsis, notes, keywords, character/location sheets
- **Canon** binder folder (`kind: canon`) for falsifiable continuity facts
- “Save Snapshot” via LibGit2
- Every binder mutation designed as a future agent tool ([TOOLS.md](../TOOLS.md); loop: [WRITING-LOOP.md](../WRITING-LOOP.md))

## Checklist

- [x] Binder tree with `react-arborist` or `react-complex-tree`
- [x] Drag-and-drop reorder, nest, and move between Draft / Research / Trash
- [x] Labels, status stamps, custom metadata columns on documents
- [x] Corkboard view with `dnd-kit` index cards (synopsis + status color)
- [ ] Freeform corkboard optional stretch goal — **deferred to Phase 6**, which makes it a core view
- [x] Outliner via TanStack Table (word count, target, status, POV, custom fields)
- [x] Corkboard ↔ outliner ↔ editor stay in sync on binder IDs
- [x] Inspector: synopsis, document notes, keywords, snapshots list
- [x] Character and location sheet types linked to scenes
- [x] **Canon folder** on new projects; `documents.kind = canon`; files under `manuscript/canon/`
- [x] Outline/synopsis fields can store plant/payoff ids for the foreshadowing ledger
- [x] Split view: draft + research reference (read-only reference pane)
- [x] LibGit2 init inside project folder; snapshot = commit
- [x] Restore snapshot UI (list commits, checkout / restore paths safely)
- [x] Import an existing folder of Markdown, mirroring the directory tree into the binder
- [x] Document which UI actions map to future tools: `create_folder`, `create_chapter`, `create_character_sheet`, `save_to_binder`, `append_canon_entry`

## Acceptance criteria

- Author can restructure a multi-chapter binder by drag-and-drop without losing files
- Switching Editor / Corkboard / Outliner shows the same set of binder items
- Creating a character sheet stores metadata in SQLite and a sheet file on disk
- New and existing novel projects expose a **Canon** binder folder; a canon entry is Markdown with a source, not lore prose
- Save Snapshot creates a recoverable LibGit2 commit

## Out of scope

- Ollama generation and RAG (Phase 3)
- Similarity / AI detection (Phase 4)
- Command bar Hermes App Controller (Phase 5)—but APIs should be tool-ready

## Tool readiness note

Prefer small, named Rust/Python commands over opaque UI-only mutations so Phase 5 can wrap them without a rewrite.

## What was built

- **Binder** on `react-arborist`: drag to reorder, nest and move between folders, virtualized, with inline rename and Cmd-click multi-select for bulk edits. A folder cannot be dropped inside itself or a descendant, and a document cannot be nested inside a non-folder — both refused in Rust, not just in the UI.
- **Corkboard** on `dnd-kit`: cards for one folder's children, dragged to reorder. A 5px activation distance keeps a click from being read as a drag. Reordering writes through the same `move_document` command the binder uses.
- **Outliner** on TanStack Table (v8): sortable columns, a column picker, POV and Target hidden by default, target rendered as progress against the real word count.
- **Inspector**: synopsis, label, status, **notes**, **keywords**, **POV**, **per-document target**, snapshots, and the plant/payoff fields for the foreshadowing ledger. Keywords are backed by a project-wide vocabulary table that suggests what the project already uses.
- **Sheets**: `create_sheet` makes a character or location sheet from a template with headings, files it in the right folder, and can link it to the scene it was created from. Locations are their own `kind` with `manuscript/locations/`.
- **Canon**: `append_canon_entry` refuses an entry without both a fact and the document that establishes it — a canon entry with no source is an opinion.
- **Import** (`src-tauri/src/import.rs`): File → Import Markdown Folder… surveys the folder first and creates nothing until the author confirms, so the count of documents about to appear is known in advance. Sub-folders become binder folders, `.md`/`.markdown`/`.txt` files become documents, and a file's own first `# heading` names it in preference to its filename. It refuses a folder that is already an AegisDraft project, refuses to import the project into itself, skips hidden tooling (`.git`, `.obsidian`) without reporting it as a loss, and names every other file it passed over. Because each document is created through `create_document`, which allocates its own sequence-numbered filename, an import can never overwrite prose already in the project — importing the same folder twice gives two copies, not a merge.
- **Snapshots** via LibGit2: a snapshot is a real Git commit inside the project folder, so the history outlives the app. Identical trees make no commit. Restoring a file snapshots any uncommitted work first, so an undo cannot eat unsaved text. SQLite side files are ignored; `project.sqlite` is committed, so structure restores with the prose.

Fifteen Rust tests cover the round trip, import (mirroring, refusals, titling, and that a second import overwrites nothing), file numbering, folder deletion, move/reorder including the cycle guard, snapshot round trip, restore-preserves-unsaved-work, and a project-level snapshot that proves side files stay out.

## UI action → tool mapping

Every mutation is already a small named command, so Phase 5 can wrap each one without a rewrite ([TOOLS.md](../TOOLS.md)).

| UI action | Rust command | Future tool |
| :--- | :--- | :--- |
| Binder “+ folder” | `create_document(kind=folder)` | `create_folder` |
| Binder “+ scene” | `create_document(kind=scene)` | `create_chapter` |
| File → New Character Sheet | `create_sheet(kind=sheet)` | `create_character_sheet` |
| File → New Location Sheet | `create_sheet(kind=location)` | `create_location_sheet` |
| Drag in binder or corkboard | `move_document(id, parent, index)` | `move_document` |
| Inspector synopsis / label / status | `update_document_meta` | `save_to_binder` |
| Inspector notes / POV / target / plants / payoffs | `update_document_extended` | `set_document_fields` |
| Inspector keywords | `set_keywords`, `project_keywords` | `bulk_set_metadata` (Phase 6) |
| Sheet linked to a scene | `link_documents(relation="appears_in")` | `link_documents` |
| Canon entry | `append_canon_entry(fact, source)` | `append_canon_entry` |
| File → Import Markdown Folder… | `import_survey`, `import_folder` | human-in-the-loop only |
| Take snapshot | `snapshot_create(message)` | `snapshot_project` |
| Restore a version | `snapshot_restore_file(commit, path)` | human-in-the-loop only |

Deletion, compile and web access keep their confirm requirement; `snapshot_restore_file` is deliberately not proposed as an agent tool.
