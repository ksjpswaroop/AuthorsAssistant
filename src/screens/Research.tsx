import { useState } from 'react'
import { PhaseScreen, Rail, SampleNote, SidePanel } from '../components/PhaseScreen'
import { Button, Callout, Chip, SectionLabel } from '../components/ui'
import { Icon } from '../components/Icon'
import { color, font, radius, tint } from '../lib/tokens'

const QUESTIONS = [
  { id: 'all', n: '∗', text: 'Everything', meta: '7 sources', tone: color.lineFaint, fg: color.ink2 },
  { id: 'q1', n: '1', text: 'How is recall actually measured in long-document retrieval?', meta: '3 sources · settled', tone: color.greenTint, fg: color.greenInk },
  { id: 'q2', n: '2', text: 'What chunk sizes do published systems actually use?', meta: '4 sources · settled', tone: color.greenTint, fg: color.greenInk },
  { id: 'q3', n: '3', text: 'Does overlap help recall, or only pad the index?', meta: '2 sources · thin evidence', tone: color.amberMark, fg: color.yellowInk },
  { id: 'q4', n: '4', text: 'What breaks first past 100k tokens?', meta: '3 sources · reading', tone: color.blueTint, fg: color.blueDeep },
]

const SOURCES = [
  { id: 'c1', q: ['q1', 'q2'], title: 'Dense Passage Retrieval for Open-Domain Question Answering', meta: 'Karpukhin et al. · EMNLP 2020 · in your library', why: 'Fixes passages at 100 words and reports top-k accuracy — the convention most later work inherits.', local: true },
  { id: 'c2', q: ['q1'], title: 'Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks', meta: 'Lewis et al. · NeurIPS 2020 · arxiv.org', why: 'Establishes the retrieve-then-generate framing this paper argues against for book-length inputs.', local: false },
  { id: 'c3', q: ['q2', 'q3'], title: 'ColBERT: Passage Search via Contextualized Late Interaction', meta: 'Khattab & Zaharia · SIGIR 2020 · aclanthology.org', why: 'Late interaction changes what a chunk boundary costs you. Directly relevant to the overlap question.', local: false },
  { id: 'c4', q: ['q4'], title: 'Approximate Nearest Neighbor Search Using HNSW Graphs', meta: 'Malkov & Yashunin · IEEE TPAMI 2020 · in your library', why: 'Index-side, not chunk-side. Kept to rule out the index as the cause of recall drops.', local: true },
  { id: 'c5', q: ['q2', 'q4'], title: 'Leveraging Passage Retrieval with Generative Models', meta: 'Izacard & Grave · EACL 2021 · arxiv.org', why: 'Scales with passage count rather than passage size — the closest published result to the claim.', local: false },
  { id: 'c6', q: ['q1', 'q3'], title: 'The Probabilistic Relevance Framework: BM25 and Beyond', meta: 'Robertson & Zaragoza · FnTIR 2009 · in your library', why: 'Length normalisation was solved for documents long before it was un-solved for chunks.', local: true },
]

const FINDINGS: Record<string, string> = {
  all: 'Nothing in the retrieved literature measures recall at book length. Every result on the shortlist evaluates passages from Wikipedia-scale documents, where a chunk is a fraction of the document rather than a fraction of a chapter.',
  q1: 'Recall is almost always reported as top-k retrieval accuracy over a fixed passage length, inherited without restatement. No source on this list reports recall as a function of chunk length.',
  q2: 'Published chunk sizes cluster at 100 words and 512 tokens, and the choice is nearly always justified by the encoder input limit rather than by any measurement.',
  q3: 'Nothing retrieved isolates overlap as a variable. Treat this as an open question, not a settled result.',
  q4: 'On the evidence so far, ranking degrades before recall does: the right chunk stays in the candidate set and falls out of the top-k.',
}

