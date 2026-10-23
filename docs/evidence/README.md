# Test evidence

How to reproduce everything in this folder, and what each run proves.

## Running it

```bash
scripts/verify.sh <label>     # removes the built app, rebuilds, runs every check
```

Each run writes `docs/evidence/<label>-<utc>.log` containing the full output. The
script fails on a clippy warning, a failing test, a type error or a broken build,
so a log ending in `RESULT: PASS` means all of them held at that commit.

Individual layers:

```bash
cd src-tauri && cargo test              # unit + integration
cd src-tauri && cargo test -- --nocapture | grep EVIDENCE   # live Ollama output
npx vitest run                          # frontend logic
npx tsc --noEmit                        # types
```

## What is covered

| Layer | Tests | What they establish |
| :--- | ---: | :--- |
| `project` | 8 | Round trip to disk, file numbering, folder delete trashes children, move/reorder refuses cycles, every table exists |
| `snapshots` | 3 | Commits are real Git, no empty commits, restore preserves unsnapshotted work, SQLite side files stay out |
| `collections` | 5 | Manual order, smart membership follows metadata, folders never join, files never move |
| `scrivenings` | 7 | Stitch/split round trip, edits land in one file, a broken boundary refuses the whole save |
| `compile` | 15 | Filters narrow, section types drive headings, HTML escapes prose, EPUB/DOCX are valid archives |
| `rag` | 10 | Chunking loses no words, overlap ≥ size is refused, cosine never returns NaN, reindex replaces |
| `ollama` | 7 | Absence is a state not an error; live generation; embeddings probed before asserting |
| `integrity` | 12 | Reuse found and sourced, containment is asymmetric, signals safe on empty input |
| `analysis` | 15 | Beats placed by word count, undated scenes keep their place, edges are facts not inferences |
| `agent` | 13 | Every guarded tool refuses without confirmation, the documented App Controller sequence runs |
| **integration** | **8** | All of the above against a real project folder: real SQLite, real Markdown, real Git |
| frontend | 12 | Word counting matches Rust, cloud-proxied models hidden, guarded tool list |

## Bugs these tests caught

Recorded because each was found by a test rather than by reading, and each would
have reached an author:

1. **`rag::migrate` was never called** from `store::open`, so the `chunks` table
   did not exist and indexing would have failed the first time anyone used it.
   Now asserted by `a_new_project_has_every_table_each_feature_depends_on`, which
   fails for any module whose migration is not wired.
2. **Test isolation via environment variables.** Two Ollama tests set a global
   env var; cargo runs tests in parallel threads, so they silently redirected the
   live tests to a dead port and every live assertion skipped while reporting
   green. Fixed by passing the host explicitly — no test mutates process state.
3. **`:cloud` model tags** report a tiny size, sort first by size, and return
   empty output because they run remotely. Both the test helper and the UI now
   exclude them; offering one would look like a working choice that fails.
4. **`is_none_or` requires Rust 1.82** but the crate declares 1.77 — it would not
   have compiled on the version we claim to support. Caught by clippy's MSRV lint.

## A wrong diagnosis, corrected

An earlier version of this document said embeddings could not be verified here
because "the installed Ollama was started without embedding support". **That was
wrong**, and it was stated as fact.

What actually happened: the test helper picked *the smallest local model* to
embed with, which was a chat model. Ollama answers that with

```
{"error":"This server does not support embeddings. Start it with `--embeddings`"}
```

The message blames the server. The real problem is the model. I took the message
at face value, reported a daemon limitation that did not exist, and wrote it into
three documents.

The fix is not a workaround — embedding and generation models are genuinely not
interchangeable, and the code now says so:

- `ollama::embedding_models` / `generation_models` split the pools by name, and
  `generation_and_embedding_pools_never_overlap` asserts they stay disjoint.
- Asking a chat model to embed now reports **"`<model>` is not an embedding
  model"**; asking an embedding model to write prose reports that it cannot.
  `a_chat_model_asked_to_embed_is_told_what_is_actually_wrong` pins that message.
- HTTP status is no longer treated as an opaque error. Ollama's own explanation
  is read from the response body, because "http status: 400" told me nothing and
  cost an hour.

The same latent bug existed in the other direction and was caught by these
changes: `live_generation_returns_text` began failing because the newly-installed
0.4 GB embedding model sorted first by size and was handed a prose prompt.

## Verified live, on this machine

| Assertion | Evidence |
| :--- | :--- |
| Embeddings are real and deterministic | `model=xagi/qwen3-embedding:0.6b-q4_k_m-1024 dims=1024 repeat=1.0000` |
| Embeddings are semantically ordered | `near=0.7059 far=0.3786` — two sentences about a dry well beat one about accounting |
| Retrieval finds the right passage | Query "Who was hurt, and who treated the injury?" → *"The descent took the leg in the second hour. The surgeon splinted it badly."* |
| A whole project indexes | `docs=5 chunks=5 stored=(5,5)`, and re-indexing replaces rather than duplicates |
| Generation writes prose | `llama3.2:3b` → 483 characters continuing a real scene, with the document left unmodified |

To reproduce: `cd src-tauri && cargo test -- --nocapture | grep EVIDENCE`.
