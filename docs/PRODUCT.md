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
- Rubric / feedback scores (pacing, show-don’t-tell, dialogue tags, **mechanical slop**, custom criteria)
- Exportable **integrity packet**: similarity + AI detection + slop spans + citation/rubric notes

Scores alone are not enough. The Integrity agent synthesizes *why* a section was flagged (e.g. match to a web source plus low perplexity) so the UI can show charts and inline comments. See [AGENTS.md](AGENTS.md).

## AI writing UX

- Book-wide RAG **plus a canon of hard facts**: chapter 20 can remember character rules from chapter 1 without relying on similarity alone
- Scene continuation (`Ctrl+Space`), rewrite, tone/style shifter — rewrites that replace more than a sentence use a **KEEP / CHANGE brief** so good sentences survive
- Devil’s advocate critique before “submit chapter” (Phase 3); later a multi-persona panel as Integrity/Continuity signal, not an autonomous rewrite loop
- Integrity-aware generation: flag or rewrite passages that would fail a check, including **mechanical slop** (banned LLM diction, fiction tells)
- **Rewrite in my voice** using a local style profile (Part 2) learned from accepted prose; guardrails (Part 1) always apply

Single-shot tools land in Phase 3; multi-step **hermes-agent** orchestration lands in Phase 5 ([ROADMAP.md](ROADMAP.md), [HERMES-RUNTIME.md](HERMES-RUNTIME.md)).

## Agent UX

- **Command bar** for App Controller intents (hermes-agent profile)
- **Inspector agent log** showing profile, plan steps, skills used, and style match (local traces only)
- Agents **write into** the binder and editor; chat is a transcript of work done, not the only deliverable
- **Voice & skills:** style profile from accepted prose (Part 2) plus built-in guardrails (Part 1); proposed skills after repeated workflows (user confirms)
- Guardrails: confirm before delete, compile, or web search; optional snapshot before bulk writes with inspector **Keep / Discard**

## Privacy as a product requirement

| Rule | Requirement |
| :--- | :--- |
| Default offline | Drafting, RAG, hermes-agent, snapshots work with no network |
| No cloud manuscript | Local by default. Text leaves only two ways, both off by default and both naming what goes: a remote model turned on for that session, or a phrase the author ticks for a web check. See the golden rule in [ARCHITECTURE.md](ARCHITECTURE.md#golden-rule). |
| Local models + runtime | Any tool-calling model Ollama serves; the agent loop runs in-process against Ollama's own endpoint, with no second daemon to install |
| Local traces & style | Agent plans, memory, style profile, and skills stay on disk with the project |
| Opt-in web | SearXNG + Trafilatura only for highlighted / selected phrases, never a full-book upload |

Architecture details: [ARCHITECTURE.md](ARCHITECTURE.md). Writing loop: [WRITING-LOOP.md](WRITING-LOOP.md).
