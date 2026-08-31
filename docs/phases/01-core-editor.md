# Phase 1: Core editor

Purpose: bootstrap the Tauri writing shell so authors can open a project, edit scenes in TipTap, and save to local SQLite + Markdown. Status: **not started**. Months 1–2. Parent: [ROADMAP.md](../ROADMAP.md).

## Goals

- Tauri + React + TypeScript + Tailwind app boots on desktop
- Three-pane chrome: binder stub | TipTap editor | inspector stub
- Project folder on disk with metadata in SQLite and scene bodies as Markdown
- Save/load loop; crash resilience (“if the app dies, text remains”)

## Library choices (npm + Tauri)

Scaffold with **Vite + React + TypeScript**, not Next.js. Next.js SSR/routing adds no value inside a Tauri webview; use `npm create tauri-app@latest` → React TS. Full stack context: [TECH-STACK.md](../TECH-STACK.md).

### Locked for Phase 1

| Need | Package | Why |
| :--- | :--- | :--- |
| Scaffold | `create-tauri-app` (`react-ts`), `@tauri-apps/api` | Official Tauri 2 path |
| Layout | `react-resizable-panels` | Binder \| editor \| inspector; collapsible panes |
| Styling | `tailwindcss`, `@radix-ui/react-tabs`, `@radix-ui/react-scroll-area` | Dense chrome + inspector tabs |
| Editor | `@tiptap/react`, `@tiptap/pm`, `@tiptap/starter-kit` | Headless ProseMirror; MIT core |
| Markdown I/O | `@tiptap/markdown` | Serialize scenes to `.md` on disk |
| Word count | `@tiptap/extension-character-count` | Session/project stats later |
| Menus / floating UI | `@floating-ui/dom` | TipTap v3 bubble/floating menus |
| UI state | `zustand` | Open doc, dirty flag, pane visibility |
| Files | `@tauri-apps/plugin-fs`, `@tauri-apps/plugin-dialog` | Open project folder; read/write Markdown |
| Metadata DB | `@tauri-apps/plugin-sql` (Cargo `sqlite`) | Binder rows; **not** full manuscript text |

Suggested installs (when implementation starts):

```bash
npm i react-resizable-panels zustand \
  @radix-ui/react-tabs @radix-ui/react-scroll-area \
  @tiptap/react @tiptap/pm @tiptap/starter-kit @tiptap/markdown \
  @tiptap/extension-character-count @floating-ui/dom \
  @tauri-apps/plugin-fs @tauri-apps/plugin-dialog @tauri-apps/plugin-sql
```

Rust crates (pair with the JS plugins): `tauri-plugin-fs`, `tauri-plugin-dialog`, `tauri-plugin-sql` with `features = ["sqlite"]`.

### Binder tree (stub now, DnD in Phase 2)

| Package | Fit |
| :--- | :--- |
| Phase 1 | Flat list or minimal tree is enough |
| `react-arborist` | Prefer for Phase 2: virtualized, Finder/VS Code–like binder |
| `react-complex-tree` | Strong a11y + multi-select DnD, zero deps |
| `@headless-tree/core` | Headless alternative if we own all styling |

### Editor alternatives considered (not chosen)

| Library | Notes |
| :--- | :--- |
| **TipTap** | Chosen — matches product docs |
| Milkdown | Markdown-first ProseMirror; viable if MD-only UX were primary |
| BlockNote / Novel | Notion-style blocks; heavier product opinion |
| Lexical / Plate | Capable; more DIY or Slate/shadcn-specific than needed |

### Explicit non-choices

- **Next.js** as the frontend host
- Electron-only `fs` in the renderer (use Tauri plugins)
- Paid TipTap Pro extensions for Phase 1
- Storing manuscript body only in SQLite (breaks “text remains as Markdown”)

## Checklist

- [ ] Create Tauri 2 + React + Vite project structure (docs-only until implementation)
- [ ] Wire Tailwind and a basic Radix layout shell
- [ ] Implement resizable three-pane layout with `react-resizable-panels`
- [ ] Left: flat or stub tree of open documents (full DnD in Phase 2)
- [ ] Center: TipTap + `@tiptap/markdown` serialization
- [ ] Right: empty inspector panel with Radix tabs placeholder
- [ ] Define project folder convention (e.g. `Project.aegis/` with `db/`, `manuscript/`, `research/`)
- [ ] SQLite schema for documents: id, title, parent_id nullable, path, updated_at (`plugin-sql`)
- [ ] Write/read Markdown files for scene content (`plugin-fs`)
- [ ] Open project / New project / Save document via dialog + Rust/commands
- [ ] Autosave or explicit save with dirty-state indicator (`zustand`)
- [ ] Verify: kill app mid-edit → Markdown on disk still readable in a text editor

## File conventions (target)

```
MyBook.aegis/
  project.sqlite
  manuscript/
    scenes/
      scene-001.md
  research/
  assets/
```

## Acceptance criteria

- Author can create a project, add at least one scene, type in TipTap, save, quit, reopen, and see the same text
- Scene text is present as a plain Markdown file outside the DB
- UI shows binder | editor | inspector panes (stubs allowed for binder/inspector)

## Out of scope

- Corkboard, outliner, character sheets, Git snapshots (Phase 2)
- Ollama / RAG (Phase 3)
- Integrity (Phase 4)
- Hermes agents (Phase 5)

## Notes

Keep mutation APIs (create doc, update body) clean—they become tools later ([TOOLS.md](../TOOLS.md)).
