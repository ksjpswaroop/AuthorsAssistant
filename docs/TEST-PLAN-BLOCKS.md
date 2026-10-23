# Test plan — block identity and provenance

The suite for [BLOCK-IDENTITY.md](BLOCK-IDENTITY.md). **Written before the
implementation**: every case below should be red first, and the order in §1 is
the order to make them green.

Conventions match the existing suite — a test name is a sentence about
behaviour, not a function name, and an assertion carries the message a person
needs when it fails. Run with `scripts/verify.sh block-identity`; `RESULT: PASS`
is the gate.

**ID scheme.** `BI-` block identity · `PV-` provenance · `NEG-` negative ·
`BB-` black box · `UF-` user flow · `PERF-` performance · `MIG-` migration.
The same ids appear in [jira/block-identity-stories.csv](jira/block-identity-stories.csv).

---

## 1. TDD order

Each step is red → green → next. Nothing later is started while anything earlier
is red.

| # | Step | First failing test |
| ---: | :--- | :--- |
| 1 | `normalise` + `content_hash` | BI-01 |
| 2 | `split` — all eight kinds | BI-05 |
| 3 | `anchor` — exact pass | BI-14 |
| 4 | `anchor` — fuzzy pass and thresholds | BI-18 |
| 5 | `anchor` — uncertainty flags | BI-24 |
| 6 | Schema + `reconcile` persistence | BI-28 |
| 7 | `provenance::append` + hash chain | PV-01 |
| 8 | `verify_chain` | PV-06 |
| 9 | `classify` — the §7.2 table | PV-10 |
| 10 | `summarise` | PV-18 |
| 11 | Wiring: write_body, open, import, restore, compile | UF-01 … UF-05 |
| 12 | Wiring: agent actor threading | UF-06 |
| 13 | `assist_accept` | UF-07 |
| 14 | Commands + export | BB-01 … BB-08 |
| 15 | Migration of an existing project | MIG-01 |

---

## 2. Positive cases — block identity

### 2.1 Normalisation and hashing (`blocks::normalise`, `content_hash`)

| ID | Given | Then |
| :--- | :--- | :--- |
| BI-01 | `"The well  was\n  dry."` | normalises to `"The well was dry."` |
| BI-02 | The same paragraph hard-wrapped at 80 columns vs. on one line | **identical** hashes — a reflow is not an edit |
| BI-03 | Leading/trailing blank lines and spaces | trimmed; hash unchanged |
| BI-04 | Two texts differing by one comma | **different** hashes — punctuation is content |

### 2.2 Splitting (`blocks::split`)

| ID | Given | Then |
| :--- | :--- | :--- |
| BI-05 | Three paragraphs separated by blank lines | 3 blocks, all `Paragraph`, ordinals 0,1,2 |
| BI-06 | `## Chapter Two` followed by a paragraph | 2 blocks: `Heading`, `Paragraph` |
| BI-07 | A fenced code block containing a blank line | **1** block, `Code`, blank line preserved inside |
| BI-08 | Four contiguous `> ` lines | 1 block, `Quote` |
| BI-09 | A five-item bullet list | **1** block, `List` — not five |
| BI-10 | A pipe table with a separator row | 1 block, `Table` |
| BI-11 | `---` YAML fence at position 0 | 1 block, `FrontMatter`; a later `---` is `Rule` |
| BI-12 | Empty string / only whitespace | zero blocks, no panic |
| BI-13 | Windows line endings (`\r\n`) throughout | same result as `\n` |

### 2.3 Anchoring — exact (`blocks::anchor`)

| ID | Given | Then |
| :--- | :--- | :--- |
| BI-14 | Document unchanged between loads | every block `Same`; **zero** events written |
| BI-15 | A paragraph inserted in the middle | the one new block is `New`; every other keeps its id and ordinal shifts |
| BI-16 | Two blocks swapped in order | both `Same` with new ordinals — reordering is not rewriting |
| BI-17 | Two **identical** paragraphs, one edited | the edited one anchors to the **nearer** original, not the first |

