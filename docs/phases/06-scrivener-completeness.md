# Phase 6: Scrivener Completeness (v2)

Purpose: close Scrivener-class gaps omitted from Phase 2—collections, Scrivenings, section types, studio polish—without duplicating binder files. Status: **not started**. Months 11–12. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 2. Does not replace [02-scrivener-features.md](02-scrivener-features.md).

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

- [ ] Manual collections: add/remove binder docs without moving them
- [ ] Smart collections: auto-populate from metadata queries
- [ ] Saved search → collection: pin frequent queries in the binder sidebar
- [ ] Reading-order / alternate order independent of binder hierarchy (collection order)

### Scrivenings and split editing

- [ ] Scrivenings mode: select folder (or multi-select) → stitch children into one continuous editable view
- [ ] Scene boundaries visible but non-destructive; edits persist to underlying Markdown files
- [ ] Multi-pane editor: two arbitrary documents side-by-side
- [ ] Multi-pane: same project in two views (e.g. outliner + editor, or corkboard + editor)
- [ ] Clarify and supersede Phase 2’s “draft + research” as true dual-doc / dual-view split

### Structure and metadata

- [ ] Section types: Chapter Heading, Scene, Part Heading, Front Matter (extensible)
- [ ] Section type drives inspector + future compile behavior
- [ ] Bulk metadata operations: batch-set status, labels, keywords, section type on multi-select
- [ ] Project keywords: centralized vocabulary; assign from list (not freeform-only)
- [ ] Custom document/folder icons in the binder
- [ ] Freeform corkboard as a first-class view (commit/revert spatial layout)

### Snapshots, goals, focus, templates, links

- [ ] Snapshot compare: side-by-side / visual diff of current text vs any LibGit2 snapshot before restore
- [ ] Writing goals: per-document, session, and project targets with progress UI
- [ ] Composition mode: full-screen distraction-free writing with floating mini-inspector
- [ ] Project templates: novel, nonfiction, academic starters
- [ ] Document templates: reusable character/location (and custom) sheet starters
- [ ] Bookmarks: internal cross-document links + inspector bookmarks pane

### Tool readiness

- [ ] Expose `create_collection`, `query_smart_collection`, `open_scrivenings`, `bulk_set_metadata` (see [TOOLS.md](../TOOLS.md#v2-tools))

## Acceptance criteria

- Author creates a collection “Needs revision” from status filter; adding a scene’s status updates the smart collection without moving the file
- Selecting a chapter folder opens Scrivenings; editing paragraph in scene 2 updates only that scene’s Markdown on disk
- Multi-select of 20 scenes can set status + section type in one action
- Snapshot compare shows what changed before restore
- Composition mode hides chrome; floating inspector still shows synopsis/notes subset
- New project from template includes starter binder structure and sheet templates

## Out of scope

- Compile/export presets (Phase 7)
- AI plot diagrams, timelines, character graphs (Phase 8)
- Mind maps, clustering, pacing analytics (Phase 9)
- Editing Phase 2 checklist itself

## Agent tool readiness

Prefer small named commands so Hermes App Controller can create collections, open Scrivenings, and bulk-tag without menu clicks. Full schemas: [TOOLS.md](../TOOLS.md#v2-tools).
