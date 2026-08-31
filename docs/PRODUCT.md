# Product

Purpose: define what AegisDraft is for authors—positioning, personas, UX pillars, and privacy as a hard requirement—not how the code is structured.

## Positioning

AegisDraft is a **local-first AI co-author and editor** for long-form work. It combines:

- A Scrivener-inspired writing studio (binder, corkboard, outliner, inspector, compile)
- Turnitin-class integrity (similarity, AI-writing detection, rubrics, exportable reports)
- **hermes-agent** (Hermes 3 via Ollama) that mutates the project via skills—not a passive autocomplete box

Authors keep unpublished manuscripts on their own machines. The co-author plans multi-step work, writes into the binder and editor, explains integrity findings, **learns the author’s voice**, and **grows project skills** the more it is used.

## Non-goals

- Cloud SaaS editor that uploads manuscripts by default
- Closed Turnitin-style proprietary corpus or institutional database
- Chat-only AI that suggests text but cannot create folders, sheets, or highlights
- Replacing human judgment on publish/submit decisions

## Personas

| Persona | Needs |
| :--- | :--- |
| **Novelist** | Acts, scenes, character sheets, continuity across 80k+ words |
| **Nonfiction author** | Research binder, citations, tone control, chapter targets |
| **Academic** | Integrity checks, rubrics, citation coverage before submission |
| **Teacher / editor** | Inline feedback, GradeMark-style comments, exportable integrity packet |

## Writing studio UX (Scrivener-inspired)

- **Three panes:** Binder (left) | Editor (center) | Inspector (right)
- **Alternative views:** Corkboard (index cards) and Outliner (sortable grid) bound to the same binder items
- **Compose mode:** Distraction-free editor with typewriter scrolling
- **Compile:** Manuscript export (DOCX, PDF, EPUB, Markdown, Fountain) with layouts and filters
- **Command bar:** Natural-language app control (“Create Act 2 with three chapters…”) routed to the App Controller agent—so authors are not trapped in Scrivener’s steep menu tree

## Integrity UX (Turnitin-inspired)

Integrity is part of the writing loop, not a last-minute upload.

- Similarity report with color-coded matches and source context
- AI-writing detection at paragraph and document level
- Rubric / feedback scores (pacing, show-don’t-tell, dialogue tags, custom criteria)
- Exportable **integrity packet**: similarity + AI detection + citation/rubric notes

Scores alone are not enough. The Integrity agent synthesizes *why* a section was flagged (e.g. match to a web source plus low perplexity) so the UI can show charts and inline comments. See [AGENTS.md](AGENTS.md).

## AI writing UX

- Book-wide RAG: chapter 20 can “remember” character rules from chapter 1
- Scene continuation (`Ctrl+Space`), rewrite, tone/style shifter
- Devil’s advocate critique before “submit chapter”
- Integrity-aware generation: flag or rewrite passages that would fail a check
- **Rewrite in my voice** using a local style profile learned from accepted prose

Single-shot tools land in Phase 3; multi-step **hermes-agent** orchestration lands in Phase 5 ([ROADMAP.md](ROADMAP.md), [HERMES-RUNTIME.md](HERMES-RUNTIME.md)).

## Agent UX

- **Command bar** for App Controller intents (hermes-agent profile)
- **Inspector agent log** showing profile, plan steps, skills used, and style match (local traces only)
- Agents **write into** the binder and editor; chat is a transcript of work done, not the only deliverable
- **Voice & skills:** style profile from accepted prose; proposed skills after repeated workflows (user confirms)
- Guardrails: confirm before delete, compile, or web search; optional snapshot before bulk writes

## Privacy as a product requirement

| Rule | Requirement |
| :--- | :--- |
| Default offline | Drafting, RAG, hermes-agent, snapshots work with no network |
| No cloud manuscript | Text does not leave the machine unless the user explicitly opts into web similarity |
| Local models + runtime | Hermes 3 / draft models via Ollama; hermes-agent on loopback; no cloud portal by default |
| Local traces & style | Agent plans, memory, style profile, and skills stay on disk with the project |
| Opt-in web | SearXNG + Trafilatura only for highlighted / selected phrases, never a full-book upload |

Architecture details: [ARCHITECTURE.md](ARCHITECTURE.md).