### 2.4 Anchoring — fuzzy

| ID | Given | Then |
| :--- | :--- | :--- |
| BI-18 | A typo fixed in a 60-word paragraph | `Revised`, same id, `revision + 1`, `uncertain = false` |
| BI-19 | Half the sentences rewritten | `Revised`, same id, similarity between 0.35 and 0.60, `uncertain = true` |
| BI-20 | Paragraph replaced with unrelated prose | old → removed, new → `New`. Identity is **not** forced |
| BI-21 | Paragraph split into two | first anchors `Revised`; second is `New`. Documented, not ideal |
| BI-22 | Two paragraphs merged into one | anchors to whichever contributes more; the other is removed |
| BI-23 | A 3-word heading changed to another 3-word heading | `New` + removed, **never** fuzzy-matched (below `MIN_FUZZY_WORDS`) |

### 2.5 Uncertainty

| ID | Given | Then |
| :--- | :--- | :--- |
| BI-24 | Best match 0.72, runner-up 0.31 | `uncertain = false` |
| BI-25 | Best match 0.72, runner-up 0.66 | `uncertain = true` — the margin, not the score |
| BI-26 | Best match 0.40, no rival | `uncertain = true` — below `CONFIDENT_THRESHOLD` |
| BI-27 | Best match 0.34 | no anchor at all; delete + create |

### 2.6 Persistence (`blocks::reconcile`)

| ID | Given | Then |
| :--- | :--- | :--- |
| BI-28 | First reconcile of a new document | one `blocks` row per parsed block, `revision = 1`, `removed_at` null |
| BI-29 | Reconcile after an edit | revision bumped on the changed row only; others' `updated_at` unchanged |
| BI-30 | A block deleted from the text | row **kept**, `removed_at` set. Never a hard delete |
| BI-31 | A deleted block's text retyped verbatim later | a **new** id. Ids are never resurrected or reused |
| BI-32 | Document deleted | its blocks cascade; their events **remain** |
| BI-33 | `body_hash` matches the file | `reconcile` returns early, writes nothing |

---

## 3. Positive cases — provenance

### 3.1 Append and chain (`provenance::append`, `verify_chain`)

| ID | Given | Then |
| :--- | :--- | :--- |
| PV-01 | First event in an empty project | `seq = 1`, `prev_hash` = 64 zeros, `entry_hash` non-empty |
| PV-02 | Three events appended | `seq` 1,2,3 gapless; each `prev_hash` equals the previous `entry_hash` |
| PV-03 | Two events with identical field values | **different** `entry_hash` — `seq` participates |
| PV-04 | An `ai_model` event | `model` and `model_params` stored and read back |
| PV-05 | An event with `source_refs` | round-trips as a JSON array |
| PV-06 | Untouched chain of 50 events | `verify_chain` → `None` |
| PV-07 | Row 25's `action` edited directly in SQL | `verify_chain` reports a break **at seq 25** |
| PV-08 | Row 25 deleted directly in SQL | break reported at the gap |
| PV-09 | Empty ledger | `verify_chain` → `None`, not an error |

### 3.2 Classification (`provenance::classify`) — K2P-OS §7.2

| ID | Chain | Expect |
| :--- | :--- | :--- |
| PV-10 | `create(human)` | `Human`, no transformation |
| PV-11 | `create(human)` → `rewrite(ai)` → `accept(human)` | `Human`, "AI-assisted editing" |
| PV-12 | `create(ai)` → `accept(human)` | `Ai`, "Human-reviewed" |
| PV-13 | `create(ai)` → `accept(human)` → `rewrite(human, similarity 0.3)` | `Mixed`, "Human substantial rewrite" |
| PV-14 | `create(tool: import)` | `External` |
| PV-15 | Any chain containing an `inferred` event | `uncertain = true`, whatever the origin |
| PV-16 | **Empty** chain | `Unknown` with `unrecorded = true` — never `Human` |
| PV-17 | `create(ai)` → `reject(human)` | the rejection is preserved and readable |

### 3.3 Summary (`provenance::summarise`)

