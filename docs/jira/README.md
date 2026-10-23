# Jira import

`block-identity-stories.csv` — 6 epics, 26 stories, 116 points, covering
[BLOCK-IDENTITY.md](../BLOCK-IDENTITY.md) and the suite in
[TEST-PLAN-BLOCKS.md](../TEST-PLAN-BLOCKS.md).

## Importing

**Jira → Settings → System → External System Import → CSV**, then map:

| CSV column | Jira field |
| :--- | :--- |
| `Issue Type` | Issue Type |
| `Summary` | Summary |
| `Epic Name` | Epic Name *(epic rows only)* |
| `Epic Link` | Epic Link *(story rows; matches an `Epic Name`)* |
| `Description` | Description |
| `Acceptance Criteria` | Acceptance Criteria *(custom field — create it, or map to Description)* |
| `Story Points` | Story Points |
| `Priority` | Priority |
| `Labels` | Labels *(space-separated; Jira splits them)* |
| `Component` | Component/s |
| `Test IDs` | a text custom field, or a label — see below |

Notes that save a re-import:

- **Import epics and stories in one pass.** The file is ordered epics-first and
  `Epic Link` matches `Epic Name`, which is how Jira resolves the parent. Two
  separate imports leave the stories orphaned.
- **Team-managed projects** use `Parent` rather than `Epic Link`. Rename the
  column before importing, or map `Epic Link` → `Parent`.
- **`Acceptance Criteria`** is not a default field. Either create the custom
  field first or map it into Description; leaving it unmapped silently drops the
  most useful column in the sheet.
- **`Test IDs`** points at cases in `TEST-PLAN-BLOCKS.md` (`BI-`, `PV-`, `NEG-`,
  `BB-`, `UF-`, `PERF-`, `MIG-`). Keeping it means a ticket can be closed against
  a named test rather than an opinion.

## Suggested order

The epics are already in dependency order, and it is not arbitrary:

1. **BLOCKS** — nothing else can be built first; every event points at a block id
2. **LEDGER** — needs blocks to point at
3. **WIRING** — needs both, and is where the feature becomes real rather than
   theoretical
4. **REPORT** — needs a populated ledger to report on
5. **UI** — needs the commands
6. **MIGRATE** — last, because it migrates *to* a finished schema

Within WIRING, **"Never launder agent writes as human authorship"** is the one
to schedule early rather than late. A ledger that silently attributes AI edits to
the author is worse than no ledger, because people would trust it.
