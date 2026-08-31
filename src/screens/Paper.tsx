import { useState } from 'react'
import { PhaseScreen, Rail, SampleNote, SidePanel } from '../components/PhaseScreen'
import { Button, Callout, Chip, SectionLabel, Tabs } from '../components/ui'
import { Icon } from '../components/Icon'
import { color, font, radius, tint } from '../lib/tokens'

const SECTIONS = [
  { id: 'abstract', num: '', title: 'Title and abstract', group: 'FRONT MATTER' },
  { id: 'intro', num: '1', title: 'Introduction', group: 'BODY' },
  { id: 'related', num: '2', title: 'Related Work', group: 'BODY' },
  { id: 'method', num: '3', title: 'Method', group: 'BODY' },
  { id: 'length', num: '3.2', title: 'Length effects', group: 'BODY', indent: true, flagged: true },
  { id: 'results', num: '5', title: 'Results', group: 'BODY' },
  { id: 'references', num: '', title: 'References', group: 'BACK MATTER' },
]

const FIGURES = [
  { title: '1 System overview', meta: 'static SVG · cited in §3', warn: false },
  { title: '2 Recall by chunk length', meta: 'inputs changed — the built plot is stale', warn: true },
  { title: '3 Overlap ablation', meta: 'overlap_ablation.py · cited in §5', warn: false },
  { title: '4 Failure taxonomy', meta: 'never referenced in the text', warn: true },
]

