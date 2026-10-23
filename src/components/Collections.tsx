import { useState } from 'react'
import { Icon, type IconName } from './Icon'
import { SectionLabel } from './ui'
import { useApp } from '../lib/store'
import { useAsync } from '../lib/useAsync'
import { collections as api, type Collection, type CollectionKind } from '../lib/phases'
import { isTauri } from '../lib/api'
import { color, radius, tint } from '../lib/tokens'

const KIND_ICON: Record<CollectionKind, IconName> = { manual: 'layers', smart: 'filter', search: 'search' }

/**
 * Collections rail: virtual sets over binder ids. A smart collection stores a
 * query and is re-evaluated on every read, so a document joins or leaves it
 * when its metadata changes — the file never moves.
 */
export function CollectionsRail() {
  const path = useApp((s) => s.project?.path)
  const select = useApp((s) => s.select)
  const [active, setActive] = useState<string | null>(null)
  const [adding, setAdding] = useState(false)
  const [name, setName] = useState('')
  const ready = Boolean(path && isTauri)

  const list = useAsync(() => api.list(path!), [path], ready)
  const members = useAsync(() => api.members(path!, active!), [path, active], Boolean(ready && active))

  const create = async (kind: CollectionKind) => {
    if (!path || !name.trim()) return
    // A smart collection needs a query; "documents still in draft" is the one
    // authors reach for first, so it is the default rather than a blank form.
    await api.create(path, name.trim(), kind, kind === 'smart' ? { status: 'Draft' } : undefined)
    setName('')
    setAdding(false)
    list.reload()
  }

  const remove = async (id: string) => {
    if (!path) return
    await api.remove(path, id)
    if (active === id) setActive(null)
    list.reload()
  }

  const rows: Collection[] = list.data ?? []

  return (
    <div style={{ padding: '4px 0 10px 0' }}>
      <div style={{ display: 'flex', alignItems: 'center', padding: '10px 6px 6px 6px', gap: 8 }}>
        <SectionLabel>COLLECTIONS</SectionLabel>
        <div style={{ flexGrow: 1 }} />
        <button
          onClick={() => setAdding((v) => !v)}
          aria-label="New collection"
          style={{ display: 'flex', color: color.ink4, cursor: 'pointer' }}
        >
          <Icon name={adding ? 'close' : 'plus'} size={14} strokeWidth={2.2} />
        </button>
      </div>

      {adding && (
        <div style={{ padding: '0 6px 8px 6px' }}>
          <input
            autoFocus
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void create('manual')
              if (e.key === 'Escape') setAdding(false)
            }}
            placeholder="Name this collection"
            style={{
              width: '100%', height: 28, padding: '0 9px', fontSize: 12.5,
              border: `1px solid ${color.line}`, borderRadius: radius.md, outline: 'none',
            }}
          />
          <div style={{ display: 'flex', gap: 6, marginTop: 6 }}>
            <button
              onClick={() => void create('manual')}
              style={{ fontSize: 11.5, fontWeight: 600, color: color.accent, cursor: 'pointer' }}
            >
              Hand-picked
            </button>
            <button
              onClick={() => void create('smart')}
              title="Auto-updates from a metadata query, starting with status = Draft"
              style={{ fontSize: 11.5, fontWeight: 600, color: color.accent, cursor: 'pointer' }}
            >
              Smart (drafts)
            </button>
          </div>
        </div>
      )}

      {!ready && (
        <div style={{ padding: '4px 12px', fontSize: 11.5, lineHeight: 1.5, color: color.ink5 }}>
          Collections need the desktop app.
        </div>
      )}

      {ready && rows.length === 0 && !adding && (
        <div style={{ padding: '4px 12px', fontSize: 11.5, lineHeight: 1.5, color: color.ink5 }}>
          None yet. A collection gathers documents without moving them.
        </div>
      )}

      {rows.map((c) => {
        const on = active === c.id
        return (
          <div key={c.id}>
            <div
              className={`row ${on ? 'on' : ''}`}
              onClick={() => setActive(on ? null : c.id)}
              title={c.filters ? `Smart collection — ${c.filters}` : 'Hand-picked collection'}
              style={{
                display: 'flex', alignItems: 'center', gap: 8, height: 27, padding: '0 8px 0 12px',
                borderRadius: radius.md - 1, cursor: 'pointer', fontSize: 12.5,
                background: on ? tint(color.accent, 0.1) : 'transparent',
                color: on ? color.accent : color.ink2, fontWeight: on ? 600 : 450,
              }}
            >
              <Icon name={KIND_ICON[c.kind]} size={13} strokeWidth={1.7} opacity={0.7} />
              <span style={{ flexGrow: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {c.name}
              </span>
              <span style={{ color: color.ink4, fontWeight: 450, fontSize: 11.5 }}>{c.count}</span>
              <button
                onClick={(e) => { e.stopPropagation(); void remove(c.id) }}
                aria-label={`Delete ${c.name}`}
                title="Delete this collection. The documents stay where they are."
                style={{ display: 'flex', color: color.ink5, cursor: 'pointer' }}
              >
                <Icon name="close" size={11} strokeWidth={2.4} />
              </button>
            </div>

            {on && (
              <div style={{ margin: '4px 8px 6px 20px' }}>
                {c.filters && (
                  <div style={{ fontSize: 11, color: color.ink5, marginBottom: 5 }}>
                    Auto-updates from {c.filters}
                  </div>
                )}
                {(members.data ?? []).map((m) => {
                  const doc = m as { id: string; title: string }
                  return (
                    <button
                      key={doc.id}
                      onClick={() => void select(doc.id)}
                      className="row"
                      style={{
                        display: 'block', width: '100%', textAlign: 'left', height: 24, padding: '0 8px',
                        borderRadius: radius.md - 2, fontSize: 12, color: color.ink3, cursor: 'pointer',
                        overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
                      }}
                    >
                      {doc.title}
                    </button>
                  )
                })}
                {members.data?.length === 0 && (
                  <div style={{ fontSize: 11.5, color: color.ink5 }}>Nothing matches yet.</div>
                )}
              </div>
            )}
          </div>
        )
      })}
    </div>
  )
}
