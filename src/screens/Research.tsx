import { useState } from 'react'
import { PhaseScreen, Rail, SidePanel } from '../components/PhaseScreen'
import { AsyncBlock } from '../components/AsyncBlock'
import { Button, Callout, Chip, SectionLabel, Toggle } from '../components/ui'
import { Icon } from '../components/Icon'
import { useApp } from '../lib/store'
import { useAsync } from '../lib/useAsync'
import { api, isTauri } from '../lib/api'
import {
  ai,
  generationModels,
  research,
  type Brief,
  type Claim,
  type Source,
} from '../lib/phases'
import { color, font, radius } from '../lib/tokens'
import { words } from '../lib/format'

type View = 'sources' | 'claims' | 'ask'

function useProject() {
  const path = useApp((s) => s.project?.path)
  return { path, ready: Boolean(path && isTauri) }
}

/* -------------------------------------------------------------------------- */
/* Sources                                                                     */
/* -------------------------------------------------------------------------- */

function SourceList({ model }: { model: string | null }) {
  const { path, ready } = useProject()
  const sources = useAsync(() => research.sources(path!), [path], ready)
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [note, setNote] = useState<string | null>(null)

  const add = async () => {
    if (!path) return
    setError(null)
    setNote(null)
    try {
      const file = await api.pickFile?.('Add a research source')
      if (!file) return
      setBusy('adding')
      const [source, existed] = await research.register(path, file)
      setNote(
        existed
          ? `“${source.title}” was already registered — the same material is one source, however often it is offered.`
          : `Registered “${source.title}”.`,
      )
      sources.reload()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(null)
    }
  }

  const mine = async (s: Source) => {
    if (!path || !model) return
    setError(null)
    setNote(null)
    setBusy(s.id)
    try {
      const out = await research.extract(path, s.id, model)
      setNote(
        `${out.claims.length} claim${out.claims.length === 1 ? '' : 's'} from “${s.title}”` +
          (out.dropped.length ? `, ${out.dropped.length} left out.` : '.'),
      )
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(null)
    }
  }

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Sources</h2>
        <span style={{ fontSize: 12, color: color.ink4 }}>
          {sources.data?.length ?? 0} registered
        </span>
        <div style={{ flexGrow: 1 }} />
        <Button height={30} icon="plus" disabled={busy !== null || !isTauri} onClick={() => void add()}>
          {busy === 'adding' ? 'Reading…' : 'Add source'}
        </Button>
      </div>

      {note && (
        <Callout tone="quiet" icon="check" style={{ marginBottom: 12 }}>
          {note}
        </Callout>
      )}
      {error && (
        <Callout tone="warn" icon="alert" style={{ marginBottom: 12 }}>
          {error}
        </Callout>
      )}

      <AsyncBlock
        state={sources}
        isEmpty={(d) => d.length === 0}
        empty={{
          title: 'No sources yet',
          body: 'Add a paper, a report or a note. Markdown, text, HTML and DOCX are read here; a PDF needs pdftotext installed.',
        }}
      >
        {(list) => (
          <div style={{ border: `1px solid ${color.line}`, borderRadius: radius.xl, overflow: 'hidden' }}>
            {list.map((s, i) => (
              <div
                key={s.id}
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: 12,
                  padding: '12px 14px',
                  borderBottom: i < list.length - 1 ? `1px solid ${color.lineSoft}` : 'none',
                }}
              >
                <Icon
                  name={s.kind === 'web' ? 'globe' : 'doc'}
                  size={15}
                  strokeWidth={1.8}
                  style={{ color: color.ink4, marginTop: 2 }}
                />
                <div style={{ flexGrow: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 13.5, fontWeight: 600 }}>{s.title}</div>
                  <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 2 }}>
                    {[
                      s.authors.length ? s.authors.join(', ') : null,
                      s.year ? String(s.year) : null,
                      `${words(s.wordCount)} words`,
                      s.kind,
                    ]
                      .filter(Boolean)
                      .join(' · ')}
                  </div>
                  {(s.doi || s.url) && (
                    <div
                      style={{
                        fontSize: 11,
                        color: color.ink4,
                        fontFamily: font.mono,
                        marginTop: 2,
                        wordBreak: 'break-all',
                      }}
                    >
                      {s.doi ? `doi:${s.doi}` : s.url}
                    </div>
                  )}
                </div>
                <Button
                  height={27}
                  variant="ghost"
                  disabled={busy !== null || !model}
                  title={model ? 'Extract checkable claims from this source' : 'No model available'}
                  onClick={() => void mine(s)}
                >
                  {busy === s.id ? 'Reading…' : 'Extract claims'}
                </Button>
              </div>
            ))}
          </div>
        )}
      </AsyncBlock>
    </div>
  )
}

