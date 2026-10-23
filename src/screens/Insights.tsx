import { useState } from 'react'
import { PhaseScreen, Rail, SidePanel } from '../components/PhaseScreen'
import { AsyncBlock } from '../components/AsyncBlock'
import { useApp } from '../lib/store'
import { useAsync } from '../lib/useAsync'
import {
  ai,
  diagrams,
  generationModels,
  story,
  collections as collectionsApi,
  type Diagram,
  type DiagramKind,
} from '../lib/phases'
import { isTauri } from '../lib/api'
import { words } from '../lib/format'
import { Button, Callout, Chip, SectionLabel } from '../components/ui'
import { Icon } from '../components/Icon'
import { color, font, radius, tint } from '../lib/tokens'
import { AXIS, GRID, LABEL, hueFor, linePath, scaleX, scaleY } from '../lib/viz'

type View = 'pacing' | 'clusters' | 'mindmap' | 'flowchart' | 'chrono' | 'infographic'


function useProject() {
  const path = useApp((s) => s.project?.path)
  return { path, ready: Boolean(path && isTauri) }
}

function PacingView() {
  const { path, ready } = useProject()
  const [hover, setHover] = useState<number | null>(null)
  const state = useAsync(() => story.pacing(path!), [path], ready)

  const W = 760
  const H = 300
  const pad = { l: 46, r: 20, t: 20, b: 46 }
  const x0 = pad.l
  const x1 = W - pad.r
  const y0 = pad.t
  const y1 = H - pad.b

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 12 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700, letterSpacing: '-0.01em' }}>Pacing by scene</h2>
        <span style={{ fontSize: 12.5, color: color.ink3 }}>tempo measured from sentence length</span>
      </div>

      <AsyncBlock
        state={state}
        isEmpty={(d) => d.length === 0}
        empty={{ title: 'No scenes with text yet', body: 'Pacing is measured from the prose, so a scene needs words first.' }}
      >
        {(points) => {
          const maxWords = Math.max(1, ...points.map((p) => p.wordCount))
          const maxTempo = Math.max(0.001, ...points.map((p) => p.tempo))
          const pts = points.map((p, i) => ({
            x: scaleX(i, 0, Math.max(1, points.length - 1), x0, x1),
            y: scaleY(p.tempo, 0, maxTempo, y0, y1),
          }))
          const flat = points.length > 3 && Math.max(...points.map((p) => p.variance)) < 12

          return (
            <>
              <div style={{ display: 'flex', gap: 16, marginBottom: 6 }}>
                <span style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12, color: color.ink2 }}>
                  <span style={{ width: 12, height: 10, borderRadius: 2, background: color.lineFaint }} />
                  Scene length
                </span>
                <span style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12, color: color.ink2 }}>
                  <span style={{ width: 14, height: 2.5, borderRadius: 2, background: hueFor(0) }} />
                  Tempo
                </span>
              </div>

              <svg width={W} height={H} role="img" aria-label="Tempo and scene length across the manuscript">
                {[0, 0.5, 1].map((t) => (
                  <line key={t} x1={x0} x2={x1} y1={scaleY(t * maxTempo, 0, maxTempo, y0, y1)} y2={scaleY(t * maxTempo, 0, maxTempo, y0, y1)} stroke={GRID} />
                ))}
                {points.map((p, i) => {
                  const bw = Math.max(4, (x1 - x0) / points.length - 10)
                  const cx = scaleX(i, 0, Math.max(1, points.length - 1), x0, x1)
                  const h = (p.wordCount / maxWords) * (y1 - y0)
                  return (
                    <rect
                      key={p.documentId}
                      x={cx - bw / 2}
                      y={y1 - h}
                      width={bw}
                      height={h}
                      rx={4}
                      fill={hover === i ? '#DDE4EA' : color.lineFaint}
                      onMouseEnter={() => setHover(i)}
                      onMouseLeave={() => setHover(null)}
                      style={{ cursor: 'pointer' }}
                    />
                  )
                })}
                <line x1={x0} x2={x1} y1={y1} y2={y1} stroke={AXIS} />
                <text x={(x0 + x1) / 2} y={H - 8} textAnchor="middle" fontSize={11} fill={LABEL}>
                  Scene, in binder order
                </text>
                <path d={linePath(pts)} fill="none" stroke={hueFor(0)} strokeWidth={2} strokeLinecap="round" />
                {pts.map((p, i) => (
                  <circle key={i} cx={p.x} cy={p.y} r={hover === i ? 6 : 4} fill={hueFor(0)} stroke="#FFFFFF" strokeWidth={2} />
                ))}
                {hover !== null && (
                  <g transform={`translate(${Math.min(pts[hover].x + 10, x1 - 200)} ${Math.max(pts[hover].y - 52, 4)})`}>
                    <rect width={196} height={46} rx={8} fill="#1C2B33" opacity={0.96} />
                    <text x={11} y={19} fontSize={12} fontWeight={600} fill="#FFFFFF">
                      {points[hover].title}
                    </text>
                    <text x={11} y={35} fontSize={11} fill="#C6D3DC">
                      {words(points[hover].wordCount)} words · tempo {points[hover].tempo.toFixed(2)}
                    </text>
                  </g>
                )}
              </svg>

              {flat && (
                <Callout tone="warn" style={{ marginTop: 8 }}>
                  Sentence length barely varies across these scenes. That reads flat aloud, whatever the plot is doing.
                </Callout>
              )}
              <Callout tone="quiet" style={{ marginTop: 10 }}>
                Tempo is a measured proxy — shorter sentences read faster. It is not a claim about tension, which no
                measurement can see.
              </Callout>
            </>
          )
        }}
      </AsyncBlock>
    </div>
  )
}

