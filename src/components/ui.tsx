import type { CSSProperties, ReactNode } from 'react'
import { Icon, type IconName } from './Icon'
import { color, font, radius, sectionLabel, tint } from '../lib/tokens'

/* -------------------------------------------------------------------------- */
/* Buttons                                                                     */
/* -------------------------------------------------------------------------- */

type ButtonProps = {
  children: ReactNode
  onClick?: () => void
  icon?: IconName
  variant?: 'primary' | 'ghost' | 'quiet'
  height?: number
  tone?: string
  disabled?: boolean
  title?: string
  style?: CSSProperties
}

export function Button({
  children,
  onClick,
  icon,
  variant = 'primary',
  height = 34,
  tone = color.accent,
  disabled,
  title,
  style,
}: ButtonProps) {
  const base: CSSProperties = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 7,
    height,
    padding: `0 ${height >= 34 ? 15 : 12}px`,
    borderRadius: radius.lg,
    fontSize: height >= 34 ? 13 : 12.5,
    fontWeight: 600,
    whiteSpace: 'nowrap',
    cursor: disabled ? 'not-allowed' : 'pointer',
    ...(variant === 'primary'
      ? { background: tone, color: '#FFFFFF' }
      : variant === 'ghost'
        ? { border: `1px solid ${color.line}`, color: color.ink2 }
        : { color: color.ink2 }),
    ...style,
  }
  const cls = [variant === 'primary' ? 'btn' : 'ghost', disabled ? 'disabled' : ''].join(' ')
  return (
    <button className={cls} style={base} onClick={disabled ? undefined : onClick} title={title} disabled={disabled}>
      {icon && <Icon name={icon} size={height >= 34 ? 15 : 14} strokeWidth={2} />}
      {children}
    </button>
  )
}

export function IconButton({
  icon,
  onClick,
  title,
  active,
  size = 28,
  tone = color.accent,
}: {
  icon: IconName
  onClick?: () => void
  title?: string
  active?: boolean
  size?: number
  tone?: string
}) {
  return (
    <button
      className="ghost"
      title={title}
      aria-label={title}
      onClick={onClick}
      style={{
        width: size,
        height: size,
        borderRadius: radius.md,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        cursor: 'pointer',
        color: active ? tone : color.ink4,
      }}
    >
      <Icon name={icon} size={size >= 28 ? 16 : 15} />
    </button>
  )
}

/* -------------------------------------------------------------------------- */
/* Text bits                                                                   */
/* -------------------------------------------------------------------------- */

export function SectionLabel({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return <div style={{ ...sectionLabel, ...style }}>{children}</div>
}

export function Chip({
  children,
  fg = color.ink2,
  bg = color.lineSoft,
  bold = true,
}: {
  children: ReactNode
  fg?: string
  bg?: string
  bold?: boolean
}) {
  return (
    <span
      style={{
        fontSize: 11,
        fontWeight: bold ? 600 : 500,
        color: fg,
        background: bg,
        borderRadius: radius.sm + 1,
        padding: '3px 8px',
        whiteSpace: 'nowrap',
      }}
    >
      {children}
    </span>
  )
}

export function Dot({ tone, size = 7 }: { tone: string; size?: number }) {
  return <span style={{ width: size, height: size, borderRadius: '50%', background: tone, flexShrink: 0 }} />
}

export function Mono({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return <span style={{ fontFamily: font.mono, fontSize: 11.5, ...style }}>{children}</span>
}

export function Kbd({ children }: { children: ReactNode }) {
  return (
    <span
      style={{
        fontFamily: font.mono,
        fontSize: 11,
        color: color.ink4,
        border: `1px solid ${color.line}`,
        borderRadius: 4,
        padding: '1px 4px',
      }}
    >
      {children}
    </span>
  )
}

/* -------------------------------------------------------------------------- */
/* Controls                                                                    */
/* -------------------------------------------------------------------------- */

export function Segmented<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T
  options: { id: T; label: string; icon?: IconName }[]
  onChange: (id: T) => void
}) {
  return (
    <div style={{ display: 'flex', gap: 2, padding: 3, background: color.lineFaint, borderRadius: radius.lg }}>
      {options.map((o) => {
        const on = o.id === value
        return (
          <button
            key={o.id}
            className="seg no-drag"
            onClick={() => onChange(o.id)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              height: 26,
              padding: '0 13px',
              borderRadius: radius.md - 1,
              fontSize: 12.5,
              fontWeight: 600,
              cursor: 'pointer',
              background: on ? color.surface : 'transparent',
              color: on ? color.accent : color.ink3,
              boxShadow: on ? '0 1px 2px rgba(28,43,51,0.12)' : 'none',
            }}
          >
            {o.icon && <Icon name={o.icon} size={13} strokeWidth={2} />}
            {o.label}
          </button>
        )
      })}
    </div>
  )
}