/* -------------------------------------------------------------------------- */
/* Claims                                                                      */
/* -------------------------------------------------------------------------- */

function ClaimList({ model }: { model: string | null }) {
  const { path, ready } = useProject()
  const claims = useAsync(() => research.claims(path!, null), [path], ready)
  const sources = useAsync(() => research.sources(path!), [path], ready)
  const links = useAsync(() => research.links(path!), [path], ready)
  const gaps = useAsync(() => research.gaps(path!), [path], ready)
  const [busy, setBusy] = useState(false)
  const [report, setReport] = useState<string | null>(null)

  const titleOf = (c: Claim) =>
    sources.data?.find((s) => s.id === c.sourceId)?.title ?? c.sourceId

  const check = async () => {
    if (!path || !model) return
    setBusy(true)
    try {
      const out = await research.contradictions(path, model)
      setReport(
        `${out.links.length} relationship${out.links.length === 1 ? '' : 's'} found across ` +
          `${out.examined} candidate pairs from ${out.claims} claims. ${out.method}`,
      )
      links.reload()
    } finally {
      setBusy(false)
    }
  }

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
        <h2 style={{ margin: 0, fontSize: 15, fontWeight: 700 }}>Claims</h2>
        <span style={{ fontSize: 12, color: color.ink4 }}>
          {claims.data?.length ?? 0} extracted
        </span>
        <div style={{ flexGrow: 1 }} />
        <Button
          height={30}
          variant="ghost"
          disabled={busy || !model || (claims.data?.length ?? 0) < 2}
          title="Compare claims from different sources for disagreement"
          onClick={() => void check()}
        >
          {busy ? 'Comparing…' : 'Find contradictions'}
        </Button>
      </div>

      {report && (
        <Callout tone="quiet" icon="info" style={{ marginBottom: 12 }}>
          {report}
        </Callout>
      )}

      {(links.data?.length ?? 0) > 0 && (
        <div style={{ marginBottom: 16 }}>
          <SectionLabel style={{ marginBottom: 8 }}>Where the sources disagree</SectionLabel>
          {links.data!
            .filter((l) => l.relation === 'contradicts')
            .map((l, i) => (
              <Callout key={i} tone="warn" icon="alert" style={{ marginBottom: 6 }}>
                {l.reason}
              </Callout>
            ))}
        </div>
      )}

      <AsyncBlock
        state={claims}
        isEmpty={(d) => d.length === 0}
        empty={{
          title: 'No claims yet',
          body: 'Extract claims from a source. Each one is a checkable assertion pinned to the passage that carries it.',
        }}
      >
        {(list) => (
          <div>
            {list.map((c) => (
              <div
                key={c.id}
                style={{
                  padding: '11px 0',
                  borderBottom: `1px solid ${color.lineFaint}`,
                }}
              >
                <div style={{ display: 'flex', alignItems: 'baseline', gap: 9 }}>
                  <span style={{ fontSize: 13, color: color.inkProse, flexGrow: 1 }}>{c.text}</span>
                  <Chip fg={color.ink3} bg={color.surfaceSoft}>
                    {c.kind}
                  </Chip>
                </div>
                <div style={{ fontSize: 12, color: color.ink3, marginTop: 4, fontStyle: 'italic' }}>
                  “{c.excerpt}”
                </div>
                <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 3 }}>
                  {titleOf(c)} · read by {c.extractedBy}
                </div>
              </div>
            ))}
          </div>
        )}
      </AsyncBlock>

      {(gaps.data?.length ?? 0) > 0 && (
        <div style={{ marginTop: 20 }}>
          <SectionLabel style={{ marginBottom: 8 }}>Where the corpus is thin</SectionLabel>
          {gaps.data!.map((g, i) => (
            <div key={i} style={{ fontSize: 12.5, color: color.ink3, padding: '4px 0', lineHeight: 1.55 }}>
              {g.detail}
            </div>
          ))}
          <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 8 }}>
            Counted from your own corpus. No model was asked.
          </div>
        </div>
      )}
    </div>
  )
}

