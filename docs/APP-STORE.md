# Mac App Store submission

Purpose: everything App Store Connect asks for, written out, plus the build and signing steps. Brand assets: [BRAND.md](BRAND.md). What actually works today: [IMPLEMENTATION.md](IMPLEMENTATION.md).

> **Read this first.** AegisDraft is at Phase 1 of nine. It writes, saves and organises — genuinely — but compile, local AI, integrity and the agents are not built. This document is submission-ready *material*, not a recommendation to submit. Shipping now would put an app on the Store whose own screens say "arrives in Phase 3". The realistic first submission is after Phase 2 (compile and snapshots), and the copy below is written to be honest at that point, not to oversell it.

## Listing

**Name.** AegisDraft *(30 char limit — 11 used)*

**Subtitle.** A book studio that stays local *(30 limit — 30 used)*

**Category.** Primary: Productivity. Secondary: Reference.
`LSApplicationCategoryType` = `public.app-category.productivity`

**Age rating.** 4+ — no user-generated content is shared, no network, no ads.

**Price.** *(decide)* — a one-time purchase suits the promise better than a subscription. A local-first app that stops working when a subscription lapses contradicts its own pitch.

### Promotional text *(170 limit, editable without review)*

> Your manuscript is a folder of Markdown files on your Mac. Binder, corkboard and outliner on top of it. No account, no upload, no subscription holding your book hostage.

### Description

> **AegisDraft is a writing studio for long-form work that keeps the work on your machine.**
>
> Open a project and you get a folder — not a proprietary database. Scenes are Markdown files you can read in any text editor, twenty years from now, with or without this app. The structure lives in a small SQLite file beside them. Nothing is uploaded, because there is nowhere to upload it to.
>
> **Organise a book, not a document**
> • Binder — nest acts, chapters, scenes, character sheets and research
> • Corkboard — every scene as an index card with its synopsis, label and status
> • Outliner — sort the whole manuscript by length, status or when you last touched it
> • Inspector — synopsis, label and status per document, saved as you type
>
> **Write without ceremony**
> • A clean editor with a proper measure and typography built for reading
> • Autosave that writes 800ms after you stop, and again when you switch away
> • Saves are atomic — a crash mid-save cannot truncate a scene
> • Delete moves files to the project's own trash, never straight to nothing
>
> **Privacy that is structural, not a policy**
> AegisDraft has no network client. Not a disabled one — none. No account to create, no telemetry, no crash reporting, no sync. The app cannot send your manuscript anywhere because it has no way to.
>
> **Built to outlive itself**
> Every scene is `manuscript/scenes/0004-nightfall-at-the-rim.md`. If AegisDraft vanished tomorrow, your book would be exactly where you left it, in a format everything can read.

*(When Phase 2 ships, add a Compile section. Do not describe AI features until Phase 3 is real. The screens that mention later phases are visible in the app and say so plainly; the listing must not get ahead of them.)*

### Keywords *(100 char limit, comma separated, no spaces)*

```
writing,novel,manuscript,author,markdown,binder,outliner,corkboard,offline,local,scrivener,drafts
```

*99 characters.* Do not put "AegisDraft" or the category name in keywords — Apple indexes the name and category already.

### Support and marketing URLs

| Field | Value |
| :--- | :--- |
| Support URL | *(required — a page that answers "where is my work stored")* |
| Marketing URL | *(optional)* |
| Privacy Policy URL | *(required even when nothing is collected)* |

## App privacy — the nutrition label

Answer **"Data Not Collected"** for every category. That answer is true and, unusually, easy to defend: there is no network entitlement in the build.

A privacy policy is still mandatory. It can be four sentences:

> AegisDraft does not collect, transmit or store any personal data. Your projects are files on your own device, in folders you choose. The application makes no network requests and contains no analytics, telemetry or crash reporting. If a future version adds an optional feature that uses the network, it will be off by default, will ask before each use, and will be described here before it ships.

**Encryption.** Set `ITSAppUsesNonExemptEncryption` to `false` in `Info.plist` — AegisDraft uses no encryption — so every submission skips the export-compliance questionnaire.

## Sandbox and entitlements

`src-tauri/entitlements.plist` is written and minimal:

| Entitlement | Why |
| :--- | :--- |
| `com.apple.security.app-sandbox` | Required for the Store |
| `com.apple.security.files.user-selected.read-write` | The project folder the author picks — the app's entire filesystem need |
| `com.apple.security.files.bookmarks.app-scope` | Reopening a recent project without asking again |

| `com.apple.security.network.client` | Ollama on loopback. Added when Phase 3 shipped — the sandbox blocks outgoing connections without it, loopback included |

## Two builds, and why they differ

There are **two** entitlements files, and using the wrong one produces an app that installs cleanly and then fails.

| File | For | App Sandbox |
| :--- | :--- | :--- |
| `entitlements.plist` | Mac App Store | **on** — required |
| `entitlements-direct.plist` | A DMG people download, signed with a Developer ID | **off** |

The sandbox is mandatory for the Store and optional outside it, and switching it on for a direct build silently breaks two features:

- the **wigolo sidecar**, launched through `npx` ([SIDECAR.md](SIDECAR.md))
- **PDF sources**, which shell out to `pdftotext` ([KNOWLEDGE-OS.md](KNOWLEDGE-OS.md))

A sandboxed download would fail at both with no error a person could act on. So the App Store build ships without them — they are listed as known limits — and the direct build ships with them and without the sandbox.

