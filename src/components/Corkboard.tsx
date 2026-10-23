import { useMemo, useState } from 'react'
import {
  DndContext,
  PointerSensor,
  closestCenter,
  useDraggable,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import { SortableContext, rectSortingStrategy, useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useApp } from '../lib/store'
import type { DocumentRow } from '../lib/types'
import { color, labels, radius, statuses, type LabelName, type StatusName } from '../lib/tokens'
import { Button, EmptyState } from './ui'
import { words } from '../lib/format'

const CARD_W = 240
const CARD_H = 150

function statusOf(doc: DocumentRow): StatusName {
  if (doc.status && doc.status in statuses) return doc.status as StatusName
  return doc.wordCount > 0 ? 'Draft' : 'Empty'
}

function labelOf(doc: DocumentRow): LabelName {
  return doc.label && doc.label in labels ? (doc.label as LabelName) : 'None'
}

function Card({ doc, freeform = false }: { doc: DocumentRow; freeform?: boolean }) {
  const selectedId = useApp((s) => s.selectedId)
  const select = useApp((s) => s.select)
  const setView = useApp((s) => s.setView)
  // On a freeform board the wrapper owns the drag, so the card must not also
  // register as sortable — two handles on one element fight each other.
  const sortable = useSortable({ id: doc.id, disabled: freeform })
  const { transform, transition, isDragging } = sortable
  const attributes = freeform ? {} : sortable.attributes
  const listeners = freeform ? undefined : sortable.listeners
  const setNodeRef = freeform ? undefined : sortable.setNodeRef

  const on = doc.id === selectedId
  const status = statusOf(doc)
  const label = labelOf(doc)
  const empty = doc.wordCount === 0

  return (
    <div
      ref={setNodeRef}
      {...attributes}
      {...listeners}
      onClick={() => void select(doc.id)}
      onDoubleClick={() => {
        void select(doc.id)
        setView('editor')
      }}
      title="Drag to reorder · double-click to open"
      className="card"
      style={{
        transform: freeform ? undefined : CSS.Transform.toString(transform),
        transition: freeform ? undefined : transition,
        zIndex: isDragging ? 5 : undefined,
        opacity: isDragging ? 0.85 : 1,
        background: empty ? color.surfaceSoft : color.surface,
        border: on ? `2px solid ${color.accent}` : `1px ${empty ? 'dashed' : 'solid'} ${empty ? color.disabled : color.line}`,
        borderRadius: radius.xl,
        overflow: 'hidden',
        cursor: isDragging ? 'grabbing' : 'grab',
        boxShadow: isDragging
          ? '0 10px 26px rgba(28,43,51,0.18)'
          : on
            ? '0 2px 10px rgba(0,100,224,0.13)'
            : '0 1px 2px rgba(28,43,51,0.05)',
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
          <span style={{ fontSize: 11.5, color: empty ? color.disabled : color.ink4 }}>{words(doc.wordCount)} w</span>
        </div>
      </div>
    </div>
  )
}

/**
 * Phase 6: the same card, placed where the author dropped it.
 *
 * `useDraggable` rather than `useSortable` — on a freeform board there is no
 * order to sort into, only a position. A card that has never been placed keeps
 * `x`/`y` of null and is laid out on the grid, which is why "unplaced" and
 * "placed at (0, 0)" are stored differently: the second is a choice.
 */
function FreeCard({ doc, fallback }: { doc: DocumentRow; fallback: { x: number; y: number } }) {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({ id: doc.id })
  const x = doc.x ?? fallback.x
  const y = doc.y ?? fallback.y

  return (
    <div
      style={{
        position: 'absolute',
        left: x,
        top: y,
        width: CARD_W,
        transform: CSS.Translate.toString(transform),
        zIndex: isDragging ? 10 : undefined,
      }}
    >
      <div ref={setNodeRef} {...attributes} {...listeners}>
        <Card doc={doc} freeform />
      </div>
    </div>
  )
}

export function Corkboard() {
  const docs = useApp((s) => s.docs)
  const selectedId = useApp((s) => s.selectedId)
  const moveDocument = useApp((s) => s.moveDocument)
  const placeCard = useApp((s) => s.placeCard)
  const [dragging, setDragging] = useState(false)
  const [freeform, setFreeform] = useState(false)

  // Cards show one folder's children — the same scoping Scrivener uses, so
  // dragging a card means something definite about where it lands.
  const selected = docs.find((d) => d.id === selectedId)
  const parentId = selected ? (selected.kind === 'folder' ? selected.id : selected.parentId) : null
  const parent = docs.find((d) => d.id === parentId)

  const cards = useMemo(
    () =>
      docs
        .filter((d) => d.parentId === parentId && d.kind !== 'folder')
        .sort((a, b) => a.position - b.position),
    [docs, parentId],
  )

  // A small distance constraint keeps a click from being read as a drag.
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 5 } }))

  const onDragEnd = (e: DragEndEvent) => {
    setDragging(false)
    const { active, over } = e
    if (!over || active.id === over.id) return
    const to = cards.findIndex((c) => c.id === over.id)
    if (to >= 0) void moveDocument(String(active.id), parentId, to)
  }

  // Where an unplaced card sits: the grid it would have had. Laying them out
  // this way means turning freeform on does not scatter the board.
  const gridSpot = (i: number) => ({ x: (i % 3) * (CARD_W + 24) + 26, y: Math.floor(i / 3) * (CARD_H + 24) + 16 })

  const onFreeDragEnd = (e: DragEndEvent) => {
    setDragging(false)
    const { active, delta } = e
    if (!delta || (delta.x === 0 && delta.y === 0)) return
    const i = cards.findIndex((c) => c.id === active.id)
    if (i < 0) return
    const card = cards[i]
    const base = { x: card.x ?? gridSpot(i).x, y: card.y ?? gridSpot(i).y }
    // Clamped at the left and top only: a board can grow to the right and down,
    // but a card dragged off the top-left would be unreachable.
    void placeCard(card.id, Math.max(0, base.x + delta.x), Math.max(0, base.y + delta.y))
  }

  const placed = cards.filter((c) => c.x !== null && c.y !== null).length

  if (cards.length === 0) {
    return (
      <EmptyState
        icon="grid"
        title="No cards here"
        body="Select a folder in the binder to see its documents as index cards."
      />
    )
  }

  return (
    <div style={{ flexGrow: 1, overflowY: 'auto', background: color.rail }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '14px 26px 4px 26px' }}>
        <span style={{ fontSize: 12, color: color.ink3 }}>
          {parent ? `${parent.title} · ` : ''}
          {cards.length} cards
          {dragging ? (freeform ? ' · drop to place' : ' · drop to reorder') : ''}
          {freeform && !dragging && placed > 0 ? ` · ${placed} placed` : ''}
        </span>
        <div style={{ flexGrow: 1 }} />
        <Button
          height={24}
          variant={freeform ? 'primary' : 'ghost'}
          onClick={() => setFreeform(!freeform)}
          title={freeform ? 'Back to the ordered grid' : 'Place cards anywhere'}
        >
          Freeform
        </Button>
        {freeform && placed > 0 && (
          <Button
            height={24}
            variant="ghost"
            onClick={() => cards.forEach((c) => void placeCard(c.id, null, null))}
            title="Return every card to the grid"
          >
            Reset {placed}
          </Button>
        )}
        {(['Plot', 'Character', 'Setup'] as LabelName[]).map((l) => (
          <span key={l} style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12, color: color.ink3 }}>
            <span style={{ width: 9, height: 9, borderRadius: 3, background: labels[l].dot }} />
            {l}
          </span>
        ))}
      </div>

      {freeform ? (
        <DndContext
          sensors={sensors}
          onDragStart={() => setDragging(true)}
          onDragCancel={() => setDragging(false)}
          onDragEnd={onFreeDragEnd}
        >
          <div
            style={{
              position: 'relative',
              margin: '10px 26px 40px 0',
              minHeight: Math.ceil(cards.length / 3) * (CARD_H + 24) + 80,
            }}
          >
            {cards.map((doc, i) => (
              <FreeCard key={doc.id} doc={doc} fallback={gridSpot(i)} />
            ))}
          </div>
        </DndContext>
      ) : (
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragStart={() => setDragging(true)}
        onDragCancel={() => setDragging(false)}
        onDragEnd={onDragEnd}
      >
        <SortableContext items={cards.map((c) => c.id)} strategy={rectSortingStrategy}>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))',
              gap: 18,
              padding: '16px 26px 40px 26px',
            }}
          >
            {cards.map((doc) => (
              <Card key={doc.id} doc={doc} />
            ))}
          </div>
        </SortableContext>
      </DndContext>
      )}
    </div>
  )
}
