import { useState } from 'react'
import { PhaseScreen, SidePanel } from '../components/PhaseScreen'
import { AsyncBlock } from '../components/AsyncBlock'
import { Button, Callout, Chip, ProgressBar, SectionLabel, Tabs } from '../components/ui'
import { useApp } from '../lib/store'
import { useAsync } from '../lib/useAsync'
import { integrity, lineage, sidecar, type WebSearchResult } from '../lib/phases'
import { color, font, radius } from '../lib/tokens'
import { api } from '../lib/api'
import { words } from '../lib/format'
import { isTauri } from '../lib/api'

type Tab = 'similarity' | 'signals' | 'rubric' | 'web' | 'authorship'

function Ring({ pct, tone, label, sub }: { pct: number; tone: string; label: string; sub: string }) {
  const safe = Math.max(0, Math.min(100, pct))
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 11 }}>
      <div
        style={{
          width: 38,
          height: 38,
          borderRadius: '50%',
          background: `conic-gradient(${tone} 0% ${safe}%, ${color.lineFaint} ${safe}% 100%)`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <span
          style={{
            width: 29,
            height: 29,
            borderRadius: '50%',
            background: color.surface,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: 11.5,
            fontWeight: 700,
            color: tone,
          }}
        >
          {Math.round(safe)}
        </span>
      </div>
      <div>
        <div style={{ fontSize: 12.5, fontWeight: 600 }}>{label}</div>
        <div style={{ fontSize: 11.5, color: color.ink4 }}>{sub}</div>
      </div>
    </div>
  )
}

/**
 * Phase 4: the one screen in AegisDraft where the author's words leave the
 * machine.
 *
 * Three things are load-bearing here and none of them are decoration:
 *
 * 1. **It says "leaves this machine", not "local".** wigolo listens on
 *    loopback, but it queries the internet on the author's behalf. Calling that
 *    local would be technically arguable and practically a lie.
 * 2. **The word count is shown before the send, not after.** The author sees
 *    the size of what is about to go.
 * 3. **wigolo's evidence score and our overlap stay in separate columns.** They
 *    answer different questions and averaging them would invent a number.
 */