export function Research() {
  const [step, setStep] = useState('all')

  return (
    <PhaseScreen
      title="Deep Research"
      phase="Phase 5"
      explains="No research agent and no network client exist yet. This is the designed session with sample sources — real papers, invented metrics."
      right={<Button height={28} disabled icon="folder" title="Filing sources arrives in Phase 5">File sources</Button>}
      statusRight={<span>Sources would be read on this machine · nothing uploaded</span>}
    >
      <Rail width={318}>
        <div style={{ padding: '12px 8px' }}>
          <SectionLabel style={{ padding: '4px 6px 8px 6px' }}>PLAN · 4 SUB-QUESTIONS</SectionLabel>
          {QUESTIONS.map((q) => {
            const on = q.id === step
            return (
              <button
                key={q.id}
                className={`row ${on ? 'on' : ''}`}
                onClick={() => setStep(q.id)}
                style={{
                  display: 'flex',
                  gap: 10,
                  width: '100%',
                  padding: '10px 11px',
                  borderRadius: radius.lg,
                  cursor: 'pointer',
                  marginBottom: 3,
                  background: on ? tint(color.accent, 0.1) : 'transparent',
                }}
              >
                <span
                  style={{
                    width: 20,
                    height: 20,
                    borderRadius: 6,
                    background: q.tone,
                    color: q.fg,
                    fontSize: 11,
                    fontWeight: 700,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                  }}
                >
                  {q.n}
                </span>
                <div style={{ minWidth: 0, flexGrow: 1 }}>
                  <div style={{ fontSize: 13, lineHeight: 1.45, color: on ? color.accent : color.ink2, fontWeight: on ? 600 : 450 }}>
                    {q.text}
                  </div>
                  <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 3 }}>{q.meta}</div>
                </div>
              </button>
            )
          })}
        </div>
        <div style={{ flexGrow: 1 }} />
        <div style={{ borderTop: `1px solid ${color.lineFaint}`, padding: '12px 14px' }}>
          <SectionLabel style={{ marginBottom: 8 }}>TOOL TRACE</SectionLabel>
          <div style={{ fontFamily: font.mono, fontSize: 10.5, lineHeight: 1.9, color: color.ink3 }}>
            <div>plan_research(depth=2)</div>
            <div>search_library · search_web</div>
            <div>fetch_source · extract_claims</div>
            <div style={{ color: color.ink5 }}>not implemented</div>
          </div>
        </div>
      </Rail>

      <div style={{ flexGrow: 1, minWidth: 0, display: 'flex', flexDirection: 'column', background: color.surfaceSoft }}>
        <div
          style={{
            height: 42,
            flexShrink: 0,
            display: 'flex',
            alignItems: 'center',
            gap: 10,
            padding: '0 20px',
            borderBottom: `1px solid ${color.lineSoft}`,
            background: color.surface,
          }}
        >
          <span style={{ fontSize: 13, fontWeight: 600 }}>
            {step === 'all' ? `${SOURCES.length} sample sources` : `Question ${step.slice(1)}`}
          </span>
          <div style={{ flexGrow: 1 }} />
          {[['in your library', color.blueMark], ['fetched from the web', color.orangeMark]].map(([l, c]) => (
            <span key={l} style={{ display: 'flex', alignItems: 'center', gap: 5, fontSize: 11.5, color: color.ink3 }}>
              <span style={{ width: 9, height: 9, borderRadius: 3, background: c }} />
              {l}
            </span>
          ))}
        </div>

        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '14px 20px 30px 20px', display: 'flex', flexDirection: 'column', gap: 9 }}>
          {SOURCES.map((s) => {
            const dim = step !== 'all' && !s.q.includes(step)
            return (
              <div
                key={s.id}
                style={{
                  border: `1px solid ${color.line}`,
                  background: color.surface,
                  borderRadius: radius.xl,
                  padding: '13px 15px',
                  opacity: dim ? 0.3 : 1,
                  transition: 'opacity 140ms ease',
                  display: 'flex',
                  gap: 11,
                }}
              >
                <span
                  style={{
                    width: 26,
                    height: 26,
                    borderRadius: radius.md,
                    background: s.local ? color.blueMark : color.orangeMark,
                    color: s.local ? color.blueDeep : '#A34500',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                  }}
                >
                  <Icon name={s.local ? 'doc' : 'globe'} size={14} />
                </span>
                <div style={{ minWidth: 0, flexGrow: 1 }}>
                  <div style={{ fontSize: 13.5, fontWeight: 600, lineHeight: 1.4 }}>{s.title}</div>
                  <div style={{ fontSize: 12, color: color.ink3, marginTop: 3 }}>{s.meta}</div>
                  <div style={{ fontSize: 12.5, lineHeight: 1.6, color: color.ink2, marginTop: 8 }}>{s.why}</div>
                </div>
                <Chip fg={color.ink3} bg={color.lineSoft}>
                  {s.q.map((q) => q.toUpperCase()).join(' · ')}
                </Chip>
              </div>
            )
          })}
        </div>
      </div>

      <SidePanel width={396}>
        <div style={{ height: 42, flexShrink: 0, display: 'flex', alignItems: 'center', padding: '0 16px', borderBottom: `1px solid ${color.lineSoft}` }}>
          <SectionLabel>WHAT THE SOURCES SAY</SectionLabel>
        </div>
        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '16px 16px 24px 16px' }}>
          <div style={{ fontSize: 13.5, lineHeight: 1.68, color: color.inkProse }}>{FINDINGS[step]}</div>
          <Callout tone="warn" icon="offline" style={{ marginTop: 18 }}>
            Deep Research is the one part of the product that needs the network. The design keeps the promise narrow:
            only generated questions are sent, never the manuscript, and every request is logged.
          </Callout>
          <SampleNote>
            The papers cited are real and well known; every score, count and timing on this screen is invented.
          </SampleNote>
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}
