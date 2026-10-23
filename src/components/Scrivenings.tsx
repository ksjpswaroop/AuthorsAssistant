import { useEffect, useState } from 'react'
import { useApp } from '../lib/store'
import { scrivenings } from '../lib/phases'
import { isTauri } from '../lib/api'
import { Button, Callout, EmptyState } from './ui'
import { color, font, radius } from '../lib/tokens'
import { words as fmt } from '../lib/format'

const SEPARATOR = '\n\n<!-- aegis:scrivening -->\n\n'

/**
 * Scrivenings: a folder's children edited as one continuous document.
 *
 * The separator is visible and must survive editing. If it does not, the save
 * is refused outright rather than guessing which text belongs to which file —
 * a partial write here would shred a chapter across several documents.
 */
export function Scrivenings() {
  const path = useApp((s) => s.project?.path)
  const docs = useApp((s) => s.docs)
  const selectedId = useApp((s) => s.selectedId)
  const reloadDocs = useApp((s) => s.openProjectAt)
  const project = useApp((s) => s.project)

  const selected = docs.find((d) => d.id === selectedId)
  const parentId = selected ? (selected.kind === 'folder' ? selected.id : selected.parentId) : null
  const parent = docs.find((d) => d.id === parentId)

  const [text, setText] = useState('')
  const [ids, setIds] = useState<string[]>([])
  const [dirty, setDirty] = useState(false)
  const [status, setStatus] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  useEffect(() => {
    if (!path || !parentId || !isTauri) return
    let live = true
    setError(null)
    setStatus(null)
    scrivenings
      .open(path, parentId)
      .then(([combined, openedIds]) => {
        if (!live) return
        setText(combined)
        setIds(openedIds)
        setDirty(false)
      })
      .catch((e: unknown) => live && setError(e instanceof Error ? e.message : String(e)))
    return () => {
      live = false
    }
  }, [path, parentId])

  const sections = text.split(SEPARATOR).length
  const aligned = sections === ids.length

  const save = async () => {
    if (!path) return
    setBusy(true)
    setError(null)
    try {
      const n = await scrivenings.save(path, text, ids)
      setDirty(false)
      setStatus(`Saved ${n} document${n === 1 ? '' : 's'}.`)
      if (project) await reloadDocs(project.path)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  if (!isTauri) {
    return <EmptyState icon="stack" title="Scrivenings needs the desktop app" />
  }
  if (!parent || ids.length === 0) {
    return (
      <EmptyState
        icon="stack"
        title="Nothing to stitch together"
        body="Select a folder with documents inside it to read and edit them as one continuous piece."
      />
    )
  }

  return (
    <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      <div
        style={{
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '10px 20px',
          borderBottom: `1px solid ${color.lineSoft}`,
        }}
      >
        <span style={{ fontSize: 12.5, color: color.ink3 }}>
          {ids.length} documents from <b style={{ fontWeight: 600, color: color.ink2 }}>{parent.title}</b> ·{' '}
          {fmt(text.split(/\s+/).filter(Boolean).length)} words
        </span>
        <div style={{ flexGrow: 1 }} />
        {status && <span style={{ fontSize: 12, color: color.greenInk }}>{status}</span>}
        {dirty && !aligned && (
          <span style={{ fontSize: 12, fontWeight: 600, color: color.yellowInk }}>
            {sections} sections, {ids.length} documents
          </span>
        )}
        <Button height={30} disabled={!dirty || busy || !aligned} onClick={() => void save()}>
          {busy ? 'Saving…' : 'Save all'}
        </Button>
      </div>

      {!aligned && (
        <Callout tone="warn" icon="alert" style={{ margin: '12px 20px 0 20px' }}>
          A separator has been changed, so the text no longer maps onto {ids.length} documents. Put it back — saving is
          blocked rather than guessing which words belong to which file.
        </Callout>
      )}
      {error && (
        <Callout tone="warn" icon="alert" style={{ margin: '12px 20px 0 20px' }}>
          {error}
        </Callout>
      )}

      <div style={{ flexGrow: 1, overflowY: 'auto', display: 'flex', justifyContent: 'center', minHeight: 0 }}>
        <textarea
          value={text}
          onChange={(e) => {
            setText(e.target.value)
            setDirty(true)
            setStatus(null)
          }}
          spellCheck
          style={{
            width: 700,
            minHeight: '100%',
            padding: '40px 0 160px 0',
            border: 'none',
            outline: 'none',
            resize: 'none',
            background: 'transparent',
            fontFamily: font.prose,
            fontSize: 17.5,
            lineHeight: 1.78,
            color: color.inkProse,
          }}
        />
      </div>

      <div
        style={{
          flexShrink: 0,
          padding: '10px 20px',
          borderTop: `1px solid ${color.lineSoft}`,
          fontSize: 11.5,
          color: color.ink5,
          background: color.surfaceSoft,
          borderRadius: radius.sm,
        }}
      >
        Each document keeps its own file. The comment line between sections is the boundary — edit around it, not
        through it.
      </div>
    </div>
  )
}
