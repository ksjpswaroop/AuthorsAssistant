import { useState } from 'react'
import { PhaseScreen, SampleNote, SidePanel } from '../components/PhaseScreen'
import { Button, Callout, ProgressBar, SectionLabel, Tabs } from '../components/ui'
import { color, font, radius } from '../lib/tokens'

type Tab = 'similarity' | 'ai' | 'rubric'

function Ring({ pct, tone, label, sub }: { pct: number; tone: string; label: string; sub: string }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 11 }}>
      <div
        style={{
          width: 38,
          height: 38,
          borderRadius: '50%',
          background: `conic-gradient(${tone} 0% ${pct}%, ${color.lineFaint} ${pct}% 100%)`,
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
          {pct}%
        </span>
      </div>
      <div>
        <div style={{ fontSize: 12.5, fontWeight: 600 }}>{label}</div>
        <div style={{ fontSize: 11.5, color: color.ink4 }}>{sub}</div>
      </div>
    </div>
  )
}

export function Integrity() {
  const [tab, setTab] = useState<Tab>('similarity')

  return (
    <PhaseScreen
      title="Integrity"
      phase="Phase 4"
      explains="Similarity, AI-writing detection and rubrics are not implemented. This is the designed report, filled with sample findings."
      right={<Button height={28} disabled title="Export arrives in Phase 4" icon="download">Export packet</Button>}
      statusRight={<span>Packet will export as PDF + JSON</span>}
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
          <Ring pct={14} tone={color.accent} label="Similarity" sub="2 internal · 1 unchecked" />
          <Ring pct={31} tone={color.purpleInk} label="AI writing" sub="1 paragraph flagged" />
          <Ring pct={78} tone={color.greenInk} label="Rubric" sub="4 criteria scored" />
        </div>

        <div style={{ flexGrow: 1, overflowY: 'auto', display: 'flex', justifyContent: 'center', background: color.surfaceSoft }}>
          <div style={{ width: 640, padding: '40px 0 120px 0' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 9, marginBottom: 22 }}>
              <SectionLabel>SAMPLE SOURCE TEXT</SectionLabel>
              <div style={{ flexGrow: 1, height: 1, background: color.lineFaint }} />
              {[
                ['internal', color.blueMark],
                ['web', color.orangeMark],
                ['AI writing', color.purpleMark],
              ].map(([l, c]) => (
                <span key={l} style={{ display: 'flex', alignItems: 'center', gap: 5, fontSize: 11.5, color: color.ink3 }}>
                  <span style={{ width: 9, height: 9, borderRadius: 3, background: c }} />
                  {l}
                </span>
              ))}
            </div>

            <div style={{ fontFamily: font.prose, fontSize: 17.5, lineHeight: 1.82, color: color.inkProse }}>
              <p style={{ margin: '0 0 20px 0' }}>
                Below them{' '}
                <span style={{ background: color.blueMark, borderRadius: 3, padding: '1px 2px' }}>
                  the salt went on being white in the dark, refusing to be black like everything else
                </span>{' '}
                — she wanted to be exact about that later.
              </p>
              <p style={{ margin: '0 0 20px 0' }}>
                <span style={{ background: color.orangeMark, borderRadius: 3, padding: '1px 2px' }}>
                  Evaporite basins of this kind form where inflow is exceeded by evaporation over geological timescales
                </span>
                . Tamsin had read that somewhere and repeated it like a prayer she did not believe.
              </p>
              <p style={{ margin: 0 }}>
                <span style={{ background: color.purpleMark, borderRadius: 3, padding: '1px 2px' }}>
                  The vast expanse of the salt flats stretched endlessly before them, a testament to the relentless power
                  of nature and the passage of time.
                </span>
              </p>
            </div>

            <SampleNote>
              Sample text and findings, not your manuscript. Phase 4 runs these checks locally against the project you
              have open.
            </SampleNote>
          </div>
        </div>
      </div>

      <SidePanel width={434}>
        <Tabs
          value={tab}
          onChange={setTab}
          options={[
            { id: 'similarity', label: 'Similarity' },
            { id: 'ai', label: 'AI writing' },
            { id: 'rubric', label: 'Rubric' },
          ]}
        />
        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '16px 16px 26px 16px' }}>
          {tab === 'similarity' && (
            <>
              <Callout tone="warn" icon="offline">
                Web check not run. When it exists, only the phrases you approve leave this machine — never the manuscript.
              </Callout>
              <SectionLabel style={{ margin: '18px 0 9px 0' }}>MATCHES · 3</SectionLabel>
              {[
                { n: 1, src: 'Your own draft · Kharsa v2', meta: '18 words · internal similarity', pct: '91%', fg: color.accent, bg: color.blueMark },
                { n: 2, src: 'Research clipping · evaporites.md', meta: '24 words · pasted, uncited', pct: '100%', fg: color.orangeInk, bg: color.orangeMark },
                { n: 3, src: 'No source match', meta: '42 words · flagged by AI detection', pct: '—', fg: color.ink5, bg: color.purpleMark },
              ].map((m) => (
                <div
                  key={m.n}
                  style={{
                    display: 'flex',
                    gap: 11,
                    padding: '11px 12px',
                    borderRadius: radius.lg + 1,
                    boxShadow: `inset 0 0 0 1px ${color.lineSoft}`,
                    marginBottom: 3,
                  }}
                >
                  <span
                    style={{
                      width: 19,
                      height: 19,
                      borderRadius: radius.sm,
                      background: m.bg,
                      color: m.fg,
                      fontSize: 11,
                      fontWeight: 700,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: 0,
                    }}
                  >
                    {m.n}
                  </span>
                  <div style={{ minWidth: 0, flexGrow: 1 }}>
                    <div style={{ fontSize: 13, fontWeight: 600 }}>{m.src}</div>
                    <div style={{ fontSize: 12, color: color.ink3, marginTop: 2 }}>{m.meta}</div>
                  </div>
                  <span style={{ fontSize: 13, fontWeight: 700, color: m.fg }}>{m.pct}</span>
                </div>
              ))}
            </>
          )}

          {tab === 'ai' && (
            <>
              <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
                <span style={{ fontSize: 27, fontWeight: 700, color: color.purpleInk, letterSpacing: '-0.02em' }}>31%</span>
                <span style={{ fontSize: 12.5, color: color.ink3 }}>of this chapter reads as machine-generated</span>
              </div>
              <div style={{ marginTop: 18, display: 'flex', flexDirection: 'column', gap: 9 }}>
                {[8, 12, 6, 22, 94, 9].map((v, i) => (
                  <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 11 }}>
                    <span style={{ width: 20, fontSize: 11.5, color: v > 50 ? color.purpleInk : color.ink5, fontWeight: v > 50 ? 700 : 400 }}>
                      P{i + 1}
                    </span>
                    <div style={{ flexGrow: 1 }}>
                      <ProgressBar pct={v} tone={v > 50 ? color.purple : '#C7D2DB'} height={8} />
                    </div>
                    <span style={{ width: 34, textAlign: 'right', fontSize: 11.5, color: color.ink3, fontVariantNumeric: 'tabular-nums' }}>
                      {v}%
                    </span>
                  </div>
                ))}
              </div>
              <Callout tone="quiet" style={{ marginTop: 18 }}>
                Detection runs on-device through ONNX Runtime. No text leaves the machine to produce these numbers.
              </Callout>
            </>
          )}

          {tab === 'rubric' && (
            <>
              <div style={{ display: 'flex', alignItems: 'baseline', gap: 9, marginBottom: 18 }}>
                <span style={{ fontSize: 27, fontWeight: 700, color: color.greenInk, letterSpacing: '-0.02em' }}>78</span>
                <span style={{ fontSize: 12.5, color: color.ink3 }}>/ 100 · your own rubric, not a shared standard</span>
              </div>
              {[
                ['Pacing', 86, color.green],
                ['Show, don’t tell', 54, color.orange],
                ['Dialogue tags', 91, color.green],
                ['Citation coverage', 40, color.yellow],
              ].map(([label, v, tone]) => (
                <div key={label as string} style={{ marginBottom: 15 }}>
                  <div style={{ display: 'flex', alignItems: 'baseline', marginBottom: 6 }}>
                    <span style={{ fontSize: 13, fontWeight: 600 }}>{label}</span>
                    <div style={{ flexGrow: 1 }} />
                    <span style={{ fontSize: 12.5, fontWeight: 600, color: color.ink2 }}>{v}</span>
                  </div>
                  <ProgressBar pct={v as number} tone={tone as string} />
                </div>
              ))}
              <SampleNote>Scores are a reading aid. Nothing here decides whether a chapter is finished — you do.</SampleNote>
            </>
          )}
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}