function WebTab({ path, docBody }: { path: string; docBody: string }) {
  const wigolo = useAsync(() => sidecar.status(), [], isTauri)
  const [starting, setStarting] = useState(false)
  const [phrase, setPhrase] = useState('')
  const [confirming, setConfirming] = useState(false)
  const [result, setResult] = useState<WebSearchResult | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const trace = useAsync(() => integrity.trace(path), [path, result], isTauri)

  // The longest sentences are the ones worth checking: a short one matches the
  // whole web by coincidence, and says nothing.
  const candidates = docBody
    .split(/(?<=[.!?])\s+/)
    .map((s) => s.trim())
    .filter((s) => {
      const n = s.split(/\s+/).length
      return n >= 8 && n <= 60
    })
    .slice(0, 40)

  const wordCount = phrase.trim() ? phrase.trim().split(/\s+/).length : 0

  const send = async () => {
    setBusy(true)
    setError(null)
    setConfirming(false)
    try {
      setResult(await integrity.webSimilarity(path, phrase.trim(), true))
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const status = wigolo.data

  return (
    <div style={{ maxWidth: 720 }}>
      <Callout tone={status?.running ? 'quiet' : 'warn'} icon={status?.running ? 'globe' : 'alert'}>
        {status?.running ? (
          <>
            wigolo is running at <span style={{ fontFamily: font.mono }}>{status.base}</span>
            {status.health ? ` (${status.health})` : ''}
            {status.managed ? ', started by AegisDraft and stopped when it quits' : ''}. It listens
            on your machine, but it queries the internet for you —{' '}
            <strong style={{ fontWeight: 650 }}>an approved phrase does leave this machine</strong>,
            through a component you run and can inspect.
          </>
        ) : (
          <>
            <div>
              {status?.resolution.detail ?? 'Checking for wigolo…'} Web checks need it; everything
              else on this screen works without a network.
            </div>
            {status?.resolution.launcher && (
              <div style={{ marginTop: 9, display: 'flex', alignItems: 'center', gap: 9 }}>
                <Button
                  height={28}
                  disabled={starting}
                  onClick={async () => {
                    setStarting(true)
                    try {
                      await sidecar.start()
                      wigolo.reload()
                    } catch (e) {
                      setError(e instanceof Error ? e.message : String(e))
                    } finally {
                      setStarting(false)
                    }
                  }}
                >
                  {starting ? 'Starting…' : 'Start wigolo'}
                </Button>
                <span style={{ fontSize: 11.5, color: color.ink4 }}>
                  {status.resolution.launcher.via === 'npx'
                    ? 'First start downloads about 1.5 GB into ~/.wigolo.'
                    : 'Runs on loopback and stops when AegisDraft quits.'}
                </span>
              </div>
            )}
          </>
        )}
      </Callout>

      <SectionLabel style={{ margin: '18px 0 8px' }}>Pick one sentence</SectionLabel>
      {candidates.length === 0 ? (
        <div style={{ fontSize: 12.5, color: color.ink4 }}>
          No sentence in this document is long enough to be worth checking. A short phrase matches
          the whole web by coincidence.
        </div>
      ) : (
        <div
          style={{
            maxHeight: 190,
            overflowY: 'auto',
            border: `1px solid ${color.lineSoft}`,
            borderRadius: radius.md,
          }}
        >
          {candidates.map((s, i) => (
            <button
              key={i}
              onClick={() => {
                setPhrase(s)
                setResult(null)
              }}
              style={{
                display: 'block',
                width: '100%',
                textAlign: 'left',
                padding: '8px 11px',
                fontSize: 12.5,
                lineHeight: 1.5,
                cursor: 'pointer',
                color: phrase === s ? color.accent : color.inkProse,
                background: phrase === s ? color.blueTint : 'transparent',
                borderBottom: `1px solid ${color.lineFaint}`,
              }}
            >
              {s}
            </button>
          ))}
        </div>
      )}

      {phrase && (
        <div
          style={{
            marginTop: 14,
            padding: '12px 14px',
            border: `1px solid ${color.orangeMark}`,
            borderRadius: radius.md,
            background: color.orangeTint,
          }}
        >
          <div style={{ fontSize: 12.5, lineHeight: 1.6, color: color.inkProse }}>
            <strong style={{ fontWeight: 650 }}>{wordCount} words</strong> will be sent to wigolo at{' '}
            <span style={{ fontFamily: font.mono }}>{status?.base ?? '—'}</span>, which will query
            search engines on the internet. Nothing else from this document goes with it.
          </div>
          <div style={{ marginTop: 10, display: 'flex', alignItems: 'center', gap: 8 }}>
            {confirming ? (
              <>
                <Button onClick={() => void send()} disabled={busy || !status?.running}>
                  {busy ? 'Sending…' : 'Yes, send this phrase'}
                </Button>
                <Button variant="ghost" onClick={() => setConfirming(false)}>
                  Cancel
                </Button>
              </>
            ) : (
              <Button
                onClick={() => setConfirming(true)}
                disabled={!status?.running || wordCount === 0}
                title={status?.running ? undefined : 'wigolo is not running'}
              >
                Check this phrase on the web
              </Button>
            )}
          </div>
        </div>
      )}

      {error && (
        <Callout tone="warn" icon="alert" style={{ marginTop: 14 }}>
          {error}
        </Callout>
      )}

      {result && (
        <div style={{ marginTop: 20 }}>
          <SectionLabel style={{ marginBottom: 8 }}>
            {result.matches.length} {result.matches.length === 1 ? 'source' : 'sources'}
            {result.engines.length > 0 && ` · ${result.engines.join(', ')}`}
          </SectionLabel>
          {result.warning && (
            <Callout tone="warn" icon="alert" style={{ marginBottom: 12 }}>
              wigolo reported: {result.warning}
            </Callout>
          )}
          {result.matches.length === 0 ? (
            <div style={{ fontSize: 12.5, color: color.ink3, lineHeight: 1.6 }}>
              Nothing came back. That is weak evidence of originality, not proof — search engines
              do not see everything.
            </div>
          ) : (
            result.matches.map((m) => (
              <div
                key={m.url}
                style={{
                  padding: '11px 0',
                  borderBottom: `1px solid ${color.lineFaint}`,
                }}
              >
                <div style={{ display: 'flex', alignItems: 'baseline', gap: 10 }}>
                  <span style={{ fontSize: 13, fontWeight: 650, flexGrow: 1, minWidth: 0 }}>
                    {m.title}
                  </span>
                  <Chip
                    fg={m.overlap > 0.5 ? color.orangeInk : color.ink3}
                    bg={m.overlap > 0.5 ? color.orangeTint : color.surfaceSoft}
                  >
                    {Math.round(m.overlap * 100)}% of your phrase
                  </Chip>
                  {m.evidenceScore !== null && (
                    <Chip fg={color.ink3} bg={color.surfaceSoft}>
                      wigolo {m.evidenceScore.toFixed(2)}
                    </Chip>
                  )}
                </div>
                <div style={{ fontSize: 11.5, color: color.ink4, fontFamily: font.mono, marginTop: 2, wordBreak: 'break-all' }}>
                  {m.url}
                  {m.citationId && ` · ${m.citationId}`}
                  {m.sourceSpan && ` · bytes ${m.sourceSpan[0]}–${m.sourceSpan[1]}`}
                </div>
                {m.excerpt && (
                  <div style={{ fontSize: 12.5, color: color.ink2, marginTop: 5, lineHeight: 1.55 }}>
                    {m.excerpt}
                  </div>
                )}
              </div>
            ))
          )}
          <div style={{ marginTop: 12, fontSize: 11.5, color: color.ink4, lineHeight: 1.55 }}>
            The percentage is measured here — three-word runs of your phrase found in the excerpt.
            wigolo&rsquo;s own score rates the result, not the resemblance; the two are kept apart
            on purpose.
          </div>
        </div>
      )}

      {(trace.data?.length ?? 0) > 0 && (
        <div style={{ marginTop: 24 }}>
          <SectionLabel style={{ marginBottom: 8 }}>
            Everything this project has sent ({trace.data!.length})
          </SectionLabel>
          {trace.data!.map((t, i) => (
            <div
              key={i}
              style={{
                fontSize: 12,
                color: color.ink3,
                padding: '5px 0',
                borderBottom: `1px solid ${color.lineFaint}`,
                lineHeight: 1.5,
              }}
            >
              <span style={{ color: color.ink4, fontFamily: font.mono, marginRight: 8 }}>
                {t.wordCount}w
              </span>
              {t.query}
            </div>
          ))}
          <div style={{ marginTop: 8, fontSize: 11.5, color: color.ink4 }}>
            Kept in <span style={{ fontFamily: font.mono }}>research-trace.json</span> in your
            project folder. A phrase not on this list was never sent.
          </div>
        </div>
      )}
    </div>
  )
}

/**
 * Authorship, derived from the ledger rather than guessed from the prose.
 *
 * This is what replaced AI detection. The numbers here are not a classifier's
 * opinion about writing style — they are counts of what was actually recorded
 * while the text was written. Where nothing was recorded, the screen says so and
 * counts it separately, because silence is not evidence of a human author.
 */
function AuthorshipTab({ path, documentId }: { path: string; documentId: string | null }) {
  const summary = useAsync(() => lineage.summary(path, documentId), [path, documentId], isTauri)
  const chain = useAsync(() => lineage.verify(path), [path], isTauri)
  const [busy, setBusy] = useState(false)

  const s = summary.data
  const rows: [string, number, string][] = s
    ? [
        ['Yours', s.human, color.green],
        ['Model-written', s.ai, color.purple],
        ['Imported', s.external, color.orangeInk],
        ['Mixed', s.mixed, color.blueDeep],
        ['No record', s.unknown, color.disabled],
      ]
    : []

  return (
    <div style={{ maxWidth: 640 }}>
      <Callout tone="quiet" icon="shield" style={{ marginBottom: 18 }}>
        Counted from what was recorded as the text was written — not inferred from how it reads.
        AegisDraft ships no AI-writing detector, and these numbers are not one.
      </Callout>

      {!s || s.blocks === 0 ? (
        <Callout tone="quiet" icon="info">
          Nothing recorded for this document yet. Saving it gives each paragraph a stable identity
          and starts its history.
        </Callout>
      ) : (
        <>
          <div style={{ display: 'flex', gap: 3, height: 10, borderRadius: 5, overflow: 'hidden', marginBottom: 12 }}>
            {rows
              .filter(([, n]) => n > 0)
              .map(([label, n, c]) => (
                <div key={label} style={{ flexGrow: n, background: c }} title={`${label}: ${n}`} />
              ))}
          </div>

          <div style={{ border: `1px solid ${color.line}`, borderRadius: radius.xl, overflow: 'hidden' }}>
            {rows.map(([label, n, c], i) => (
              <div
                key={label}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 11,
                  padding: '10px 14px',
                  borderBottom: i < rows.length - 1 ? `1px solid ${color.lineSoft}` : 'none',
                }}
              >
                <span style={{ width: 9, height: 9, borderRadius: 3, background: c, flexShrink: 0 }} />
                <span style={{ fontSize: 13, flexGrow: 1 }}>{label}</span>
                <span style={{ fontSize: 13, fontVariantNumeric: 'tabular-nums', color: color.ink2 }}>
                  {n}
                </span>
              </div>
            ))}
          </div>

          {s.unknown > 0 && (
            <div style={{ fontSize: 12.5, color: color.ink3, marginTop: 10, lineHeight: 1.6 }}>
              <strong style={{ fontWeight: 650 }}>{s.unknown} paragraphs have no record.</strong>{' '}
              That is normal for text written before the ledger existed, or edited in another
              editor. It is not evidence that anyone in particular wrote them, and it is counted
              on its own line rather than added to yours.
            </div>
          )}

          {s.uncertain > 0 && (
            <div style={{ fontSize: 12.5, color: color.ink3, marginTop: 8, lineHeight: 1.6 }}>
              {s.uncertain} have a lineage that was partly reconstructed by matching text after an
              edit made outside AegisDraft, rather than observed as it happened.
            </div>
          )}

          {s.models.length > 0 && (
            <div style={{ fontSize: 12.5, color: color.ink3, marginTop: 8 }}>
              Models involved: {s.models.join(', ')}.
            </div>
          )}

          <div style={{ marginTop: 18, display: 'flex', alignItems: 'center', gap: 10 }}>
            <Chip
              fg={chain.data === null ? color.greenInk : color.orangeInk}
              bg={chain.data === null ? color.greenTint : color.orangeTint}
            >
              {chain.loading ? 'checking…' : chain.data === null ? 'Ledger intact' : 'Ledger altered'}
            </Chip>
            <Button
              height={28}
              variant="ghost"
              disabled={busy}
              title="Re-read files changed outside AegisDraft"
              onClick={async () => {
                setBusy(true)
                try {
                  await lineage.reconcile(path)
                  summary.reload()
                  chain.reload()
                } finally {
                  setBusy(false)
                }
              }}
            >
              {busy ? 'Reconciling…' : 'Re-check files'}
            </Button>
          </div>

          {chain.data && (
            <Callout tone="warn" icon="alert" style={{ marginTop: 12 }}>
              {chain.data.detail} The ledger is tamper-evident, not tamper-proof: anyone who can
              write the project database can rebuild it, so this tells you something changed, not
              who changed it.
            </Callout>
          )}
        </>
      )}
    </div>
  )
}

