import { useState } from 'react'
import { Overlay } from '../components/Overlay'
import { Icon, type IconName } from '../components/Icon'
import { useApp } from '../lib/store'
import { api, isTauri } from '../lib/api'
import { compile, defaultFilters, type ExportFormat } from '../lib/phases'
import { Button, Callout, Checkbox, Chip, SectionLabel, SelectLook } from '../components/ui'
import { color, font, radius, tint } from '../lib/tokens'
import { words } from '../lib/format'

const FORMATS: { id: ExportFormat; label: string; icon: IconName; ext: string; hint: string }[] = [
  { id: 'markdown', label: 'Markdown', icon: 'markdown', ext: 'md', hint: 'Scene bodies are already Markdown on disk, so this is close to a straight concatenation with your layout rules applied.' },
  { id: 'docx', label: 'Word (.docx)', icon: 'doc', ext: 'docx', hint: 'Part and chapter headings map to Word heading styles; scenes become body paragraphs.' },
  { id: 'epub', label: 'EPUB', icon: 'book', ext: 'epub', hint: 'Reflowable EPUB 3 with one document per binder item and a generated table of contents.' },
  { id: 'html', label: 'HTML', icon: 'markdown', ext: 'html', hint: 'A single styled page with print rules attached.' },
  { id: 'pdf', label: 'PDF', icon: 'doc', ext: 'html', hint: 'Exports print-ready HTML. Open it and print to PDF — this build does not bundle a typesetting engine, and saying otherwise would be a lie.' },
  { id: 'fountain', label: 'Fountain', icon: 'film', ext: 'fountain', hint: 'Screenplay format. Binder titles become scene headings.' },
]

const PRESETS = ['Standard Manuscript', 'Paperback', 'Ebook', 'Academic', 'Working Draft']