/* -------------------------------------------------------------------------- */
/* Deep research                                                               */
/* -------------------------------------------------------------------------- */

/**
 * The research loop. A model is never asked what is true — only to break the
 * question up, read passages the author owns, and write prose citing the claims
 * already gathered. Every citation that cannot be resolved is removed before
 * the brief is shown, because a confident unverifiable reference is the worst
 * thing a research tool can produce.
 */
function Ask({ model }: { model: string | null }) {
  const { path } = useProject()
  const [question, setQuestion] = useState('')
  const [useWeb, setUseWeb] = useState(false)
  const [confirming, setConfirming] = useState(false)
  const [brief, setBrief] = useState<Brief | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [saved, setSaved] = useState<string | null>(null)

  const go = async () => {
    if (!path || !model || !question.trim()) return
    setBusy(true)
    setError(null)
    setBrief(null)
    setSaved(null)
    setConfirming(false)
    try {
      setBrief(await research.run(path, question.trim(), model, useWeb))
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const claimCount = brief?.findings.reduce((n, f) => n + f.claims.length, 0) ?? 0
  const dropped = brief?.findings.flatMap((f) => f.dropped) ?? []

  return (
    <div style={{ maxWidth: 760 }}>
      <h2 style={{ margin: '0 0 4px 0', fontSize: 15, fontWeight: 700 }}>Ask the corpus</h2>
      <div style={{ fontSize: 12.5, color: color.ink3, marginBottom: 14, lineHeight: 1.6 }}>
        The question is broken into parts, each answered from your own sources, and every sentence
        in the answer cites the claim it rests on.
      </div>

      <div style={{ display: 'flex', gap: 8, marginBottom: 12 }}>
        <input
          value={question}
          onChange={(e) => setQuestion(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && !busy && (useWeb ? setConfirming(true) : void go())}
          placeholder="How does logical replication differ from physical replication?"
          style={{
            flexGrow: 1,
            fontSize: 13.5,
            padding: '8px 12px',
            borderRadius: radius.md,
            border: `1px solid ${color.line}`,
            background: color.surface,
          }}
        />
        <Button
          icon="search"
          disabled={busy || !question.trim() || !model}
          onClick={() => (useWeb ? setConfirming(true) : void go())}
        >
          {busy ? 'Researching…' : 'Research'}
        </Button>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '9px 12px',
          borderRadius: radius.lg,
          background: useWeb ? color.orangeTint : color.surfaceSoft,
          border: `1px solid ${useWeb ? color.orangeMark : color.lineSoft}`,
          marginBottom: 14,
        }}
      >
        <div style={{ flexGrow: 1, fontSize: 12.5, color: color.inkProse, lineHeight: 1.5 }}>
          {useWeb ? (
            <>
              <strong style={{ fontWeight: 650 }}>Each sub-question will be sent to the web</strong>{' '}
              through your own wigolo. Those queries leave this machine and are recorded in the
              research trace.
            </>
          ) : (
            'Searching your own sources only. Nothing leaves this machine.'
          )}
        </div>
        <Toggle on={useWeb} onChange={() => setUseWeb(!useWeb)} tone={color.orangeInk} />
      </div>

      {confirming && (
        <Callout tone="warn" icon="alert" style={{ marginBottom: 14 }}>
          This will send your sub-questions to search engines through wigolo.
          <div style={{ display: 'flex', gap: 8, marginTop: 10 }}>
            <Button height={28} onClick={() => void go()}>
              Yes, search the web
            </Button>
            <Button height={28} variant="ghost" onClick={() => setConfirming(false)}>
              Cancel
            </Button>
          </div>
        </Callout>
      )}

      {error && (
        <Callout tone="warn" icon="alert" style={{ marginBottom: 14 }}>
          {error}
        </Callout>
      )}

      {brief && (
        <>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 8 }}>
            <SectionLabel>Answer</SectionLabel>
            <span style={{ fontSize: 11.5, color: color.ink4 }}>
              {brief.plan.length} sub-questions · {claimCount} claims · {brief.bibliography.length}{' '}
              sources
            </span>
            <div style={{ flexGrow: 1 }} />
            <Button
              height={27}
              variant="ghost"
              icon="download"
              onClick={async () => {
                if (!path) return
                const doc = await research.save(path, brief)
                setSaved(doc.title)
                await useApp.getState().refreshDocs()
              }}
            >
              Save to binder
            </Button>
          </div>

          {brief.synthesis ? (
            <div
              style={{
                padding: '13px 15px',
                borderRadius: radius.lg,
                background: color.surfaceSoft,
                border: `1px solid ${color.lineSoft}`,
                fontSize: 14,
                lineHeight: 1.72,
                fontFamily: font.prose,
                color: color.inkProse,
              }}
            >
              {brief.synthesis}
            </div>
          ) : (
            <Callout tone="warn" icon="alert">
              Nothing in the corpus supported a claim about this question. Rather than write around
              that, the brief says so.
            </Callout>
          )}

          {saved && (
            <Callout tone="quiet" icon="check" style={{ marginTop: 12 }}>
              Saved as “{saved}” under Research — attributed to {brief.model}, not to you.
            </Callout>
          )}

          {brief.contradictions.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <SectionLabel style={{ marginBottom: 8 }}>Where the sources disagree</SectionLabel>
              {brief.contradictions.map((l, i) => (
                <Callout key={i} tone="warn" icon="alert" style={{ marginBottom: 6 }}>
                  {l.reason}
                </Callout>
              ))}
            </div>
          )}

          <div style={{ marginTop: 16 }}>
            <SectionLabel style={{ marginBottom: 8 }}>Evidence</SectionLabel>
            {claimCount === 0 && (
              <div style={{ fontSize: 12.5, color: color.ink4 }}>None was found.</div>
            )}
            {brief.findings.map((f, fi) =>
              f.claims.map((c, ci) => {
                const n =
                  brief.findings.slice(0, fi).reduce((t, x) => t + x.claims.length, 0) + ci + 1
                return (
                  <div key={c.id} style={{ padding: '7px 0', borderBottom: `1px solid ${color.lineFaint}` }}>
                    <div style={{ fontSize: 12.5, color: color.inkProse }}>
                      <span style={{ color: color.ink4, fontFamily: font.mono, marginRight: 6 }}>
                        [{n}]
                      </span>
                      {c.text}
                    </div>
                    <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 2, fontStyle: 'italic' }}>
                      “{c.excerpt}”
                    </div>
                  </div>
                )
              }),
            )}
          </div>

          {brief.bibliography.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <SectionLabel style={{ marginBottom: 8 }}>Sources</SectionLabel>
              {brief.bibliography.map(([id, line]) => (
                <div key={id} style={{ fontSize: 12.5, color: color.ink2, padding: '3px 0' }}>
                  {line}
                </div>
              ))}
            </div>
          )}

          {dropped.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <SectionLabel style={{ marginBottom: 8 }}>Left out ({dropped.length})</SectionLabel>
              {dropped.map((d, i) => (
                <div key={i} style={{ fontSize: 12, color: color.ink3, padding: '3px 0', lineHeight: 1.5 }}>
                  {d}
                </div>
              ))}
            </div>
          )}

          <div style={{ marginTop: 16, fontSize: 11.5, color: color.ink4, lineHeight: 1.6 }}>
            {brief.method}
          </div>
        </>
      )}
    </div>
  )
}

