# Writing loop

Purpose: define how AegisDraft drafts, checks, and revises **inside the binder**—canon, mechanical slop, revision briefs, and context packing—without becoming an autonomous novel factory.

This is the contract for Phases 2–5. Tools: [TOOLS.md](TOOLS.md). Agents: [AGENTS.md](AGENTS.md). Runtime: [HERMES-RUNTIME.md](HERMES-RUNTIME.md). Architecture: [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Product rule

The author writes. Agents mutate the project **after** a snapshot and **only** keep changes the author (or an explicit keep action) accepts.

AegisDraft does **not** run an overnight seed-to-PDF pipeline. Patterns here were informed by a working agentic novel loop (Nous Research autonovel): layer files, a hard-facts canon, regex slop as a first immune system, “cut fat” instead of 1–10 score chasing, and revision briefs that name what to keep. The product, runtime, and privacy model stay ours: local hermes-agent, Ollama, human-in-the-loop.

---

## Layer stack → binder

A novel is five co-evolving layers plus a fact database. Each layer is binder content, not a chat transcript.

| Layer | Question | Binder home | `documents.kind` |
| :--- | :--- | :--- | :--- |
| Voice | How we write | Project `agent/style-profile.json` + Settings → Voice | (not a binder doc; see Voice below) |
| World | What exists | Worldbuilding folder | `lore` |
| Characters | Who acts | Characters folder | `sheet` |
| Outline | What happens | Manuscript folders + synopses; optional Outline doc | `folder` / `scene` synopses |
| Prose | The manuscript | Manuscript scenes | `scene` |
| **Canon** | **What is true** | **Canon folder** | **`canon`** |

Changes propagate both ways:

- Lore change → check outline + scenes that mention it.
- Writing a scene that establishes a fact → append a canon entry.
- A scene that contradicts canon is a continuity bug, not a style issue.

Foreshadowing lives on the outline: every plant has a planned payoff (scene id + beat). Deep Continuity treats “planted, never harvested” as a first-class issue.

---

## Canon

Canon entries are **short, falsifiable statements with a source**. They are not prose. RAG finds similar passages; canon catches “she is left-handed in ch 1 and writes with her right in ch 7.”

### On disk

```
MyBook.aegis/
  manuscript/canon/0001-tamsin-left-handed.md
```

New projects seed an empty **Canon** binder folder. Each entry is one Markdown file (or a section in a category file) whose body is scannable, not narrative.

Example body:

```markdown
## Continuity facts

- Tamsin Reyes is left-handed. Source: Characters/Tamsin Reyes; 1. The Well at Kharsa.
- The Kharsa well is going dry. Source: 1. The Well at Kharsa.
- The basin tariff is nine days. Source: 3. Salt and Debt.
- The cartographer's left? right? leg was broken on the descent and splinted. Source: 2. What the Cartographer Kept.
```

Prefer one fact (or a tight group) per file when agents will append often; category files (Geography, Cast, Timeline) are fine for author-edited batches.

### Entry shape (tool JSON)

```json
{
  "statement": "Tamsin Reyes is left-handed.",
  "source_doc_id": "<uuid>",
  "source_span": { "start_index": 0, "end_index": 42 },
  "category": "cast",
  "status": "canonical"
}
```

`status` is `canonical` | `disputed` | `debt`. Disputed entries are inspector comments, not silent overwrites.

### Propagation debts

When drafting or lore generation reveals a gap (magic needs a cost; a year count disagrees), log a **debt** rather than inventing a fix:

```json
{
  "trigger": "scene: Nightfall at the Rim — moon rises south of the Rim",
  "affected_doc_ids": ["<worldbuilding-basin>", "<scene-ch1>"],
  "status": "pending"
}
```

Debts live under `MyBook.aegis/agent/debts.json` (local only). Continuity and World-Builder must surface pending debts in the inspector agent log. The author resolves them.

---

## Voice: two parts

| Part | What | Where | Who consumes it |
| :--- | :--- | :--- | :--- |
| **1. Guardrails** | Universal anti-slop. Same for every project. | Built-in rubric + `run_slop_scan` | Integrity, rewrite, `draft_scene` |
| **2. Identity** | This book’s tone, rhythm, vocabulary, exemplars **and anti-exemplars** | `agent/style-profile.json` + hermes memory | World-Builder, “rewrite in my voice” |

Part 1 is not learned from the author’s prose. Part 2 is. Authors can edit or reset Part 2 in Settings → Voice & skills. Guardrails stay on unless the author disables individual rubric criteria.

Style-profile fields (Part 2):

- `tone`, `sentence_rhythm`, `vocabulary_wells` (project-specific word lists), `pov`, `tense`, `dialogue_conventions`
- `exemplars[]` / `anti_exemplars[]` — short passages, local only
- `style_match` — inspector indicator on generations (cosine or rubric overlap against exemplars)

---

## Two immune systems

1. **Mechanical** (`run_slop_scan`) — no LLM. Regex and counts. Deterministic, offline, fast enough for on-save.
2. **Local judge** — Hermes 3 or a draft model **other than the writer** when possible, against the author’s rubric. Avoids self-congratulation.

Do not treat a 1–10 “overall quality” score as a revision plan. Absolute scores collapse. Prefer:

- Mechanical hits with spans
- Adversarial “what would you cut” with classified quotes
- Rubric dimensions the author named

---

## Mechanical slop rubric (Phase 4)

Tool: `run_slop_scan`. No network. No model.

### Output

```json
{
  "word_count": 912,
  "slop_penalty": 2.4,
  "spans": [
    {
      "start_index": 140,
      "end_index": 168,
      "category": "fiction_tell",
      "label": "a sense of",
      "excerpt": "a sense of dread settled"
    }
  ],
  "counts": {
    "tier1_banned": 0,
    "tier2_clusters": 0,
    "tier3_filler": 1,
    "fiction_tells": 2,
    "telling": 1,
    "structural_tics": 0,
    "em_dash_per_1k": 8.2,
    "sentence_length_cv": 0.41,
    "transition_opener_ratio": 0.05
  }
}
```

`slop_penalty` is 0–10 (0 = clean). Integrity charts it as a rubric dimension. `highlight_text_in_editor` can mark each span.

### Categories (starter lists)

**Tier 1 — kill on sight** (LLM-overrepresented): delve, utilize, leverage (verb), facilitate, elucidate, embark, endeavor, encompass, multifaceted, tapestry, paradigm, synergy, holistic, catalyze, juxtapose, myriad, plethora, testament to.

**Tier 2 — suspicious in clusters** (three in one paragraph): robust, comprehensive, seamless, cutting-edge, innovative, streamline, empower, foster, enhance, elevate, optimize, pivotal, intricate, profound, resonate, underscore, harness, cultivate, bolster, galvanize, cornerstone, scalable.

**Tier 3 — filler:** “it’s worth noting that”, “it’s important to note”, “importantly,” “notably,” “interestingly,” “let’s dive into”, “as we can see”, paragraph-starting furthermore/moreover/additionally, “at the end of the day”, “it goes without saying”, “when it comes to”, “not just X, but Y”.

**Fiction tells:** “a sense of …”, “couldn’t help but feel”, “the weight of …”, “the air was thick with”, “eyes widened”, “a wave of … washed over”, “a pang of …”, “heart pounded in … chest”, “let out a breath … didn’t know …”, “something dark/ancient/primal stirred”.

**Telling:** `felt|was|seemed` + emotion adjective; emotion `-ly` adverbs (angrily, sadly, nervously, …).

**Structural tics:** “I’m not saying X. I’m saying Y.”; “not just X, but Y”; “there’s a difference.”; “those are different things.”

**Rhythm:** em-dash density above ~15 / 1000 words; sentence-length coefficient of variation below ~0.3; >30% of paragraphs starting with transition words.

Authors can disable or add criteria in the rubric editor. Defaults stay conservative: highlight and explain, never auto-rewrite.

### Adversarial cuts (Integrity / rewrite, not a score)

Optional tool `propose_cuts`: ask the local judge to name 10–20 passages to cut or rewrite. Each item quotes **exact** text (10+ words), classifies `FAT | REDUNDANT | OVER-EXPLAIN | GENERIC | TELL | STRUCTURAL`, and may include a rewrite. Always include `tightest_passage` (do not touch) and `loosest_passage`.

OVER-EXPLAIN and REDUNDANT are the expected majority. Apply cuts only through `replace_range` / rewrite **after confirm**, and only with a revision brief that lists KEEP.

---

## Revision brief

Every `draft_scene` and rewrite that replaces more than a sentence must accept (or the gateway must build) a brief. Rewriting without KEEP destroys the sentences that already work.

```json
{
  "target_doc_id": "<uuid>",
  "kind": "tighten | compress | dramatize | deepen | polish | continue",
  "problem": "Panel/eval/slop: what is wrong, in one short block.",
  "keep": [
    "Exact sentences or beats that must survive."
  ],
  "change": [
    {
      "action": "cut | rewrite | add",
      "quote": "exact span if cut/rewrite, else null",
      "instruction": "what to do instead"
    }
  ],
  "voice_rules": [
    "Body-first emotion; no telling after showing.",
    "Inject style-profile identity when present."
  ],
  "target_word_count": 2800,
  "context_pack": { "see": "Drafting context pack" }
}
```

`kind` defaults:

| kind | Typical target length |
| :--- | :--- |
| `continue` | +2–4 paragraphs; no length cut |
| `tighten` | ~85% of current |
| `compress` | ~55–70% of current; never below ~1800 words for a full chapter unless the author sets it |
| `dramatize` | same length; change how information arrives |
| `deepen` | +private/unguarded beat in an existing scene; do not add a new scene unless asked |
| `polish` | slop + cuts only |

Expansion briefs must name **which beats** grow. “Make it longer” is not a brief. Compression below ~1800 words on a chapter-scale scene is a known failure mode: the scene becomes the new weakest.

The inspector shows the brief next to the generation. Accepting prose updates Part 2 of the style profile from the **kept** sentences, not from discarded drafts.

---

## Drafting context pack

`draft_scene` and rewrite load a pack. RAG is retrieval; the pack is the **default window** so chapter 20 still sees chapter 1’s rules.

**Always (cap ~8k tokens, summaries if needed):**

- Voice Part 1 (guardrail bullets) + Part 2 (style profile)
- Character sheets for people in the scene
- Worldbuilding summary / relevant lore docs
- Scene synopsis + parent folder synopses (outline)
- Foreshadowing plants/payoffs that mention this scene
- Canon entries whose statements match extracted entities

**Per task:**

- Target document (full)
- Previous scene, last ~1000 words
- Next scene synopsis (so the ending can flow)
- Pending debts that list this doc

Do not stuff the entire manuscript. `search_binder` fills holes the pack does not cover.

---

## Snapshots: keep / discard

Before bulk binder writes (World-Builder draft into a scene, Integrity apply-cuts, Continuity suggested fixes applied):

1. Optional LibGit2 snapshot (Phase 2; confirm in Settings, default **on** for agent bulk writes).
2. Mutate via tools.
3. Inspector offers **Keep** / **Discard**. Discard restores the snapshot. Keep is the author’s action, not a score threshold.

Never `git reset` the author’s work unattended. Scores and slop penalties are signals in the inspector, not an optimizer the app chases.

---

## Tools (summary)

Full schemas: [TOOLS.md](TOOLS.md).

| Tool | Phase | Role |
| :--- | :--- | :--- |
| `append_canon_entry` / `check_against_canon` | 2–5 | Fact database |
| `log_propagation_debt` | 5 | Gaps without silent invention |
| `run_slop_scan` | 4 | Mechanical immune system |
| `propose_cuts` | 4–5 | Classified fat; always returns KEEP passage |
| `draft_scene` | 3–5 | Requires context pack; brief when replacing |
| `rewrite_range` | 3–5 | Brief with keep + change |

---

## Explicit non-goals

- Autonomous seed → full novel → PDF overnight
- Cloud writer/judge (Anthropic, etc.) or uploading the manuscript to score it
- Replacing the four hermes profiles with a Python pipeline of scripts
- Treating 6.0/10 as “good enough, commit forever”
- Auto-applying OVER-EXPLAIN cuts without confirm
