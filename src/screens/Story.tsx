import { useRef, useState } from 'react'
import { PhaseScreen, Rail, SidePanel } from '../components/PhaseScreen'
import { AsyncBlock } from '../components/AsyncBlock'
import { useApp } from '../lib/store'
import { useAsync } from '../lib/useAsync'
import { canvas, continuity, story, type CommitPlan } from '../lib/phases'
import { isTauri } from '../lib/api'
import { Button, Callout, Chip, SectionLabel, Segmented } from '../components/ui'
import { Icon } from '../components/Icon'
import { color, font, radius, tint } from '../lib/tokens'
import { AXIS, GRID, LABEL, LABEL_STRONG, hueFor, scaleX } from '../lib/viz'
import { relativeTime, words } from '../lib/format'

type View = 'plot' | 'timeline' | 'graph' | 'canvas' | 'continuity'


/* -------------------------------------------------------------------------- */
/* Views over the real project                                                */
/* -------------------------------------------------------------------------- */

function useProject() {
  const path = useApp((s) => s.project?.path)
  return { path, ready: Boolean(path && isTauri) }
}

function PlotDiagram() {
  const { path, ready } = useProject()
  const [method, setMethod] = useState<'three_act' | 'freytag' | 'heros_journey'>('three_act')
  const state = useAsync(() => story.beats(path!, method), [path, method], ready)

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700, letterSpacing: '-0.01em' }}>Structure</h2>
        <div style={{ flexGrow: 1 }} />
        <Segmented
          value={method}
          onChange={setMethod}
          options={[
            { id: 'three_act', label: 'Three act' },
            { id: 'freytag', label: 'Freytag' },
            { id: 'heros_journey', label: 'Hero’s journey' },
          ]}
        />
      </div>

      <AsyncBlock
        state={state}
        isEmpty={(d) => d.length === 0}
        empty={{ title: 'No scenes with text yet', body: 'Beats are placed by word count, so a scene needs prose before it can be positioned.' }}
      >
        {(beats) => {
          // Beat data is one-dimensional: a position in the manuscript and a
          // label. Plotting it as a line needs a second axis, and the only
          // candidate — confidence — barely varies, which draws a flat line
          // that says nothing. A position strip shows exactly what is known.
          const W = 820
          const rowH = 46
          const x0 = 30
          const x1 = W - 30
          const H = 70 + beats.length * rowH
          const slots = [...new Set(beats.map((b) => b.label))]

          return (
            <>
              <svg width={W} height={H} role="img" aria-label="Scenes positioned through the manuscript by word count">
                {[0, 0.25, 0.5, 0.75, 1].map((t) => (
                  <g key={t}>
                    <line x1={scaleX(t, 0, 1, x0, x1)} x2={scaleX(t, 0, 1, x0, x1)} y1={28} y2={H - 26} stroke={GRID} />
                    <text x={scaleX(t, 0, 1, x0, x1)} y={18} textAnchor="middle" fontSize={10.5} fill={LABEL}>
                      {Math.round(t * 100)}%
                    </text>
                  </g>
                ))}

                {beats.map((b, i) => {
                  const y = 44 + i * rowH
                  const x = scaleX(b.position, 0, 1, x0, x1)
                  const slot = slots.indexOf(b.label)
                  // Past the midpoint there is no room to the right, so the
                  // label flips to the other side rather than being clipped.
                  const flip = b.position > 0.55
                  const tx = flip ? x - 13 : x + 13
                  const anchor = flip ? 'end' : 'start'
                  return (
                    <g key={b.documentId}>
                      <line x1={x0} x2={x1} y1={y} y2={y} stroke={GRID} />
                      <circle cx={x} cy={y} r={7} fill={hueFor(slot % 5)} stroke="#FFFFFF" strokeWidth={2} />
                      <text x={tx} y={y - 3} textAnchor={anchor} fontSize={12.5} fontWeight={600} fill={color.ink}>
                        {b.title}
                      </text>
                      <text x={tx} y={y + 12} textAnchor={anchor} fontSize={11} fill={LABEL}>
                        {b.label} · {Math.round(b.position * 100)}% in · confidence {Math.round(b.confidence * 100)}%
                      </text>
                    </g>
                  )
                })}

                <text x={(x0 + x1) / 2} y={H - 8} textAnchor="middle" fontSize={11} fill={LABEL}>
                  Position in the manuscript, by word count
                </text>
              </svg>

              <div style={{ display: 'flex', flexWrap: 'wrap', gap: 14, marginTop: 4 }}>
                {slots.map((slot, i) => (
                  <span key={slot} style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12, color: color.ink2 }}>
                    <span style={{ width: 10, height: 10, borderRadius: '50%', background: hueFor(i % 5) }} />
                    {slot}
                  </span>
                ))}
              </div>

              <Callout tone="quiet" style={{ marginTop: 12 }}>
                Positions are measured, not guessed: each scene sits where its words fall in the manuscript. The label is
                the structural slot at that position; confidence rises when a scene has a synopsis to reason from.
              </Callout>
            </>
          )
        }}
      </AsyncBlock>
    </div>
  )
}

