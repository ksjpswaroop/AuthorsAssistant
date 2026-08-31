import { useState } from 'react'
import { PhaseScreen, Rail, SampleNote, SidePanel } from '../components/PhaseScreen'
import { Button, Chip, SectionLabel } from '../components/ui'
import { Icon } from '../components/Icon'
import { color, font, radius, tint } from '../lib/tokens'

interface Ref {
  id: string
  title: string
  authors: string
  venue: string
  year: string
  cited: string
  status: { text: string; fg: string; bg: string }
  note: string
  bib: string
}

const OK = { fg: color.greenInk, bg: color.greenTint }
const NEW = { fg: color.blueDeep, bg: color.blueTint }
const WARN = { fg: color.yellowInk, bg: color.amberMark }

const REFS: Ref[] = [
  {
    id: 'dpr',
    title: 'Dense Passage Retrieval for Open-Domain Question Answering',
    authors: 'Karpukhin, Oğuz, Min, Lewis, Wu, Edunov, Chen, Yih',
    venue: 'EMNLP',
    year: '2020',
    cited: '3',
    status: { text: 'Indexed', ...OK },
    note: 'The 100-word passage convention here is the baseline §3 measures against. Cite it for the convention, not the result.',
    bib: '@inproceedings{karpukhin2020dpr,\n  author    = {Karpukhin, Vladimir and others},\n  title     = {Dense Passage Retrieval for\n               Open-Domain Question Answering},\n  booktitle = {EMNLP},\n  year      = {2020}\n}',
  },
  {
    id: 'rag',
    title: 'Retrieval-Augmented Generation for Knowledge-Intensive NLP Tasks',
    authors: 'Lewis, Perez, Piktus, Petroni, Karpukhin, Goyal et al.',
    venue: 'NeurIPS',
    year: '2020',
    cited: '2',
    status: { text: 'Indexed', ...OK },
    note: 'Sets up the retrieve-then-generate framing this paper argues does not survive book-length inputs.',
    bib: '@inproceedings{lewis2020rag,\n  author    = {Lewis, Patrick and others},\n  title     = {Retrieval-Augmented Generation for\n               Knowledge-Intensive NLP Tasks},\n  booktitle = {NeurIPS},\n  year      = {2020}\n}',
  },
  {
    id: 'colbert',
    title: 'ColBERT: Efficient and Effective Passage Search via Late Interaction',
    authors: 'Khattab, Zaharia',
    venue: 'SIGIR',
    year: '2020',
    cited: '—',
    status: { text: 'New', ...NEW },
    note: 'Late interaction changes what a chunk boundary costs — the closest published argument to the overlap claim.',
    bib: '@inproceedings{khattab2020colbert,\n  author    = {Khattab, Omar and Zaharia, Matei},\n  title     = {ColBERT: Efficient and Effective\n               Passage Search},\n  booktitle = {SIGIR},\n  year      = {2020}\n}',
  },
  {
    id: 'hnsw',
    title: 'Approximate Nearest Neighbor Search Using HNSW Graphs',
    authors: 'Malkov, Yashunin',
    venue: 'IEEE TPAMI',
    year: '2020',
    cited: '1',
    status: { text: 'No pages', ...WARN },
    note: 'Volume and page range could not be read from the PDF. A journal citation without them is flagged at submission.',
    bib: '@article{malkov2020hnsw,\n  author  = {Malkov, Yury A. and\n             Yashunin, Dmitry A.},\n  title   = {Efficient and Robust ANN Search\n             Using HNSW Graphs},\n  journal = {IEEE TPAMI},\n  year    = {2020},\n  pages   = {MISSING}\n}',
  },
  {
    id: 'bm25',
    title: 'The Probabilistic Relevance Framework: BM25 and Beyond',
    authors: 'Robertson, Zaragoza',
    venue: 'Foundations and Trends in IR',
    year: '2009',
    cited: '1',
    status: { text: 'Indexed', ...OK },
    note: 'Length normalisation was settled for documents long before it was unsettled for chunks.',
    bib: '@article{robertson2009bm25,\n  author  = {Robertson, Stephen and\n             Zaragoza, Hugo},\n  title   = {The Probabilistic Relevance Framework},\n  journal = {Foundations and Trends in IR},\n  year    = {2009}\n}',
  },
]

const COLUMNS = 'minmax(0, 1fr) 62px 78px 104px'

