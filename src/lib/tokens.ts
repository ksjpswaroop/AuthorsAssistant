/**
 * Design tokens, lifted verbatim from the design canvas (design/*.dc.html).
 * Every screen reads colours, type and spacing from here — if a value is not in
 * this file, it does not belong in a component.
 */

export const color = {
  // Meta palette
  accent: '#0064E0',
  accentHover: '#0052BA',
  accentBright: '#0082FB',
  purple: '#A033FF',
  purpleInk: '#8B24E0',
  orange: '#FF5C01',
  orangeInk: '#C24A00',
  yellow: '#FFC800',
  yellowInk: '#B26A00',
  green: '#31A24C',
  greenInk: '#1D8A3C',
  red: '#FA383E',

  // Neutrals
  ink: '#1C2B33',
  inkProse: '#22323B',
  ink2: '#465A69',
  ink3: '#7B8B98',
  ink4: '#93A2AE',
  ink5: '#A3B0BB',
  line: '#DCE3E9',
  lineSoft: '#EDF1F4',
  lineFaint: '#E4EAEF',
  rail: '#F5F7F9',
  railSoft: '#F7F9FB',
  surface: '#FFFFFF',
  surfaceSoft: '#FBFCFD',
  disabled: '#C3D0DA',

  // Semantic tints — blue = internal, orange = web, purple = AI, yellow = continuity
  blueTint: '#E7F0FD',
  blueMark: '#C9DEFB',
  blueDeep: '#0052BA',
  orangeTint: '#FFEDE0',
  orangeMark: '#FFDCC2',
  purpleTint: '#F3E8FF',
  purpleMark: '#EBD8FF',
  purpleDeep: '#6B12A8',
  amberTint: '#FFFAF0',
  amberMark: '#FFF1D6',
  amberLine: '#F2D9A4',
  amberInk: '#7A5200',
  amberInk2: '#A05A1E',
  greenTint: '#E4F5E9',
  infoTint: '#F0F6FF',
  infoLine: '#CFE1FA',
  infoInk: '#1F4F8F',
} as const

export const font = {
  ui: "'Hanken Grotesk', system-ui, -apple-system, sans-serif",
  prose: 'Literata, Georgia, serif',
  mono: "'IBM Plex Mono', ui-monospace, Menlo, monospace",
} as const

export const radius = {
  sm: 5,
  md: 7,
  lg: 9,
  xl: 11,
  pill: 999,
} as const

/** Fixed chrome heights, matching the artboards exactly. */
export const chrome = {
  titleBar: 40,
  toolBar: 44,
  statusBar: 28,
  railHeader: 36,
  binderWidth: 254,
  inspectorWidth: 322,
  /** macOS traffic lights are drawn by the OS over our title bar. */
  trafficLightInset: 78,
} as const

export const sectionLabel = {
  fontSize: 10.5,
  fontWeight: 700,
  letterSpacing: '0.09em',
  color: color.ink5,
} as const

/** rgba() tint of a hex colour, for selection backgrounds. */
export function tint(hex: string, alpha: number): string {
  let h = hex.replace('#', '')
  if (h.length === 3) h = h[0] + h[0] + h[1] + h[1] + h[2] + h[2]
  const n = Number.parseInt(h, 16)
  if (Number.isNaN(n)) return `rgba(0,100,224,${alpha})`
  return `rgba(${(n >> 16) & 255},${(n >> 8) & 255},${n & 255},${alpha})`
}

/** Label colours used by the corkboard, outliner and inspector. */
export const labels = {
  Plot: { dot: color.accent, fg: color.blueDeep, bg: color.blueTint },
  Character: { dot: color.purple, fg: color.purpleInk, bg: color.purpleTint },
  Setup: { dot: color.orange, fg: color.orangeInk, bg: color.orangeTint },
  None: { dot: color.line, fg: color.ink3, bg: color.lineSoft },
} as const

export type LabelName = keyof typeof labels

export const statuses = {
  Done: { fg: color.greenInk, bg: color.greenTint },
  Draft: { fg: color.yellowInk, bg: color.amberMark },
  Empty: { fg: color.ink3, bg: color.lineSoft },
} as const

export type StatusName = keyof typeof statuses
