# Brand

Purpose: the assets, and the rules for using them. Source files are in `brand/src/`; rendered output is in `brand/export/` and should never be edited by hand — re-render from source instead.

## The mark

One silhouette that reads two ways: a **page** (rounded top, two written rules) that tapers to a **shield** point. Not a book inside a shield inside a square — one shape, because it has to survive 16px in a menu bar.

The name is the argument: *aegis* is a shield. The product's promise is that your unpublished manuscript is protected by staying where it already is.

| File | Use |
| :--- | :--- |
| `brand/src/mark.svg` | Gradient mark on light surfaces |
| `brand/src/mark-white.svg` | On the ink surface (`#1C2B33`) and on photography |
| `brand/src/mark-mono.svg` | Single colour — inherits `currentColor`; for menus, print, stamps |
| `brand/src/app-icon.svg` | The application tile — gradient squircle, white mark |
| `brand/src/wordmark.html` | Mark plus wordmark, rendered with the real font |

In the app the mark is a component, not an asset: `AppMark` in `src/components/Icon.tsx`. Use that, so the title bar and the splash can never drift apart.

### Rules

- **Clear space** on all sides is the height of one written rule in the mark (≈ 8% of the mark's height).
- **Minimum size** 16px. Below that the two rules merge; use the mono mark instead.
- Never re-colour the gradient, rotate the mark, outline it, add a drop shadow, or set it on a busy photograph.
- Never stretch the wordmark. If the space is narrow, use the mark alone.

## Application icon

`brand/export/app-icon-1024.png` → the whole `src-tauri/icons/` set via `npx tauri icon`.

The 1024 canvas carries an 824px tile with a 185px corner radius, centred — the proportion macOS expects, so AegisDraft sits at the same optical size as its neighbours in the Dock rather than looking oversized.

To change the icon, edit `brand/src/app-icon.svg`, then:

```bash
rsvg-convert -w 1024 -h 1024 brand/src/app-icon.svg -o brand/export/app-icon-1024.png
npx tauri icon brand/export/app-icon-1024.png
```

## Colour

The palette is Meta's, used with intent. Full token list and the semantic mapping: [DESIGN-SYSTEM.md](DESIGN-SYSTEM.md).

| Role | Value |
| :--- | :--- |
| Primary | `#0064E0` |
| Gradient | `#0082FB → #0064E0 → #A033FF` (the mark only) |
| Ink | `#1C2B33` |
| Rail | `#F5F7F9` |

The gradient belongs to the mark. It is not a background, a button fill, or a heading treatment.

### Charts are a separate palette

Meaning colours are reserved (blue = internal match, orange = web, purple = AI writing, yellow = continuity), so a chart may not borrow them — a purple series would be claiming something untrue. Charts use their own validated ramp in `src/lib/viz.ts`, checked with the palette validator for lightness band, chroma floor, colour-vision separation and contrast. Dark-mode steps are deliberately absent: they need re-validating against a dark surface, not flipping.

## Typography

| Use | Face | Fallback |
| :--- | :--- | :--- |
| UI, chrome, labels | Hanken Grotesk | `system-ui, -apple-system, sans-serif` |
| Manuscript, headings | Literata | `Georgia, serif` |
| Paths, model tags, code, agent logs | IBM Plex Mono | `ui-monospace, Menlo, monospace` |

Fallbacks are picked for close metrics so a webfont that fails to load does not reflow the page.

## Voice

How the product talks, which matters more than the logo:

- **Say what is true, including when it is unflattering.** "Ollama not connected" beats a green dot. A control that does nothing is disabled and says which phase builds it.
- **Concrete over aspirational.** "Scenes are Markdown files on disk" beats "your work, your way".
- **The author is the authority.** Scores and analyses are a reading aid. "Nothing here decides whether the chapter is finished — you do."
- **Never imply cloud.** No "sync", no "workspace", no "account" language anywhere in the product.

## Splash

`splash.html` — a real Tauri window shown while the app boots, dismissed by the frontend calling `frontend_ready`. A 10-second watchdog in Rust reveals the main window anyway if the webview never reports in, so a failure cannot strand someone on a splash screen.

It carries the mark, the tagline, an indeterminate progress bar that respects `prefers-reduced-motion`, and the line "Starting locally — no account, no network", because the first thing the product should say is the thing that makes it different.

## Press kit

`brand/export/` holds icon PNGs at 16–1024, the mark in white and mono at 256, and the wordmark with transparent and white backgrounds. Regenerate everything:

```bash
for s in 16 32 64 128 256 512 1024; do
  rsvg-convert -w $s -h $s brand/src/app-icon.svg -o brand/export/icon-$s.png
done
```
