# Agents

Purpose: define the four hermes-agent–powered co-author roles that mutate the binder, editor, and integrity reports—not chat-only assistants.

Shared runtime, voice/style learning, skill growth, and guardrails are at the end. Tool schemas: [TOOLS.md](TOOLS.md). Runtime: [HERMES-RUNTIME.md](HERMES-RUNTIME.md). Architecture: [ARCHITECTURE.md](ARCHITECTURE.md). Canon, slop, and revision briefs: [WRITING-LOOP.md](WRITING-LOOP.md).

---

## 1. App Controller

**Role:** Natural-language command center for Scrivener’s steep UI. Authors control the project without hunting through menus.

**hermes-agent:** Profile / skill pack focused on binder mutations.

**User trigger:** Command bar text, e.g. *“Create a new folder for Act 2, add three empty chapters inside it, and create a character sheet for the antagonist.”*

**Plan steps (example):**

1. Break the prompt into distinct actions
2. Call `create_folder(name="Act 2")`
3. Call `create_chapter(parent="Act 2", count=3)`
4. Call `create_character_sheet(name="Antagonist", parent="Characters")`
5. Emit UI refresh events

**Tools used:** `create_folder`, `create_chapter`, `create_character_sheet` (and related binder mutations such as `save_to_binder` when needed)

**Structured output:** JSON action log (tool name, args, success/failure) for the inspector agent log

**UI effect:** Binder tree and inspector update immediately; no chat-only dump of “here’s what you should click”

---

## 2. Deep Continuity

**Role:** Reason over RAG chunks **and the project canon** to catch continuity errors a single retrieve-and-stuff LLM call would miss—the ultimate Scrivener-style “knows the whole book” feature.

**hermes-agent:** Profile with retrieval + canon + highlight skills; uses project memory of cast and timeline facts.

**User trigger:** Highlight a scene and ask, e.g. *“Check if the timeline and character locations in this scene make sense based on the rest of the book.”*

**Plan steps (example):**

1. `extract_entities(text=highlighted_scene)` → characters, times, locations
2. `check_against_canon(text=highlighted_scene)` — falsifiable facts (ages, handedness, places, dates)
3. `search_binder(query="Character X location")` (and related queries) against local Qdrant
4. `check_foreshadowing_ledger` for plants without payoffs
5. Reason over retrieved chunks (e.g. Chapter 4: injured left leg vs current scene: “sprinted away on his injured leg”)
6. If the scene establishes a new hard fact, `append_canon_entry`; if it reveals a lore gap, `log_propagation_debt` instead of inventing a fix
7. Emit a structured plot-hole report

**Tools used:** `extract_entities`, `check_against_canon`, `search_binder`, `check_foreshadowing_ledger`, `append_canon_entry`, `log_propagation_debt`, `highlight_text_in_editor`

**Structured output:** JSON report of issues: claim, conflicting evidence (binder path + excerpt **or canon statement**), severity, suggested fix

**UI effect:** Turnitin-style inline comments on the highlighted scene; optional highlights in the editor

---

## 3. Integrity and Plagiarism

**Role:** Orchestrate the Turnitin-class pipeline as a reasoned, multi-step investigation—not a single blind score.

**hermes-agent:** Profile with integrity skills; `search_web` always gated by confirm.

**User trigger:** *“Run Integrity Check”* (chapter or selection)

**Plan steps (example):**

1. `run_slop_scan(text)` — mechanical guardrails (no LLM): banned words, fiction tells, telling, rhythm
2. `check_internal_similarity(chapter_text)` — flag self-plagiarism / repeated drafts
3. If suspicious, `search_web(queries=[key_phrases])` via SearXNG + Trafilatura (**opt-in / confirm**)
4. `run_ai_detection(text)` via Binoculars / DeBERTa locally
5. Optionally `propose_cuts` — classified fat with a KEEP (`tightest_passage`); do not treat a 1–10 “quality” score as the plan
6. Synthesize a human-readable report plus chart-ready JSON