export function Library() {
  const [sel, setSel] = useState('dpr')
  const current = REFS.find((r) => r.id === sel)!

  return (
    <PhaseScreen
      title="Library"
      phase="Phase 4"
      explains="Reference management is not implemented. These are real papers standing in for the designed bibliography."
      right={<Button height={28} disabled title="BibTeX export arrives in Phase 4">Export .bib</Button>}
    >
      <Rail width={248}>
        <div style={{ padding: '12px 8px' }}>
          <SectionLabel style={{ padding: '6px 10px 8px 10px' }}>COLLECTIONS</SectionLabel>
          {[
            ['All references', REFS.length, 'book'],
            ['Cited in this paper', 4, 'check'],
            ['Read but uncited', 1, 'circle'],
            ['Incomplete metadata', 1, 'alert'],
          ].map(([label, n, icon], i) => (
            <div
              key={label as string}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 9,
                height: 30,
                padding: '0 11px',
                borderRadius: radius.lg - 1,
                fontSize: 13,
                marginBottom: 2,
                background: i === 0 ? tint(color.accent, 0.09) : 'transparent',
                color: i === 0 ? color.accent : color.ink2,
                fontWeight: i === 0 ? 600 : 450,
              }}
            >
              <Icon name={icon as 'book'} size={14} opacity={0.7} />
              <span style={{ flexGrow: 1 }}>{label}</span>
              <span style={{ color: color.ink4, fontWeight: 450 }}>{n}</span>
            </div>
          ))}

          <SectionLabel style={{ padding: '16px 10px 8px 10px' }}>TAGS</SectionLabel>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, padding: '0 8px' }}>
            {['chunking', 'retrieval', 'evaluation', 'ANN indexes'].map((t) => (
              <Chip key={t} fg={color.ink2} bg={color.lineFaint} bold={false}>
                {t}
              </Chip>
            ))}
          </div>
        </div>
      </Rail>

      <div style={{ flexGrow: 1, minWidth: 0, display: 'flex', flexDirection: 'column' }}>
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: COLUMNS,
            alignItems: 'center',
            height: 34,
            flexShrink: 0,
            background: color.rail,
            borderBottom: `1px solid ${color.line}`,
            padding: '0 18px',
            fontSize: 10.5,
            fontWeight: 700,
            letterSpacing: '0.08em',
            color: color.ink4,
          }}
        >
          <div style={{ color: color.ink2 }}>REFERENCE</div>
          <div style={{ textAlign: 'right' }}>YEAR</div>
          <div style={{ textAlign: 'right' }}>CITED</div>
          <div style={{ textAlign: 'right' }}>STATUS</div>
        </div>

        <div style={{ flexGrow: 1, overflowY: 'auto' }}>
          {REFS.map((r) => {
            const on = r.id === sel
            return (
              <div
                key={r.id}
                className={`tr ${on ? 'on' : ''}`}
                onClick={() => setSel(r.id)}
                style={{
                  display: 'grid',
                  gridTemplateColumns: COLUMNS,
                  alignItems: 'center',
                  minHeight: 58,
                  padding: '0 18px',
                  borderBottom: `1px solid ${color.lineSoft}`,
                  cursor: 'pointer',
                  background: on ? tint(color.accent, 0.08) : 'transparent',
                  boxShadow: on ? `inset 3px 0 0 0 ${color.accent}` : 'none',
                }}
              >
                <div style={{ minWidth: 0, paddingRight: 16 }}>
                  <div style={{ fontSize: 13.5, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {r.title}
                  </div>
                  <div style={{ fontSize: 12, color: color.ink3, marginTop: 2, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {r.authors} · {r.venue}
                  </div>
                </div>
                <div style={{ textAlign: 'right', fontSize: 12.5, color: color.ink2, fontVariantNumeric: 'tabular-nums' }}>{r.year}</div>
                <div style={{ textAlign: 'right', fontSize: 12.5, color: color.ink2, fontVariantNumeric: 'tabular-nums' }}>{r.cited}</div>
                <div style={{ textAlign: 'right' }}>
                  <Chip fg={r.status.fg} bg={r.status.bg}>{r.status.text}</Chip>
                </div>
              </div>
            )
          })}
        </div>
      </div>

      <SidePanel width={400}>
        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '18px 16px 24px 16px' }}>
          <div style={{ fontFamily: font.prose, fontSize: 18, lineHeight: 1.36, fontWeight: 600, letterSpacing: '-0.012em' }}>
            {current.title}
          </div>
          <div style={{ fontSize: 12.5, color: color.ink3, marginTop: 7, lineHeight: 1.5 }}>{current.authors}</div>
          <div style={{ fontSize: 12.5, color: color.ink3, marginTop: 2 }}>
            {current.venue} {current.year}
          </div>

          <div
            style={{
              marginTop: 18,
              padding: '12px 13px',
              background: color.railSoft,
              border: `1px solid ${color.lineFaint}`,
              borderRadius: radius.lg + 1,
              fontSize: 12.5,
              lineHeight: 1.62,
              color: color.inkProse,
            }}
          >
            {current.note}
          </div>

          <SectionLabel style={{ margin: '20px 0 9px 0' }}>BIBTEX</SectionLabel>
          <div
            style={{
              fontFamily: font.mono,
              fontSize: 11.5,
              lineHeight: 1.7,
              color: color.ink2,
              background: color.railSoft,
              border: `1px solid ${color.lineFaint}`,
              borderRadius: radius.lg + 1,
              padding: '12px 13px',
              whiteSpace: 'pre-wrap',
              overflowX: 'auto',
            }}
          >
            {current.bib}
          </div>

          <SampleNote>Sample library. Phase 4 reads real metadata out of PDFs you add, on this machine.</SampleNote>
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}