function TimelineView() {
  const { path, ready } = useProject()
  const [mode, setMode] = useState<'narrative' | 'chronological'>('narrative')
  const state = useAsync(() => story.timeline(path!, mode === 'chronological'), [path, mode], ready)

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 14 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Order of events</h2>
        <div style={{ flexGrow: 1 }} />
        <Segmented
          value={mode}
          onChange={setMode}
          options={[
            { id: 'narrative', label: 'Narrative order' },
            { id: 'chronological', label: 'Story chronology' },
          ]}
        />
      </div>

      <AsyncBlock state={state} isEmpty={(d) => d.length === 0} empty={{ title: 'No scenes yet' }}>
        {(rows) => (
          <>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
              {rows.map((r, i) => (
                <div
                  key={r.documentId}
                  style={{
                    display: 'grid',
                    gridTemplateColumns: '34px minmax(0, 1fr) 96px 90px',
                    alignItems: 'center',
                    gap: 12,
                    padding: '11px 13px',
                    borderRadius: radius.lg,
                    border: `1px solid ${r.outOfOrder ? color.amberLine : color.lineSoft}`,
                    background: r.outOfOrder ? color.amberTint : color.surface,
                  }}
                >
                  <span
                    style={{
                      width: 26,
                      height: 26,
                      borderRadius: '50%',
                      background: r.outOfOrder ? color.amberMark : color.blueTint,
                      color: r.outOfOrder ? color.yellowInk : color.blueDeep,
                      fontSize: 12,
                      fontWeight: 700,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                    }}
                  >
                    {i + 1}
                  </span>
                  <span style={{ fontSize: 13.5, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {r.title}
                  </span>
                  <span style={{ fontSize: 12.5, color: r.storyDay === null ? color.disabled : color.ink3 }}>
                    {r.storyDay === null ? 'no date' : `Day ${r.storyDay}`}
                  </span>
                  <span style={{ fontSize: 12, color: r.outOfOrder ? color.yellowInk : color.ink4, textAlign: 'right' }}>
                    {r.outOfOrder ? `was #${r.narrativeOrder + 1}` : 'in place'}
                  </span>
                </div>
              ))}
            </div>
            <Callout tone="quiet" style={{ marginTop: 14 }}>
              Story dates are read from a document's notes when they say “Day 12” outright. A scene with no date keeps its
              narrative position rather than being guessed into the sequence.
            </Callout>
          </>
        )}
      </AsyncBlock>
    </div>
  )
}

function CharacterGraph() {
  const { path, ready } = useProject()
  const [focus, setFocus] = useState<string | null>(null)
  const [label, setLabel] = useState<string | null>(null)
  const state = useAsync(() => story.graph(path!, label), [path, label], ready)
  const W = 780
  const H = 380

  // The longest gap between two characters who do share scenes. A number the
  // author can act on or ignore: a relationship dormant for forty scenes may be
  // forgotten or may be the point, and only they know which.
  const stalest = (state.data?.edges ?? [])
    .filter((e) => e.scenesSince !== null)
    .sort((a, b) => (b.scenesSince ?? 0) - (a.scenesSince ?? 0))[0]

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 12 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Who shares a scene with whom</h2>
        <div style={{ flexGrow: 1 }} />
        {(state.data?.labels.length ?? 0) > 0 && (
          <select
            value={label ?? ''}
            onChange={(e) => setLabel(e.target.value || null)}
            title="Narrow the graph to scenes carrying one label"
            style={{
              fontSize: 11.5,
              color: color.ink2,
              background: color.surface,
              border: `1px solid ${color.line}`,
              borderRadius: 999,
              padding: '3px 9px',
            }}
          >
            <option value="">Every scene</option>
            {state.data!.labels.map((l) => (
              <option key={l} value={l}>
                {l} scenes only
              </option>
            ))}
          </select>
        )}
      </div>

      {stalest && (
        <div style={{ fontSize: 12.5, color: color.ink3, marginBottom: 12, lineHeight: 1.6 }}>
          Longest gap: two characters who share scenes have not shared one for{' '}
          <strong style={{ fontWeight: 650 }}>{stalest.scenesSince} scenes</strong>. That may be
          forgotten or it may be the point — a count, not a complaint.
        </div>
      )}
      <AsyncBlock
        state={state}
        isEmpty={(g) => g.nodes.length === 0}
        empty={{
          title: 'No sheets yet',
          body: 'Create a character or location sheet and link it to the scenes it appears in — File → New Character Sheet.',
        }}
      >
        {(g) => {
          // Lay nodes on a circle: deterministic, and it never overlaps.
          const pos = (i: number) => {
            const a = (i / Math.max(1, g.nodes.length)) * Math.PI * 2 - Math.PI / 2
            return { x: W / 2 + Math.cos(a) * (W / 3.4), y: H / 2 + Math.sin(a) * (H / 3.2) }
          }
          const index = Object.fromEntries(g.nodes.map((n, i) => [n.id, i]))
          return (
            <>
              <svg width={W} height={H} role="img" aria-label="Character relationship graph">
                {g.edges.map((e, i) => {
                  const a = pos(index[e.from])
                  const b = pos(index[e.to])
                  const lit = focus === e.from || focus === e.to
                  return (
                    <g key={i}>
                      <line x1={a.x} y1={a.y} x2={b.x} y2={b.y} stroke={lit ? color.ink3 : AXIS} strokeWidth={lit ? 2.5 : 1.5} />
                      {lit && (
                        <text x={(a.x + b.x) / 2} y={(a.y + b.y) / 2 - 6} textAnchor="middle" fontSize={11} fill={LABEL_STRONG}>
                          {e.weight} shared
                        </text>
                      )}
                    </g>
                  )
                })}
                {g.nodes.map((n, i) => {
                  const p = pos(i)
                  const r = 12 + Math.min(20, n.weight * 3)
                  return (
                    <g key={n.id} onMouseEnter={() => setFocus(n.id)} onMouseLeave={() => setFocus(null)} style={{ cursor: 'pointer' }}>
                      <circle cx={p.x} cy={p.y} r={r + 3} fill="#FFFFFF" />
                      <circle cx={p.x} cy={p.y} r={r} fill={hueFor(n.kind === 'location' ? 3 : 0)} opacity={focus === n.id ? 1 : 0.86} />
                      <text x={p.x} y={p.y + r + 16} textAnchor="middle" fontSize={11.5} fontWeight={600} fill={color.ink}>
                        {n.label}
                      </text>
                      <text x={p.x} y={p.y + r + 29} textAnchor="middle" fontSize={10.5} fill={LABEL}>
                        {n.weight} {n.weight === 1 ? 'scene' : 'scenes'}
                      </text>
                    </g>
                  )
                })}
              </svg>
              <Callout tone="quiet" style={{ marginTop: 6 }}>
                An edge means two sheets are linked to the same scene — a fact from the project, not an inference. Node
                size is scene count, not importance.
              </Callout>
            </>
          )
        }}
      </AsyncBlock>
    </div>
  )
}

