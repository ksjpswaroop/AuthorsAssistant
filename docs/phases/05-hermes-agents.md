# Phase 5: Hermes agents

Purpose: elevate AegisDraft from smart editor to AI co-author by wiring a real tool-calling agent to real binder tools. Status: **complete for the loop**, with the hermes-agent dependency dropped. Months 9–10. Parent: [ROADMAP.md](../ROADMAP.md). Depends on Phases 2–4.

## hermes-agent: no longer needed

This phase was blocked for months on a runtime that is not installed. It turned out not to be necessary.

**Ollama's OpenAI-compatible endpoint supports native tool calling.** `POST /v1/chat/completions` with a `tools` array returns `tool_calls`, verified live here against both `llama3.2:3b` (local) and `qwen3.5:cloud` (remote). The fourteen-tool gateway in `agent.rs` was already built, guarded and tested. So the loop is a few hundred lines in `agent_loop.rs` talking to a runtime the app already depends on — no second process on `:18765`, no FastAPI gateway on `:8765`, no extra thing for an author to install and keep running.

What is lost: hermes-agent's own memory and skill-growth machinery. Those goals move to a later phase or are dropped; they are marked below rather than left looking done.

Agents: [AGENTS.md](../AGENTS.md). Tools: [TOOLS.md](../TOOLS.md). Runtime: [HERMES-RUNTIME.md](../HERMES-RUNTIME.md). Loop: [WRITING-LOOP.md](../WRITING-LOOP.md).

## Goals

- hermes-agent on `127.0.0.1:18765` + tool gateway on `127.0.0.1:8765`
- Four hermes profiles / skill packs: App Controller, Deep Continuity, Integrity, World-Builder
- TOOLS.md schemas implemented and registered as hermes skills
- Command bar → App Controller; continuity / integrity / world-builder flows as today
- **Style learner** + **skill evolution** so the co-author gets richer with use

## Checklist

### Lifecycle and bridge

- [x] ~~Start/stop hermes-agent~~ — **dropped**. The loop runs in-process against Ollama; there is no second daemon to supervise.
- [x] ~~FastAPI tool gateway on `:8765`~~ — **dropped**. `agent::dispatch` is called directly, so tools cannot be reached by anything but the app itself. That is a stronger boundary than a loopback HTTP port, not a weaker one.
- [x] Tool execution and UI refresh through Tauri commands (`agent_run`, `agent_resume`, `agent_traces`)
- [x] Snapshot before bulk agent writes — `snapshot_project` is in the Controller and World-Builder toolsets, and their prompts tell them to use it
- [x] `agent/debts.json` for propagation gaps
- [x] Local by default; a remote model is refused unless turned on for that run, and the refusal names what would leave

### The loop

- [x] Any tool-calling model Ollama serves, chosen per run — not one required model
- [x] Four profiles mapping to the product agents, each a system prompt **plus a tool subset**
- [x] Confirm for `delete_document`, `compile_manuscript`, `search_web` — enforced in `agent::dispatch`, which the loop cannot bypass
- [x] Traces written into the project as readable JSON, one file per run
- [x] Hop cap so a model stuck in a loop costs seconds, not minutes

### Wire tools as skills

- [x] Binder: `create_folder`, `create_chapter`, `create_character_sheet`, `create_location_sheet`, `save_to_binder`, `search_binder`, `append_canon_entry`, `set_metadata`, `bulk_set_metadata`, `delete_document`
- [x] Integrity: `check_internal_similarity`, `search_web`
- [x] Compile: `compile_manuscript`
- [ ] Editor: `highlight_text_in_editor`, `replace_range`, `rewrite_range` — not built; the agent creates and edits documents, it does not drive the cursor
- [x] Continuity: `extract_entities`, `check_against_canon`, `check_foreshadowing_ledger`, `log_propagation_debt` — built in `continuity.rs`, all four with no model and no network. `log_propagation_debt` is the one that matters most: a model that fills a gap in the world by choosing a fact has become the author of the world's rules, so it records the gap instead.
- [x] ~~`run_ai_detection`~~ — cut, see [Phase 4](04-integrity-engine.md)
- [x] **Every tool carries a JSON Schema** (`agent::schema`), because a catalogue line tells a model a tool exists but a schema is what lets it call one. A test walks the catalogue and fails if the two drift.
- [x] No unconstrained shell, and no HTTP hop either — the loop calls `dispatch` in-process

