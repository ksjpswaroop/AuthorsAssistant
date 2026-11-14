# Guide

Three documents written against the running macOS build, with screenshots taken
from it. Nothing here is a mock-up or a plan.

| Doc | For | What it is |
| :--- | :--- | :--- |
| [USER-GUIDE.md](USER-GUIDE.md) | someone using the app | every screen, in the order you meet them, with what each one will and will not do |
| [DEMO.md](DEMO.md) | someone presenting the app | a 12-minute script, ten steps, with the setup, the lines, and the failure table |
| [INVESTOR-PITCH.md](INVESTOR-PITCH.md) | someone funding the app | the problem, why each part of it costs money, what exists, why it is defensible, and the risks |

Screenshots live in [img/](img/). They were captured from
`src-tauri/target/release/bundle/macos/AegisDraft.app` driving
`~/Documents/AegisDraft Demo/The Salt Road.aegis`, with Ollama serving local
models and no network calls.

## Reproducing the screenshots

```bash
npm install && npm run app:build -- --bundles app
```

```bash
cd src-tauri && cargo run --example seed_project -- "$HOME/Documents/AegisDraft Demo" "The Salt Road"
```

```bash
AEGISDRAFT_OPEN="$HOME/Documents/AegisDraft Demo/The Salt Road.aegis" open src-tauri/target/release/bundle/macos/AegisDraft.app
```

The demo project includes a deliberately duplicated paragraph (so Integrity has
a real finding) and out-of-order story dates (so the timeline has a real gap).
