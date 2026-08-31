import { useEffect } from 'react'
import { useApp } from '../lib/store'
import { AppMark, Icon } from '../components/Icon'
import { Button } from '../components/ui'
import { color, font, radius, tint } from '../lib/tokens'
import { relativeTime, tildePath, words } from '../lib/format'
import { isTauri } from '../lib/api'

const KIND_TONE: Record<string, { bg: string; fg: string }> = {
  novel: { bg: color.blueTint, fg: color.accent },
  nonfiction: { bg: color.orangeTint, fg: color.orangeInk },
  academic: { bg: color.purpleTint, fg: color.purpleInk },
  default: { bg: color.lineSoft, fg: color.ink4 },
}

export function Welcome() {
  const recents = useApp((s) => s.recents)
  const loadRecents = useApp((s) => s.loadRecents)
  const openProjectAt = useApp((s) => s.openProjectAt)
  const openProjectViaDialog = useApp((s) => s.openProjectViaDialog)
  const newProject = useApp((s) => s.newProject)
  const busy = useApp((s) => s.busy)

  useEffect(() => {
    void loadRecents()
  }, [loadRecents])

  return (
    <div className="drag-region" style={{ display: 'flex', height: '100%', background: color.surface }}>
      {/* Brand rail */}
      <div
        style={{
          width: 344,
          flexShrink: 0,
          display: 'flex',
          flexDirection: 'column',
          padding: '34px 32px',
          background: color.ink,
          color: '#FFFFFF',
          position: 'relative',
          overflow: 'hidden',
        }}
      >
        <div
          style={{
            position: 'absolute',
            top: -120,
            right: -140,
            width: 340,
            height: 340,
            borderRadius: '50%',
            background:
              'radial-gradient(circle, rgba(0,130,251,0.42) 0%, rgba(160,51,255,0.16) 48%, rgba(28,43,51,0) 72%)',
          }}
        />
        <div style={{ position: 'relative', display: 'flex', alignItems: 'center', gap: 11 }}>
          <AppMark size={30} radius={9} />
          <span style={{ fontSize: 17, fontWeight: 700, letterSpacing: '-0.015em' }}>AegisDraft</span>
        </div>

        <div style={{ position: 'relative', marginTop: 46 }}>
          <div style={{ fontFamily: font.prose, fontSize: 27, lineHeight: 1.32, letterSpacing: '-0.012em' }}>
            A book studio that keeps the book on your machine.
          </div>
          <div style={{ marginTop: 16, fontSize: 13.5, lineHeight: 1.68, color: '#A9BAC6' }}>
            Binder, corkboard, outliner and a Markdown editor. Everything you write stays a readable file in a folder
            you chose.
          </div>
        </div>

        <div style={{ flexGrow: 1 }} />

        <div style={{ position: 'relative', display: 'flex', flexDirection: 'column', gap: 11 }}>
          {[
            { icon: 'doc' as const, text: 'Scenes are Markdown files on disk' },
            { icon: 'database' as const, text: 'Structure lives in a SQLite file beside them' },
            { icon: 'offline' as const, text: 'No account, no upload, no network' },
          ].map((r) => (
            <div key={r.text} style={{ display: 'flex', alignItems: 'center', gap: 10, fontSize: 12.5, color: '#C6D3DC' }}>
              <span style={{ color: color.green, display: 'flex' }}>
                <Icon name={r.icon} size={15} strokeWidth={1.9} />
              </span>
              {r.text}
            </div>
          ))}
        </div>
      </div>

      {/* Recents */}
      <div className="no-drag" style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '26px 30px 0 30px' }}>
          <span style={{ fontSize: 10.5, fontWeight: 700, letterSpacing: '0.09em', color: color.ink5 }}>
            RECENT PROJECTS
          </span>
          <div style={{ flexGrow: 1 }} />
          {!isTauri && (
            <span style={{ fontSize: 12, color: color.ink4 }}>
              Browser preview — sample project, nothing is written to disk
            </span>
          )}
        </div>

        <div
          style={{
            flexGrow: 1,
            overflowY: 'auto',
            padding: '12px 22px 8px 22px',
            display: 'flex',
            flexDirection: 'column',
            gap: 4,
          }}
        >
          {recents.length === 0 && (
            <div style={{ padding: '30px 12px', fontSize: 13, lineHeight: 1.7, color: color.ink4 }}>
              No projects yet. Create one and AegisDraft will make a <code>.aegis</code> folder holding a SQLite file
              and your scenes as Markdown.
            </div>
          )}

          {recents.map((r) => {
            const tone = KIND_TONE[r.kind] ?? KIND_TONE.default
            return (
              <button
                key={r.path}
                className="row"
                onClick={() => r.exists && void openProjectAt(r.path)}
                disabled={!r.exists}
                title={r.exists ? r.path : 'This folder has moved or been deleted'}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 13,
                  padding: '11px 12px',
                  borderRadius: radius.lg + 1,
                  cursor: r.exists ? 'pointer' : 'not-allowed',
                  opacity: r.exists ? 1 : 0.55,
                  background: 'transparent',
                }}
              >
                <div
                  style={{
                    width: 36,
                    height: 36,
                    borderRadius: radius.lg,
                    flexShrink: 0,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    background: tone.bg,
                    color: tone.fg,
                  }}
                >
                  <Icon name="book" size={18} />
                </div>
                <div style={{ minWidth: 0, flexGrow: 1 }}>
                  <div style={{ fontSize: 14, fontWeight: 600, color: color.ink }}>{r.name}</div>
                  <div
                    style={{
                      fontSize: 12,
                      color: color.ink4,
                      marginTop: 2,
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {r.kind} · {words(r.wordCount)} words · {tildePath(r.path)}
                  </div>
                </div>
                <div style={{ fontSize: 12, color: color.ink5, flexShrink: 0 }}>
                  {r.exists ? relativeTime(r.openedAt) : 'missing'}
                </div>
              </button>
            )
          })}
        </div>

        <div style={{ padding: '12px 30px 0 30px', display: 'flex', gap: 10 }}>
          <Button icon="plus" height={38} onClick={() => void newProject()} disabled={busy}>
            New project
          </Button>
          <Button icon="folder" height={38} variant="ghost" onClick={() => void openProjectViaDialog()} disabled={busy}>
            Open folder…
          </Button>
        </div>

        <div
          style={{
            margin: '20px 22px 22px 22px',
            padding: '13px 15px',
            background: color.rail,
            border: `1px solid ${color.lineFaint}`,
            borderRadius: radius.xl,
            display: 'flex',
            alignItems: 'center',
            gap: 12,
          }}
        >
          <span
            style={{
              width: 8,
              height: 8,
              borderRadius: '50%',
              background: color.disabled,
              flexShrink: 0,
            }}
          />
          <div style={{ minWidth: 0, flexGrow: 1 }}>
            <div style={{ fontSize: 13, fontWeight: 600, color: color.ink }}>Local models not connected</div>
            <div style={{ fontSize: 12, color: color.ink3, marginTop: 2 }}>
              Phase 1 is the writing studio. Ollama, RAG and the agents arrive in Phase 3 and 5.
            </div>
          </div>
          <span
            style={{
              fontSize: 11,
              fontWeight: 600,
              color: color.purpleInk,
              background: tint(color.purple, 0.1),
              borderRadius: radius.sm + 1,
              padding: '3px 8px',
              flexShrink: 0,
            }}
          >
            Phase 3
          </span>
        </div>
      </div>
    </div>
  )
}