function ClustersView() {
  const { path, ready } = useProject()
  const [threshold, setThreshold] = useState(0.6)
  const [accepted, setAccepted] = useState<string[]>([])
  const [busy, setBusy] = useState<string | null>(null)
  const state = useAsync(() => story.clusters(path!, threshold), [path, threshold], ready)

  const accept = async (name: string, ids: string[]) => {
    if (!path) return
    setBusy(name)
    try {
      const id = await collectionsApi.create(path, name, 'manual')
      for (const documentId of ids) await collectionsApi.add(path, id, documentId)
      setAccepted((a) => [...a, name])
    } finally {
      setBusy(null)
    }
  }

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 4 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Scenes that overlap</h2>
        <div style={{ flexGrow: 1 }} />
        <span style={{ fontSize: 12, color: color.ink3 }}>threshold</span>
        <input type="range" min={30} max={95} value={threshold * 100} onChange={(e) => setThreshold(Number(e.target.value) / 100)} style={{ width: 120 }} />
        <span style={{ fontSize: 12, color: color.ink2, width: 34, fontVariantNumeric: 'tabular-nums' }}>
          {Math.round(threshold * 100)}%
        </span>
      </div>
      <div style={{ fontSize: 12.5, color: color.ink3, marginBottom: 16 }}>
        Accepting a group creates a collection — a virtual set over the same files. Nothing is copied or moved.
      </div>

      <AsyncBlock
        state={state}
        isEmpty={(d) => d.length === 0}
        empty={{
          title: 'No overlapping scenes',
          body: `No two scenes share more than ${Math.round(threshold * 100)}% of their phrasing. Lower the threshold to look for looser groupings.`,
        }}
      >
        {(clusters) => (
          <>
            {clusters.map((c) => {
              const on = accepted.includes(c.name)
              return (
                <div
                  key={c.name}
                  style={{
                    border: `1px solid ${on ? color.accent : color.line}`,
                    background: on ? tint(color.accent, 0.05) : color.surface,
                    borderRadius: radius.xl,
                    padding: '14px 16px',
                    marginBottom: 10,
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <span style={{ width: 10, height: 10, borderRadius: '50%', background: hueFor(0) }} />
                    <span style={{ fontSize: 14, fontWeight: 600 }}>{c.name}</span>
                    <Chip fg={color.ink3} bg={color.lineSoft}>{c.documentIds.length} scenes</Chip>
                    <Chip fg={color.ink3} bg={color.lineSoft}>{Math.round(c.score * 100)}% shared</Chip>
                    <div style={{ flexGrow: 1 }} />
                    {on ? (
                      <span style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 12.5, fontWeight: 600, color: color.greenInk }}>
                        <Icon name="check" size={15} strokeWidth={2.4} />
                        Collection created
                      </span>
                    ) : (
                      <Button height={30} disabled={busy === c.name} onClick={() => void accept(c.name, c.documentIds)}>
                        {busy === c.name ? 'Creating…' : 'Accept as collection'}
                      </Button>
                    )}
                  </div>
                  <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginTop: 11 }}>
                    {c.titles.map((t) => (
                      <Chip key={t} fg={color.ink2} bg={color.lineFaint} bold={false}>{t}</Chip>
                    ))}
                  </div>
                  {c.score > 0.85 && (
                    <div style={{ fontSize: 12.5, lineHeight: 1.6, color: color.amberInk, marginTop: 10 }}>
                      These share most of their phrasing. That is usually a duplicate to resolve, not a group to keep.
                    </div>
                  )}
                </div>
              )
            })}
          </>
        )}
      </AsyncBlock>
    </div>
  )
}

