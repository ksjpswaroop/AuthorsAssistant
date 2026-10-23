# Phase 6: Scrivener Completeness (v2)

Purpose: close Scrivener-class gaps omitted from Phase 2—collections, Scrivenings, section types, studio polish—without duplicating binder files. Status: **complete**. Collections, Scrivenings, section types, bulk metadata, composition mode, snapshot compare, writing goals, project templates, the editable second pane, the freeform corkboard and custom icons are all built. Months 11–12. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 2. Does not replace [02-scrivener-features.md](02-scrivener-features.md).

## Goals

- Virtual **collections** (manual, smart, and saved-search) as alternative binder views
- **Scrivenings mode**: edit a folder’s children as one continuous TipTap document while files stay separate
- **Section types** for semantic structure and future compile
- Multi-pane editing, bulk metadata, snapshot compare, goals, composition mode, templates, bookmarks, project keywords, icons
- Elevate **freeform corkboard** from Phase 2 stretch to a core view here

## Architecture notes

| Concept | Model |
| :--- | :--- |
| **Collections** | Query- or list-backed virtual sets over binder IDs—no file copies |
| **Smart collections** | Auto-refresh from metadata filters (POV, status, label, keyword, section type) |
| **Saved search collections** | Persisted search queries shown as sidebar virtual folders |
| **Scrivenings** | Projection/edit session over ordered child IDs; saves write back to each scene file |
| **Section types** | SQLite metadata enum (e.g. Chapter Heading, Scene, Part Heading, Front Matter) |

Binder remains the hierarchical source of truth. Collections and Scrivenings are **views**.

## Checklist

### Collections and search

- [x] Manual collections: add/remove binder docs without moving them
- [x] Smart collections: auto-populate from metadata queries
- [x] Saved search → collection: pin frequent queries in the binder sidebar
- [x] Reading-order / alternate order independent of binder hierarchy (collection order)

### Scrivenings and split editing

- [x] Scrivenings mode: select folder (or multi-select) → stitch children into one continuous editable view
- [x] Scene boundaries visible but non-destructive; edits persist to underlying Markdown files
- [x] Multi-pane editor: two arbitrary documents side-by-side, the second one **editable**
- [x] Multi-pane: the main pane switches between editor, corkboard, outliner and Scrivenings while the second pane stays put
- [x] Supersedes Phase 2’s read-only “draft + research” pane

### Structure and metadata

- [x] Section types: Part Heading, Chapter Heading, Scene, Front Matter, Back Matter — settable per document in the inspector
- [x] Section type drives compile ([Phase 7](07-compile-export.md))
- [x] Bulk metadata operations: batch-set status, labels, section type, POV on multi-select
- [x] Project keywords: centralized vocabulary; assign from what the project already uses
- [x] Custom document/folder icons in the binder, with the kind's default when none is set
- [x] Freeform corkboard as a first-class view, with revert

### Snapshots, goals, focus, templates, links

- [x] Snapshot compare: real side-by-side diff of the current text against any LibGit2 snapshot, before restore
- [x] Writing goals: per-document, session and project targets, stored with the project
- [x] Composition mode: full-screen distraction-free writing
- [x] Project templates: novel, nonfiction, academic, technical book
- [x] Document templates — character and location sheets from templates, and **custom** ones: a template is a Markdown file under `.aegis/templates/` the author edits here or in any editor. Overriding a shipped shape replaces it; deleting the override brings it back, because it was overridden and never removed.
- [x] **Bookmarks**: internal cross-document links, with an inspector pane. Stored in the same `document_links` table sheets use, under the `bookmark` relation, and shown in **both** directions — a one-way link is how a reference quietly rots when the other document moves.

### Tool readiness

- [x] `bulk_set_metadata` is in the agent gateway
- [ ] `create_collection`, `query_smart_collection`, `open_scrivenings` are Tauri commands but are not exposed as agent tools — they are views, and an agent changing what the author is looking at is a worse experience than one that changes the work and says so

## Acceptance criteria

- Author creates a collection “Needs revision” from status filter; adding a scene’s status updates the smart collection without moving the file
- Selecting a chapter folder opens Scrivenings; editing paragraph in scene 2 updates only that scene’s Markdown on disk
- Multi-select of 20 scenes can set status + section type in one action
- Snapshot compare shows what changed before restore
- Composition mode hides chrome; floating inspector still shows synopsis/notes subset
- New project from template includes starter binder structure and sheet templates

## What was built

- **Snapshot compare** (`ProjectDialogs.tsx`) reads the real file out of the Git object store through `snapshot_read_file` and diffs it **by paragraph**, not by word. A character diff of a rewritten sentence is a confetti of highlights that is harder to read than the two versions side by side; prose is read in paragraphs, so that is the unit. Longest-common-subsequence, five tests.
- **Writing goals** live in the project's `meta` table, so they travel with the project instead of sitting in app preferences a second machine would not have. A session target needs a mark to count from, so **Start a session** stamps the current word count; before that the UI says *not started* rather than showing "0 of 2,000", which would imply a session running badly.
- **Project templates** seed a folder tree and empty files, and nothing else. A template that writes prose gives the author something to delete before they can begin. An unrecognised template name falls back to the novel tree rather than producing an empty project.
- **The second pane is editable.** It opens read-only — the common case is keeping a note in view — and an Edit toggle turns it into a real second editor with its own save-on-timer and save-on-blur. It deliberately does **not** share the main editor's `body` state: two panes on one buffer would overwrite each other.
- **Freeform corkboard** stores `x`/`y` on `documents`. `NULL` means *never placed*, which is not the same as placed at (0, 0) — the second is a choice the author made, and conflating them would make "reset to grid" impossible to express. Unplaced cards fall back to the position they would have had on the grid, so turning freeform on does not scatter the board.
- **Custom icons** on any binder row, with the kind's icon as the default. Because icon names now come from the database rather than only from code, `Icon` falls back to the generic document for a name it does not know — a silent gap in the binder is worse than a wrong icon — and `isIconName` is the guard.

### Two type-level bugs found on the way

- `IconName` was `keyof typeof PATHS` over a `Record<string, ReactNode>`, which widens to `string`. Every icon name typechecked, including wrong ones, and a wrong one rendered an empty box. The annotation is gone so the keys are inferred.
- The TypeScript `DocumentRow` had drifted from the Rust struct: `sectionType` and `icon` had existed in SQLite and in Rust for a phase without ever appearing in the frontend type. Both are now present, along with `x` and `y`.

## Out of scope

- Compile/export presets (Phase 7)
- AI plot diagrams, timelines, character graphs (Phase 8)
- Mind maps, clustering, pacing analytics (Phase 9)
- Editing Phase 2 checklist itself

## Agent tool readiness

Prefer small named commands so Hermes App Controller can create collections, open Scrivenings, and bulk-tag without menu clicks. Full schemas: [TOOLS.md](../TOOLS.md#v2-tools).