Nothing else changes. Both read and write only the project folder the author picked, both talk to Ollama on loopback, and both reach the network only through the two gated paths in the [golden rule](ARCHITECTURE.md#golden-rule).

### Building the direct-distribution DMG

```bash
APPLE_SIGNING_IDENTITY="Developer ID Application: … (TEAMID)" \
  npx tauri build --bundles dmg
```

with `bundle.macOS.entitlements` pointing at `entitlements-direct.plist`. The DMG lands in `src-tauri/target/release/bundle/dmg/`.

**Signing is not notarization.** A signed but un-notarized DMG is rejected by Gatekeeper on any machine that did not build it:

```
$ spctl -a -vv -t install AegisDraft.app
rejected
source=Unnotarized Developer ID
```

The signature itself is valid — `codesign --verify --deep --strict` passes and the app satisfies its Designated Requirement. Gatekeeper simply will not run a Developer ID app it has not seen Apple approve.

Notarizing uploads the app to Apple, so it needs credentials and a deliberate decision:

```bash
xcrun notarytool submit AegisDraft_0.1.0_aarch64.dmg \
  --apple-id <id> --team-id <team> --password <app-specific-password> --wait
xcrun stapler staple AegisDraft_0.1.0_aarch64.dmg
```

### Security-scoped bookmarks · **done**

Recent projects used to be stored as plain paths, and under the sandbox a plain path grants no access on the next launch — every recent project would fail to reopen. `bookmarks.rs` now stores a security-scoped bookmark beside each path, resolves it on open, and falls back to the plain path when unsandboxed so development is unaffected.

## Screenshots

macOS requires at least one, up to ten, in one of these sizes — 16:10, RGB, no alpha:

| Size | Notes |
| :--- | :--- |
| 2880 × 1800 | Preferred; matches the app's 1440×900 default at 2× |
| 2560 × 1600 | Also accepted |
| 1440 × 900 / 1280 × 800 | Accepted, lower quality |

The window defaults to exactly 1440×900, so a 2× capture of the window alone is 2880×1800 with no scaling.

**Shot list**, in order — the first screenshot is the only one most people see:

1. **The studio, mid-scene.** Binder, prose, inspector. The whole product in one frame.
2. **Corkboard.** Index cards with synopses — the most recognisable "this is for books" view.
3. **Outliner** sorted by word count, showing an obviously unfinished chapter.
4. **The launch screen**, with the "Scenes are Markdown files on disk" rail visible.
5. **A Finder window beside the project folder**, showing `0004-nightfall-at-the-rim.md` open in TextEdit. This is the promise made literal, and no competitor can copy it.

Rules: use the sample project (`cargo run --example seed_project`), never a real manuscript. No lorem ipsum. Do not screenshot a phase-banner screen — the banner is right for the app and wrong for a store listing.

```bash
# Capture just the window at 2× on a Retina display
cargo run --example seed_project -- /tmp/AegisShots "The Salt Road"
AEGISDRAFT_OPEN="/tmp/AegisShots/The Salt Road.aegis" npm run app
# then: cmd-shift-4, space, click the window (macOS captures at native scale)
```

## Build, sign, submit

Two different signings — do not mix them up:

| Target | Certificate | Notarize? |
| :--- | :--- | :--- |
| Direct download (.dmg) | Developer ID Application | Yes |
| Mac App Store | 3rd Party Mac Developer Application + Installer | No — App Review handles it |

```bash
# 1. Universal binary — Apple silicon and Intel in one bundle
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run tauri build -- --target universal-apple-darwin

# 2. App Store package
export APPLE_SIGNING_IDENTITY="3rd Party Mac Developer Application: <Team> (<TEAMID>)"
npm run tauri build -- --target universal-apple-darwin --bundles app

# 3. Wrap as .pkg and upload
xcrun productbuild --sign "3rd Party Mac Developer Installer: <Team> (<TEAMID>)" \
  --component "src-tauri/target/universal-apple-darwin/release/bundle/macos/AegisDraft.app" \
  /Applications AegisDraft.pkg
xcrun altool --upload-app -f AegisDraft.pkg -t macos \
  -u "<apple-id>" -p "<app-specific-password>"
```

`tauri.conf.json` already carries `minimumSystemVersion: 13.0`, the category, copyright, the descriptions and the entitlements path.

## Before the first submission

Blocking:

- [ ] **Security-scoped bookmarks** for recent projects (see above) — without this the app is broken on second launch under the sandbox
- [ ] Apple Developer Program membership and a Team ID
- [ ] Support URL and privacy policy published
- [ ] `ITSAppUsesNonExemptEncryption = false` in `Info.plist`
- [ ] Test a signed, sandboxed build — not `tauri dev` — creating and reopening a project in `~/Documents`
- [ ] Decide the price

Worth doing first:

- [ ] Phase 2, so the listing can describe compile and snapshots
- [ ] A native File menu already exists; add Open Recent
- [ ] Crash-free run over a multi-hour writing session with autosave under load

### Review notes to include

> AegisDraft is an offline writing application. It makes no network requests and has no accounts, so there are no credentials to provide. To exercise it: launch, click "New project", choose any folder, and a project is created with one empty scene. Type in the editor; text saves automatically to a Markdown file inside the folder you chose. Several screens carry a visible banner naming a later development phase — those features are intentionally not implemented in this version, and the banners are shown to users so nothing is misrepresented.