/**
 * Phase 9: a diagram a model actually generated from this project.
 *
 * Two things are deliberate.
 *
 * **A failed generation is an error, not an empty canvas.** A model that
 * answers with prose where a node list belongs has not made a poor diagram, it
 * has made no diagram — so the message says what it replied instead, and
 * nothing is drawn.
 *
 * **Whatever validation dropped is shown.** A node claiming a document that is
 * not in this project, an edge to a node that does not exist: those are removed
 * and listed. A diagram that quietly loses half its content while looking
 * complete is worse than one that admits what it left out.
 */
function GraphLikeView({ kind }: { kind: 'mindmap' | 'flowchart' | 'chrono' | 'infographic' }) {
  const path = useApp((s) => s.project?.path)
  const select = useApp((s) => s.select)
  const go = useApp((s) => s.go)
  const status = useAsync(() => ai.status(), [], isTauri)
  const models = status.data ? generationModels(status.data) : []

  const copy = {
    mindmap: {
      kind: 'mind_map' as DiagramKind,
      title: 'Mind map',
      body: 'Themes and threads across the book, built from your synopses. Click a node to open the document it came from.',
      placeholder: 'water and debt',
      icon: 'mindmap' as const,
    },
    flowchart: {
      kind: 'flowchart' as DiagramKind,
      title: 'Flowchart',
      body: 'What happens and what follows from it, in the order the book tells it.',
      placeholder: 'how the drought forces the crossing',
      icon: 'flowchart' as const,
    },
    chrono: {
      kind: 'chrono_flowchart' as DiagramKind,
      title: 'Chronology',
      body: 'The same events in the order they happened, not the order they are told. Built from the story dates in your own notes — the model is handed the sequence rather than asked to guess it.',
      placeholder: 'the events leading to the crossing',
      icon: 'timeline' as const,
    },
    infographic: {
      kind: 'infographic' as DiagramKind,
      title: 'Story bible sheet',
      body: 'A reference sheet — a family tree, a system’s rules, the places and how they relate. Saved into Worldbuilding as a document you own.',
      placeholder: 'the family tree of the Kharsa households',
      icon: 'infographic' as const,
    },
  }[kind]

  const [query, setQuery] = useState('')
  const [model, setModel] = useState<string | null>(null)
  const [diagram, setDiagram] = useState<Diagram | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [saved, setSaved] = useState<string | null>(null)
  const chosen = model ?? models[0]?.name ?? null

  const run = async () => {
    if (!path || !chosen || !query.trim()) return
    setBusy(true)
    setError(null)
    setDiagram(null)
    setSaved(null)
    try {
      setDiagram(await diagrams.generate(path, chosen, copy.kind, query.trim()))
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const save = async () => {
    if (!path || !diagram) return
    try {
      const doc = await diagrams.save(path, diagram)
      setSaved(doc.title)
      await useApp.getState().refreshDocs()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  const W = 760
  const H = 400

  // A ring layout: no dependency, deterministic, and it reads as a graph rather
  // than as a pile. Force-directed would look better and would also mean
  // shipping a physics engine to draw twelve circles.
  const laid = (diagram?.nodes ?? []).map((n, i, all) => {
    if (all.length === 1) return { ...n, x: W / 2, y: H / 2, r: 30 }
    // The first node sits in the middle when the graph is a hub, which is what
    // a mind map almost always is.
    const hub = kind === 'mindmap' && i === 0
    const others = all.length - (kind === 'mindmap' ? 1 : 0)
    const j = kind === 'mindmap' ? i - 1 : i
    const angle = (j / Math.max(1, others)) * Math.PI * 2 - Math.PI / 2
    return {
      ...n,
      x: hub ? W / 2 : W / 2 + Math.cos(angle) * (W * 0.34),
      y: hub ? H / 2 : H / 2 + Math.sin(angle) * (H * 0.36),
      r: hub ? 32 : 24,
    }
  })
  const at = (id: string) => laid.find((n) => n.id === id)

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 12, marginBottom: 4 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>{copy.title}</h2>
      </div>
      <div style={{ fontSize: 12.5, color: color.ink3, marginBottom: 14 }}>{copy.body}</div>

      <div style={{ display: 'flex', gap: 8, alignItems: 'center', marginBottom: 14, flexWrap: 'wrap' }}>
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && !busy && void run()}
          placeholder={copy.placeholder}
          style={{
            flexGrow: 1,
            minWidth: 220,
            fontSize: 13,
            padding: '7px 11px',
            borderRadius: radius.md,
            border: `1px solid ${color.line}`,
            background: color.surface,
          }}
        />
        {models.length > 0 && (
          <select
            value={chosen ?? ''}
            onChange={(e) => setModel(e.target.value)}
            style={{
              fontSize: 12,
              padding: '6px 9px',
              borderRadius: radius.md,
              border: `1px solid ${color.line}`,
              background: color.surface,
              color: color.ink2,
            }}
          >
            {models.map((m) => (
              <option key={m.name} value={m.name}>
                {m.name}
              </option>
            ))}
          </select>
        )}
        <Button
          icon="sparkle"
          disabled={busy || !query.trim() || !chosen || !isTauri}
          onClick={() => void run()}
        >
          {busy ? 'Generating…' : 'Generate'}
        </Button>
      </div>

      {!status.data?.reachable && isTauri && (
        <Callout tone="warn" icon="alert" style={{ marginBottom: 14 }}>
          Ollama is not running, so there is no model to generate with.
        </Callout>
      )}

      {error && (
        <Callout tone="warn" icon="alert" style={{ marginBottom: 14 }}>
          {error}
        </Callout>
      )}

      {!diagram && !busy && !error && (
        <div
          style={{
            border: `1px dashed ${color.disabled}`,
            borderRadius: radius.xl,
            background: color.surfaceSoft,
            height: 300,
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: 10,
          }}
        >
          <Icon name={copy.icon} size={32} strokeWidth={1.5} style={{ color: color.disabled }} />
          <div style={{ fontSize: 13, color: color.ink3 }}>Nothing generated yet</div>
          <div style={{ fontSize: 12, color: color.ink4 }}>
            Ask for something and a local model will build it from your binder.
          </div>
        </div>
      )}

      {diagram && (
        <>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 8 }}>
            <span style={{ fontSize: 13.5, fontWeight: 650 }}>{diagram.title}</span>
            <span style={{ fontSize: 11.5, color: color.ink4 }}>
              {diagram.nodes.length} nodes · {diagram.edges.length} edges · {diagram.model}
            </span>
            <div style={{ flexGrow: 1 }} />
            <Button variant="ghost" height={28} icon="download" onClick={() => void save()}>
              Save to binder
            </Button>
          </div>

          <svg
            width={W}
            height={H}
            role="img"
            aria-label={diagram.title}
            style={{
              maxWidth: '100%',
              border: `1px solid ${color.lineSoft}`,
              borderRadius: radius.xl,
              background: color.surfaceSoft,
            }}
          >
            {diagram.edges.map((e, i) => {
              const a = at(e.from)
              const b = at(e.to)
              if (!a || !b) return null
              return <line key={i} x1={a.x} y1={a.y} x2={b.x} y2={b.y} stroke={AXIS} strokeWidth={1.5} />
            })}
            {laid.map((n, i) => (
              <g
                key={n.id}
                style={{ cursor: n.documentId ? 'pointer' : 'default' }}
                onClick={() => {
                  if (!n.documentId) return
                  void select(n.documentId)
                  go('studio')
                }}
              >
                <title>{n.documentId ? `${n.label} — opens the document` : n.label}</title>
                <circle
                  cx={n.x}
                  cy={n.y}
                  r={n.r}
                  fill={i === 0 && kind === 'mindmap' ? hueFor(0) : '#FFFFFF'}
                  stroke={hueFor(i % 5)}
                  strokeWidth={2}
                />
                <text
                  x={n.x}
                  y={n.y + 4}
                  textAnchor="middle"
                  fontSize={11}
                  fontWeight={600}
                  fill={i === 0 && kind === 'mindmap' ? '#FFFFFF' : color.ink}
                >
                  {n.label.length > 12 ? `${n.label.slice(0, 11)}…` : n.label}
                </text>
              </g>
            ))}
          </svg>

          {saved && (
            <Callout tone="good" icon="check" style={{ marginTop: 12 }}>
              Saved as “{saved}” under Worldbuilding — Markdown with a Mermaid block, readable
              without this app.
            </Callout>
          )}

          {diagram.dropped.length > 0 && (
            <Callout tone="warn" icon="alert" style={{ marginTop: 12 }}>
              <strong style={{ fontWeight: 650 }}>
                Left out {diagram.dropped.length} thing{diagram.dropped.length === 1 ? '' : 's'}:
              </strong>
              <div style={{ marginTop: 5, fontSize: 12, lineHeight: 1.6 }}>
                {diagram.dropped.map((d, i) => (
                  <div key={i}>{d}</div>
                ))}
              </div>
            </Callout>
          )}

          <Callout tone="quiet" style={{ marginTop: 12 }}>
            This is {diagram.model}’s reading of your book, not a measurement of it. Nodes that name
            a real document open it when clicked; the rest are the model’s own connections, and
            worth checking before you rely on them.
          </Callout>
        </>
      )}
    </div>
  )
}

