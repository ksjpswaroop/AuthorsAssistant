# Hermes runtime

Purpose: how AegisDraft runs **Nous Research hermes-agent** as the local agent runtime (replacing LangGraph)—ports, profiles, skills, writing-style memory, and self-evolution.

Related: [AGENTS.md](AGENTS.md), [TOOLS.md](TOOLS.md), [ARCHITECTURE.md](ARCHITECTURE.md), [WRITING-LOOP.md](WRITING-LOOP.md), [phases/05-hermes-agents.md](phases/05-hermes-agents.md).

## Why hermes-agent

hermes-agent is a full agent product: multi-step tool loops, memory, extensible **skills**, and an OpenAI-compatible HTTP API. AegisDraft does not embed LangGraph. The UI talks to hermes-agent; hermes-agent calls AegisDraft binder tools through skills.

**Hermes 3** (via Ollama) remains the local model. **hermes-agent** is the runtime that plans and executes.

## Ports (locked)

| Service | Address | Role |
| :--- | :--- | :--- |
| hermes-agent API | `http://127.0.0.1:18765` | OpenAI-compatible `/v1`; AegisDraft-specific port (avoids stock hermes on `8642`) |
| Tool gateway | `http://127.0.0.1:8765` | FastAPI implementing [TOOLS.md](TOOLS.md); invoked by hermes skills |
| Ollama | `http://127.0.0.1:11434` | Hermes 3 + draft/embed models |

Bind hosts to loopback only. Bearer auth required on the hermes API (`API_SERVER_KEY`).

## Local-only policy

- Default profile: **Ollama only** — no Nous Portal, no cloud tool gateway
- Manuscript, style profile, memory, skills, and agent traces stay on disk
- Opt-in web similarity remains AegisDraft’s SearXNG path (`search_web` tool), not hermes cloud browse

Example env (illustrative):

```bash
API_SERVER_ENABLED=true
API_SERVER_HOST=127.0.0.1
API_SERVER_PORT=18765
API_SERVER_KEY=<local-dev-secret>
# Provider: Ollama / Hermes 3 — configure via hermes local profile, not portal
```

Health: `GET http://127.0.0.1:18765/health` → ok before enabling the command bar.

## Profiles = four product agents

| Product agent | hermes-agent profile / skill pack |
| :--- | :--- |
| App Controller | Binder mutation skills (`create_folder`, `create_chapter`, …) |
| Deep Continuity | `extract_entities`, `search_binder`, `check_against_canon`, highlights |
| Integrity | Similarity + detection + `run_slop_scan`; confirm on `search_web` and apply-cuts |
| World-Builder | `generate_lore`, `save_to_binder`, `draft_scene` / `rewrite_range` with brief |

Command bar and inspector pick the profile (or let a router skill choose). Details: [AGENTS.md](AGENTS.md).

## Skills → tool gateway

Each AegisDraft tool in [TOOLS.md](TOOLS.md) is wrapped as a **hermes-agent skill** that HTTP POSTs to `http://127.0.0.1:8765/...`. The gateway talks to Tauri/Rust for FS and SQLite, then emits UI events (`binder_changed`, `highlight_range`, `report_attached`, `agent_log`).

Do not give hermes-agent unconstrained shell/FS over the whole disk—skills are scoped to the open project.

## Writing-style profile

Voice is two parts ([WRITING-LOOP.md](WRITING-LOOP.md#voice-two-parts)):

1. **Guardrails (Part 1)** — built-in mechanical slop rubric. Not learned. Injected into every `draft_scene` / rewrite.
2. **Identity (Part 2)** — as the author writes and **accepts** prose:
   - Sample accepted scenes (local only), including KEEP sentences from revision briefs
   - Extract tone, sentence rhythm, dialogue habits, vocabulary wells, exemplars and anti-exemplars
   - Store under the project (e.g. `MyBook.aegis/agent/style-profile.json`) plus hermes memory
   - Inject into draft / rewrite / World-Builder (“rewrite in my voice”)

The style profile never leaves the machine. Authors can reset or edit Part 2 in Settings → Voice & skills. Bulk agent writes take an optional LibGit2 snapshot first; the inspector offers Keep / Discard.

## Self-evolution

The co-author gets richer with use:

| Mechanism | Behavior |
| :--- | :--- |
| **Memory** | hermes-agent persistent memory of project conventions, cast, prior agent outcomes |
| **Skill growth** | After repeated successful workflows, propose project-local skills (e.g. “Act 2 setup”, “antagonist sheet pack”) |
| **Inspector** | Skill library count, last skills used, style-match indicator on generations |
| **Consent** | New skills install only after user confirm; no silent cloud sync |

## Lifecycle with Tauri

1. App start → ensure Ollama reachable → start hermes-agent on `18765` → start tool gateway on `8765`
2. UI uses `http://127.0.0.1:18765/v1/chat/completions` (and/or runs API) with the local bearer key
3. App quit → stop gateway processes cleanly

## Health checks

- hermes-agent: `/health` and `/v1/models`
- Tool gateway: `/health` listing registered tools
- Ollama: list tags includes configured Hermes 3 tag

If hermes-agent is offline, command bar shows Offline state; drafting via Phase 3 single-shot Ollama may still work without multi-step agents.
