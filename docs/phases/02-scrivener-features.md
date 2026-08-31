# Phase 2: Scrivener features

Purpose: deliver binder, corkboard, outliner, inspector metadata, and LibGit2 snapshots so AegisDraft feels like a Scrivener-class studio. Status: **not started**. Months 3–4. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 1.

## Goals

- Hierarchical binder with drag-and-drop
- Corkboard and outliner bound to the same binder items
- Inspector for synopsis, notes, keywords, character/location sheets
- “Save Snapshot” via LibGit2
- Every binder mutation designed as a future agent tool ([TOOLS.md](../TOOLS.md))

## Checklist

- [ ] Binder tree with `react-arborist` or `react-complex-tree`
- [ ] Drag-and-drop reorder, nest, and move between Draft / Research / Trash
- [ ] Labels, status stamps, custom metadata columns on documents
- [ ] Corkboard view with `dnd-kit` index cards (synopsis + status color)
- [ ] Freeform corkboard optional stretch goal
- [ ] Outliner via TanStack Table (word count, target, status, POV, custom fields)
- [ ] Corkboard ↔ outliner ↔ editor stay in sync on binder IDs
- [ ] Inspector: synopsis, document notes, keywords, snapshots list
- [ ] Character and location sheet types linked to scenes
- [ ] Split view: draft + research reference
- [ ] LibGit2 init inside project folder; snapshot = commit
- [ ] Restore snapshot UI (list commits, checkout / restore paths safely)
- [ ] Document which UI actions map to future tools: `create_folder`, `create_chapter`, `create_character_sheet`, `save_to_binder`

## Acceptance criteria

- Author can restructure a multi-chapter binder by drag-and-drop without losing files
- Switching Editor / Corkboard / Outliner shows the same set of binder items
- Creating a character sheet stores metadata in SQLite and a sheet file on disk
- Save Snapshot creates a recoverable LibGit2 commit

## Out of scope

- Ollama generation and RAG (Phase 3)
- Similarity / AI detection (Phase 4)
- Command bar Hermes App Controller (Phase 5)—but APIs should be tool-ready

## Tool readiness note

Prefer small, named Rust/Python commands over opaque UI-only mutations so Phase 5 can wrap them without a rewrite.
