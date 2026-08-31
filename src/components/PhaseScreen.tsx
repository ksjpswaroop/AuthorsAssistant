import type { ReactNode } from 'react'
import { useApp } from '../lib/store'
import { TitleBar, StatusBar, OllamaStatus, OfflineStatus, Divider } from './Chrome'
import { Chip, IconButton, PhaseBanner } from './ui'
import { color } from '../lib/tokens'

/**
 * Chrome shared by the screens whose design is finished but whose behaviour
 * belongs to a later phase. The banner is not decoration: it is the difference
 * between a prototype and a lie.
 */
export function PhaseScreen({
  title,
  badge,
  phase,
  explains,
  right,
  statusLeft,
  statusRight,
  children,
}: {
  title: string
  badge?: { text: string; fg: string; bg: string }
  phase: string
  explains: string
  right?: ReactNode
  statusLeft?: ReactNode
  statusRight?: ReactNode
  children: ReactNode
}) {
  const go = useApp((s) => s.go)
  const project = useApp((s) => s.project)

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: color.surface }}>
      <TitleBar
        title={project?.name ?? 'AegisDraft'}
        subtitle={title}
        badge={badge ? <Chip fg={badge.fg} bg={badge.bg}>{badge.text}</Chip> : undefined}
        right={
          <>
            {right}
            {right ? <Divider /> : null}
            <IconButton icon="close" title="Back to the studio" onClick={() => go('studio')} />
          </>
        }
      />
      <PhaseBanner phase={phase}>{explains}</PhaseBanner>
      <div style={{ flexGrow: 1, display: 'flex', minHeight: 0 }}>{children}</div>
      <StatusBar
        left={
          statusLeft ?? (
            <>
              <OllamaStatus />
              <OfflineStatus />
            </>
          )
        }
        right={statusRight}
      />
    </div>
  )
}

/** A left rail with a heading, used by several of the later-phase screens. */
export function Rail({ width = 258, children }: { width?: number; children: ReactNode }) {
  return (
    <div
      style={{
        width,
        flexShrink: 0,
        display: 'flex',
        flexDirection: 'column',
        background: color.rail,
        borderRight: `1px solid ${color.line}`,
        overflowY: 'auto',
      }}
    >
      {children}
    </div>
  )
}

export function SidePanel({ width = 340, children }: { width?: number; children: ReactNode }) {
  return (
    <div
      style={{
        width,
        flexShrink: 0,
        display: 'flex',
        flexDirection: 'column',
        background: color.surface,
        borderLeft: `1px solid ${color.line}`,
        minHeight: 0,
      }}
    >
      {children}
    </div>
  )
}

/** Sample data that is not derived from the author's project is labelled. */
export function SampleNote({ children }: { children: ReactNode }) {
  return (
    <div style={{ fontSize: 11.5, lineHeight: 1.55, color: color.ink5, marginTop: 14 }}>
      {children}
    </div>
  )
}
