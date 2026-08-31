import { useState } from 'react'
import { Overlay } from '../components/Overlay'
import { Icon, type IconName } from '../components/Icon'
import { useApp } from '../lib/store'
import { Button, Callout, Checkbox, SectionLabel, SelectLook } from '../components/ui'
import { color, font, radius, tint } from '../lib/tokens'
import { words } from '../lib/format'

type Format = 'docx' | 'pdf' | 'epub' | 'md' | 'latex' | 'fountain'

const FORMATS: { id: Format; label: string; icon: IconName; ext: string; hint: string; phase: string }[] = [
  { id: 'docx', label: 'Word (.docx)', icon: 'doc', ext: '.docx', phase: 'Phase 2', hint: 'Chapter titles map to Heading 1, scene text to Body.' },
  { id: 'pdf', label: 'PDF', icon: 'doc', ext: '.pdf', phase: 'Phase 2', hint: 'Typeset from the layout preset — page size and margins come from the preset, not the editor.' },
  { id: 'epub', label: 'EPUB', icon: 'book', ext: '.epub', phase: 'Phase 2', hint: 'Reflowable EPUB 3 with a table of contents built from the binder hierarchy.' },
  { id: 'md', label: 'Markdown', icon: 'markdown', ext: '.md', phase: 'Phase 2', hint: 'Scene bodies are already Markdown on disk, so this is close to a straight concatenation.' },
  { id: 'latex', label: 'LaTeX', icon: 'latex', ext: '.tex', phase: 'Phase 4', hint: 'LaTeX source plus a .bib generated from the library. Needs the paper project type.' },
  { id: 'fountain', label: 'Fountain', icon: 'film', ext: '.fountain', phase: 'Phase 2', hint: 'Screenplay format. Binder titles become scene headings.' },
]