**Tools used:** `run_slop_scan`, `check_internal_similarity`, `search_web`, `run_ai_detection`, `propose_cuts`, `highlight_text_in_editor`

**Structured output:** Integrity packet JSON—slop penalty + spans, similarity scores, match sources, AI-detection spans, optional cut list, prose explanation (*“Paragraph 3 has a 92% match with … and unusually low perplexity.”*)

**UI effect:** Report panel with charts; inline flags; exportable integrity packet

---

## 4. World-Builder and Research

**Role:** Keep lore out of the chat box and **in the Scrivener-style project**—then draft from that context, matching the author’s voice when a style profile exists.

**hermes-agent:** Profile with lore + draft skills; injects style profile **and a revision brief** into `draft_scene` / `rewrite_range`.

**User trigger:** e.g. *“I need to write a scene where they travel through a desert. Generate the lore for this desert, add it to the world-building binder, and draft the scene.”*

**Plan steps (example):**

1. `generate_lore(prompt="desert environment")`
2. `save_to_binder(category="Worldbuilding", content=generated_lore)`
3. New hard facts → `append_canon_entry`; gaps → `log_propagation_debt`
4. Snapshot, then `draft_scene` with the [context pack](WRITING-LOOP.md#drafting-context-pack) and a [brief](WRITING-LOOP.md#revision-brief) (`keep` / `change` / `voice_rules`). Prefer **author voice profile** (Part 2) plus guardrails (Part 1).
5. Place prose in the editor (and optionally create a new scene file). Inspector offers Keep / Discard against the snapshot.

**Tools used:** `generate_lore`, `save_to_binder`, `draft_scene`, `rewrite_range`, `append_canon_entry`, optionally `create_folder` / `create_chapter`

**Structured output:** Lore document metadata + scene draft status + brief id

**UI effect:** New binder items under Worldbuilding; draft text in the center editor—not only a chat reply

---

## Shared runtime

| Item | Choice |
| :--- | :--- |
| Agent runtime | **hermes-agent** on `127.0.0.1:18765` |
| Model | **Hermes 3** (8B default; 70B optional) via **Ollama** |
| Tool bridge | Skills → FastAPI `:8765` + Tauri IPC ([TOOLS.md](TOOLS.md)) |
| Profiles | One skill pack per agent above |

### Why Hermes 3 + hermes-agent

- **Strict JSON** for Turnitin-style reports and chart UIs
- **Function calling / skills** so binder capabilities become real actions
- **Multi-step planning** without losing the thread
- **Memory + skills** so the co-author deepens with use
- **Local and private**—thought process and tool calls never leave the machine

### Voice and style

- **Part 1 — Guardrails:** built-in anti-slop (mechanical rubric). Same for every book.
- **Part 2 — Identity:** learn from **accepted** manuscript prose (local samples only); store tone, rhythm, dialogue habits, exemplars and anti-exemplars
- Surface **style match** in the inspector on generations
- “Rewrite in my voice” uses Part 2 and still runs Part 1—no cloud fine-tune required
- Rewrites that replace more than a sentence require a brief with a KEEP list so good sentences survive

### Skill growth (self-evolution)

- After successful multi-step runs, propose project-local skills (e.g. “Act 2 setup”)
- User confirms before install; inspector shows skill library growth
- Skills call the same tool gateway—richer workflows, same privacy boundary

### Guardrails

- Confirm before **delete**, **compile**, or **web search**
- Optional **snapshot** (LibGit2) before bulk binder writes; inspector Keep / Discard — never unattended reset
- hermes-agent / skill policy caps runaway tool loops
- Human-in-the-loop interrupts on high-impact tools
- No unconstrained shell over the whole filesystem

Phase delivery: [phases/05-hermes-agents.md](phases/05-hermes-agents.md). Full ports and lifecycle: [HERMES-RUNTIME.md](HERMES-RUNTIME.md).
