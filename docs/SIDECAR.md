# wigolo as a sidecar

AegisDraft starts, health-gates and stops wigolo, and wigolo dies with the app.
That is the *operational* meaning of a sidecar, and it is the one implemented in
[`src-tauri/src/sidecar.rs`](../src-tauri/src/sidecar.rs).

It is deliberately **not** the Tauri `externalBin` meaning — a self-contained
executable copied into the app bundle. Two reasons, both checked rather than
assumed.

## 1. There is no binary to bundle

Tauri's `externalBin` needs a native executable named `<name>-<target-triple>`.
Upstream publishes none:

| Checked | Result |
| :--- | :--- |
| GitHub releases (11 of them, as of v0.2.1) | **zero** binary assets on every one |
| `docs/installation.md` | "single-file binary, Homebrew, hosted install script … publish with an upcoming release … so you don't chase an artifact that isn't live yet" |
| npm `wigolo@0.2.1` | `bin: dist/index.js`, `engines: {node: ">=20"}`, 6.4 MB unpacked — JavaScript, not a binary |

Building one ourselves would mean bundling a Node runtime (50–110 MB per
platform) plus wigolo's code. That is possible; it is not free, and it is not
what upstream intends people to ship yet.

## 2. wigolo is AGPL-3.0-only

`license: AGPL-3.0-only` on the npm package. Putting an AGPL program *inside* a
distributed desktop app is a copyleft question that reaches the app around it.
Two separate programs talking over a loopback REST API is the arm's-length
arrangement, and it happens to also be the one that works today.

So the binary stays the user's own, and AegisDraft manages its lifecycle. If
upstream ships a standalone executable later, only `resolve()` changes — every
other line of the supervisor is already the code you would need.

## How a launcher is resolved

Most explicit first, and every outcome explains itself:

1. `AEGISDRAFT_WIGOLO_BIN` — an explicit path. If it is set and is *not* an
   executable, that is reported, not silently skipped: a wrong setting the app
   quietly ignores is worse than one it complains about.
2. `wigolo` on `PATH`.
3. `npx -y wigolo`, when Node ≥ 20 is present. This is the common case, not the
   fallback — npm is the channel upstream actually publishes.
4. Nothing — and the message names the fix (install Node, or set the variable).

## Four rules

### Adopt, never duplicate

If something already answers `/health` on the port, it is used and **never**
killed. A wigolo the author started is theirs, and a second daemon fighting for
the same `~/.wigolo` cache helps nobody. `managed: false` says so in the UI:
*"yours, not ours to stop"*.

### Only ever stop what we started — but stop all of it

This is the rule that was wrong on the first live run, and the reason this
document exists.

The child was spawned, `/health` answered, a real search came back — and then
`stop()` reported success while the daemon **was still serving**:

```
EVIDENCE stopped: running=true managed=false
```

`npx wigolo` runs the real server as a *grandchild*. `Child::kill` signals one
process, so killing the npx wrapper left node orphaned onto init (`PPID 1`),
still serving, still holding port 3333. The app would have leaked a daemon on
every quit.

The fix is two parts, and both matter:

- The child is spawned with `process_group(0)`, and `SIGTERM` goes to the whole
  group (`kill(-pgid)`), escalating to `SIGKILL` after two seconds. A daemon
  gets the chance to close its database cleanly first.
- **`stop()` verifies.** It polls `/health` afterwards and returns an error if
  something is still listening, rather than reporting a success it did not
  achieve. A stop that does not check is a stop that can lie — this one did.

The live test now asserts `running == false` after a stop, so the bug cannot
come back quietly.

### Loopback only, always

`--host 127.0.0.1` is hard-coded, not configurable. wigolo fails closed on a
public bind and would then require a bearer token; binding only to loopback
means that situation cannot arise from here. `--allow-unauthenticated` is never
passed, and a test asserts both.

### Never started automatically

Starting wigolo starts something that reaches the internet on the author's
behalf. Per the [golden rule](ARCHITECTURE.md#golden-rule) that is the author's
choice, so it is a button in Settings and on the Integrity → Web tab, not a
launch step. `RunEvent::Exit` stops it on the way out.

## What the first start costs

Through npx, the first start downloads wigolo, a browser engine and on-device
models — around 1.5 GB into `~/.wigolo`. The UI says this **before** the button
is pressed rather than after, and the readiness timeout is 90 seconds so a slow
first run is not reported as a broken one.

## Verified live

wigolo 0.2.1, started through `npx` on Node v22.22.3:

```
EVIDENCE resolution: No wigolo installed, but Node v22.22.3 is here — it will run through npx
EVIDENCE started: running=true managed=true health=Some("healthy") base=http://127.0.0.1:3333
EVIDENCE search: 3 matches, engines ["duckduckgo"], warning None
EVIDENCE   0.25 overlap | evidence Some(0.219) | Wigolo: Local-First Web Intelligence… | https://mcpmarket.com/server/wigolo
EVIDENCE   0.00 overlap | evidence Some(1.000) | wigolo — local-first web intelligence… | https://knockoutez.github.io/wigolo/
EVIDENCE stopped: running=false managed=false
```

Two things worth reading twice in that output:

- The daemon sent `snippet` with an `evidence_score` and **no** `citation_id` or
  `source_span` — the REST-reference shape, not the README one. Parsing both was
  not over-engineering; a client that trusted the README would have got nothing.
- wigolo's own site scored `evidence 1.00` with `overlap 0.00`: unmistakably the
  canonical source, and it does not contain the phrase that was sent. Averaging
  those two numbers into one would have destroyed the only useful thing either
  of them said. This is why they stay in separate columns.

The live test is behind `AEGISDRAFT_LIVE_WIGOLO=1`, because launching a daemon
that queries the internet must never be a side effect of `cargo test`:

```bash
cd src-tauri && AEGISDRAFT_LIVE_WIGOLO=1 cargo test --lib live_sidecar -- --nocapture
```

## Known limits

- **App Store builds.** A sandboxed app may spawn a bundled helper, but wigolo
  is resolved from the user's `PATH` or `npx` and drives a headless browser —
  neither survives the App Store sandbox. The sidecar is for direct-distribution
  builds; a sandboxed build should treat wigolo as absent, and Integrity still
  works offline without it.
- **Windows** has no process groups in this implementation. `terminate` falls
  back to `Child::kill`, which has the same orphaning problem the Unix path
  fixes. Not built, and stated rather than assumed working.