### Voice, memory, skill growth

- [x] Style learner — built in [Phase 3](03-ai-integration.md) as `style.rs`, from prose marked **Done**
- [x] Injected into rewrite and continue
- [x] Inspector: style-match indicator (Inspector → Voice)
- [ ] Propose project-local skills after repeated workflows — **not built**, and it was hermes-agent's feature. Traces are recorded, which is the raw material, but nothing proposes a skill from them.
- [ ] Skill library in the inspector — not built, same reason

### UX

- [x] Command bar picks the profile and the model, and runs the loop
- [x] Inspector agent log reads real traces: profile, model, each tool call, the answer
- [ ] Continuity issues as inline comments in the editor — not built; the report is prose in the command bar
- [x] Integrity report panel with the packet export ([Phase 4](04-integrity-engine.md))
- [x] World-Builder writes into the binder, not into a reply
- [x] With Ollama down the command bar says so plainly instead of failing on submit

## Acceptance criteria

- **Primary:** A single natural-language command creates **Act 2**, **three empty chapters**, and an **antagonist character sheet**; the binder and inspector update **without a menu click**
- Deep Continuity on a planted contradiction **or canon clash** produces a structured JSON issue and an inline comment
- Integrity Check produces a synthesized report (why flagged), including slop spans, not only raw percentages
- World-builder flow adds a Worldbuilding binder document and a scene draft on disk; new facts land in Canon
- **“Rewrite in my voice”** uses the local style profile (no cloud) and a KEEP list when replacing more than a sentence
- After repeating a workflow, the app can **propose a new project skill** and install it only on confirm
- With network disabled, all agents except confirmed `search_web` still function

## What was built

`src-tauri/src/agent_loop.rs`.

- **The loop.** Send the profile's tools with their schemas → run the `tool_calls` that come back through `agent::dispatch` → feed each result in as a `role: "tool"` message → repeat, up to 12 hops.
- **Approval is a pause, not a flag.** When the model asks for a guarded tool the run stops with *nothing executed* and returns the whole conversation plus the pending call. `resume` continues from exactly that point. This matters: approving a re-derived call would approve whatever the model phrased on its second attempt, which is not what the author agreed to. A model that sets `confirmed: true` itself is ignored — there is a test for precisely that.
- **Declining is a turn, not a cancellation.** The refusal goes back as the tool's result so the model can say what it will do instead.
- **The tool subset is the real guardrail.** A World-Builder that is not given `delete_document` cannot delete a document however it is prompted. A call to a tool outside the profile comes back as data (`"not available to the World-Builder profile"`), so the model picks another rather than the run dying.
- **A failing tool is data too.** "That id does not exist" is exactly what a model needs in order to choose a different one, so failures are fed back rather than thrown.
- **Traces** in `traces/<started>-<profile>.json`, readable without the app.

### Verified live

- `llama3.2:3b` — one hop, folder created in the real binder.
- `qwen3.5:cloud` — two hops, and it used the folder id returned by the first call as the parent of the second. Multi-step with state carried between steps, which is the whole point.
- The remote run is behind `AEGISDRAFT_LIVE_CLOUD=1` in the test suite, because a prompt leaving the machine must never be a side effect of `cargo test`.

### A bug the live run found

`llama3.2:3b` sent `"parent_id": ""` instead of omitting the optional field. That empty string went to SQLite, which answered `FOREIGN KEY constraint failed` — an error about the author's database for what was really a model filling in a blank. The gateway now treats a blank optional string as absent (`agent::opt`), with the model's actual arguments quoted in the regression test. Stub tests would never have caught this; only a real model would produce it.

## Out of scope

- Cloud agent hosts, Nous Portal default, or remote telemetry of traces
- Replacing Phases 1–4 features; this phase only orchestrates them
- LangGraph / AutoGen (explicitly not used)
- Autonomous seed-to-novel overnight pipeline (author stays in the loop)

## Success definition

Scrivener features become database entries the AI can read/write; Turnitin features become multi-step investigations; hermes-agent is integrated into the file system and UI—capable of *doing* things, learning the author’s voice, and growing skills over time—not only *suggesting* text.