export function CompileDialog() {
  const close = () => useApp.getState().setCompile(false)
  const docs = useApp((s) => s.docs)
  const project = useApp((s) => s.project)
  const [format, setFormat] = useState<Format>('md')
  const [includeDone, setIncludeDone] = useState(true)
  const [includeDraft, setIncludeDraft] = useState(true)
  const [includeSynopses, setIncludeSynopses] = useState(false)

  const current = FORMATS.find((f) => f.id === format)!
  const texts = docs.filter((d) => d.kind !== 'folder')
  const done = texts.filter((d) => d.status === 'Done')
  const draft = texts.filter((d) => d.status !== 'Done' && d.wordCount > 0)

  const selected = [...(includeDone ? done : []), ...(includeDraft ? draft : [])]
  const totalWords = selected.reduce((n, d) => n + d.wordCount, 0)

  return (
    <Overlay onClose={close} label="Compile manuscript" width={880} top={80}>
      <div
        style={{
          height: 54,
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          padding: '0 20px',
          borderBottom: `1px solid ${color.lineSoft}`,
        }}
      >
        <span style={{ color: color.accent, display: 'flex' }}>
          <Icon name="download" size={18} strokeWidth={1.9} />
        </span>
        <div>
          <div style={{ fontSize: 15, fontWeight: 700, letterSpacing: '-0.01em' }}>Compile manuscript</div>
          <div style={{ fontSize: 12, color: color.ink4, marginTop: 1 }}>{project?.name}</div>
        </div>
        <div style={{ flexGrow: 1 }} />
        <button className="ghost" onClick={close} aria-label="Close" style={{ width: 30, height: 30, borderRadius: 8, display: 'flex', alignItems: 'center', justifyContent: 'center', color: color.ink3 }}>
          <Icon name="close" size={16} strokeWidth={2.2} />
        </button>
      </div>

      <div style={{ display: 'flex', minHeight: 0, flexGrow: 1 }}>
        <div
          style={{
            width: 196,
            flexShrink: 0,
            background: color.rail,
            borderRight: `1px solid ${color.line}`,
            padding: '14px 10px',
            overflowY: 'auto',
          }}
        >
          <SectionLabel style={{ padding: '4px 11px 8px 11px' }}>FORMAT</SectionLabel>
          {FORMATS.map((f) => {
            const on = f.id === format
            return (
              <button
                key={f.id}
                className={`row ${on ? 'on' : ''}`}
                onClick={() => setFormat(f.id)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  width: '100%',
                  height: 34,
                  padding: '0 11px',
                  borderRadius: radius.lg - 1,
                  cursor: 'pointer',
                  fontSize: 13,
                  marginBottom: 2,
                  background: on ? tint(color.accent, 0.09) : 'transparent',
                  color: on ? color.accent : color.ink2,
                  fontWeight: on ? 600 : 500,
                }}
              >
                <Icon name={f.icon} size={15} strokeWidth={1.7} />
                {f.label}
              </button>
            )
          })}
        </div>

        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '18px 22px 22px 22px', minWidth: 0 }}>
          <Callout tone="info">{current.hint}</Callout>

          <SectionLabel style={{ margin: '20px 0 9px 0' }}>LAYOUT PRESET</SectionLabel>
          <SelectLook height={36}>Standard manuscript — 12pt serif, double-spaced</SelectLook>

          <SectionLabel style={{ margin: '20px 0 9px 0' }}>INCLUDE</SectionLabel>
          <div style={{ border: `1px solid ${color.line}`, borderRadius: radius.lg + 1, overflow: 'hidden' }}>
            {[
              { on: includeDone, set: setIncludeDone, label: 'Documents marked Done', n: done.length, w: done.reduce((n, d) => n + d.wordCount, 0) },
              { on: includeDraft, set: setIncludeDraft, label: 'Everything else with text', n: draft.length, w: draft.reduce((n, d) => n + d.wordCount, 0) },
            ].map((r, i) => (
              <button
                key={r.label}
                onClick={() => r.set(!r.on)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 11,
                  width: '100%',
                  padding: '11px 13px',
                  borderBottom: `1px solid ${color.lineSoft}`,
                  cursor: 'pointer',
                  background: i % 2 ? color.surface : color.surface,
                }}
              >
                <Checkbox on={r.on} />
                <span style={{ fontSize: 13.5, flexGrow: 1 }}>{r.label}</span>
                <span style={{ fontSize: 12.5, color: color.ink4, fontVariantNumeric: 'tabular-nums' }}>
                  {r.n} docs · {words(r.w)} w
                </span>
              </button>
            ))}
            <button
              onClick={() => setIncludeSynopses(!includeSynopses)}
              style={{ display: 'flex', alignItems: 'center', gap: 11, width: '100%', padding: '11px 13px', cursor: 'pointer' }}
            >
              <Checkbox on={includeSynopses} />
              <span style={{ fontSize: 13.5, flexGrow: 1 }}>Synopses as comments</span>
              <span style={{ fontSize: 12.5, color: color.ink4 }}>for your editor</span>
            </button>
          </div>

          <Callout tone="warn" style={{ marginTop: 20 }}>
            Compile lands in Phase 2. The counts above are real — they come from the project you have open — but nothing
            is written yet.
          </Callout>
        </div>
      </div>

      <div
        style={{
          height: 62,
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          gap: 14,
          padding: '0 20px',
          borderTop: `1px solid ${color.lineSoft}`,
          background: color.surfaceSoft,
        }}
      >
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: 13, fontWeight: 600, fontVariantNumeric: 'tabular-nums' }}>
            {selected.length} documents · {words(totalWords)} words
          </div>
          <div style={{ fontSize: 12, color: color.ink4, marginTop: 2, fontFamily: font.mono }}>
            {project?.name ?? 'Untitled'}
            {current.ext}
          </div>
        </div>
        <div style={{ flexGrow: 1 }} />
        <Button variant="ghost" height={36} onClick={close}>
          Cancel
        </Button>
        <Button icon="download" height={36} disabled title={`Compile arrives in ${current.phase}`}>
          Compile
        </Button>
      </div>
    </Overlay>
  )
}
