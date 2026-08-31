# Phase 5: Hermes agents

Purpose: elevate AegisDraft from smart editor to AI co-author by wiring **hermes-agent** + Hermes 3 to real binder tools over a FastAPI tool gateway and Tauri IPC—including voice/style learning and evolving skills. Status: **not started**. Months 9–10. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phases 2–4.

Agents: [AGENTS.md](../AGENTS.md). Tools: [TOOLS.md](../TOOLS.md). Runtime: [HERMES-RUNTIME.md](../HERMES-RUNTIME.md).

## Goals

- hermes-agent on `127.0.0.1:18765` + tool gateway on `127.0.0.1:8765`
- Four hermes profiles / skill packs: App Controller, Deep Continuity, Integrity, World-Builder
- TOOLS.md schemas implemented and registered as hermes skills
- Command bar → App Controller; continuity / integrity / world-builder flows as today
- **Style learner** + **skill evolution** so the co-author gets richer with use

## Checklist

### Lifecycle and bridge

- [ ] Start/stop hermes-agent with the app; health `GET :18765/health`
- [ ] Start FastAPI tool gateway on `:8765` (loopback only)
- [ ] Tauri IPC / HTTP client for tool execution and UI events
- [ ] Events: `binder_changed`, `highlight_range`, `report_attached`, `agent_log`
- [ ] Optional LibGit2 snapshot before bulk agent writes
- [ ] Local-only hermes profile (Ollama); no cloud portal by default

### hermes-agent + Hermes 3

- [ ] Ollama Hermes 3 (8B default; document 70B path)
- [ ] Four profiles or skill packs mapping to product agents
- [ ] Skill policy: confirm for delete, compile, `search_web`
- [ ] Persist agent traces + hermes memory under project/app data (local only)

### Wire tools as skills

- [ ] Binder: `create_folder`, `create_chapter`, `create_character_sheet`, `save_to_binder`, `search_binder`
- [ ] Editor: `highlight_text_in_editor`, `draft_scene`, `replace_range`
- [ ] Continuity: `extract_entities`
- [ ] Integrity: `check_internal_similarity`, `search_web`, `run_ai_detection`
- [ ] Lore: `generate_lore`
- [ ] Each skill HTTP-calls the tool gateway—no unconstrained shell

### Voice, memory, skill growth

- [ ] Style learner: analyze accepted prose → project `style-profile` (tone, rhythm, dialogue habits)
- [ ] Inject style profile into draft/rewrite / World-Builder context
- [ ] Inspector: style-match indicator on generations
- [ ] After successful multi-step runs, propose project-local skills (user confirms)
- [ ] Inspector: skill library count and last skills used

### UX

- [ ] Command bar (cmdk / Radix) routes to App Controller profile
- [ ] Inspector agent log shows hermes profile, plan steps, and skill/tool results
- [ ] Continuity issues render as Turnitin-style inline comments
- [ ] Integrity report panel charts JSON scores + explanations
- [ ] World-Builder creates binder items and fills the editor—not chat-only
- [ ] Offline hermes: command bar shows offline; Phase 3 single-shot draft may still work

## Acceptance criteria

- **Primary:** A single natural-language command creates **Act 2**, **three empty chapters**, and an **antagonist character sheet**; the binder and inspector update **without a menu click**
- Deep Continuity on a planted contradiction produces a structured JSON issue and an inline comment
- Integrity Check produces a synthesized report (why flagged), not only raw percentages
- World-builder flow adds a Worldbuilding binder document and a scene draft on disk
- **“Rewrite in my voice”** uses the local style profile (no cloud)
- After repeating a workflow, the app can **propose a new project skill** and install it only on confirm
- With network disabled, all agents except confirmed `search_web` still function

## Out of scope

- Cloud agent hosts, Nous Portal default, or remote telemetry of traces
- Replacing Phases 1–4 features; this phase only orchestrates them
- LangGraph / AutoGen (explicitly not used)

## Success definition

Scrivener features become database entries the AI can read/write; Turnitin features become multi-step investigations; hermes-agent is integrated into the file system and UI—capable of *doing* things, learning the author’s voice, and growing skills over time—not only *suggesting* text.
