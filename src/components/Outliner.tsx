import { useMemo, useState } from 'react'
import {
  createColumnHelper,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
  type SortingState,
  type VisibilityState,
} from '@tanstack/react-table'
import { useApp } from '../lib/store'
import type { DocumentRow } from '../lib/types'
import { color, labels, radius, statuses, type LabelName, type StatusName } from '../lib/tokens'
import { Icon } from './Icon'
import { Chip, EmptyState, IconButton } from './ui'
import { relativeTime, words } from '../lib/format'

function statusOf(doc: DocumentRow): StatusName {
  if (doc.status && doc.status in statuses) return doc.status as StatusName
  return doc.wordCount > 0 ? 'Draft' : 'Empty'
}

const col = createColumnHelper<DocumentRow>()

export function Outliner() {
  const docs = useApp((s) => s.docs)
  const selectedId = useApp((s) => s.selectedId)
  const select = useApp((s) => s.select)
  const setView = useApp((s) => s.setView)
  const [sorting, setSorting] = useState<SortingState>([{ id: 'position', desc: false }])
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({ pov: false, target: false })
  const [showColumns, setShowColumns] = useState(false)

  const data = useMemo(() => docs.filter((d) => d.kind !== 'folder'), [docs])

  const columns = useMemo(
    () => [
      col.accessor('position', { header: 'Order', size: 70 }),
      col.accessor('title', {
        header: 'Title',
        size: 300,
        cell: (c) => (
          <div style={{ minWidth: 0 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13, fontWeight: 600 }}>
              <Icon name="doc" size={13} strokeWidth={1.7} opacity={0.6} />
              <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {c.getValue()}
              </span>
            </div>
            {c.row.original.synopsis && (
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
                {c.row.original.synopsis}
              </div>
            )}
          </div>
        ),
      }),
      col.accessor((d) => d.label ?? 'None', {
        id: 'label',
        header: 'Label',
        size: 110,
        cell: (c) => {
          const l = (c.getValue() in labels ? c.getValue() : 'None') as LabelName
          return (
            <Chip fg={labels[l].fg} bg={labels[l].bg}>
              {l}
            </Chip>
          )
        },
      }),
      col.accessor((d) => statusOf(d), {
        id: 'status',
        header: 'Status',
        size: 96,
        cell: (c) => {
          const s = c.getValue() as StatusName
          return <span style={{ fontSize: 12, fontWeight: 600, color: statuses[s].fg }}>{s}</span>
        },
      }),
      col.accessor('pov', {
        header: 'POV',
        size: 110,
        cell: (c) => <span style={{ fontSize: 12.5, color: color.ink2 }}>{c.getValue() ?? '—'}</span>,
      }),
      col.accessor('wordCount', {
        header: 'Words',
        size: 84,
        cell: (c) => (
          <span style={{ fontSize: 12.5, color: color.ink2, fontVariantNumeric: 'tabular-nums' }}>
            {words(c.getValue())}
          </span>
        ),
      }),
      col.accessor('target', {
        header: 'Target',
        size: 96,
        cell: (c) => {
          const target = c.getValue()
          if (!target) return <span style={{ fontSize: 12.5, color: color.disabled }}>—</span>
          const pct = Math.min(100, (c.row.original.wordCount / target) * 100)
          return (
            <div style={{ display: 'flex', alignItems: 'center', gap: 7 }}>
              <div style={{ width: 34, height: 5, borderRadius: 3, background: color.lineSoft, overflow: 'hidden' }}>
                <div style={{ width: `${pct}%`, height: 5, borderRadius: 3, background: color.accent }} />
              </div>
              <span style={{ fontSize: 12, color: color.ink3, fontVariantNumeric: 'tabular-nums' }}>
                {words(target)}
              </span>
            </div>
          )
        },
      }),
      col.accessor('updatedAt', {
        header: 'Edited',
        size: 110,
        cell: (c) => <span style={{ fontSize: 12, color: color.ink4 }}>{relativeTime(c.getValue())}</span>,
      }),
    ],
    [],
  )

  const table = useReactTable({
    data,
    columns,
    state: { sorting, columnVisibility },
    onSortingChange: setSorting,
    onColumnVisibilityChange: setColumnVisibility,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  })

  if (data.length === 0) {
    return <EmptyState icon="list" title="Nothing to outline yet" body="Add a scene in the binder to see it here." />
  }

  return (
    <div style={{ flexGrow: 1, overflow: 'auto', position: 'relative' }}>
      <div
        style={{
          position: 'sticky',
          top: 0,
          zIndex: 2,
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          height: 34,
          padding: '0 20px',
          background: color.rail,
          borderBottom: `1px solid ${color.line}`,
        }}
      >
        {table.getHeaderGroups()[0].headers.map((header) => (
          <button
            key={header.id}
            onClick={header.column.getToggleSortingHandler()}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 5,
              width: header.getSize(),
              flexShrink: 0,
              cursor: 'pointer',
              fontSize: 10.5,
              fontWeight: 700,
              letterSpacing: '0.08em',
              textTransform: 'uppercase',
              color: header.column.getIsSorted() ? color.ink2 : color.ink4,
            }}
          >
            {flexRender(header.column.columnDef.header, header.getContext())}
            {header.column.getIsSorted() && (
              <Icon
                name="chevronDown"
                size={11}
                strokeWidth={2.6}
                style={{ transform: header.column.getIsSorted() === 'asc' ? 'none' : 'rotate(180deg)' }}
              />
            )}
          </button>
        ))}
        <div style={{ flexGrow: 1 }} />
        <IconButton icon="checklist" size={24} title="Choose columns" onClick={() => setShowColumns((v) => !v)} />
      </div>

      {showColumns && (
        <div
          style={{
            position: 'absolute',
            right: 16,
            top: 38,
            zIndex: 3,
            background: color.surface,
            border: `1px solid ${color.line}`,
            borderRadius: radius.xl,
            boxShadow: '0 12px 28px rgba(28,43,51,0.16)',
            padding: 8,
            minWidth: 170,
          }}
        >
          {table.getAllLeafColumns().map((c) => (
            <button
              key={c.id}
              onClick={() => c.toggleVisibility()}
              className="row"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 9,
                width: '100%',
                padding: '7px 9px',
                borderRadius: radius.md,
                cursor: 'pointer',
                fontSize: 13,
                color: color.ink2,
                textTransform: 'capitalize',
              }}
            >
              <Icon
                name={c.getIsVisible() ? 'check' : 'circle'}
                size={14}
                strokeWidth={2.2}
                style={{ color: c.getIsVisible() ? color.green : color.disabled }}
              />
              {String(c.columnDef.header)}
            </button>
          ))}
        </div>
      )}

      {table.getRowModel().rows.map((row) => {
        const on = row.original.id === selectedId
        return (
          <div
            key={row.id}
            className={`tr ${on ? 'on' : ''}`}
            onClick={() => void select(row.original.id)}
            onDoubleClick={() => {
              void select(row.original.id)
              setView('editor')
            }}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              minHeight: 46,
              padding: '0 20px',
              borderBottom: `1px solid ${color.lineSoft}`,
              cursor: 'pointer',
              background: on ? 'rgba(0,100,224,0.08)' : 'transparent',
            }}
          >
            {row.getVisibleCells().map((cell) => (
              <div key={cell.id} style={{ width: cell.column.getSize(), flexShrink: 0, minWidth: 0 }}>
                {cell.column.id === 'position' ? (
                  <span style={{ fontSize: 12, color: color.ink5, fontVariantNumeric: 'tabular-nums' }}>
                    {row.index + 1}
                  </span>
                ) : (
                  flexRender(cell.column.columnDef.cell, cell.getContext())
                )}
              </div>
            ))}
          </div>
        )
      })}
    </div>
  )
}
