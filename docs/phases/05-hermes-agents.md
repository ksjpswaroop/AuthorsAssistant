# Phase 5: Hermes agents

Purpose: elevate AegisDraft from smart editor to AI co-author by wiring Hermes 3 + LangGraph agents to real tools over a FastAPI sidecar and Tauri IPC. Status: **not started**. Months 9–10. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phases 2–4.

Agents: [AGENTS.md](../AGENTS.md). Tools: [TOOLS.md](../TOOLS.md).

## Goals

- Local FastAPI sidecar + Tauri IPC event bus
- LangGraph state machines per agent (max steps; interrupt on web/delete)
- Implement TOOLS.md schemas against Phase 2–4 capabilities
- Command bar → App Controller
- Scene highlight → Deep Continuity → inline comment JSON
- Run Integrity Check → Integrity agent → report UI
- World-builder prompt → lore in binder + draft in editor

## Checklist

### Bridge

- [ ] FastAPI sidecar starts with the app (loopback only)
- [ ] Tauri IPC / HTTP client for tool execution and events
- [ ] Events: `binder_changed`, `highlight_range`, `report_attached`, `agent_log`
- [ ] Optional LibGit2 snapshot before bulk agent writes

### LangGraph + Hermes 3

- [ ] Ollama Hermes 3 (8B default; document 70B path)
- [ ] One graph per agent: App Controller, Deep Continuity, Integrity, World-Builder
- [ ] Max tool hops; human-in-the-loop for delete, compile, `search_web`
- [ ] Persist agent traces under the project directory (local only)

### Wire tools

- [ ] Binder: `create_folder`, `create_chapter`, `create_character_sheet`, `save_to_binder`, `search_binder`
- [ ] Editor: `highlight_text_in_editor`, `draft_scene`, `replace_range`
- [ ] Continuity: `extract_entities`
- [ ] Integrity: `check_internal_similarity`, `search_web`, `run_ai_detection`
- [ ] Lore: `generate_lore`

### UX

- [ ] Command bar (cmdk / Radix) routes to App Controller
- [ ] Inspector agent log shows plan steps and tool results
- [ ] Continuity issues render as Turnitin-style inline comments
- [ ] Integrity report panel charts JSON scores + explanations
- [ ] World-Builder creates binder items and fills the editor—not chat-only

## Acceptance criteria

- **Primary:** A single natural-language command creates **Act 2**, **three empty chapters**, and an **antagonist character sheet**; the binder and inspector update **without a menu click**
- Deep Continuity on a planted contradiction produces a structured JSON issue and an inline comment
- Integrity Check produces a synthesized report (why flagged), not only raw percentages
- World-builder flow adds a Worldbuilding binder document and a scene draft on disk
- With network disabled, all agents except confirmed `search_web` still function

## Out of scope

- Cloud agent hosts or remote telemetry of traces
- Replacing Phases 1–4 features; this phase only orchestrates them

## Success definition

Scrivener features become database entries the AI can read/write; Turnitin features become multi-step investigations; the AI is integrated into the file system and UI—capable of *doing* things, not only *suggesting* them.
