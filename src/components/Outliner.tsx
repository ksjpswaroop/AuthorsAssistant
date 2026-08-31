import { useState } from 'react'
import { useApp } from '../lib/store'
import type { DocumentRow } from '../lib/types'
import { color, labels, radius, statuses, type LabelName, type StatusName } from '../lib/tokens'
import { Icon } from './Icon'
import { EmptyState } from './ui'
import { words, relativeTime } from '../lib/format'

type SortKey = 'position' | 'title' | 'wordCount' | 'updatedAt'

const COLUMNS = 'minmax(0, 1fr) 108px 96px 78px 104px'

function statusOf(doc: DocumentRow): StatusName {
  if (doc.status && doc.status in statuses) return doc.status as StatusName
  return doc.wordCount > 0 ? 'Draft' : 'Empty'
}

export function Outliner() {
  const docs = useApp((s) => s.docs)
  const selectedId = useApp((s) => s.selectedId)
  const select = useApp((s) => s.select)
  const setView = useApp((s) => s.setView)
  const [sort, setSort] = useState<SortKey>('position')
  const [asc, setAsc] = useState(true)

  const rows = docs
    .filter((d) => d.kind !== 'folder')
    .slice()
    .sort((a, b) => {
      const dir = asc ? 1 : -1
      if (sort === 'title') return dir * a.title.localeCompare(b.title)
      if (sort === 'wordCount') return dir * (a.wordCount - b.wordCount)
      if (sort === 'updatedAt') return dir * (a.updatedAt - b.updatedAt)
      return dir * (a.position - b.position)
    })

  if (rows.length === 0) {
    return <EmptyState icon="list" title="Nothing to outline yet" body="Add a scene in the binder to see it here." />
  }

  const head = (key: SortKey, label: string, align: 'left' | 'right' = 'left') => (
    <button
      onClick={() => {
        if (sort === key) setAsc((a) => !a)
        else {
          setSort(key)
          setAsc(true)
        }
      }}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 5,
        justifyContent: align === 'right' ? 'flex-end' : 'flex-start',
        color: sort === key ? color.ink2 : color.ink4,
        cursor: 'pointer',
        fontSize: 10.5,
        fontWeight: 700,
        letterSpacing: '0.08em',
      }}
    >
      {label}
      {sort === key && (
        <Icon
          name="chevronDown"
          size={11}
          strokeWidth={2.6}
          style={{ transform: asc ? 'none' : 'rotate(180deg)' }}
        />
      )}
    </button>
  )

  return (
    <div style={{ flexGrow: 1, overflow: 'auto' }}>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: COLUMNS,
          alignItems: 'center',
          height: 34,
          background: color.rail,
          borderBottom: `1px solid ${color.line}`,
          padding: '0 20px',
          position: 'sticky',
          top: 0,
          zIndex: 1,
        }}
      >
        {head('title', 'TITLE')}
        <div style={{ fontSize: 10.5, fontWeight: 700, letterSpacing: '0.08em', color: color.ink4 }}>LABEL</div>
        <div style={{ fontSize: 10.5, fontWeight: 700, letterSpacing: '0.08em', color: color.ink4 }}>STATUS</div>
        {head('wordCount', 'WORDS', 'right')}
        {head('updatedAt', 'EDITED', 'right')}
      </div>

      {rows.map((doc) => {
        const on = doc.id === selectedId
        const status = statusOf(doc)
        const label: LabelName = doc.label && doc.label in labels ? (doc.label as LabelName) : 'None'
        return (
          <div
            key={doc.id}
            className={`tr ${on ? 'on' : ''}`}
            onClick={() => void select(doc.id)}
            onDoubleClick={() => {
              void select(doc.id)
              setView('editor')
            }}
            style={{
              display: 'grid',
              gridTemplateColumns: COLUMNS,
              alignItems: 'center',
              minHeight: 46,
              padding: '0 20px',
              borderBottom: `1px solid ${color.lineSoft}`,
              cursor: 'pointer',
              background: on ? 'rgba(0,100,224,0.08)' : 'transparent',
            }}
          >
            <div style={{ minWidth: 0, paddingRight: 20 }}>
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  fontSize: 13,
                  fontWeight: 600,
                  color: on ? color.accent : color.ink,
                }}
              >
                <Icon name="doc" size={13} strokeWidth={1.7} opacity={0.6} />
                <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{doc.title}</span>
              </div>
              {doc.synopsis && (
                <div
                  style={{
                    fontSize: 12,
                    color: color.ink4,
                    marginTop: 2,
                    paddingLeft: 21,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {doc.synopsis}
                </div>
              )}
            </div>
            <div>
              <span
                style={{
                  fontSize: 11,
                  fontWeight: 600,
                  color: labels[label].fg,
                  background: labels[label].bg,
                  borderRadius: radius.sm,
                  padding: '3px 8px',
                }}
              >
                {label}
              </span>
            </div>
            <div style={{ fontSize: 12, fontWeight: 600, color: statuses[status].fg }}>{status}</div>
            <div style={{ textAlign: 'right', fontSize: 12.5, color: color.ink2, fontVariantNumeric: 'tabular-nums' }}>
              {words(doc.wordCount)}
            </div>
            <div style={{ textAlign: 'right', fontSize: 12, color: color.ink4 }}>{relativeTime(doc.updatedAt)}</div>
          </div>
        )
      })}
    </div>
  )
}