export function Paper() {
  const [sec, setSec] = useState('length')
  const [tab, setTab] = useState<'citations' | 'figures' | 'checks'>('citations')
  const current = SECTIONS.find((s) => s.id === sec)!
  let lastGroup = ''

  return (
    <PhaseScreen
      title="Research paper"
      badge={{ text: 'PAPER', fg: color.blueDeep, bg: color.blueTint }}
      phase="Phase 4"
      explains="The paper project type — citations, equations, figure slots and submission checks — is designed, not built. Sample content."
      right={<Button height={28} disabled title="LaTeX build arrives in Phase 4">Build PDF</Button>}
      statusRight={<span>LaTeX build not implemented</span>}
    >
      <Rail>
        <div style={{ padding: '8px 8px 16px 8px' }}>
          {SECTIONS.map((s) => {
            const showGroup = s.group !== lastGroup
            lastGroup = s.group
            const on = s.id === sec
            return (
              <div key={s.id}>
                {showGroup && <SectionLabel style={{ padding: '12px 6px 4px 6px' }}>{s.group}</SectionLabel>}
                <button
                  className={`row ${on ? 'on' : ''}`}
                  onClick={() => setSec(s.id)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    width: '100%',
                    height: 27,
                    padding: `0 8px 0 ${s.indent ? 27 : 12}px`,
                    borderRadius: radius.md - 1,
                    cursor: 'pointer',
                    fontSize: 12.5,
                    background: on ? tint(color.accent, 0.1) : 'transparent',
                    color: on ? color.accent : color.ink2,
                    fontWeight: on ? 600 : 450,
                  }}
                >
                  {s.num && <span style={{ width: s.indent ? 22 : 15, color: color.ink5, fontVariantNumeric: 'tabular-nums' }}>{s.num}</span>}
                  <span style={{ flexGrow: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{s.title}</span>
                  {s.flagged && <span style={{ width: 6, height: 6, borderRadius: '50%', background: color.orange }} />}
                </button>
              </div>
            )
          })}
        </div>
      </Rail>

      <div style={{ flexGrow: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        <div style={{ flexGrow: 1, overflowY: 'auto', display: 'flex', justifyContent: 'center' }}>
          <div style={{ width: 660, padding: '44px 0 140px 0' }}>
            <h2 style={{ margin: '0 0 22px 0', fontFamily: font.prose, fontSize: 24, fontWeight: 600, letterSpacing: '-0.012em' }}>
              {current.num ? `${current.num}  ` : ''}
              {current.title}
            </h2>

            {sec === 'length' ? (
              <div style={{ fontFamily: font.prose, fontSize: 16.5, lineHeight: 1.78, color: color.inkProse }}>
                <p style={{ margin: '0 0 20px 0' }}>
                  Length normalisation is not a new problem. The probabilistic relevance framework already accounts for
                  document length when scoring{' '}
                  <span
                    style={{
                      background: color.blueTint,
                      borderRadius: 4,
                      padding: '1px 4px',
                      fontFamily: font.ui,
                      fontSize: 14,
                      fontWeight: 500,
                      color: color.blueDeep,
                    }}
                  >
                    (Robertson &amp; Zaragoza, 2009)
                  </span>
                  . What has not been settled is the same question one level down, where the unit being scored is a
                  chunk the system itself chose.
                </p>

                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 16,
                    padding: '18px 20px',
                    background: color.surfaceSoft,
                    border: `1px solid ${color.lineSoft}`,
                    borderRadius: radius.lg + 1,
                    marginBottom: 22,
                  }}
                >
                  <div style={{ flexGrow: 1, textAlign: 'center', fontSize: 18 }}>
                    <i>R</i>@<i>k</i> &nbsp;=&nbsp;
                    <span style={{ display: 'inline-flex', flexDirection: 'column', alignItems: 'center', verticalAlign: 'middle', fontSize: 14 }}>
                      <span style={{ padding: '0 6px 2px 6px', borderBottom: `1px solid ${color.ink}` }}>1</span>
                      <span style={{ paddingTop: 2 }}>|<i>Q</i>|</span>
                    </span>
                    &nbsp;<span style={{ fontSize: 24, verticalAlign: -3 }}>∑</span>
                    <sub style={{ fontSize: 12 }}><i>q</i>∈<i>Q</i></sub>&nbsp; 1[ rank(<i>q</i>) ≤ <i>k</i> ]
                  </div>
                  <span style={{ fontSize: 14, color: color.ink3, flexShrink: 0 }}>(3)</span>
                </div>

                <div style={{ border: `1px dashed ${color.disabled}`, borderRadius: radius.xl, background: color.surfaceSoft, overflow: 'hidden' }}>
                  <div style={{ height: 168, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 9 }}>
                    <span style={{ color: color.disabled }}>
                      <Icon name="chart" size={30} strokeWidth={1.6} />
                    </span>
                    <div style={{ fontSize: 12.5, color: color.ink4, fontFamily: font.ui }}>
                      Renders at build time from <span style={{ fontFamily: font.mono, fontSize: 11.5 }}>figures/recall_by_length.py</span>
                    </div>
                  </div>
                  <div style={{ padding: '11px 16px', borderTop: `1px dashed ${color.line}`, fontSize: 13.5, lineHeight: 1.55, color: color.ink2 }}>
                    <b style={{ fontWeight: 600 }}>Figure 2:</b> Recall@10 against chunk length in tokens.
                  </div>
                </div>
              </div>
            ) : (
              <Callout tone="quiet">
                Outline only. The paper project type arrives in Phase 4; this rail and inspector show what it will look
                like.
              </Callout>
            )}

            <SampleNote>Sample manuscript, not your project.</SampleNote>
          </div>
        </div>
      </div>

      <SidePanel>
        <Tabs
          value={tab}
          onChange={setTab}
          options={[
            { id: 'citations', label: 'Citations' },
            { id: 'figures', label: 'Figures' },
            { id: 'checks', label: 'Checks', badge: 2 },
          ]}
        />
        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '16px 16px 24px 16px' }}>
          {tab === 'citations' && (
            <>
              <SectionLabel style={{ marginBottom: 10 }}>IN THIS SECTION</SectionLabel>
              {[
                ['Robertson & Zaragoza, 2009', 'In the library · complete', color.green],
                ['Khattab & Zaharia, 2020', 'Added by Deep Research · first use', color.accent],
                ['Malkov & Yashunin, 2020', 'Page numbers missing', color.yellow],
              ].map(([t, s, tone]) => (
                <div key={t as string} style={{ display: 'flex', gap: 10, padding: '10px 11px', borderRadius: radius.lg }}>
                  <span style={{ width: 7, height: 7, borderRadius: '50%', background: tone as string, marginTop: 6, flexShrink: 0 }} />
                  <div>
                    <div style={{ fontSize: 13, fontWeight: 600 }}>{t}</div>
                    <div style={{ fontSize: 12, color: color.ink3, marginTop: 2 }}>{s}</div>
                  </div>
                </div>
              ))}
              <Callout tone="warn" style={{ marginTop: 16 }}>
                “Recall falls away sharply past 512 tokens” — an empirical claim with neither a citation nor a number.
              </Callout>
            </>
          )}

          {tab === 'figures' && (
            <>
              <SectionLabel style={{ marginBottom: 10 }}>FIGURES · 4</SectionLabel>
              {FIGURES.map((f) => (
                <div
                  key={f.title}
                  style={{
                    border: `1px solid ${f.warn ? color.amberLine : color.lineSoft}`,
                    background: f.warn ? color.amberTint : color.surface,
                    borderRadius: radius.lg + 1,
                    padding: '11px 12px',
                    marginBottom: 4,
                  }}
                >
                  <div style={{ fontSize: 12.5, fontWeight: 600, color: f.warn ? color.amberInk : color.ink }}>
                    {f.title}
                  </div>
                  <div style={{ fontSize: 11.5, color: f.warn ? color.amberInk2 : color.ink3, marginTop: 3 }}>
                    {f.meta}
                  </div>
                </div>
              ))}
            </>
          )}

          {tab === 'checks' && (
            <>
              <SectionLabel style={{ marginBottom: 11 }}>BEFORE SUBMISSION</SectionLabel>
              {[
                ['Every citation resolves to the library', true],
                ['No overlap with your own thesis', true],
                ['2 references are missing fields', false],
                ['1 unsupported empirical claim', false],
              ].map(([t, ok]) => (
                <div
                  key={t as string}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 10,
                    padding: '10px 11px',
                    borderRadius: radius.lg,
                    background: ok ? 'transparent' : color.amberTint,
                  }}
                >
                  <span style={{ color: ok ? color.green : color.yellowInk, display: 'flex' }}>
                    <Icon name={ok ? 'check' : 'alert'} size={16} strokeWidth={2.2} />
                  </span>
                  <span style={{ fontSize: 13, color: ok ? color.inkProse : color.amberInk, flexGrow: 1 }}>{t}</span>
                </div>
              ))}
              <div style={{ marginTop: 18 }}>
                <Chip fg={color.purpleDeep} bg={color.purpleTint}>AI writing · 4% of the body</Chip>
              </div>
            </>
          )}
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}

