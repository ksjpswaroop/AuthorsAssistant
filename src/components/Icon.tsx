import type { CSSProperties } from 'react'

/**
 * Every icon in the app, on one 24px grid, stroke-based so they scale and
 * recolour. Adding an icon means adding it here — never inline SVG in a screen.
 *
 * Deliberately *not* annotated `Record<string, ReactNode>`: that widened
 * `IconName` to `string`, so `icon="export"` typechecked and then rendered an
 * empty box at runtime. Letting the keys be inferred is what makes `IconName`
 * mean something.
 */
const PATHS = {
  chevronDown: <path d="M6 9l6 6 6-6" />,
  chevronRight: <path d="M9 5l7 7-7 7" />,
  plus: <path d="M12 5v14M5 12h14" />,
  close: <path d="M6 6l12 12M18 6L6 18" />,
  dots: <path d="M5 12h.01M12 12h.01M19 12h.01" />,
  check: <path d="M5 12.5l4.5 4.5L19 7.5" />,
  search: (
    <>
      <circle cx="11" cy="11" r="6.6" />
      <path d="M16 16l4.5 4.5" />
    </>
  ),
  folder: <path d="M3 7a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />,
  doc: (
    <>
      <path d="M6 3h8l4 4v14a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z" />
      <path d="M14 3v4h4" />
    </>
  ),
  book: <path d="M4 19.5V5a2 2 0 0 1 2-2h13v18H6a2 2 0 0 1-2-1.5z" />,
  person: (
    <>
      <circle cx="12" cy="8" r="3.4" />
      <path d="M4.6 20a7.4 7.4 0 0 1 14.8 0" />
    </>
  ),
  globe: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M3.5 12h17M12 3.5c2.4 2.5 2.4 14.5 0 17M12 3.5c-2.4 2.5-2.4 14.5 0 17" />
    </>
  ),
  offline: (
    <>
      <path d="M18.4 6.2A9 9 0 1 0 21 12" />
      <path d="M3 3l18 18" />
    </>
  ),
  sparkle: (
    <path d="M12 3v6M12 15v6M5.6 5.6l4.2 4.2M14.2 14.2l4.2 4.2M3 12h6M15 12h6M5.6 18.4l4.2-4.2M14.2 9.8l4.2-4.2" />
  ),
  clock: (
    <>
      <circle cx="12" cy="12" r="8.6" />
      <path d="M12 7.4V12l3 1.8" />
    </>
  ),
  database: (
    <>
      <path d="M4 7c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3z" />
      <path d="M4 7v10c0 1.7 3.6 3 8 3s8-1.3 8-3V7" />
      <path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" />
    </>
  ),
  shield: <path d="M12 3l7.5 3v6c0 4.6-3.1 8.1-7.5 9.4C7.6 20.1 4.5 16.6 4.5 12V6z" />,
  shieldCheck: (
    <>
      <path d="M12 3l7.5 3v6c0 4.6-3.1 8.1-7.5 9.4C7.6 20.1 4.5 16.6 4.5 12V6z" />
      <path d="M9.2 12.2l2 2 3.6-4" />
    </>
  ),
  download: (
    <>
      <path d="M12 3v11M8 10.5l4 4 4-4" />
      <path d="M4 20h16" />
    </>
  ),
  upload: (
    <>
      <path d="M12 16V5M8 8.5l4-4 4 4" />
      <path d="M4 20h16" />
    </>
  ),
  refresh: (
    <>
      <path d="M4 12a8 8 0 1 1 2.6 5.9" />
      <path d="M4 20v-5h5" />
    </>
  ),
  play: <path d="M6 4l13 8-13 8z" />,
  alert: (
    <>
      <circle cx="12" cy="12" r="8.6" />
      <path d="M12 8v5M12 16.4h.01" />
    </>
  ),
  info: (
    <>
      <circle cx="12" cy="12" r="8.6" />
      <path d="M12 11v5.5M12 7.6h.01" />
    </>
  ),
  circle: <circle cx="12" cy="12" r="8.6" />,
  grid: (
    <>
      <rect x="3" y="3" width="7" height="7" rx="1.5" />
      <rect x="14" y="3" width="7" height="7" rx="1.5" />
      <rect x="3" y="14" width="7" height="7" rx="1.5" />
      <rect x="14" y="14" width="7" height="7" rx="1.5" />
    </>
  ),
  list: <path d="M8 6h13M8 12h13M8 18h13M3.5 6h.01M3.5 12h.01M3.5 18h.01" />,
  lines: <path d="M4 5h16M4 10h16M4 15h11M4 20h7" />,
  panelLeft: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="2.5" />
      <path d="M9 4v16" />
    </>
  ),
  panelRight: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="2.5" />
      <path d="M15 4v16" />
    </>
  ),
  expand: (
    <path d="M8 3H5a2 2 0 0 0-2 2v3M16 3h3a2 2 0 0 1 2 2v3M8 21H5a2 2 0 0 1-2-2v-3M16 21h3a2 2 0 0 0 2-2v-3" />
  ),
  settings: (
    <>
      <circle cx="12" cy="12" r="3.2" />
      <path d="M12 3v2.4M12 18.6V21M3 12h2.4M18.6 12H21M5.6 5.6l1.7 1.7M16.7 16.7l1.7 1.7M18.4 5.6l-1.7 1.7M7.3 16.7l-1.7 1.7" />
    </>
  ),
  chart: <path d="M4 19V5M4 19h16M7 15l3.5-4.5L14 13l4-6" />,
  markdown: <path d="M4 17V7l4 5 4-5v10M17 7v7M17 14l-2.5-2.5M17 14l2.5-2.5" />,
  latex: <path d="M5 8V5h14v3M12 5v14M9 19h6" />,
  film: (
    <>
      <rect x="3" y="5" width="18" height="14" rx="2" />
      <path d="M3 9h18M8 5v4M14 5v4" />
    </>
  ),
  // --- Phase 6: collections, Scrivenings, structure -----------------------
  layers: (
    <>
      <path d="M12 3.2 3.6 7.5 12 11.8l8.4-4.3z" />
      <path d="M3.6 12 12 16.3 20.4 12" />
      <path d="M3.6 16.4 12 20.7l8.4-4.3" />
    </>
  ),
  filter: <path d="M4 5h16l-6.2 7.4v5.3l-3.6 2v-7.3z" />,
  stack: (
    <>
      <rect x="4" y="3.5" width="16" height="5" rx="1.4" />
      <rect x="4" y="10.5" width="16" height="5" rx="1.4" />
      <path d="M4 18.5h16" />
    </>
  ),
  heading: (
    <>
      <path d="M5 5v14M13 5v14M5 12h8" />
      <path d="M17 10.5h3M18.5 10.5V19" />
    </>
  ),
  split: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="2.5" />
      <path d="M12 4v16" />
    </>
  ),
  checklist: (
    <>
      <path d="M4 6.5 5.6 8l2.6-2.6M4 12.5 5.6 14l2.6-2.6M4 18.5 5.6 20l2.6-2.6" />
      <path d="M12 6.7h8M12 12.7h8M12 18.7h8" />
    </>
  ),
  compare: (
    <>
      <rect x="3" y="4.5" width="7.5" height="15" rx="1.6" />
      <rect x="13.5" y="4.5" width="7.5" height="15" rx="1.6" />
    </>
  ),
  target: (
    <>
      <circle cx="12" cy="12" r="8.4" />
      <circle cx="12" cy="12" r="4.4" />
      <circle cx="12" cy="12" r="0.9" />
    </>
  ),
  template: (
    <>
      <rect x="3.5" y="4" width="17" height="16" rx="2.2" />
      <path d="M3.5 9.2h17M9.5 9.2V20" />
    </>
  ),
  bookmark: <path d="M7 3.6h10a1 1 0 0 1 1 1v15.4l-6-3.6-6 3.6V4.6a1 1 0 0 1 1-1z" />,
  tag: (
    <>
      <path d="M11.2 3.6H20a.6.6 0 0 1 .6.6v8.8L11.5 22a1 1 0 0 1-1.4 0L2.5 14.4a1 1 0 0 1 0-1.4z" />
      <circle cx="16.4" cy="7.8" r="1.3" />
    </>
  ),
  // --- Phase 8: story visualization ---------------------------------------
  timeline: (
    <>
      <path d="M3 12h18" />
      <circle cx="7" cy="12" r="2.1" />
      <circle cx="14.5" cy="12" r="2.1" />
      <path d="M7 9.9V5.5M14.5 14.1v4.4" />
    </>
  ),
  network: (
    <>
      <circle cx="6" cy="6.5" r="2.4" />
      <circle cx="18" cy="8.5" r="2.4" />
      <circle cx="11" cy="18" r="2.4" />
      <path d="M8.1 7.6 15.7 8.2M7.2 8.6l2.7 7.2M16.6 10.6l-4 5.6" />
    </>
  ),
  canvas: (
    <>
      <rect x="3" y="4.5" width="18" height="15" rx="2.2" />
      <rect x="6" y="7.5" width="5" height="4" rx="1" />
      <rect x="13.5" y="12" width="4.5" height="4.5" rx="1" />
      <path d="M11 9.5h2.5v4" />
    </>
  ),
  // --- Phase 9: advanced visualization and analytics ----------------------
  mindmap: (
    <>
      <circle cx="5" cy="12" r="2.2" />
      <circle cx="18.5" cy="6" r="2" />
      <circle cx="18.5" cy="12" r="2" />
      <circle cx="18.5" cy="18" r="2" />
      <path d="M7.2 12h2.3M9.5 12V7.2h7M9.5 12h7M9.5 12v4.8h7" />
    </>
  ),
  flowchart: (
    <>
      <rect x="8.5" y="3" width="7" height="4.5" rx="1.2" />
      <rect x="2.5" y="16.5" width="7" height="4.5" rx="1.2" />
      <rect x="14.5" y="16.5" width="7" height="4.5" rx="1.2" />
      <path d="M12 7.5v3.5M6 16.5V11h12v5.5" />
    </>
  ),
  infographic: (
    <>
      <rect x="3.5" y="3.5" width="17" height="17" rx="2.2" />
      <path d="M7.5 16.5v-4M12 16.5V8M16.5 16.5v-6" />
    </>
  ),
  cluster: (
    <>
      <circle cx="7.5" cy="8" r="1.6" />
      <circle cx="11" cy="6" r="1.6" />
      <circle cx="10.2" cy="10.6" r="1.6" />
      <circle cx="16.8" cy="16" r="1.6" />
      <circle cx="19" cy="12.4" r="1.6" />
      <path d="M4.5 13.5a7 7 0 0 1 8-8" opacity="0.5" />
    </>
  ),
  pulse: <path d="M3 12.5h3.6l2.2-6 3.4 11 2.6-7.4 1.7 2.4H21" />,
  link: (
    <>
      <path d="M10.4 13.6a3.6 3.6 0 0 0 5.1 0l2.6-2.6a3.6 3.6 0 0 0-5.1-5.1l-1.3 1.3" />
      <path d="M13.6 10.4a3.6 3.6 0 0 0-5.1 0l-2.6 2.6a3.6 3.6 0 0 0 5.1 5.1l1.3-1.3" />
    </>
  ),
  eye: (
    <>
      <path d="M2.6 12S6 6.4 12 6.4 21.4 12 21.4 12 18 17.6 12 17.6 2.6 12 2.6 12z" />
      <circle cx="12" cy="12" r="2.6" />
    </>
  ),
  copy: (
    <>
      <rect x="8.5" y="8.5" width="12" height="12" rx="2" />
      <path d="M15.5 5.5H5.5a2 2 0 0 0-2 2v10" />
    </>
  ),
  wand: (
    <>
      <path d="M14.5 4.5 19.5 9.5M4 20 15.6 8.4a1.4 1.4 0 0 1 2 0l2 2a1.4 1.4 0 0 1 0 2L8 24" />
      <path d="M5.5 4v3M4 5.5h3M18 16v2.4M16.8 17.2h2.4" />
    </>
  ),
  trash: (
    <>
      <path d="M4 7h16M9.5 7V5.5a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1V7" />
      <path d="M6 7l1 12.5a1 1 0 0 0 1 .9h8a1 1 0 0 0 1-.9L18 7" />
    </>
  ),
}

export type IconName = keyof typeof PATHS

/** True when a string names a real icon. Icon overrides are stored in the
 *  project database, so a name can outlive the icon it referred to. */
export function isIconName(name: string | null | undefined): name is IconName {
  return !!name && name in PATHS
}

export function Icon({
  name,
  size = 16,
  strokeWidth = 1.8,
  opacity,
  style,
}: {
  name: IconName
  size?: number
  strokeWidth?: number
  opacity?: number
  style?: CSSProperties
}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      opacity={opacity}
      style={{ flexShrink: 0, ...style }}
      aria-hidden="true"
    >
      {/* An unknown name renders the generic document rather than an empty
          box: icon names come from the project database and can outlive a
          rename, and a silent gap in the binder is worse than a wrong icon. */}
      {PATHS[name] ?? PATHS.doc}
    </svg>
  )
}

/** The app mark: the Meta gradient square used in every title bar. */
export function AppMark({ size = 18, radius = 5 }: { size?: number; radius?: number }) {
  return (
    <div
      style={{
        width: size,
        height: size,
        borderRadius: radius,
        flexShrink: 0,
        background: 'linear-gradient(135deg, #0082FB 0%, #0064E0 55%, #A033FF 100%)',
      }}
    />
  )
}
