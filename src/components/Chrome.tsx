import type { ReactNode } from 'react'
import { AppMark, Icon } from './Icon'
import { Dot } from './ui'
import { chrome, color, font } from '../lib/tokens'
import { useAsync } from '../lib/useAsync'
import { ai, localModels } from '../lib/phases'
import { isTauri } from '../lib/api'

/** macOS draws its traffic lights over our title bar; leave room for them. */
const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.userAgent)

export function TitleBar({
  title,
  subtitle,
  badge,
  center,
  right,
}: {
  title: string
  subtitle?: ReactNode
  badge?: ReactNode
  center?: ReactNode
  right?: ReactNode
}) {
  return (
    <div
      className="drag-region"
      style={{
        height: chrome.titleBar,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        gap: 14,
        padding: `0 12px 0 ${isMac ? chrome.trafficLightInset : 12}px`,
        background: color.rail,
        borderBottom: `1px solid ${color.line}`,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 9, minWidth: 0 }}>
        <AppMark />
        <span
          style={{
            fontSize: 13,
            fontWeight: 600,
            letterSpacing: '-0.005em',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
          }}
        >
          {title}
        </span>
        {badge}
        {subtitle && <span style={{ fontSize: 12, color: color.ink4, whiteSpace: 'nowrap' }}>{subtitle}</span>}
      </div>

      <div style={{ flexGrow: 1 }} />
      {center && <div className="no-drag">{center}</div>}
      <div style={{ flexGrow: 1 }} />

      <div className="no-drag" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        {right}
      </div>
    </div>
  )
}

export function Divider() {
  return <div style={{ width: 1, height: 18, background: color.line, flexShrink: 0 }} />
}

export function StatusBar({ left, right }: { left?: ReactNode; right?: ReactNode }) {
  return (
    <div
      style={{
        height: chrome.statusBar,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: '0 14px',
        background: color.rail,
        borderTop: `1px solid ${color.line}`,
        fontSize: 11.5,
        color: color.ink3,
      }}
    >
      {left}
      <div style={{ flexGrow: 1 }} />
      {right}
    </div>
  )
}

export function StatusItem({ children, tone }: { children: ReactNode; tone?: string }) {
  return (
    <span style={{ display: 'flex', alignItems: 'center', gap: 6, color: tone, whiteSpace: 'nowrap' }}>{children}</span>
  )
}

/**
 * Live Ollama state. This was hardcoded to "not connected" while Phase 3 was
 * unbuilt; leaving it that way once the client existed made the status bar lie
 * about a running daemon, which is exactly the failure the honesty rules exist
 * to prevent.
 */
export function OllamaStatus() {
  const status = useAsync(() => ai.status(), [], isTauri)
  const models = status.data ? localModels(status.data) : []

  if (!isTauri) {
    return (
      <StatusItem>
        <Dot tone={color.disabled} />
        <span>Browser preview — no local models</span>
      </StatusItem>
    )
  }
  if (status.loading) {
    return (
      <StatusItem>
        <Dot tone={color.disabled} />
        <span>Checking for Ollama…</span>
      </StatusItem>
    )
  }
  if (!status.data?.reachable) {
    return (
      <StatusItem>
        <Dot tone={color.disabled} />
        <span>Ollama not running</span>
      </StatusItem>
    )
  }
  return (
    <StatusItem>
      <Dot tone={models.length ? color.green : color.yellow} />
      <span>
        Ollama · {models.length ? `${models.length} local model${models.length === 1 ? '' : 's'}` : 'no local models'}
      </span>
    </StatusItem>
  )
}

export function OfflineStatus() {
  return (
    <StatusItem>
      <Icon name="offline" size={12} strokeWidth={1.9} />
      Offline — no network calls
    </StatusItem>
  )
}

export function MonoStatus({ children }: { children: ReactNode }) {
  return <span style={{ fontFamily: font.mono, whiteSpace: 'nowrap' }}>{children}</span>
}