export function CompileDialog() {
  const close = () => useApp.getState().setCompile(false)
  const docs = useApp((s) => s.docs)
  const project = useApp((s) => s.project)
  const [format, setFormat] = useState<ExportFormat>('markdown')
  const [preset, setPreset] = useState(PRESETS[0])
  const [filters, setFilters] = useState(defaultFilters())
  const [preview, setPreview] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [result, setResult] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  const current = FORMATS.find((f) => f.id === format)!
  const texts = docs.filter((d) => d.kind === 'scene')
  const eligible = texts.filter((d) => (filters.skipEmpty ? d.wordCount > 0 : true))
  const totalWords = eligible.reduce((n, d) => n + d.wordCount, 0)

  const run = async (mode: 'preview' | 'export') => {
    if (!project) return
    setBusy(true)
    setError(null)
    setResult(null)
    try {
      if (mode === 'preview') {
        setPreview(await compile.preview(project.path, preset, filters))
      } else {
        const dest = await api.pickSaveFile('Compile manuscript', `${project.name}.${current.ext}`)
        if (!dest) return
        setResult(await compile.export(project.path, preset, format, filters, dest, project.name))
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Overlay onClose={close} label="Compile manuscript" width={900} top={70}>
      <div style={{ height: 54, flexShrink: 0, display: 'flex', alignItems: 'center', gap: 12, padding: '0 20px', borderBottom: `1px solid ${color.lineSoft}` }}>
        <Icon name="download" size={18} strokeWidth={1.9} style={{ color: color.accent }} />
        <div>
          <div style={{ fontSize: 15, fontWeight: 700, letterSpacing: '-0.01em' }}>Compile manuscript</div>
          <div style={{ fontSize: 12, color: color.ink4, marginTop: 1 }}>{project?.name}</div>
        </div>
        <div style={{ flexGrow: 1 }} />
        {!isTauri && <Chip fg={color.yellowInk} bg={color.amberMark}>Needs the desktop app</Chip>}
        <button className="ghost" onClick={close} aria-label="Close" style={{ width: 30, height: 30, borderRadius: 8, display: 'flex', alignItems: 'center', justifyContent: 'center', color: color.ink3 }}>
          <Icon name="close" size={16} strokeWidth={2.2} />
        </button>
      </div>

      <div style={{ display: 'flex', minHeight: 0, flexGrow: 1 }}>
        <div style={{ width: 190, flexShrink: 0, background: color.rail, borderRight: `1px solid ${color.line}`, padding: '14px 10px', overflowY: 'auto' }}>
          <SectionLabel style={{ padding: '4px 11px 8px 11px' }}>FORMAT</SectionLabel>
          {FORMATS.map((f) => {
            const on = f.id === format
            return (
              <button
                key={f.id}
                className={`row ${on ? 'on' : ''}`}
                onClick={() => { setFormat(f.id); setPreview(null); setResult(null) }}
                style={{
                  display: 'flex', alignItems: 'center', gap: 10, width: '100%', height: 34, padding: '0 11px',
                  borderRadius: radius.lg - 1, cursor: 'pointer', fontSize: 13, marginBottom: 2,
                  background: on ? tint(color.accent, 0.09) : 'transparent',
                  color: on ? color.accent : color.ink2, fontWeight: on ? 600 : 500,
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
          <SelectLook height={36} onClick={() => setPreset(PRESETS[(PRESETS.indexOf(preset) + 1) % PRESETS.length])}>
            {preset}
          </SelectLook>

          <SectionLabel style={{ margin: '20px 0 9px 0' }}>INCLUDE</SectionLabel>
          <div style={{ border: `1px solid ${color.line}`, borderRadius: radius.lg + 1, overflow: 'hidden' }}>
            {([
              ['skipEmpty', 'Skip empty documents', `${texts.length - eligible.length} would be skipped`, true],
              ['includeSheets', 'Character and location sheets', 'usually working material, not manuscript', false],
              ['includeResearch', 'Research and canon', 'notes you wrote for yourself', false],
            ] as const).map(([key, label, hint], i) => (
              <button
                key={key}
                onClick={() => { setFilters({ ...filters, [key]: !filters[key] }); setPreview(null) }}
                style={{ display: 'flex', alignItems: 'center', gap: 11, width: '100%', padding: '11px 13px', borderBottom: i < 2 ? `1px solid ${color.lineSoft}` : 'none', cursor: 'pointer' }}
              >
                <Checkbox on={filters[key]} />
                <span style={{ fontSize: 13.5, flexGrow: 1, textAlign: 'left' }}>{label}</span>
                <span style={{ fontSize: 12, color: color.ink4 }}>{hint}</span>
              </button>
            ))}
          </div>

          {preview !== null && (
            <>
              <SectionLabel style={{ margin: '20px 0 9px 0' }}>PREVIEW</SectionLabel>
              <pre
                style={{
                  margin: 0, maxHeight: 240, overflow: 'auto', background: color.surfaceSoft,
                  border: `1px solid ${color.lineFaint}`, borderRadius: radius.xl, padding: '12px 14px',
                  fontFamily: font.mono, fontSize: 11.5, lineHeight: 1.65, color: color.ink2, whiteSpace: 'pre-wrap',
                }}
              >
                {preview.slice(0, 4000)}
                {preview.length > 4000 ? '\n\n…' : ''}
              </pre>
            </>
          )}

          {error && <Callout tone="warn" icon="alert" style={{ marginTop: 16 }}>{error}</Callout>}
          {result && (
            <Callout tone="good" icon="check" style={{ marginTop: 16 }}>
              Written to <span style={{ fontFamily: font.mono }}>{result}</span>. Your source files were not modified.
            </Callout>
          )}
        </div>
      </div>

      <div style={{ height: 62, flexShrink: 0, display: 'flex', alignItems: 'center', gap: 14, padding: '0 20px', borderTop: `1px solid ${color.lineSoft}`, background: color.surfaceSoft }}>
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: 13, fontWeight: 600, fontVariantNumeric: 'tabular-nums' }}>
            {eligible.length} documents · {words(totalWords)} words
          </div>
          <div style={{ fontSize: 12, color: color.ink4, marginTop: 2, fontFamily: font.mono }}>
            {project?.name}.{current.ext}
          </div>
        </div>
        <div style={{ flexGrow: 1 }} />
        <Button variant="ghost" height={36} onClick={close}>Cancel</Button>
        <Button variant="ghost" height={36} icon="eye" disabled={busy || !isTauri} onClick={() => void run('preview')}>
          Preview
        </Button>
        <Button icon="download" height={36} disabled={busy || !isTauri || eligible.length === 0} onClick={() => void run('export')}>
          {busy ? 'Working…' : 'Compile'}
        </Button>
      </div>
    </Overlay>
  )
}
