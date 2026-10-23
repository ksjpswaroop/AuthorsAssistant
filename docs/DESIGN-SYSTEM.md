# Design system

Purpose: the visual rules the app is built from, and where they live in code. The pixel reference is the design canvas in `design/` (ten artboards, `.dc.html`); the runtime source of truth is `src/lib/tokens.ts`. If a value is not in that file, it does not belong in a component.

## Palette

Meta's palette, used with intent rather than decoration.

| Role | Token | Value |
| :--- | :--- | :--- |
| Primary action, selection, focus | `color.accent` | `#0064E0` |
| Primary text | `color.ink` | `#1C2B33` |
| Prose text | `color.inkProse` | `#22323B` |
| Secondary text | `color.ink2` | `#465A69` |
| Muted / meta text | `color.ink3`, `ink4`, `ink5` | `#7B8B98`, `#93A2AE`, `#A3B0BB` |
| Borders | `color.line`, `lineSoft`, `lineFaint` | `#DCE3E9`, `#EDF1F4`, `#E4EAEF` |
| Rails and surfaces | `color.rail`, `surface`, `surfaceSoft` | `#F5F7F9`, `#FFFFFF`, `#FBFCFD` |

### The meaning colours

The rest of the Meta family is reserved for meaning. Nothing uses these decoratively:

| Colour | Means |
| :--- | :--- |
| Blue `#0064E0` | Internal match — your own drafts and library |
| Orange `#FF5C01` | The network — anything fetched from the web |
| Purple `#A033FF` | AI writing — generated or detected as generated |
| Yellow `#FFC800` | Continuity — a claim that conflicts with the book |
| Green `#31A24C` | Verified, local, passing |

The gradient `#0082FB → #0064E0 → #A033FF` appears in exactly one place: the app mark (`AppMark` in `src/components/Icon.tsx`).

## Typography

| Use | Family | Fallback |
| :--- | :--- | :--- |
| Chrome, labels, UI | Hanken Grotesk | `system-ui, -apple-system, sans-serif` |
| Manuscript and headings | Literata | `Georgia, serif` |
| Agent logs, model tags, paths, code | IBM Plex Mono | `ui-monospace, Menlo, monospace` |

Loaded once from Google Fonts in `index.html`. Fallbacks are chosen for close metrics, because a webfont that fails to load must not reflow the page.

Manuscript text is 18.5px / 1.78 in a 680px column — the measure the design canvas settled on. Section labels are 10.5px, 700 weight, `0.09em` tracking, `color.ink5`.

## Chrome measurements

Fixed and shared, in `chrome`:

| Element | Height |
| :--- | :--- |
| Title bar | 40 |
| Toolbar / tab strip | 44 |
| Status bar | 28 |
| Rail header | 36 |
| Binder row | 27 |

`chrome.trafficLightInset` (78px) is the space the title bar reserves on macOS for the native window controls, which the OS draws over our bar.

## Components

Everything is assembled from `src/components/`:

- **`Icon.tsx`** — every icon in the app, one 24px stroke grid. Never inline an SVG in a screen; add it here.
- **`ui.tsx`** — `Button`, `IconButton`, `Chip`, `Dot`, `Kbd`, `Mono`, `SectionLabel`, `Segmented`, `Tabs`, `Toggle`, `Checkbox`, `SelectLook`, `ProgressBar`, `Callout`, `Card`, `EmptyState`, `PhaseBanner`.
- **`Chrome.tsx`** — `TitleBar`, `StatusBar`, `StatusItem`, `OllamaStatus`, `OfflineStatus`.
- **`PhaseScreen.tsx`** — the scaffold every later-phase screen uses: title bar, phase banner, rails, side panel.

### Selection and hover

Selected rows carry an accent tint (`tint(color.accent, 0.10)`) as an inline style. Hover comes from CSS, written as `.row:not(.on):hover` so a hover never overrides a selection. This pattern is why rows take a `className` of `row`/`tr` plus `on`.

### Layout

Flex and grid with `gap`, never margins between siblings and never whitespace-as-spacing. Panels in the studio use `react-resizable-panels` v4 (`Group` / `Panel` / `Separator`); percentages are passed as strings.

## Honesty rules

These are design rules, not engineering ones, and they are the reason the app can be trusted:

1. **A control that does nothing is disabled and says why.** `Button` takes `disabled` and a `title` naming the phase.
2. **A screen that is not implemented says so at the top.** `PhaseBanner` names the phase and what is missing.
3. **Sample data is labelled.** `SampleNote` marks anything not derived from the author's own project.
4. **Status reflects reality.** `OllamaStatus` shows a grey dot and "not connected" because there is no model runtime yet. A green dot would be a lie.
5. **Native things stay native.** The design canvas draws its own traffic lights; the app uses the real ones, because a painted dot that does not close the window is worse than no dot.

## Mapping the canvas to the code

| Artboard | Screen |
| :--- | :--- |
| `Main.dc.html` | `screens/Studio.tsx` + `components/{Binder,Editor,Corkboard,Outliner,Inspector}` |
| `Welcome.dc.html` | `screens/Welcome.tsx` |
| `Command.dc.html` | `screens/CommandBar.tsx` |
| `Compile.dc.html` | `screens/CompileDialog.tsx` |
| `Integrity.dc.html` | `screens/Integrity.tsx` |
| `Models.dc.html` | `screens/Settings.tsx` (Ollama, hermes-agent `:18765`, Voice & skills) |
| `Research.dc.html` | `screens/Research.tsx` |
| `Library.dc.html` | `screens/Library.tsx` |
| `Paper.dc.html` | `screens/Paper.tsx` |
| `TechBook.dc.html` | `screens/TechBook.tsx` |

Standalone `.dc.html` artboards are the source of truth for agent/runtime copy (hermes-agent, style, skills). The bundled `aegisdraft-tauri-app.html` may lag; prefer the artboards when updating messaging.

Agent logs and skill chips use **IBM Plex Mono** (same as model tags and paths).

The sample manuscript in the canvas is the same one the browser preview uses (`src/lib/demoProject.ts`), so a design review and a code review are looking at the same book.

## Components added for phases 6–9

All built from the same primitives and marked with `PhaseBanner` / `Chip` so a viewer always knows what is real.

| Component | Phase | File |
| :--- | :--- | :--- |
| `CollectionsRail` — manual, smart and saved-search collections | 6 | `components/Collections.tsx` |
| `Scrivenings` — a folder's children as one continuous read, seams visible | 6 | `components/Scrivenings.tsx` |
| `BulkBar` — floating multi-select metadata bar | 6 | `components/BulkBar.tsx` |
| `CompositionMode` — chrome-free writing, Escape to leave | 6 | inside `screens/Studio.tsx` |
| Section type, bookmarks, project keywords | 6 | `components/Inspector.tsx` |
| `ProjectDialogs` — templates, writing goals, snapshot compare | 6 | `screens/ProjectDialogs.tsx` |
| `Story` — plot diagram, timeline, character graph, infinite canvas | 8 | `screens/Story.tsx` |
| `Insights` — pacing, clusters, mind map, flowchart, story bible | 9 | `screens/Insights.tsx` |

The Studio's view switcher now carries four views (Editor, Corkboard, Outliner, Scrivenings), and the title bar reaches Story and Insights directly.

## Charts

Chart parameters live in `src/lib/viz.ts`, deliberately separate from the UI palette.

The meaning colours are reserved — blue is an internal match, orange is the network, purple is AI writing, yellow is continuity — so a chart series may not borrow them. A purple line in a pacing chart would be asserting something untrue. Charts use their own ramp:

```
#0064E0  #E8701F  #7A1FB8  #1A9AA6  #C2185B
```

Stepped from the same families and checked with the palette validator: lightness band, chroma floor, colour-vision separation (worst adjacent pair ΔE 11.8 deutan), normal-vision floor and contrast against the light surface all pass. Assign in fixed order by entity — never cycle, never reorder when a filter changes the series count.

Rules the charts follow: one axis only, never two scales; a legend whenever there are two or more series; direct labels on the structural extremes rather than every point; hit targets larger than the marks; grid and axes recessive.

**Dark mode is absent on purpose.** Dark chart steps have to be re-validated against a dark surface — flipping these would fail the lightness band immediately. The app is light-only today; when dark mode is designed, the ramp gets its own validated steps.

## Assets

The mark, icon, splash and voice are specified in [BRAND.md](BRAND.md). Two rules that touch code:

- The mark in the UI is the `AppMark` component, never an image file, so the title bar and splash cannot drift apart.
- Chrome heights and the macOS traffic-light inset are tokens in `chrome`, not per-screen numbers.
