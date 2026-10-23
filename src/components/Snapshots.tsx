import { useEffect, useState } from 'react'
import { useApp } from '../lib/store'
import { Button, Callout, SectionLabel } from './ui'
import { Icon } from './Icon'
import { color, font, radius, tint } from '../lib/tokens'
import { relativeTime } from '../lib/format'

/**
 * Snapshot list for the open document (Phase 2). Each entry is a real Git
 * commit inside the project folder, so this history survives the app.
 */
export function Snapshots() {
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const snapshots = useApp((s) => s.snapshots)
  const busy = useApp((s) => s.snapshotBusy)
  const loadSnapshots = useApp((s) => s.loadSnapshots)
  const takeSnapshot = useApp((s) => s.takeSnapshot)
  const restoreSnapshot = useApp((s) => s.restoreSnapshot)
  const setDialog = useApp((s) => s.setDialog)
  const [confirming, setConfirming] = useState<string | null>(null)

  const docPath = doc?.path ?? null

  useEffect(() => {
    void loadSnapshots(docPath)
  }, [docPath, loadSnapshots])

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 9 }}>
        <SectionLabel>SNAPSHOTS</SectionLabel>
        <div style={{ flexGrow: 1 }} />
        <button
          onClick={() => setDialog('snapshot')}
          disabled={snapshots.length === 0}
          style={{
            fontSize: 12,
            fontWeight: 600,
            color: snapshots.length ? color.accent : color.disabled,
            cursor: snapshots.length ? 'pointer' : 'not-allowed',
          }}
        >
          Compare…
        </button>
      </div>

      <Button
        icon="clock"
        height={31}
        variant="ghost"
        disabled={busy}
        onClick={() => void takeSnapshot(doc ? `Snapshot before editing ${doc.title}` : 'Snapshot')}
        style={{ width: '100%', justifyContent: 'center' }}
      >
        {busy ? 'Saving…' : 'Take snapshot'}
      </Button>

      {snapshots.length === 0 ? (
        <div style={{ fontSize: 12.5, lineHeight: 1.6, color: color.ink4, marginTop: 11 }}>
          No snapshots of this document yet. A snapshot commits the whole project, so one is enough to get back to today.
        </div>
      ) : (
        <div style={{ marginTop: 11, display: 'flex', flexDirection: 'column', gap: 3 }}>
          {snapshots.slice(0, 8).map((s) => (
            <div
              key={s.id}
              style={{
                border: `1px solid ${confirming === s.id ? color.yellow : color.lineSoft}`,
                background: confirming === s.id ? color.amberTint : 'transparent',
                borderRadius: radius.lg,
                padding: '9px 11px',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <Icon name="clock" size={13} strokeWidth={1.9} style={{ color: color.ink4 }} />
                <span style={{ fontSize: 12.5, color: color.ink2, flexGrow: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                  {s.message}
                </span>
                <span style={{ fontSize: 11.5, color: color.ink5, flexShrink: 0 }}>{relativeTime(s.at)}</span>
              </div>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 6 }}>
                <span style={{ fontFamily: font.mono, fontSize: 10.5, color: color.ink5 }}>{s.id.slice(0, 7)}</span>
                <div style={{ flexGrow: 1 }} />
                {confirming === s.id ? (
                  <>
                    <button
                      onClick={() => setConfirming(null)}
                      style={{ fontSize: 11.5, fontWeight: 600, color: color.ink3, cursor: 'pointer' }}
                    >
                      Cancel
                    </button>
                    <button
                      onClick={() => {
                        if (docPath) void restoreSnapshot(s.id, docPath)
                        setConfirming(null)
                      }}
                      style={{ fontSize: 11.5, fontWeight: 700, color: color.yellowInk, cursor: 'pointer' }}
                    >
                      Yes, restore
                    </button>
                  </>
                ) : (
                  <button
                    onClick={() => setConfirming(s.id)}
                    disabled={!docPath}
                    style={{
                      fontSize: 11.5,
                      fontWeight: 600,
                      color: docPath ? color.accent : color.disabled,
                      cursor: docPath ? 'pointer' : 'not-allowed',
                    }}
                  >
                    Restore this version
                  </button>
                )}
              </div>
              {confirming === s.id && (
                <div style={{ fontSize: 11.5, lineHeight: 1.55, color: color.amberInk, marginTop: 7 }}>
                  This replaces the current text. Anything unsaved is snapshotted first, so you can come back.
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      {snapshots.length > 8 && (
        <div style={{ fontSize: 11.5, color: color.ink5, marginTop: 8 }}>
          {snapshots.length - 8} older snapshots not shown.
        </div>
      )}

      <Callout tone="quiet" style={{ marginTop: 12, background: tint(color.accent, 0.04) }}>
        Snapshots are Git commits in the project folder. Any Git client can read this history if AegisDraft is not
        around.
      </Callout>
    </div>
  )
}
