# Phase 7: Compile and Export (v2)

Purpose: ship a Scrivener-class compile system so authors produce agent manuscript, EPUB, PDF, and other formats without rewriting source formatting. Status: **not started**. Months 13–14. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phase 6 (section types, collections, filters). Does not replace phases 01–05.

## Goals

- Assemble binder, collection, or filtered set into a manuscript
- Format-specific presets: DOCX, PDF, EPUB, Markdown, Fountain
- Per–section-type formatting rules (chapter break, scene separator, front matter)
- Include/exclude by label, status, section type, or collection
- Agent-ready `compile_manuscript` with confirm before overwrite

## Checklist

- [ ] Compile UI: choose content root (Draft folder, collection, or smart filter)
- [ ] Section-type layout rules: title page, Part Heading, Chapter Heading, Scene, Front/Back Matter
- [ ] Presets: Standard Manuscript (agent), Paperback, Ebook, Academic, Screenplay/Fountain
- [ ] Export targets: DOCX, PDF, EPUB, Markdown, Fountain
- [ ] Filters: include/exclude by label, status, keyword, section type
- [ ] Front matter / back matter slots (title, copyright, dedication, about the author)
- [ ] As-is vs transform modes (preserve editor styles vs apply compile stylesheet)
- [ ] Scene separators and chapter numbering options
- [ ] Progress + cancel for long compiles
- [ ] Output path via dialog; never silent overwrite without confirm
- [ ] Tool: `compile_manuscript(preset, filters) -> path` (see [TOOLS.md](../TOOLS.md#v2-tools))

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

## Notes

Section types from Phase 6 are required so compile does not treat every document identically. Collections supply alternate compile orders without restructuring the binder.
