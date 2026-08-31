import type { CSSProperties, ReactNode } from 'react'

/**
 * Every icon in the app, on one 24px grid, stroke-based so they scale and
 * recolour. Adding an icon means adding it here — never inline SVG in a screen.
 */
const PATHS: Record<string, ReactNode> = {
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
  trash: (
    <>
      <path d="M4 7h16M9.5 7V5.5a1 1 0 0 1 1-1h3a1 1 0 0 1 1 1V7" />
      <path d="M6 7l1 12.5a1 1 0 0 0 1 .9h8a1 1 0 0 0 1-.9L18 7" />
    </>
  ),
}

export type IconName = keyof typeof PATHS

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
      {PATHS[name]}
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