export function Integrity() {
  const [tab, setTab] = useState<Tab>('similarity')
  const [threshold, setThreshold] = useState(0.5)
  const path = useApp((s) => s.project?.path)
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const ready = Boolean(path && doc && isTauri)

  const sim = useAsync(() => integrity.similarity(path!, doc!.id, threshold), [path, doc?.id, threshold], ready)
  const sig = useAsync(() => integrity.signals(path!, doc!.id), [path, doc?.id], ready)
  const rub = useAsync(() => integrity.rubric(path!, doc!.id), [path, doc?.id], ready)

  const overall = rub.data ? Math.round(rub.data.reduce((n, r) => n + r.score, 0) / rub.data.length) : 0
  const reuse = sim.data?.length ?? 0
  const body = useApp((s) => s.body)
  const [exported, setExported] = useState<string | null>(null)

  // The packet is the thing an author hands to someone else, so it goes through
  // the same save dialog as a compile: chosen location, no silent overwrite.
  const exportPacket = async () => {
    if (!path || !doc) return
    const out = await api.pickSaveFile('Save the integrity report', `${doc.title} — integrity.json`)
    if (!out) return
    const [json, html] = await integrity.exportPacket(path, doc.id, threshold, out)
    setExported(`${json} and ${html}`)
  }

  return (
    <PhaseScreen
      title="Integrity"
      phase="Phase 4"
      explains="Reuse, measurements and rubric are computed here, on this machine, from the document you have open. They describe the prose. AegisDraft ships no AI-writing detector and makes no claim about who or what wrote anything."
      right={
        <div style={{ display: 'flex', gap: 8 }}>
          <Button height={28} variant="ghost" icon="download" onClick={() => void exportPacket()} disabled={!doc || !isTauri}>
            Export packet
          </Button>
          <Button height={28} icon="refresh" onClick={() => { sim.reload(); sig.reload(); rub.reload() }}>Re-run</Button>
        </div>
      }
      statusRight={<span>{doc ? doc.title : 'No document selected'}</span>}
    >
      <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <div
          style={{
            height: 62,
            flexShrink: 0,
            display: 'flex',
            alignItems: 'center',
            gap: 26,
            padding: '0 22px',
            borderBottom: `1px solid ${color.line}`,
          }}
        >
          <Ring
            pct={reuse === 0 ? 0 : Math.min(100, reuse * 20)}
            tone={reuse ? color.orangeInk : color.greenInk}
            label="Reused passages"
            sub={reuse === 0 ? 'none found' : `${reuse} above ${Math.round(threshold * 100)}%`}
          />
          <Ring
            pct={sig.data ? Math.min(100, sig.data.typeTokenRatio * 160) : 0}
            tone={color.accent}
            label="Vocabulary range"
            sub={sig.data ? `${words(sig.data.wordCount)} words` : '—'}
          />
          <Ring pct={overall} tone={overall >= 60 ? color.greenInk : color.yellowInk} label="Rubric" sub={`${rub.data?.length ?? 0} criteria`} />
          <div style={{ flexGrow: 1 }} />
          {!isTauri && <Chip fg={color.yellowInk} bg={color.amberMark}>Needs the desktop app</Chip>}
        </div>

        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '20px 24px 40px 24px' }}>
          {exported && (
            <Callout tone="quiet" icon="check" style={{ marginBottom: 16 }}>
              Written to {exported}. The JSON carries the numbers; the HTML is the version a reader
              opens.
            </Callout>
          )}
          {!doc ? (
            <Callout tone="quiet">Select a document in the binder to check it.</Callout>
          ) : tab === 'authorship' ? (
            <AuthorshipTab path={path!} documentId={doc.id} />
          ) : tab === 'web' ? (
            <WebTab path={path!} docBody={body} />
          ) : tab === 'similarity' ? (
            <AsyncBlock
              state={sim}
              isEmpty={(d) => d.length === 0}
              empty={{
                title: 'Nothing reused',
                body: `No passage in “${doc.title}” repeats phrasing from elsewhere in the project above ${Math.round(threshold * 100)}%.`,
              }}
            >
              {(matches) => (
                <div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 16 }}>
                    <SectionLabel>PASSAGES ALSO FOUND ELSEWHERE</SectionLabel>
                    <div style={{ flexGrow: 1 }} />
                    <span style={{ fontSize: 12, color: color.ink3 }}>threshold</span>
                    <input
                      type="range"
                      min={20}
                      max={95}
                      value={threshold * 100}
                      onChange={(e) => setThreshold(Number(e.target.value) / 100)}
                      style={{ width: 120 }}
                    />
                    <span style={{ fontSize: 12, color: color.ink2, width: 34, fontVariantNumeric: 'tabular-nums' }}>
                      {Math.round(threshold * 100)}%
                    </span>
                  </div>
                  {matches.map((m, i) => (
                    <div
                      key={`${m.sourceId}-${i}`}
                      style={{
                        border: `1px solid ${color.line}`,
                        borderRadius: radius.xl,
                        padding: '14px 16px',
                        marginBottom: 10,
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 9 }}>
                        <span style={{ width: 9, height: 9, borderRadius: 3, background: color.accent }} />
                        <span style={{ fontSize: 13.5, fontWeight: 600 }}>Also in “{m.sourceTitle}”</span>
                        <div style={{ flexGrow: 1 }} />
                        <Chip fg={color.blueDeep} bg={color.blueTint}>
                          {Math.round(m.score * 100)}% shared
                        </Chip>
                      </div>
                      <div style={{ fontFamily: font.prose, fontSize: 14.5, lineHeight: 1.65, color: color.inkProse }}>
                        {m.excerpt}
                      </div>
                    </div>
                  ))}
                  <Callout tone="quiet" style={{ marginTop: 14 }}>
                    Reuse is measured by shared five-word sequences against your own project. Nothing is sent anywhere,
                    and repeating yourself is sometimes deliberate.
                  </Callout>
                </div>
              )}
            </AsyncBlock>
          ) : tab === 'signals' ? (
            <AsyncBlock state={sig} empty={{ title: 'Nothing to measure' }}>
              {(s) => (
                <div style={{ maxWidth: 620 }}>
                  <Callout tone="quiet" icon="info" style={{ marginBottom: 18 }}>
                    These describe the prose — how long the sentences run, how much the vocabulary
                    varies, how abstract the words are. They are not a judgement about who wrote it,
                    and AegisDraft will not make one.
                  </Callout>
                  {[
                    ['Words', words(s.wordCount), null],
                    ['Sentences', String(s.sentenceCount), null],
                    ['Mean sentence length', `${s.meanSentenceLength.toFixed(1)} words`, null],
                    [
                      'Sentence length variance',
                      s.sentenceLengthVariance.toFixed(1),
                      s.sentenceLengthVariance < 10 && s.sentenceCount > 3
                        ? 'Sentences are close to the same length, which reads flat aloud.'
                        : null,
                    ],
                    [
                      'Distinct words',
                      `${Math.round(s.typeTokenRatio * 100)}%`,
                      s.typeTokenRatio < 0.35 && s.wordCount > 100 ? 'A narrow vocabulary for this length.' : null,
                    ],
                    [
                      'Abstraction density',
                      `${(s.abstractDensity * 100).toFixed(1)}%`,
                      s.abstractDensity > 0.01 ? 'Abstractions are standing in for things a reader could picture.' : null,
                    ],
                  ].map(([label, value, note]) => (
                    <div key={label as string} style={{ padding: '11px 0', borderBottom: `1px solid ${color.lineSoft}` }}>
                      <div style={{ display: 'flex', alignItems: 'baseline' }}>
                        <span style={{ fontSize: 13.5, color: color.ink2 }}>{label}</span>
                        <div style={{ flexGrow: 1 }} />
                        <span style={{ fontSize: 13.5, fontWeight: 600, fontVariantNumeric: 'tabular-nums' }}>{value}</span>
                      </div>
                      {note && <div style={{ fontSize: 12, color: color.yellowInk, marginTop: 5 }}>{note}</div>}
                    </div>
                  ))}
                </div>
              )}
            </AsyncBlock>
          ) : (
            <AsyncBlock state={rub} empty={{ title: 'Nothing to score' }}>
              {(scores) => (
                <div style={{ maxWidth: 620 }}>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 9, marginBottom: 18 }}>
                    <span style={{ fontSize: 27, fontWeight: 700, color: overall >= 60 ? color.greenInk : color.yellowInk }}>
                      {overall}
                    </span>
                    <span style={{ fontSize: 12.5, color: color.ink3 }}>/ 100 across {scores.length} criteria</span>
                  </div>
                  {scores.map((r) => (
                    <div key={r.criterion} style={{ marginBottom: 16 }}>
                      <div style={{ display: 'flex', alignItems: 'baseline', marginBottom: 6 }}>
                        <span style={{ fontSize: 13, fontWeight: 600 }}>{r.criterion}</span>
                        <div style={{ flexGrow: 1 }} />
                        <span style={{ fontSize: 12.5, fontWeight: 600, color: color.ink2 }}>{r.score}</span>
                      </div>
                      <ProgressBar pct={r.score} tone={r.score >= 60 ? color.green : color.yellow} />
                      <div style={{ fontSize: 12, color: color.ink3, marginTop: 6 }}>{r.note}</div>
                    </div>
                  ))}
                  <Callout tone="quiet" style={{ marginTop: 18 }}>
                    Scores are a reading aid. Nothing here decides whether the chapter is finished — you do.
                  </Callout>
                </div>
              )}
            </AsyncBlock>
          )}
        </div>
      </div>

      <SidePanel width={320}>
        <Tabs
          value={tab}
          onChange={setTab}
          options={[
            { id: 'similarity', label: 'Reuse' },
            { id: 'signals', label: 'Signals' },
            { id: 'rubric', label: 'Rubric' },
            { id: 'web', label: 'Web' },
            { id: 'authorship', label: 'Authorship' },
          ]}
        />
        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '16px 16px 24px 16px' }}>
          <SectionLabel style={{ marginBottom: 10 }}>WHAT RAN</SectionLabel>
          <div style={{ fontSize: 12.5, lineHeight: 1.68, color: color.ink2 }}>
            Reuse, Signals and Rubric were computed from your own files, on this machine, with no
            model and no network. <strong style={{ fontWeight: 650 }}>Web</strong> is the exception
            and it always asks first.
          </div>
          <div style={{ marginTop: 18 }}>
            <SectionLabel style={{ marginBottom: 9 }}>DELIBERATELY ABSENT</SectionLabel>
            <div style={{ fontSize: 12.5, lineHeight: 1.68, color: color.ink3 }}>
              AI-writing detection. Binoculars needs token log-probabilities Ollama does not expose,
              and a bundled classifier would be wrong often enough to ruin someone with no way to
              appeal. The measurements stay; the verdict is not ours to give.
            </div>
            <div style={{ fontSize: 12.5, lineHeight: 1.68, color: color.ink3, marginTop: 10 }}>
              <strong style={{ fontWeight: 650 }}>Authorship</strong> answers the same question
              honestly: it is recorded as the text is written, not guessed from how it reads.
            </div>
          </div>
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}