export function Tabs<T extends string>({
  value,
  options,
  onChange,
}: {
  value: T
  options: { id: T; label: string; badge?: number | string }[]
  onChange: (id: T) => void
}) {
  return (
    <div
      style={{
        height: 44,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'stretch',
        gap: 2,
        padding: '0 12px',
        borderBottom: `1px solid ${color.lineSoft}`,
      }}
    >
      {options.map((o) => {
        const on = o.id === value
        return (
          <button
            key={o.id}
            className="seg"
            onClick={() => onChange(o.id)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '0 11px',
              fontSize: 12.5,
              fontWeight: 600,
              cursor: 'pointer',
              color: on ? color.accent : color.ink3,
              boxShadow: on ? `inset 0 -2px 0 0 ${color.accent}` : 'none',
            }}
          >
            {o.label}
            {o.badge != null && (
              <span
                style={{
                  fontSize: 10,
                  fontWeight: 700,
                  color: '#FFFFFF',
                  background: color.orange,
                  borderRadius: 9,
                  padding: '1px 5px',
                }}
              >
                {o.badge}
              </span>
            )}
          </button>
        )
      })}
    </div>
  )
}

export function Toggle({ on, onChange, tone = color.accent }: { on: boolean; onChange: () => void; tone?: string }) {
  return (
    <button
      role="switch"
      aria-checked={on}
      onClick={onChange}
      style={{
        width: 40,
        height: 23,
        borderRadius: 13,
        flexShrink: 0,
        cursor: 'pointer',
        position: 'relative',
        background: on ? tone : '#D3DCE3',
        transition: 'background 120ms ease',
      }}
    >
      <span
        style={{
          position: 'absolute',
          top: 2.5,
          left: on ? 19 : 2,
          width: 18,
          height: 18,
          borderRadius: '50%',
          background: '#FFFFFF',
          boxShadow: '0 1px 3px rgba(28,43,51,0.28)',
          transition: 'left 120ms ease',
        }}
      />
    </button>
  )
}

export function Checkbox({ on, tone = color.accent }: { on: boolean; tone?: string }) {
  return (
    <span
      style={{
        width: 17,
        height: 17,
        borderRadius: radius.sm,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: on ? tone : '#FFFFFF',
        border: `1.5px solid ${on ? tone : color.disabled}`,
        color: '#FFFFFF',
      }}
    >
      {on && <Icon name="check" size={11} strokeWidth={3.2} />}
    </span>
  )
}

/** A select-looking control. Phase 1 has no menus yet, so it takes a click handler. */
export function SelectLook({
  children,
  onClick,
  height = 33,
  style,
}: {
  children: ReactNode
  onClick?: () => void
  height?: number
  style?: CSSProperties
}) {
  return (
    <button
      onClick={onClick}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        width: '100%',
        height,
        padding: '0 11px',
        border: `1px solid ${color.line}`,
        borderRadius: radius.lg - 1,
        fontSize: 13,
        color: color.ink,
        cursor: 'pointer',
        background: color.surface,
        ...style,
      }}
    >
      <span style={{ flexGrow: 1, textAlign: 'left', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
        {children}
      </span>
      <Icon name="chevronDown" size={13} strokeWidth={2.4} style={{ color: color.ink4 }} />
    </button>
  )
}