/**
 * Phase 8: the one view that writes back to the binder.
 *
 * Every other view here is a projection — the binder is the truth and the plot
 * diagram, timeline and graph are ways of looking at it. This runs the other
 * way, so the write-back is deliberately two steps: dragging a card changes
 * only `.aegis/viz/canvas.json`, and **Commit** first shows exactly which
 * documents would move before anything happens. A card nudged by accident must
 * never silently rewrite the reading order of a book.
 */
function InfiniteCanvas() {
  const { path, ready } = useProject()
  const docs = useApp((s) => s.docs)
  const selectedId = useApp((s) => s.selectedId)
  const refreshDocs = useApp((s) => s.refreshDocs)

  // Cards are one folder's children, like the corkboard — a canvas of the whole
  // book would commit an order nobody meant.
  const selected = docs.find((d) => d.id === selectedId)
  const parentId = selected ? (selected.kind === 'folder' ? selected.id : selected.parentId) : null
  const parent = docs.find((d) => d.id === parentId)
  const cards = docs
    .filter((d) => d.parentId === parentId && d.kind !== 'folder')
    .sort((a, b) => a.position - b.position)

  const layout = useAsync(() => canvas.load(path!), [path], ready)
  const [plan, setPlan] = useState<CommitPlan | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [drag, setDrag] = useState<{ id: string; dx: number; dy: number } | null>(null)
  const [noteText, setNoteText] = useState('')
  const [linking, setLinking] = useState<string | null>(null)
  // Positions moved this session, layered over what is on disk. Kept apart from
  // `layout.data` so a drag never mutates the cached load — a stale cache here
  // would show the author a layout that is not the one being committed.
  const [moved, setMoved] = useState<Record<string, { x: number; y: number }>>({})
  const board = useRef<HTMLDivElement>(null)

  const gridSpot = (i: number) => ({ x: 24 + (i % 4) * 190, y: 24 + Math.floor(i / 4) * 116 })
  const spotOf = (id: string, i: number) => {
    if (moved[id]) return moved[id]
    const placed = layout.data?.cards.find((c) => c.id === id)
    return placed ? { x: placed.x, y: placed.y } : gridSpot(i)
  }

  const onPointerDown = (e: React.PointerEvent, id: string, spot: { x: number; y: number }) => {
    const rect = board.current?.getBoundingClientRect()
    if (!rect) return
    e.currentTarget.setPointerCapture(e.pointerId)
    setDrag({ id, dx: e.clientX - rect.left - spot.x, dy: e.clientY - rect.top - spot.y })
  }

  const onPointerMove = (e: React.PointerEvent) => {
    if (!drag) return
    const rect = board.current?.getBoundingClientRect()
    if (!rect) return
    setMoved((m) => ({
      ...m,
      [drag.id]: {
        x: Math.max(0, e.clientX - rect.left - drag.dx),
        y: Math.max(0, e.clientY - rect.top - drag.dy),
      },
    }))
    setPlan(null)
  }

  const onPointerUp = async () => {
    if (!drag) return
    const spot = moved[drag.id]
    setDrag(null)
    // The file is written on drop, not on every pointer move: a drag across the
    // board would otherwise be a hundred writes.
    if (spot && path) await canvas.place(path, drag.id, spot.x, spot.y)
  }

  const preview = async () => {
    if (!path) return
    setBusy(true)
    setError(null)
    try {
      setPlan(await canvas.plan(path, parentId))
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const commit = async () => {
    if (!path) return
    setBusy(true)
    setError(null)
    try {
      await canvas.commit(path, parentId)
      await refreshDocs()
      setPlan(null)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  // A note is thinking beside the work. It is never a document, never part of
  // the reading order, and committing the layout ignores it entirely.
  const notes = layout.data?.notes ?? []
  const arrows = layout.data?.arrows ?? []
  const anchorOf = (id: string): { x: number; y: number } | null => {
    const note = notes.find((n) => n.id === id)
    if (note) return { x: note.x + 80, y: note.y + 24 }
    const i = cards.findIndex((c) => c.id === id)
    return i < 0 ? null : { x: spotOf(id, i).x + 86, y: spotOf(id, i).y + 30 }
  }

  const addNote = async () => {
    if (!path || !noteText.trim()) return
    await canvas.note(path, null, 40, Math.max(200, cards.length * 20), noteText.trim())
    setNoteText('')
    layout.reload()
  }

  const connect = async (to: string) => {
    if (!path || !linking) return
    try {
      await canvas.arrow(path, linking, to)
      layout.reload()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setLinking(null)
    }
  }

  const placedIds = new Set([
    ...(layout.data?.cards ?? []).map((c) => c.id),
    ...Object.keys(moved),
  ])
  const placedCount = cards.filter((d) => placedIds.has(d.id)).length

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Freeform canvas</h2>
        <span style={{ fontSize: 12, color: color.ink4 }}>
          {parent ? `${parent.title} · ` : ''}
          {cards.length} cards
          {placedCount > 0 ? ` · ${placedCount} placed` : ''}
        </span>
        <div style={{ flexGrow: 1 }} />
        {placedCount > 0 && (
          <Button
            variant="ghost"
            height={30}
            disabled={busy || !isTauri}
            onClick={async () => {
              if (!path) return
              await canvas.clear(path)
              setMoved({})
              layout.reload()
              setPlan(null)
            }}
          >
            Reset layout
          </Button>
        )}
        <input
          value={noteText}
          onChange={(e) => setNoteText(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && void addNote()}
          placeholder="Add a note…"
          style={{
            width: 190,
            fontSize: 12,
            padding: '5px 9px',
            borderRadius: radius.md,
            border: `1px solid ${color.line}`,
            background: color.surface,
          }}
        />
        <Button
          variant="ghost"
          height={30}
          icon="layers"
          disabled={busy || placedCount === 0 || !isTauri}
          title={placedCount === 0 ? 'Drag a card first' : 'See what committing would change'}
          onClick={() => void preview()}
        >
          Commit to binder
        </Button>
      </div>

      {cards.length === 0 ? (
        <Callout tone="quiet">
          Select a folder in the binder to lay out its scenes.
        </Callout>
      ) : (
        <>
          <div
            ref={board}
            onPointerMove={onPointerMove}
            onPointerUp={() => void onPointerUp()}
            onPointerCancel={() => setDrag(null)}
            style={{
              position: 'relative',
              height: Math.max(380, Math.ceil(cards.length / 4) * 116 + 80),
              borderRadius: radius.xl,
              border: `1px solid ${color.line}`,
              background: `${color.surfaceSoft} radial-gradient(${color.line} 1px, transparent 1px)`,
              backgroundSize: '22px 22px',
              overflow: 'hidden',
              touchAction: 'none',
            }}
          >
            <svg
              width="100%"
              height="100%"
              style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}
            >
              <defs>
                <marker id="arrowhead" markerWidth="8" markerHeight="8" refX="7" refY="3"
                        orient="auto">
                  <path d="M0,0 L7,3 L0,6 z" fill={color.ink4} />
                </marker>
              </defs>
              {arrows.map((a) => {
                const from = anchorOf(a.from)
                const to = anchorOf(a.to)
                if (!from || !to) return null
                return (
                  <line
                    key={a.id}
                    x1={from.x}
                    y1={from.y}
                    x2={to.x}
                    y2={to.y}
                    stroke={color.ink4}
                    strokeWidth={1.5}
                    markerEnd="url(#arrowhead)"
                  />
                )
              })}
            </svg>

            {notes.map((n) => (
              <div
                key={n.id}
                onClick={() => linking && void connect(n.id)}
                style={{
                  position: 'absolute',
                  left: n.x,
                  top: n.y,
                  width: 160,
                  padding: '9px 11px',
                  borderRadius: radius.md,
                  background: color.amberTint,
                  border: `1px solid ${linking ? color.accent : color.amberLine}`,
                  fontSize: 12,
                  lineHeight: 1.5,
                  color: color.inkProse,
                  cursor: linking ? 'crosshair' : 'default',
                }}
              >
                {n.text}
                <div style={{ display: 'flex', gap: 8, marginTop: 6 }}>
                  <button
                    onClick={(e) => {
                      e.stopPropagation()
                      setLinking(linking === n.id ? null : n.id)
                    }}
                    style={{ fontSize: 10.5, color: color.accent, cursor: 'pointer' }}
                  >
                    {linking === n.id ? 'pick a target' : 'connect'}
                  </button>
                  <button
                    onClick={async (e) => {
                      e.stopPropagation()
                      if (!path) return
                      await canvas.removeNote(path, n.id)
                      layout.reload()
                    }}
                    style={{ fontSize: 10.5, color: color.ink4, cursor: 'pointer' }}
                  >
                    remove
                  </button>
                </div>
              </div>
            ))}

            {cards.map((doc, i) => {
              const spot = spotOf(doc.id, i)
              const dragging = drag?.id === doc.id
              return (
                <div
                  key={doc.id}
                  onPointerDown={(e) => !linking && onPointerDown(e, doc.id, spot)}
                  onClick={() => linking && void connect(doc.id)}
                  style={{
                    position: 'absolute',
                    left: spot.x,
                    top: spot.y,
                    width: 172,
                    background: color.surface,
                    border: `1px solid ${dragging ? color.accent : color.line}`,
                    borderRadius: radius.lg + 1,
                    boxShadow: dragging
                      ? '0 6px 18px rgba(28,43,51,0.18)'
                      : '0 2px 6px rgba(28,43,51,0.08)',
                    overflow: 'hidden',
                    cursor: 'grab',
                    zIndex: dragging ? 5 : undefined,
                    userSelect: 'none',
                  }}
                >
                  <div style={{ height: 3, background: hueFor(i % 5) }} />
                  <div style={{ padding: '10px 12px 12px 12px' }}>
                    <div
                      style={{
                        fontSize: 12.5,
                        fontWeight: 600,
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        whiteSpace: 'nowrap',
                      }}
                    >
                      {doc.title}
                    </div>
                    <div style={{ fontSize: 11.5, color: color.ink3, marginTop: 4 }}>
                      {words(doc.wordCount)} words
                    </div>
                  </div>
                </div>
              )
            })}
          </div>

          {plan && (
            <div
              style={{
                marginTop: 12,
                padding: '13px 15px',
                borderRadius: radius.lg,
                background: color.orangeTint,
                border: `1px solid ${color.orangeMark}`,
              }}
            >
              <div style={{ fontSize: 13, fontWeight: 650, color: color.inkProse }}>
                {plan.moves.length === 0
                  ? `${plan.parentTitle} is already in this order.`
                  : `${plan.moves.length} document${plan.moves.length === 1 ? '' : 's'} would move in ${plan.parentTitle}.`}
              </div>
              {plan.moves.length > 0 && (
                <div style={{ marginTop: 8, fontSize: 12.5, color: color.ink2, lineHeight: 1.7 }}>
                  {plan.moves.map((m) => (
                    <div key={m.id}>
                      {m.title} — position {m.from + 1} → {m.to + 1}
                    </div>
                  ))}
                </div>
              )}
              {plan.unplaced.length > 0 && (
                <div style={{ marginTop: 8, fontSize: 12, color: color.ink3 }}>
                  Not placed, so they keep their order and follow: {plan.unplaced.join(', ')}.
                </div>
              )}
              {plan.orphans.length > 0 && (
                <div style={{ marginTop: 8, fontSize: 12, color: color.ink3 }}>
                  Placed but in another folder, so they are left alone: {plan.orphans.join(', ')}.
                </div>
              )}
              <div style={{ display: 'flex', gap: 9, marginTop: 12, alignItems: 'center' }}>
                <Button disabled={busy || plan.moves.length === 0} onClick={() => void commit()}>
                  {busy ? 'Committing…' : 'Reorder the binder'}
                </Button>
                <Button variant="ghost" onClick={() => setPlan(null)}>
                  Cancel
                </Button>
                <span style={{ fontSize: 11.5, color: color.ink3 }}>
                  A snapshot is taken first, so this can be walked back.
                </span>
              </div>
            </div>
          )}

          {error && (
            <Callout tone="warn" icon="alert" style={{ marginTop: 12 }}>
              {error}
            </Callout>
          )}

          <Callout tone="quiet" style={{ marginTop: 12 }}>
            Cards are your real scenes; notes are yours to think in. Drag anything — that only
            writes{' '}
            <span style={{ fontFamily: font.mono }}>.aegis/viz/canvas.json</span>. Committing reads
            the layout as rows top to bottom and left to right within a row, and reorders the folder
            to match. <strong style={{ fontWeight: 650 }}>Notes and arrows are never committed</strong>
            — a note between two scenes must not quietly become a scene.
          </Callout>
        </>
      )}
    </div>
  )
}

/**
 * Deep Continuity: what the book has already established, against what the open
 * scene says.
 *
 * Everything here is arithmetic over the author's own project — canon entries,
 * plants and payoffs, character sheets. No model runs, so it works with Ollama
 * stopped, and it cannot hallucinate a contradiction that is not there.
 *
 * It is deliberately **retrieval, not judgement**. It surfaces the rule a
 * paragraph is standing near and leaves the reading to a person. Deciding here
 * from word overlap would produce confident nonsense: "his left leg" and "his
 * right leg" overlap almost perfectly.
 */
function ContinuityView() {
  const { path, ready } = useProject()
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const select = useApp((s) => s.select)

  const report = useAsync(
    () => continuity.report(path!, doc!.id),
    [path, doc?.id, doc?.updatedAt],
    ready && !!doc,
  )
  const [gap, setGap] = useState('')
  const [busy, setBusy] = useState(false)

  const logGap = async () => {
    if (!path || !doc || gap.trim().split(/\s+/).length < 3) return
    setBusy(true)
    try {
      await continuity.logDebt(path, gap.trim(), doc.title)
      setGap('')
      report.reload()
    } finally {
      setBusy(false)
    }
  }

  if (!doc) {
    return <Callout tone="quiet">Select a scene in the binder to check it.</Callout>
  }

  return (
    <div style={{ maxWidth: 760 }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 4 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Continuity</h2>
        <span style={{ fontSize: 12, color: color.ink4 }}>{doc.title}</span>
      </div>
      <div style={{ fontSize: 12.5, color: color.ink3, marginBottom: 16, lineHeight: 1.6 }}>
        Computed from your canon entries, your plants and payoffs, and your character sheets. No
        model runs here.
      </div>

      <AsyncBlock state={report} empty={{ title: 'Nothing to check yet' }}>
        {(r) => (
          <>
            <SectionLabel style={{ marginBottom: 8 }}>
              Established facts this scene is near
            </SectionLabel>
            {r.canon.candidates.length === 0 ? (
              <div style={{ fontSize: 12.5, color: color.ink4, lineHeight: 1.6, marginBottom: 18 }}>
                {r.canon.canonEntries === 0
                  ? 'No canon entries yet. A canon entry is a falsifiable fact plus the document that establishes it — the inspector writes them.'
                  : `None of your ${r.canon.canonEntries} established facts share wording with this scene.`}
              </div>
            ) : (
              <div style={{ marginBottom: 18 }}>
                {r.canon.candidates.map((c, i) => (
                  <div
                    key={i}
                    style={{
                      padding: '11px 13px',
                      borderRadius: radius.lg,
                      border: `1px solid ${c.overlap > 0.3 ? color.amberLine : color.lineSoft}`,
                      background: c.overlap > 0.3 ? color.amberTint : color.surfaceSoft,
                      marginBottom: 6,
                    }}
                  >
                    <div style={{ fontSize: 13, fontWeight: 600, color: color.inkProse }}>
                      {c.fact}
                    </div>
                    <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 3 }}>
                      established in {c.source} · {Math.round(c.overlap * 100)}% shared wording
                    </div>
                    <div style={{ fontSize: 12.5, color: color.ink2, marginTop: 7, lineHeight: 1.55, fontStyle: 'italic' }}>
                      “{c.passageExcerpt}”
                    </div>
                  </div>
                ))}
                <div style={{ fontSize: 11.5, color: color.ink4, lineHeight: 1.55 }}>
                  {r.canon.method}
                </div>
              </div>
            )}

            <SectionLabel style={{ marginBottom: 8 }}>Promises the book has not kept</SectionLabel>
            {r.ledger.unpaid.length === 0 && r.ledger.unplanted.length === 0 ? (
              <div style={{ fontSize: 12.5, color: color.ink4, marginBottom: 18, lineHeight: 1.6 }}>
                Nothing outstanding. Plants and payoffs are the ids you write in the inspector; a
                thread planted in one scene and paid off in another closes here.
              </div>
            ) : (
              <div style={{ marginBottom: 18 }}>
                {r.ledger.unpaid.map((t) => (
                  <div key={t.id} style={{ fontSize: 12.5, color: color.ink2, padding: '5px 0', lineHeight: 1.55 }}>
                    <Chip fg={color.yellowInk} bg={color.amberMark}>unpaid</Chip>{' '}
                    <span style={{ fontFamily: font.mono }}>{t.id}</span> — planted in{' '}
                    {t.plantedIn.map(([id, title], i) => (
                      <span key={id}>
                        {i > 0 && ', '}
                        <button
                          onClick={() => void select(id)}
                          style={{ color: color.accent, cursor: 'pointer' }}
                        >
                          {title}
                        </button>
                      </span>
                    ))}
                    , never paid off.
                  </div>
                ))}
                {r.ledger.unplanted.map((t) => (
                  <div key={t.id} style={{ fontSize: 12.5, color: color.ink2, padding: '5px 0', lineHeight: 1.55 }}>
                    <Chip fg={color.orangeInk} bg={color.orangeTint}>unplanted</Chip>{' '}
                    <span style={{ fontFamily: font.mono }}>{t.id}</span> — paid off in{' '}
                    {t.paidOffIn.map(([, title]) => title).join(', ')} with nothing planted.
                  </div>
                ))}
              </div>
            )}

            <SectionLabel style={{ marginBottom: 8 }}>Who and where this scene names</SectionLabel>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5, marginBottom: 18 }}>
              {r.entities.length === 0 ? (
                <span style={{ fontSize: 12.5, color: color.ink4 }}>
                  Nothing recognised. Entities are matched against your own character and location
                  sheets, so a name without a sheet is not guessed at.
                </span>
              ) : (
                r.entities.map((e) => (
                  <button
                    key={`${e.kind}-${e.name}`}
                    onClick={() => e.documentId && void select(e.documentId)}
                    title={e.documentId ? 'Open the sheet' : 'Not a document'}
                    style={{
                      display: 'inline-flex',
                      alignItems: 'center',
                      gap: 6,
                      padding: '3px 9px',
                      borderRadius: 999,
                      fontSize: 12,
                      cursor: e.documentId ? 'pointer' : 'default',
                      color: color.ink2,
                      background: color.surfaceSoft,
                      border: `1px solid ${color.lineSoft}`,
                    }}
                  >
                    <span
                      style={{
                        width: 6,
                        height: 6,
                        borderRadius: 3,
                        background:
                          e.kind === 'character'
                            ? color.accent
                            : e.kind === 'location'
                              ? color.green
                              : color.ink4,
                      }}
                    />
                    {e.name}
                    <span style={{ color: color.ink4 }}>{e.mentions}</span>
                  </button>
                ))
              )}
            </div>

            <SectionLabel style={{ marginBottom: 8 }}>
              Gaps in the world ({r.openDebts.length})
            </SectionLabel>
            <div style={{ fontSize: 12.5, color: color.ink3, marginBottom: 8, lineHeight: 1.6 }}>
              Something the book needs and has never established. Writing it down keeps the
              decision yours — an agent that finds one records it here instead of inventing a fact
              to fill it.
            </div>
            {r.openDebts.map((d) => (
              <div
                key={d.id}
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: 9,
                  padding: '7px 0',
                  borderBottom: `1px solid ${color.lineFaint}`,
                }}
              >
                <div style={{ flexGrow: 1, fontSize: 12.5, color: color.ink2, lineHeight: 1.55 }}>
                  {d.gap}
                  <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 2 }}>
                    found in {d.foundIn} by {d.foundBy} · {relativeTime(d.at)}
                  </div>
                </div>
                <Button
                  height={24}
                  variant="ghost"
                  onClick={async () => {
                    if (!path) return
                    await continuity.resolveDebt(path, d.id)
                    report.reload()
                  }}
                >
                  Settled
                </Button>
              </div>
            ))}
            <div style={{ display: 'flex', gap: 8, marginTop: 10 }}>
              <input
                value={gap}
                onChange={(e) => setGap(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && !busy && void logGap()}
                placeholder="What does the book need that it has never established?"
                style={{
                  flexGrow: 1,
                  fontSize: 12.5,
                  padding: '6px 10px',
                  borderRadius: radius.md,
                  border: `1px solid ${color.line}`,
                  background: color.surface,
                }}
              />
              <Button
                height={30}
                variant="ghost"
                disabled={busy || gap.trim().split(/\s+/).length < 3}
                onClick={() => void logGap()}
              >
                Note it
              </Button>
            </div>
          </>
        )}
      </AsyncBlock>
    </div>
  )
}