/* -------------------------------------------------------------------------- */

const VIEWS: { id: View; label: string; sub: string; icon: 'doc' | 'checklist' | 'search' }[] = [
  { id: 'sources', label: 'Sources', sub: 'what you have read', icon: 'doc' },
  { id: 'claims', label: 'Claims', sub: 'what they assert', icon: 'checklist' },
  { id: 'ask', label: 'Ask', sub: 'research a question', icon: 'search' },
]

export function Research() {
  const [view, setView] = useState<View>('sources')
  const status = useAsync(() => ai.status(), [], isTauri)
  const models = status.data ? generationModels(status.data) : []
  const [model, setModel] = useState<string | null>(null)
  const chosen = model ?? models[0]?.name ?? null

  return (
    <PhaseScreen
      title="Research"
      phase="Knowledge"
      explains="Sources are registered with a stable identity, claims are extracted and verified word-for-word against the passage they came from, and every answer cites the claim it rests on. Local by default."
      statusRight={<span>{chosen ?? 'no model'}</span>}
      right={
        models.length > 0 ? (
          <select
            value={chosen ?? ''}
            onChange={(e) => setModel(e.target.value)}
            style={{
              fontSize: 11.5,
              color: color.ink2,
              background: color.surface,
              border: `1px solid ${color.line}`,
              borderRadius: 999,
              padding: '3px 8px',
            }}
          >
            {models.map((m) => (
              <option key={m.name} value={m.name}>
                {m.name}
              </option>
            ))}
          </select>
        ) : undefined
      }
    >
      <Rail width={210}>
        <div style={{ padding: '12px 10px', display: 'flex', flexDirection: 'column', gap: 3 }}>
          {VIEWS.map((v) => {
            const on = v.id === view
            return (
              <button
                key={v.id}
                onClick={() => setView(v.id)}
                className={`row ${on ? 'on' : ''}`}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  padding: '9px 11px',
                  borderRadius: radius.lg,
                  cursor: 'pointer',
                  textAlign: 'left',
                  background: on ? color.blueTint : 'transparent',
                }}
              >
                <Icon
                  name={v.icon}
                  size={15}
                  strokeWidth={1.9}
                  style={{ color: on ? color.accent : color.ink4 }}
                />
                <span style={{ minWidth: 0 }}>
                  <span style={{ display: 'block', fontSize: 13, fontWeight: on ? 600 : 500 }}>
                    {v.label}
                  </span>
                  <span style={{ display: 'block', fontSize: 11.5, color: color.ink4, marginTop: 1 }}>
                    {v.sub}
                  </span>
                </span>
              </button>
            )
          })}
        </div>
      </Rail>

      <div style={{ flexGrow: 1, minWidth: 0, overflowY: 'auto', padding: '20px 24px 40px 24px' }}>
        {view === 'sources' && <SourceList model={chosen} />}
        {view === 'claims' && <ClaimList model={chosen} />}
        {view === 'ask' && <Ask model={chosen} />}
      </div>

      <SidePanel width={290}>
        <div style={{ padding: '18px 16px', overflowY: 'auto' }}>
          <SectionLabel style={{ marginBottom: 10 }}>HOW THIS WORKS</SectionLabel>
          <div style={{ fontSize: 12.5, lineHeight: 1.68, color: color.ink2 }}>
            A model is never asked what is true. It is asked to break a question up, to read a
            passage you own and say what it asserts, and to write prose citing the claims already
            gathered.
          </div>
          <div style={{ marginTop: 18 }}>
            <SectionLabel style={{ marginBottom: 9 }}>WHAT IS CHECKED</SectionLabel>
            {[
              'Every claim quotes its source word for word, or it is dropped',
              'Every citation resolves to a gathered claim, or it is removed',
              'The same material is one source, however often it is added',
              'Everything left out is listed, never silently lost',
            ].map((t) => (
              <div
                key={t}
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: 9,
                  padding: '5px 0',
                  fontSize: 12.5,
                  color: color.ink2,
                  lineHeight: 1.5,
                }}
              >
                <Icon
                  name="check"
                  size={14}
                  strokeWidth={2.4}
                  style={{ color: color.green, marginTop: 2, flexShrink: 0 }}
                />
                {t}
              </div>
            ))}
          </div>
          <div style={{ marginTop: 18, paddingTop: 16, borderTop: `1px solid ${color.lineSoft}` }}>
            <SectionLabel style={{ marginBottom: 9 }}>STORED WHERE</SectionLabel>
            <div style={{ fontFamily: font.mono, fontSize: 11.5, lineHeight: 1.7, color: color.ink3 }}>
              research/sources/
              <br />
              project.sqlite
              <br />
              research-trace.json
            </div>
            <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 8, lineHeight: 1.55 }}>
              Extracted text is a plain file in your project. It outlives this app.
            </div>
          </div>
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}
