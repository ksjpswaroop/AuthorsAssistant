/**
 * Chart parameters for the visualization screens.
 *
 * The categorical ramp is NOT the brand palette used for UI meaning — those
 * colours are reserved (blue = internal, orange = web, purple = AI, yellow =
 * continuity) and reusing them in a chart would say something untrue. This is a
 * separate ramp, stepped from the same families and checked with the palette
 * validator: lightness band, chroma floor, CVD separation, normal-vision floor
 * and contrast all pass against a light surface.
 *
 * Dark mode is deliberately absent. A dark chart surface needs its own steps
 * re-validated, not an automatic flip of these, and the app is light-only today.
 */

/** Fixed order. Assign by entity, never cycle, never reorder on filter. */
export const CATEGORICAL = ['#0064E0', '#E8701F', '#7A1FB8', '#1A9AA6', '#C2185B'] as const

/** Single hue, light to dark, for magnitude. */
export const SEQUENTIAL = ['#E7F0FD', '#B9D4F8', '#7FB0F0', '#3D82E4', '#0064E0', '#0A47A0'] as const

/** Recessive chart furniture. */
export const AXIS = '#DCE3E9'
export const GRID = '#EDF1F4'
export const LABEL = '#7B8B98'
export const LABEL_STRONG = '#465A69'

/** Assign a stable colour to an entity by its index in a fixed list. */
export function hueFor(index: number): string {
  return CATEGORICAL[index % CATEGORICAL.length]
}

/** Maps a value in [min,max] to an x position inside a plot box. */
export function scaleX(v: number, min: number, max: number, x0: number, x1: number): number {
  if (max === min) return x0
  return x0 + ((v - min) / (max - min)) * (x1 - x0)
}

export function scaleY(v: number, min: number, max: number, y0: number, y1: number): number {
  if (max === min) return y1
  return y1 - ((v - min) / (max - min)) * (y1 - y0)
}

/** Smooth path through points, for tension and pacing curves. */
export function linePath(points: { x: number; y: number }[]): string {
  if (points.length === 0) return ''
  if (points.length < 3) return points.map((p, i) => `${i ? 'L' : 'M'}${p.x} ${p.y}`).join(' ')
  let d = `M${points[0].x} ${points[0].y}`
  for (let i = 0; i < points.length - 1; i++) {
    const p = points[i]
    const n = points[i + 1]
    const cx = (p.x + n.x) / 2
    d += ` C${cx} ${p.y}, ${cx} ${n.y}, ${n.x} ${n.y}`
  }
  return d
}
