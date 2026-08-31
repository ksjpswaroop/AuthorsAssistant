import { useState } from 'react'
import { PhaseScreen, Rail, SampleNote, SidePanel } from '../components/PhaseScreen'
import { Button, Callout, ProgressBar, SectionLabel, Tabs } from '../components/ui'
import { Icon } from '../components/Icon'
import { color, font, radius, tint } from '../lib/tokens'

const CHAPTERS = [
  { id: 'c1', num: '1', title: 'Why exact search stops working', part: 'PART I · FOUNDATIONS' },
  { id: 'c2', num: '2', title: 'What a vector index is', part: 'PART I · FOUNDATIONS' },
  { id: 'c4', num: '4', title: 'Chunking for recall', part: 'PART II · BUILDING' },
  { id: 'c43', num: '4.3', title: 'Overlap without waste', part: 'PART II · BUILDING', indent: true, flagged: true },
  { id: 'c5', num: '5', title: 'Indexing at scale', part: 'PART II · BUILDING' },
  { id: 'c7', num: '7', title: 'Measuring what users feel', part: 'PART III · OPERATING' },
]

const KW = { color: color.purpleInk }
const FN = { color: color.accent }
const STR = { color: color.greenInk }
const NUM = { color: color.orangeInk }

export function TechBook() {
  const [ch, setCh] = useState('c43')
  const [tab, setTab] = useState<'listings' | 'terms' | 'checks'>('listings')
  const current = CHAPTERS.find((c) => c.id === ch)!
  let lastPart = ''

  return (
    <PhaseScreen
      title="Technical book"
      badge={{ text: 'TECHNICAL BOOK', fg: color.purpleDeep, bg: color.purpleTint }}
      phase="Phase 4"
      explains="Tested code listings, a glossary built from the manuscript and chapter balance are designed, not built. Sample content."
      right={<Button height={28} disabled title="Listing tests arrive in Phase 4" icon="refresh">Test listings</Button>}
      statusRight={<span>Listing test runner not implemented</span>}
    >
      <Rail>
        <div style={{ padding: '8px 8px 16px 8px' }}>
          {CHAPTERS.map((c) => {
            const showPart = c.part !== lastPart
            lastPart = c.part
            const on = c.id === ch
            return (
              <div key={c.id}>
                {showPart && <SectionLabel style={{ padding: '12px 6px 4px 6px' }}>{c.part}</SectionLabel>}
                <button
                  className={`row ${on ? 'on' : ''}`}
                  onClick={() => setCh(c.id)}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    width: '100%',
                    height: 27,
                    padding: `0 8px 0 ${c.indent ? 27 : 12}px`,
                    borderRadius: radius.md - 1,
                    cursor: 'pointer',
                    fontSize: 12.5,
                    background: on ? tint(color.accent, 0.1) : 'transparent',
                    color: on ? color.accent : color.ink2,
                    fontWeight: on ? 600 : 450,
                  }}
                >
                  <span style={{ width: c.indent ? 22 : 15, color: color.ink5, fontVariantNumeric: 'tabular-nums' }}>{c.num}</span>
                  <span style={{ flexGrow: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{c.title}</span>
                  {c.flagged && <span style={{ width: 6, height: 6, borderRadius: '50%', background: color.orange }} />}
                </button>
              </div>
            )
          })}
        </div>
      </Rail>

      <div style={{ flexGrow: 1, minWidth: 0, overflowY: 'auto', display: 'flex', justifyContent: 'center' }}>
        <div style={{ width: 672, padding: '44px 0 140px 0' }}>
          <h2 style={{ margin: '0 0 22px 0', fontFamily: font.prose, fontSize: 25, fontWeight: 600, letterSpacing: '-0.012em' }}>
            {current.num}  {current.title}
          </h2>

          {ch === 'c43' ? (
            <div style={{ fontFamily: font.prose, fontSize: 16.5, lineHeight: 1.78, color: color.inkProse }}>
              <p style={{ margin: '0 0 20px 0' }}>
                Overlap exists to stop a sentence that answers the question from being cut in half. That is the whole of
                the justification, and every token you overlap is a token you store twice, embed twice, and search twice.
              </p>

              <div
                style={{
                  display: 'flex',
                  gap: 12,
                  padding: '13px 15px',
                  background: color.infoTint,
                  borderLeft: `3px solid ${color.accent}`,
                  borderRadius: '0 9px 9px 0',
                  marginBottom: 22,
                }}
              >
                <span style={{ color: color.accent, display: 'flex', marginTop: 2 }}>
                  <Icon name="info" size={17} strokeWidth={1.9} />
                </span>
                <div style={{ fontFamily: font.ui, fontSize: 13.5, lineHeight: 1.62, color: color.infoInk }}>
                  <b style={{ fontWeight: 700 }}>Note</b> — the cost is not linear in the overlap fraction. At 25 percent
                  overlap you store a third more chunks, not a quarter more.
                </div>
              </div>

              <div style={{ border: `1px solid ${color.line}`, borderRadius: radius.xl, overflow: 'hidden', marginBottom: 8 }}>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 9,
                    padding: '9px 13px',
                    background: color.rail,
                    borderBottom: `1px solid ${color.line}`,
                  }}
                >
                  <span style={{ fontFamily: font.mono, fontSize: 11.5, fontWeight: 500, color: color.ink2 }}>chunker.py</span>
                  <div style={{ flexGrow: 1 }} />
                  <span style={{ display: 'flex', alignItems: 'center', gap: 5, fontSize: 11, fontWeight: 600, color: color.greenInk }}>
                    <Icon name="check" size={12} strokeWidth={2.6} />
                    tested at 3f9a1c2
                  </span>
                </div>
                <div style={{ display: 'flex', fontFamily: font.mono, fontSize: 12.5, lineHeight: 1.85 }}>
                  <div style={{ flexShrink: 0, padding: '12px 0', width: 40, textAlign: 'right', color: color.disabled, userSelect: 'none', borderRight: `1px solid ${color.lineSoft}` }}>
                    {Array.from({ length: 10 }, (_, i) => (
                      <div key={i}>{i + 1}</div>
                    ))}
                  </div>
                  <pre style={{ margin: 0, padding: '12px 14px', color: color.inkProse, overflowX: 'auto' }}>
<span style={KW}>def</span> <span style={FN}>chunks</span>(text, size=<span style={NUM}>512</span>, overlap=<span style={NUM}>64</span>):{'\n'}
{'    '}<span style={STR}>"""Yield token windows with a fixed overlap."""</span>{'\n'}
{'    '}tokens = tokenizer.encode(text){'\n'}
{'    '}step = size - overlap{'\n'}
{'    '}<span style={KW}>if</span> step &lt;= <span style={NUM}>0</span>:{'\n'}
{'        '}<span style={KW}>raise</span> <span style={FN}>ValueError</span>(<span style={STR}>"overlap must be smaller than size"</span>){'\n'}
{'    '}<span style={KW}>for</span> start <span style={KW}>in</span> <span style={FN}>range</span>(<span style={NUM}>0</span>, <span style={FN}>len</span>(tokens), step):{'\n'}
{'        '}window = tokens[start:start + size]{'\n'}
{'        '}<span style={KW}>if</span> <span style={FN}>len</span>(window) &lt; overlap:{'\n'}
{'            '}<span style={KW}>break</span>
                  </pre>
                </div>
              </div>
              <div style={{ fontSize: 12, color: color.ink5, marginBottom: 22, fontFamily: font.ui }}>
                Listing 4.2 · lives in the repo, not in the manuscript
              </div>

              <div
                style={{
                  display: 'flex',
                  gap: 12,
                  padding: '13px 15px',
                  background: color.amberTint,
                  borderLeft: `3px solid ${color.yellow}`,
                  borderRadius: '0 9px 9px 0',
                }}
              >
                <span style={{ color: color.yellowInk, display: 'flex', marginTop: 2 }}>
                  <Icon name="alert" size={17} strokeWidth={2.1} />
                </span>
                <div style={{ fontFamily: font.ui, fontSize: 13.5, lineHeight: 1.62, color: color.amberInk }}>
                  <b style={{ fontWeight: 700 }}>Careful</b> — tokenizers are not interchangeable. Re-indexing after a
                  tokenizer change is not optional.
                </div>
              </div>
            </div>
          ) : (
            <Callout tone="quiet">
              Outline only. The technical book project type arrives in Phase 4.
            </Callout>
          )}

          <SampleNote>Sample manuscript, not your project.</SampleNote>
        </div>
      </div>

      <SidePanel>
        <Tabs
          value={tab}
          onChange={setTab}
          options={[
            { id: 'listings', label: 'Listings' },
            { id: 'terms', label: 'Terms' },
            { id: 'checks', label: 'Checks', badge: 3 },
          ]}
        />
        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '16px 16px 24px 16px' }}>
          {tab === 'listings' && (
            <>
              <SectionLabel style={{ marginBottom: 11 }}>IN THIS CHAPTER</SectionLabel>
              {[
                ['4.1 tokenize.py', 'python · tested · referenced once', true],
                ['4.2 chunker.py', 'python · tested · referenced twice', true],
                ['4.3 index_build.sh', 'shell · never tested · output pasted by hand', false],
                ['4.4 recall_eval.py', 'python · tested · never referenced', false],
              ].map(([name, meta, ok]) => (
                <div
                  key={name as string}
                  style={{
                    border: `1px solid ${ok ? color.lineSoft : color.amberLine}`,
                    background: ok ? color.surface : color.amberTint,
                    borderRadius: radius.lg + 1,
                    padding: '11px 12px',
                    marginBottom: 4,
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <span style={{ fontFamily: font.mono, fontSize: 12, fontWeight: 500, flexGrow: 1, color: ok ? color.ink : color.amberInk }}>
                      {name}
                    </span>
                    <span style={{ color: ok ? color.green : color.yellowInk, display: 'flex' }}>
                      <Icon name={ok ? 'check' : 'alert'} size={14} strokeWidth={2.4} />
                    </span>
                  </div>
                  <div style={{ fontSize: 11.5, color: ok ? color.ink4 : color.amberInk2, marginTop: 3 }}>{meta}</div>
                </div>
              ))}
              <SampleNote>Listings would be read from the code repo at build time, so what the reader runs is what you tested.</SampleNote>
            </>
          )}

          {tab === 'terms' && (
            <>
              <SectionLabel style={{ marginBottom: 11 }}>TERMS USED HERE</SectionLabel>
              {[
                ['overlap', 'Defined here, on first use', true],
                ['window', 'Defined in 4.1', true],
                ['tokenizer', 'Used 14 times before it is defined in chapter 5', false],
                ['recall', 'Defined in 1.2', true],
              ].map(([t, s, ok]) => (
                <div
                  key={t as string}
                  style={{ display: 'flex', gap: 10, padding: '10px 11px', borderRadius: radius.lg, background: ok ? 'transparent' : color.amberTint }}
                >
                  <span style={{ width: 7, height: 7, borderRadius: '50%', background: ok ? color.green : color.yellow, marginTop: 6, flexShrink: 0 }} />
                  <div>
                    <div style={{ fontSize: 13, fontWeight: 600, color: ok ? color.ink : color.amberInk }}>{t}</div>
                    <div style={{ fontSize: 12, color: ok ? color.ink3 : color.amberInk2, marginTop: 2 }}>{s}</div>
                  </div>
                </div>
              ))}
            </>
          )}

          {tab === 'checks' && (
            <>
              <SectionLabel style={{ marginBottom: 11 }}>BEFORE THIS CHAPTER SHIPS</SectionLabel>
              {[
                ['1 listing has never been run', false],
                ['“tokenizer” used before it is defined', false],
                ['Listing 4.4 is never referenced', false],
                ['Every cross-reference resolves', true],
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
                  <span style={{ fontSize: 13, color: ok ? color.inkProse : color.amberInk }}>{t}</span>
                </div>
              ))}

              <div style={{ marginTop: 18, border: `1px solid ${color.line}`, borderRadius: radius.xl, padding: 13 }}>
                <div style={{ fontSize: 12.5, fontWeight: 700, marginBottom: 9 }}>Chapter balance</div>
                {[
                  ['Prose', 64, color.accent],
                  ['Code', 36, color.purple],
                ].map(([label, v, tone]) => (
                  <div key={label as string} style={{ display: 'flex', alignItems: 'center', gap: 11, marginBottom: 8 }}>
                    <span style={{ width: 44, fontSize: 11.5, color: color.ink3 }}>{label}</span>
                    <div style={{ flexGrow: 1 }}>
                      <ProgressBar pct={v as number} tone={tone as string} height={8} />
                    </div>
                    <span style={{ width: 32, textAlign: 'right', fontSize: 11.5, color: color.ink3 }}>{v}%</span>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}