| ID | Given | Then |
| :--- | :--- | :--- |
| PV-18 | 10 human, 3 AI, 2 unrecorded blocks | counts exact; `unrecorded` is its **own** column, never folded into `human` |
| PV-19 | Scoped to one document | only that document's blocks counted |
| PV-20 | A broken chain | `chain_intact = false` and the summary still returns |

---

## 4. Negative and adversarial cases

| ID | Given | Then |
| :--- | :--- | :--- |
| NEG-01 | `append` called with an unknown `object_id` | accepted — the ledger records what happened, even about something since deleted |
| NEG-02 | Direct `UPDATE events` | there is **no** update function; the SQL path is proven detectable by PV-07 |
| NEG-03 | Direct `DELETE FROM events` | no delete function; detectable by PV-08 |
| NEG-04 | `reconcile` on a document with no file (a folder) | clean error naming the document, no partial write |
| NEG-05 | Markdown with an unterminated code fence | splits without hanging; the remainder is one `Code` block |
| NEG-06 | A 5 MB single-paragraph document | completes; no quadratic blow-up (see PERF-02) |
| NEG-07 | Text containing the hash-chain field separator byte | cannot be forged into a colliding `entry_hash` |
| NEG-08 | Two `reconcile` calls racing on one document | serialised by the connection; no duplicate ids, no gap in `seq` |
| NEG-09 | `classify` on events out of `seq` order | sorts before classifying; result independent of input order |
| NEG-10 | Block text that is only Markdown syntax (`***`) | classified `Rule`, not `Paragraph`; not fuzzy-anchored |
| NEG-11 | Author edits the file externally **and** in-app before reopening | in-app wins for hash comparison; the external delta reconciles as `Inferred` |
| NEG-12 | `assist_accept` with an empty proposal | refused with a reason; no event, no block |
| NEG-13 | `assist_accept` naming a block in another document | refused; blocks are document-scoped |
| NEG-14 | Agent writes prose while the actor is not threaded | **regression guard for §5.4** — the event must be `ai_model`, never `human` |
| NEG-15 | `provenance_export` to an unwritable path | clean error; nothing half-written |
| NEG-16 | Ledger of 100,000 events | `verify_chain` completes and streams; does not load all rows at once |

---

## 5. Black-box cases (the Tauri command surface)

Driven through commands only, exactly as the frontend calls them — no internal
functions.

| ID | Command sequence | Assert |
| :--- | :--- | :--- |
| BB-01 | `create_document` → `write_document` → `blocks_list` | blocks match the prose; ids stable across a second identical write |
| BB-02 | `write_document` twice with an edit → `block_history` | two events, `create` then `rewrite`, both `observed` |
| BB-03 | `block_authorship` on an author-written block | `Human`, `unrecorded = false` |
| BB-04 | `block_authorship` on a pre-existing block from before the ledger | `Unknown`, `unrecorded = true` |
| BB-05 | `provenance_summary` for a project | totals equal the sum of per-document summaries |
| BB-06 | `provenance_verify` on a healthy project | `null` |
| BB-07 | `provenance_export` | a JSON and an HTML file; the HTML states what is unrecorded and what is inferred |
| BB-08 | `assist_accept` → `block_authorship` | `Ai` + "Human-reviewed", model name present |
| BB-09 | `blocks_list` on a folder | empty list, not an error |
| BB-10 | Every new command with a path that is not a project | the standard "does not look like an AegisDraft project" refusal |

---

## 6. User-flow cases

End to end against a real project on disk, in `src-tauri/tests/phases.rs`.

