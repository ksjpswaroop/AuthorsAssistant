# Phase 7: Compile and Export (v2)

Purpose: ship a Scrivener-class compile system so authors produce agent manuscript, EPUB, PDF, and other formats without rewriting source formatting. Status: **complete** for Markdown, HTML, DOCX, EPUB and Fountain; PDF exports print-ready HTML rather than bundling a typesetter. Months 13–14. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 6 (section types, collections, filters). Does not replace phases 01–05.

## Goals

- Assemble binder, collection, or filtered set into a manuscript
- Format-specific presets: DOCX, PDF, EPUB, Markdown, Fountain
- Per–section-type formatting rules (chapter break, scene separator, front matter)
- Include/exclude by label, status, section type, or collection
- Agent-ready `compile_manuscript` with confirm before overwrite

## Checklist

- [x] Compile UI: choose content root (Draft folder, collection, or smart filter)
- [x] Section-type layout rules: title page, Part Heading, Chapter Heading, Scene, Front/Back Matter
- [x] Presets: Standard Manuscript, Paperback, Ebook, Academic, Screenplay/Fountain
- [x] Export targets: DOCX, EPUB, Markdown, HTML, Fountain — **PDF exports print-ready HTML** rather than bundling a typesetter, and the UI says so
- [x] Filters: include/exclude by label, status, section type
- [x] Front matter / back matter slots
- [x] Scene separators and chapter numbering options
- [x] Output path via dialog; never a silent overwrite
- [x] Tool: `compile_manuscript(path, preset, format)` in the agent gateway, guarded — it overwrites a file, so it needs a person's yes, and its own schema says so

## Acceptance criteria

- Author compiles Draft to “Standard Manuscript” DOCX with chapter headings formatted and scenes joined with separators—source Markdown unchanged
- Same project compiles to EPUB with different heading styles via a second preset
- Excluding status “Notes” / section type “Research” omits those docs from output
- Compiling a smart collection exports only that virtual set’s reading order
- `compile_manuscript` is callable from the sidecar with human-in-the-loop confirm

## Out of scope

- AI rewriting or integrity checks during compile (Phases 3–5 tools)
- Proprietary Final Draft binary fidelity beyond Fountain export
- Print-shop imposition / advanced typesetting engines

## Agent access

`compile_manuscript` is one of the four tools in [`agent::GUARDED`](../../src-tauri/src/agent.rs). An agent can ask to compile; `dispatch` refuses without `confirmed: true`, and the [Phase 5](05-hermes-agents.md) loop pauses the run so a person answers rather than the model asserting the flag itself. The e2e test compiles once unconfirmed (nothing is written) and once confirmed (a real file, with the project's own prose in it, sources unmodified).

## Notes

Section types from Phase 6 are required so compile does not treat every document identically. Collections supply alternate compile orders without restructuring the binder.
