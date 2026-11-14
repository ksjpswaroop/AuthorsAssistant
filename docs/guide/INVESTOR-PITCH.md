# AegisDraft — the problem we solve

*A local-first book studio with an agent that does the work and a record of who
wrote every paragraph.*

---

## 1. The problem, in one paragraph

A person writing a book now has to choose between two bad options. They can use
a writing tool that is good at structure and knows nothing about their book —
Scrivener has not shipped an idea since the iPad. Or they can use an AI tool
that is good at prose and requires them to paste their unpublished manuscript
into someone else's server, where it becomes training data, a breach waiting to
happen, or simply a dependency that ends when the subscription does. And at the
end of either path, when a publisher, a supervisor or a platform asks *"did you
write this?"*, there is no honest answer available — only a detector guessing
from the shape of the sentences, wrong often enough to end a career it has no
obligation to be right about.

**Three failures, one product:** the manuscript leaves the machine, the AI
cannot touch the project, and nobody can prove authorship.

---

## 2. Why each failure actually costs someone money

### a. The manuscript leaves the machine

An unpublished manuscript is the one asset a writer owns outright. Uploading it
is not a privacy preference, it is handing over inventory. This is why the
people with the most to lose use the least AI: the novelist under contract, the
journalist with a source, the academic under embargo, the ghostwriter under NDA,
the lawyer drafting a book from privileged matters. The current AI writing market
has effectively conceded this segment, because its architecture cannot serve it.

AegisDraft's architecture starts there. There is no network client compiled into
the build. There is nowhere to put an API key, because there is no provider to
configure. Cloud models exist only as an explicit, off-by-default, named choice
— the refusal is enforced below the UI, so no future caller can reach one by
accident.

### b. The AI cannot touch the project

Every writing AI on the market is a side-panel that produces text. The author
remains the integration layer: they read the suggestion, decide, click to the
right folder, create the file, paste, fix the metadata. The tool knows nothing
about chapter 1 while you are writing chapter 20, and it cannot create the
chapter either.

AegisDraft's agent operates the project. Typing *"create a folder for Act III
and add three empty chapters"* makes three Markdown files and three binder rows,
while the dialog is still open. The agent reads the book through a local
embedding index, checks a scene against the canon of hard facts the author
wrote, logs a gap in the world rather than inventing a fact to fill it, and
synthesises an integrity report. Chat is a transcript of work done, not the
deliverable.

### c. Nobody can prove authorship

This is the one nobody has solved, and the market is actively making it worse.
AI-detection vendors sell confident verdicts derived from statistical properties
of text — and false-positive rates that fall hardest on non-native speakers and
on anyone whose prose is plain. Institutions buy them because there is nothing
else to buy. The accused have no appeal, because there is no record to appeal to.

**AegisDraft ships no detector, and this is the position, not a gap.** Instead,
every paragraph gets a stable, content-anchored id and an entry in an
append-only, hash-chained ledger kept inside the project: who or what wrote
this, and when, *recorded as it was written*. Authorship is counted, not
inferred. Where there is no record, the report says *no record* rather than
assuming a human.

That is a claim a writer can defend and an institution can check. It is also the
only version of this problem that can be solved honestly, and we would rather
ship the honest version than the one that sells itself.

---

## 3. What exists today

Not a prototype. A shipping macOS desktop build, nine phases complete, with the
numbers reproducible by `scripts/verify.sh`:

| | |
| :--- | ---: |
| Rust tests (unit) | 291 |
| Rust tests (end-to-end, real files on disk) | 30 |
| Frontend tests | 22 |
| Tauri commands (the whole IPC surface) | 124 |
| Rust source | ~19,000 lines across 31 modules |
| Clippy warnings | 0 |

Working today, on a laptop, with no network:

- **Writing studio** — binder, corkboard (incl. freeform), outliner, scrivenings,
  inspector, collections, sheets, snapshots with paragraph-level diff and
  restore, templates, Markdown import, compile to Markdown / DOCX / EPUB / HTML /
  PDF / Fountain.
- **Local AI** — Ollama generation, chunking and embedding, semantic search over
  the whole book, a style profile measured from the prose the author marked Done
  and injected into rewrite and continue.
- **Integrity** — internal reuse, prose measurements, rubric scoring, an
  exportable packet carrying its method notes. Opt-in, phrase-scoped web
  similarity through a managed local daemon.
- **Agents** — a real tool-calling loop over 27 tools, four profiles that are
  system prompt *plus tool subset*, guarded tools that pause the run with nothing
  executed, and a readable JSON trace of every run inside the project.