| ID | Flow | Assert |
| :--- | :--- | :--- |
| UF-01 | **Write, edit, reopen.** Author writes 3 paragraphs, edits the middle one, closes and reopens | 3 blocks, middle at `revision 2`, others at 1; all events `observed`; ordinals intact |
| UF-02 | **The vim case.** Author edits the file in an external editor while the app is closed, then reopens | changed blocks keep their ids; events are `external` + `inferred`; a reflow-only edit produces **no** event (BI-02) |
| UF-03 | **Import.** Phase 2 import of a Markdown folder | every block has a `create` event with `actor = tool:import`, `observed`; authorship `External` |
| UF-04 | **Restore.** Take a snapshot, edit, restore | blocks return to the snapshot's content and **keep their ids**; a `restore` event is appended, and the intervening `rewrite` events are still in the ledger |
| UF-05 | **Compile.** Compile the manuscript | one `export` event per included document; the ledger says which edition contained which blocks |
| UF-06 | **Agent edit.** The Phase 5 loop calls `save_to_binder` | every resulting block's chain shows `actor_type = ai_model` with the model name — **never** `human`. The laundering guard |
| UF-07 | **Assist accept.** Generate a continuation, press *Use this* | a new block with `create(ai_model)` → `accept(human)`; `block_authorship` reports "AI-generated text with human review" |
| UF-08 | **Assist copy-paste.** Generate, copy manually, paste into the editor | block is `Human` **and** the report's method note states that manual insertion is not observable. The honesty test |
| UF-09 | **Mixed document report.** A chapter with author prose, one accepted AI paragraph, one imported quote | the summary separates all three; `provenance_export` HTML shows each with its lineage |
| UF-10 | **Tamper.** Edit `events` directly, then open the project | Settings reports the break at the right `seq`; the app still opens and still works |
| UF-11 | **Long life.** 50 edits to one paragraph across 20 snapshots | one block id throughout; 50 events; `classify` still correct; K2P-OS's "P-184 across 47 revisions" |

---

## 7. Performance

| ID | Case | Budget |
| :--- | :--- | :--- |
| PERF-01 | Reconcile an unchanged 2,000-block book | < 50 ms (pass 0 short-circuit) |
| PERF-02 | Reconcile a 60-block scene with one edited paragraph | < 20 ms |
| PERF-03 | Worst case: 60 blocks, all rewritten | < 200 ms |
| PERF-04 | `verify_chain` over 100,000 events | < 2 s, constant memory |

Budgets are asserted as generous upper bounds, not benchmarks: the point is to
catch a change that makes anchoring quadratic across the project, not to police
milliseconds.

---

## 8. Coverage of the design's own claims

Every load-bearing claim in [BLOCK-IDENTITY.md](BLOCK-IDENTITY.md) has a test.
If a claim here has no test, either the claim or the plan is wrong.

| Claim | Test |
| :--- | :--- |
| "A reflow is not a change" (§2.2) | BI-02, UF-02 |
| "Ids survive external editing" (§1) | UF-02 |
| "When it cannot, it says so" (§1.1) | BI-24…27, PV-15 |
| "Identical paragraphs anchor by proximity" (§2.4) | BI-17 |
| "Short blocks are exempt from fuzzy matching" (§2.4) | BI-23 |
| "Ids are never reused" (§2.3) | BI-31 |
| "Append-only; corrections supersede" (§3.1) | NEG-02, NEG-03, PV-07, PV-08 |
| "Tamper-evident, not tamper-proof" (§3.1, §7) | PV-07, UF-10 |
| "No record is not proof of human authorship" (§3.3) | PV-16, BB-04, UF-08 |
| "Classification is derived, never typed" (§3.4) | PV-10…PV-17 |
| "Agent edits must not launder as human" (§5.4) | NEG-14, UF-06 |
| "Pre-existing projects start honest and empty" (§6) | MIG-01…03 |
| "No cross-document tracking" (§7) | BI-20 documents the delete+create |

---

## 9. Migration

| ID | Given | Then |
| :--- | :--- | :--- |
| MIG-01 | A project created before this feature | opens; `blocks` and `events` are created; every block gets one `create(external, inferred)` event |
| MIG-02 | That project's summary | everything counted `unrecorded`/`Unknown`; nothing claimed as human |
| MIG-03 | That project opened by an older build afterwards | still opens and still works — the new tables are additive and no file changed |
| MIG-04 | `store::EXPECTED_TABLES` | includes `blocks` and `events`, so an unwired migration fails the existing guard test |