export function ProgressBar({ pct, tone = color.accent, height = 7 }: { pct: number; tone?: string; height?: number }) {
  return (
    <div style={{ height, borderRadius: height / 2 + 0.5, background: color.lineSoft, overflow: 'hidden' }}>
      <div
        style={{
          height,
          width: `${Math.max(0, Math.min(100, pct))}%`,
          borderRadius: height / 2 + 0.5,
          background: tone,
          transition: 'width 160ms ease',
        }}
      />
    </div>
  )
}

/* -------------------------------------------------------------------------- */
/* Blocks                                                                      */
/* -------------------------------------------------------------------------- */

const CALLOUT_TONES = {
  info: { bg: color.infoTint, border: color.infoLine, fg: color.infoInk, icon: 'info' as IconName, mark: color.accent },
  warn: { bg: color.amberTint, border: color.amberLine, fg: color.amberInk, icon: 'alert' as IconName, mark: color.yellowInk },
  good: { bg: '#EEF9F1', border: '#C9E9D3', fg: '#14502A', icon: 'check' as IconName, mark: color.greenInk },
  quiet: { bg: color.railSoft, border: color.lineFaint, fg: color.ink2, icon: 'info' as IconName, mark: color.ink3 },
}

export function Callout({
  tone = 'info',
  icon,
  children,
  action,
  style,
}: {
  tone?: keyof typeof CALLOUT_TONES
  icon?: IconName
  children: ReactNode
  action?: ReactNode
  style?: CSSProperties
}) {
  const t = CALLOUT_TONES[tone]
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 11,
        padding: '11px 13px',
        background: t.bg,
        border: `1px solid ${t.border}`,
        borderRadius: radius.lg + 1,
        ...style,
      }}
    >
      <span style={{ color: t.mark, display: 'flex' }}>
        <Icon name={icon ?? t.icon} size={16} strokeWidth={2} />
      </span>
      <div style={{ flexGrow: 1, fontSize: 12.5, lineHeight: 1.55, color: t.fg }}>{children}</div>
      {action}
    </div>
  )
}

export function Card({ children, style }: { children: ReactNode; style?: CSSProperties }) {
  return (
    <div
      className="card"
      style={{
        border: `1px solid ${color.line}`,
        borderRadius: radius.xl,
        background: color.surface,
        ...style,
      }}
    >
      {children}
    </div>
  )
}

export function EmptyState({
  icon = 'doc',
  title,
  body,
  action,
}: {
  icon?: IconName
  title: string
  body?: string
  action?: ReactNode
}) {
  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 10,
        height: '100%',
        padding: 40,
        textAlign: 'center',
      }}
    >
      <span style={{ color: color.disabled }}>
        <Icon name={icon} size={30} strokeWidth={1.5} />
      </span>
      <div style={{ fontSize: 14.5, fontWeight: 600, color: color.ink2 }}>{title}</div>
      {body && <div style={{ fontSize: 12.5, lineHeight: 1.6, color: color.ink4, maxWidth: 340 }}>{body}</div>}
      {action}
    </div>
  )
}

/**
 * Marks a screen whose visual design is finished but whose behaviour lands in a
 * later phase. Being explicit beats a demo that quietly pretends to work.
 */
export function PhaseBanner({ phase, children }: { phase: string; children: ReactNode }) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '8px 22px',
        background: tint(color.purple, 0.07),
        borderBottom: `1px solid ${color.line}`,
        flexShrink: 0,
      }}
    >
      <Chip fg={color.purpleDeep} bg={color.purpleTint}>
        {phase}
      </Chip>
      <span style={{ fontSize: 12.5, color: color.ink2 }}>{children}</span>
    </div>
  )
}