- **Knowledge to publication** — sources with stable identity, claims verified
  word-for-word against their passage, two-stage contradiction detection, gap
  analysis, and a retraction query that names every paragraph resting on a
  withdrawn source.
- **Visualisation** — plot diagram, dual-order timeline with gap detection,
  character graph, an infinite canvas that commits a reorder back to the binder
  behind a plan and a snapshot, pacing, clustering, and model-generated mind
  maps and flowcharts validated against a strict shape.

Verified live during the writing of this document: integrity caught a duplicated
paragraph at 100%; semantic search answered a question whose words appear nowhere
in the matching scene; a 1.8 GB local model drove the binder through a two-step
tool call; and a guarded delete paused, was declined, and left the file on disk.

---

## 4. Why this is defensible

**Local-first is an architecture, not a setting.** A cloud product cannot add it
later — their unit economics, their retrieval, their evals and their compliance
story all assume the manuscript is on their servers. Retrofitting means rewriting
the product and giving up the data. Meanwhile, local inference gets cheaper every
quarter and we get a free ride on every open-weight release; our marginal cost
per user is zero, so we never have to meter the thing the user came for.

**The provenance ledger compounds.** It is only worth anything if it was running
while the book was written. A writer two months in cannot switch to a competitor
and bring their record. Every day of use makes leaving more expensive — without
holding their files hostage, which is exactly the lock-in we refuse.

**Honesty is the brand, and it is costly to copy.** Every screen says what it did
not do: *"tempo is a proxy, not a claim about tension"*, *"tamper-evident, not
tamper-proof"*, *"the measurements stay; the verdict is not ours to give."* A
competitor who has already shipped an AI detector cannot say this. In a category
where the incumbent product is a confident wrong number, being the one that
refuses to make one up is a position you can own.

---

## 5. Who buys it

| Segment | Why they cannot use the alternatives | What they pay for |
| :--- | :--- | :--- |
| Professional authors under contract | the manuscript is contracted inventory; NDAs forbid upload | the studio + the agent |
| Academics and research writers | embargo, IRB, unpublished data; and they are the ones accused by detectors | claims verified against sources, the ledger |
| Universities and editors | detectors are producing false accusations they cannot defend | the exportable, checkable authorship record |
| Journalists, ghostwriters, legal and policy writers | source protection, privilege, NDA | the whole posture |

Pricing: one-time licence or a straightforward subscription for a desktop app,
with no inference cost to carry. Institutional licensing is the second motion,
sold on the authorship record rather than on detection.

> **Market sizing is UNMEASURED.** No TAM number appears here because none has
> been researched, and an invented one would contradict the one thing this
> product is for. The numbers in §3 are reproducible; a market figure would
> have to be sourced before it belongs next to them.

---

## 6. The risks, said plainly

| Risk | Where it actually stands |
| :--- | :--- |
| Local models are weaker than frontier models | True, and shrinking fast. Mitigated by design: the agent is given tools and a tool subset, not asked to be brilliant. A 1.8 GB model drives the binder correctly today. |
| Desktop distribution is harder than web | Real. The macOS build exists; the App Store path is scoped, with the one blocker (security-scoped bookmarks) already solved, and the Unix-only web-similarity sidecar excluded from the sandboxed build. |
| "Nobody cares about privacy" | The segments in §5 are the ones who demonstrably do — they are currently not customers of anybody. |
| The ledger is tamper-evident, not tamper-proof | Stated in the product UI. Tamper-proof needs a signing identity and an external timestamp; that is a credible v2, not a hidden flaw. |
| Windows and Linux | Not shipped. The core is Rust and the shell is Tauri, so this is porting work, not a rewrite. |

---

## 7. The ask

Capital to take a complete, tested, nine-phase macOS product to a shipped,
signed, distributed one — App Store submission, Windows, the signing identity
and timestamp authority that turn the ledger from tamper-evident to
tamper-proof, and the first institutional pilots for the authorship record.

**The sentence to remember:** everyone else is selling a guess about who wrote
your book. We keep the receipt.

---

### Appendix — see it

- Walkthrough of every screen: [USER-GUIDE.md](USER-GUIDE.md)
- Twelve-minute live demo script: [DEMO.md](DEMO.md)
- What is built, phase by phase, with evidence logs: [../STATUS.md](../STATUS.md)
  — including, under Phase 4, why there is no AI detector
- The object model behind the ledger: [../BLOCK-IDENTITY.md](../BLOCK-IDENTITY.md)
