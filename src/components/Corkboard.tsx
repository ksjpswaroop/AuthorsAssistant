import { useApp } from '../lib/store'
import type { DocumentRow } from '../lib/types'
import { color, labels, radius, statuses, type LabelName, type StatusName } from '../lib/tokens'
import { EmptyState } from './ui'
import { words } from '../lib/format'

function statusOf(doc: DocumentRow): StatusName {
  if (doc.status && doc.status in statuses) return doc.status as StatusName
  return doc.wordCount > 0 ? 'Draft' : 'Empty'
}

function labelOf(doc: DocumentRow): LabelName {
  return doc.label && doc.label in labels ? (doc.label as LabelName) : 'None'
}

export function Corkboard() {
  const docs = useApp((s) => s.docs)
  const selectedId = useApp((s) => s.selectedId)
  const select = useApp((s) => s.select)
  const setView = useApp((s) => s.setView)

  const cards = docs.filter((d) => d.kind !== 'folder').sort((a, b) => a.position - b.position)

  if (cards.length === 0) {
    return <EmptyState icon="grid" title="No index cards yet" body="Every document that holds text gets a card here." />
  }

  return (
    <div style={{ flexGrow: 1, overflowY: 'auto', background: color.rail }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '14px 26px 4px 26px' }}>
        <span style={{ fontSize: 12, color: color.ink3 }}>{cards.length} cards</span>
        <div style={{ flexGrow: 1 }} />
        {(['Plot', 'Character', 'Setup'] as LabelName[]).map((l) => (
          <span key={l} style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12, color: color.ink3 }}>
            <span style={{ width: 9, height: 9, borderRadius: 3, background: labels[l].dot }} />
            {l}
          </span>
        ))}
      </div>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))',
          gap: 18,
          padding: '16px 26px 40px 26px',
        }}
      >
        {cards.map((doc) => {
          const on = doc.id === selectedId
          const status = statusOf(doc)
          const label = labelOf(doc)
          const empty = doc.wordCount === 0
          return (
            <button
              key={doc.id}
              className="card"
              onClick={() => void select(doc.id)}
              onDoubleClick={() => {
                void select(doc.id)
                setView('editor')
              }}
              title="Double-click to open in the editor"
              style={{
                background: empty ? color.surfaceSoft : color.surface,
                border: on ? `2px solid ${color.accent}` : `1px ${empty ? 'dashed' : 'solid'} ${empty ? color.disabled : color.line}`,
                borderRadius: radius.xl,
                overflow: 'hidden',
                cursor: 'pointer',
                boxShadow: on ? '0 2px 10px rgba(0,100,224,0.13)' : '0 1px 2px rgba(28,43,51,0.05)',
                textAlign: 'left',
                padding: 0,
              }}
            >
              <div style={{ height: 4, background: labels[label].dot }} />
              <div style={{ padding: on ? '13px 14px 11px 14px' : '14px 15px 12px 15px' }}>
                <div
                  style={{
                    fontSize: 13.5,
                    fontWeight: 600,
                    marginBottom: 8,
                    color: empty ? color.ink5 : color.ink,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {doc.title}
                </div>
                <div
                  style={{
                    fontSize: 12.5,
                    lineHeight: 1.6,
                    color: doc.synopsis ? color.ink2 : color.disabled,
                    height: 82,
                    overflow: 'hidden',
                  }}
                >
                  {doc.synopsis ?? 'No synopsis yet.'}
                </div>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    marginTop: 12,
                    paddingTop: 11,
                    borderTop: `1px solid ${color.lineSoft}`,
                  }}
                >
                  <span
                    style={{
                      fontSize: 10.5,
                      fontWeight: 700,
                      letterSpacing: '0.05em',
                      color: statuses[status].fg,
                      background: statuses[status].bg,
                      borderRadius: radius.sm,
                      padding: '3px 7px',
                    }}
                  >
                    {status.toUpperCase()}
                  </span>
                  <div style={{ flexGrow: 1 }} />
                  <span style={{ fontSize: 11.5, color: empty ? color.disabled : color.ink4 }}>
                    {words(doc.wordCount)} w
                  </span>
                </div>
              </div>
            </button>
          )
        })}
      </div>
    </div>
  )
}