export function Story() {
  const [view, setView] = useState<View>('plot')

  return (
    <PhaseScreen
      title="Story"
      phase="Phase 8"
      explains="Every view here is computed from your own binder — positions from word counts, edges from sheet links, dates from your notes. Nothing is sent anywhere."
      statusRight={<span>Computed locally from the open project</span>}
    >
      <Rail width={228}>
        <div style={{ padding: '12px 8px' }}>
          <SectionLabel style={{ padding: '4px 10px 8px 10px' }}>VIEWS</SectionLabel>
          {(
            [
              { id: 'plot', label: 'Plot diagram', icon: 'chart', sub: 'beats and tension' },
              { id: 'timeline', label: 'Timeline', icon: 'timeline', sub: 'narrative vs story' },
              { id: 'graph', label: 'Character graph', icon: 'network', sub: 'co-occurrence' },
              { id: 'canvas', label: 'Infinite canvas', icon: 'canvas', sub: 'freeform layout' },
              { id: 'continuity', label: 'Continuity', icon: 'shieldCheck', sub: 'canon and threads' },
            ] as const
          ).map((v) => {
            const on = v.id === view
            return (
              <button
                key={v.id}
                className={`row ${on ? 'on' : ''}`}
                onClick={() => setView(v.id)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  width: '100%',
                  padding: '9px 11px',
                  borderRadius: radius.lg,
                  cursor: 'pointer',
                  marginBottom: 2,
                  background: on ? tint(color.accent, 0.1) : 'transparent',
                  color: on ? color.accent : color.ink2,
                }}
              >
                <Icon name={v.icon} size={16} />
                <span style={{ minWidth: 0 }}>
                  <span style={{ display: 'block', fontSize: 13, fontWeight: on ? 600 : 500 }}>{v.label}</span>
                  <span style={{ display: 'block', fontSize: 11.5, color: color.ink4, marginTop: 1 }}>{v.sub}</span>
                </span>
              </button>
            )
          })}
        </div>
        <div style={{ flexGrow: 1 }} />
        <div style={{ borderTop: `1px solid ${color.lineFaint}`, padding: '12px 14px' }}>
          <SectionLabel style={{ marginBottom: 8 }}>TOOLS</SectionLabel>
          <div style={{ fontFamily: font.mono, fontSize: 10.5, lineHeight: 1.9, color: color.ink3 }}>
            <div>generate_plot_diagram</div>
            <div>generate_timeline</div>
            <div>build_character_graph</div>
            <div>canvas_plan</div>
            <div>canvas_to_binder</div>
          </div>
        </div>
      </Rail>

      <div style={{ flexGrow: 1, minWidth: 0, overflowY: 'auto', padding: '20px 24px 40px 24px' }}>
        {view === 'plot' && <PlotDiagram />}
        {view === 'timeline' && <TimelineView />}
        {view === 'graph' && <CharacterGraph />}
        {view === 'canvas' && <InfiniteCanvas />}
        {view === 'continuity' && <ContinuityView />}
      </div>

      <SidePanel width={296}>
        <div style={{ padding: '18px 16px', overflowY: 'auto' }}>
          <SectionLabel style={{ marginBottom: 10 }}>HOW THIS IS BUILT</SectionLabel>
          <div style={{ fontSize: 12.5, lineHeight: 1.68, color: color.ink2 }}>
            Every view here is a projection over the binder. Nothing on this screen is a second copy of your manuscript,
            and nothing it shows can change a file unless you commit it explicitly.
          </div>
          <div style={{ marginTop: 18 }}>
            <SectionLabel style={{ marginBottom: 9 }}>READS FROM</SectionLabel>
            {['Scene synopses', 'Labels and status', 'Character sheets', 'Story-date metadata'].map((t) => (
              <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '5px 0', fontSize: 12.5, color: color.ink2 }}>
                <Icon name="check" size={14} strokeWidth={2.4} style={{ color: color.green }} />
                {t}
              </div>
            ))}
          </div>
          <div style={{ marginTop: 18 }}>
            <SectionLabel style={{ marginBottom: 9 }}>WRITES</SectionLabel>
            <div style={{ fontSize: 12.5, lineHeight: 1.6, color: color.ink2 }}>
              Only through <Chip fg={color.purpleDeep} bg={color.purpleTint}>canvas_to_binder</Chip>, and only after you
              confirm the change.
            </div>
          </div>
          <div style={{ marginTop: 18, paddingTop: 16, borderTop: `1px solid ${color.lineSoft}` }}>
            <SectionLabel style={{ marginBottom: 9 }}>STORED WHERE</SectionLabel>
            <div style={{ fontFamily: font.mono, fontSize: 11.5, lineHeight: 1.7, color: color.ink3 }}>
              .aegis/viz/canvas.json
              <br />
              .aegis/viz/beats.json
            </div>
            <div style={{ fontSize: 12, lineHeight: 1.6, color: color.ink4, marginTop: 8 }}>
              JSON sidecars, included in snapshots so a layout can be rolled back with the prose.
            </div>
          </div>
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}