export function Insights() {
  const [view, setView] = useState<View>('pacing')

  return (
    <PhaseScreen
      title="Insights"
      phase="Phase 9"
      explains="Pacing and clustering are computed from your own prose on this machine, with no model. Mind maps, flowcharts and reference sheets are generated by a local model from your binder — a reading of the book, and labelled as one."
      statusRight={<span>Computed locally · network not required</span>}
    >
      <Rail width={228}>
        <div style={{ padding: '12px 8px' }}>
          <SectionLabel style={{ padding: '4px 10px 8px 10px' }}>ANALYTICS</SectionLabel>
          {(
            [
              { id: 'pacing', label: 'Pacing and gaps', icon: 'pulse' },
              { id: 'clusters', label: 'Scene clusters', icon: 'cluster' },
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
                  height: 32,
                  padding: '0 11px',
                  borderRadius: radius.lg - 1,
                  cursor: 'pointer',
                  marginBottom: 2,
                  fontSize: 13,
                  fontWeight: on ? 600 : 500,
                  background: on ? tint(color.accent, 0.1) : 'transparent',
                  color: on ? color.accent : color.ink2,
                }}
              >
                <Icon name={v.icon} size={16} />
                {v.label}
              </button>
            )
          })}

          <SectionLabel style={{ padding: '14px 10px 8px 10px' }}>DIAGRAMS</SectionLabel>
          {(
            [
              { id: 'mindmap', label: 'Mind map', icon: 'mindmap' },
              { id: 'flowchart', label: 'Flowchart', icon: 'flowchart' },
              { id: 'chrono', label: 'Chronology', icon: 'timeline' },
              { id: 'infographic', label: 'Story bible', icon: 'infographic' },
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
                  height: 32,
                  padding: '0 11px',
                  borderRadius: radius.lg - 1,
                  cursor: 'pointer',
                  marginBottom: 2,
                  fontSize: 13,
                  fontWeight: on ? 600 : 500,
                  background: on ? tint(color.accent, 0.1) : 'transparent',
                  color: on ? color.accent : color.ink2,
                }}
              >
                <Icon name={v.icon} size={16} />
                {v.label}
              </button>
            )
          })}
        </div>
        <div style={{ flexGrow: 1 }} />
        <div style={{ borderTop: `1px solid ${color.lineFaint}`, padding: '12px 14px' }}>
          <SectionLabel style={{ marginBottom: 8 }}>TOOLS</SectionLabel>
          <div style={{ fontFamily: font.mono, fontSize: 10.5, lineHeight: 1.9, color: color.ink3 }}>
            <div>suggest_clusters</div>
            <div>generate_mind_map</div>
            <div>generate_flowchart</div>
            <div>generate_infographic</div>
          </div>
        </div>
      </Rail>

      <div style={{ flexGrow: 1, minWidth: 0, overflowY: 'auto', padding: '20px 24px 40px 24px' }}>
        {view === 'pacing' && <PacingView />}
        {view === 'clusters' && <ClustersView />}
        {(view === 'mindmap' || view === 'flowchart' || view === 'chrono' || view === 'infographic') && (
          <GraphLikeView kind={view} />
        )}
      </div>

      <SidePanel width={288}>
        <div style={{ padding: '18px 16px', overflowY: 'auto' }}>
          <SectionLabel style={{ marginBottom: 10 }}>WHAT ANALYSIS COSTS</SectionLabel>
          <div style={{ fontSize: 12.5, lineHeight: 1.68, color: color.ink2 }}>
            Clustering embeds every synopsis in the project. On a 90,000-word book that is a few seconds of local compute
            and no network traffic at all.
          </div>
          <Callout tone="quiet" style={{ marginTop: 16 }}>
            Findings are a reading aid. A flat middle act is sometimes the right shape for the book you are writing.
          </Callout>
          <div style={{ marginTop: 18 }}>
            <SectionLabel style={{ marginBottom: 9 }}>NEVER DOES</SectionLabel>
            {['Duplicate a scene', 'Move a file', 'Upload anything'].map((t) => (
              <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 9, padding: '5px 0', fontSize: 12.5, color: color.ink2 }}>
                <Icon name="close" size={14} strokeWidth={2.4} style={{ color: color.ink4 }} />
                {t}
              </div>
            ))}
          </div>
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}
