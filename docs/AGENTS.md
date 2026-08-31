# Agents

Purpose: define the four Hermes 3–powered agents that turn AegisDraft from a smart editor into an AI co-author and editor. Each agent plans multi-step work and calls tools; it does not only chat.

Shared runtime, why Hermes, and guardrails are at the end. Tool schemas: [TOOLS.md](TOOLS.md). Architecture: [ARCHITECTURE.md](ARCHITECTURE.md).

---

## 1. App Controller

**Role:** Natural-language command center for Scrivener’s steep UI. Authors control the project without hunting through menus.

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

**Role:** Reason over RAG chunks to catch continuity errors a single retrieve-and-stuff LLM call would miss—the ultimate Scrivener-style “knows the whole book” feature.

**User trigger:** Highlight a scene and ask, e.g. *“Check if the timeline and character locations in this scene make sense based on the rest of the book.”*

**Plan steps (example):**

1. `extract_entities(text=highlighted_scene)` → characters, times, locations
2. `search_binder(query="Character X location")` (and related queries) against local Qdrant
3. Reason over retrieved chunks (e.g. Chapter 4: injured left leg vs current scene: “sprinted away on his injured leg”)
4. Emit a structured plot-hole report

**Tools used:** `extract_entities`, `search_binder`, `highlight_text_in_editor` (to mark offending spans)

**Structured output:** JSON report of issues: claim, conflicting evidence (binder path + excerpt), severity, suggested fix

**UI effect:** Turnitin-style inline comments on the highlighted scene; optional highlights in the editor

---

## 3. Integrity and Plagiarism

**Role:** Orchestrate the Turnitin-class pipeline as a reasoned, multi-step investigation—not a single blind score.

**User trigger:** *“Run Integrity Check”* (chapter or selection)

**Plan steps (example):**

1. `check_internal_similarity(chapter_text)` — flag self-plagiarism / repeated drafts
2. If suspicious, `search_web(queries=[key_phrases])` via SearXNG + Trafilatura (**opt-in / confirm**)
3. `run_ai_detection(text)` via Binoculars / DeBERTa locally
4. Synthesize a human-readable report plus chart-ready JSON

**Tools used:** `check_internal_similarity`, `search_web`, `run_ai_detection`, `highlight_text_in_editor`

**Structured output:** Integrity packet JSON—scores, match sources, AI-detection spans, prose explanation (*“Paragraph 3 has a 92% match with … and unusually low perplexity.”*)

**UI effect:** Report panel with charts; inline flags; exportable integrity packet

---

## 4. World-Builder and Research

**Role:** Keep lore out of the chat box and **in the Scrivener-style project**—then draft from that context.

**User trigger:** e.g. *“I need to write a scene where they travel through a desert. Generate the lore for this desert, add it to the world-building binder, and draft the scene.”*

**Plan steps (example):**

1. `generate_lore(prompt="desert environment")`
2. `save_to_binder(category="Worldbuilding", content=generated_lore)`
3. `draft_scene(context=generated_lore, style="gritty")`
4. Place prose in the editor (and optionally create a new scene file)

**Tools used:** `generate_lore`, `save_to_binder`, `draft_scene`, optionally `create_folder` / `create_chapter`

**Structured output:** Lore document metadata + scene draft status

**UI effect:** New binder items under Worldbuilding; draft text in the center editor—not only a chat reply

---

## Shared runtime

| Item | Choice |
| :--- | :--- |
| Model | **Hermes 3** (8B default; 70B optional) via **Ollama** |
| Framework | **LangGraph** state machine per agent |
| Bridge | FastAPI sidecar + Tauri IPC ([TOOLS.md](TOOLS.md)) |

### Why Hermes 3

- **Strict JSON** for Turnitin-style reports and chart UIs—rarely breaks formatting vs base models
- **Function calling** so Tauri/Rust capabilities become tools
- **Multi-step planning** (4–5 hops) without losing the thread—book work is a series of actions, not one prompt
- **Local and private**—thought process and tool calls never leave the machine

### Guardrails

- Confirm before **delete**, **compile**, or **web search**
- Optional **snapshot** (LibGit2) before bulk binder writes
- LangGraph **max tool hops** to prevent infinite loops
- Human-in-the-loop interrupts on high-impact tools

Phase delivery: [phases/05-hermes-agents.md](phases/05-hermes-agents.md).
