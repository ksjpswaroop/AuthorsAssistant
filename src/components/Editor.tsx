import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from '@tiptap/markdown'
import { useEffect, useRef } from 'react'
import { Button } from './ui'
import { useApp } from '../lib/store'
import { useAsync } from '../lib/useAsync'
import { lineage } from '../lib/phases'
import { isTauri } from '../lib/api'
import { color, font, radius } from '../lib/tokens'

/**
 * TipTap over Markdown.
 *
 * The component is keyed on the document id by its parent, so every document
 * gets a fresh editor initialised from its own Markdown. That avoids the
 * setContent/onUpdate feedback loop entirely — there is no path by which
 * loading a document can be mistaken for the author typing.
 */
/**
 * What the ledger knows about the document currently open.
 *
 * Deliberately a strip rather than a per-paragraph gutter marker. A gutter would
 * need TipTap document positions mapped onto our content-anchored block
 * ordinals, and those drift apart while the author is mid-edit. A marker
 * mis-aligned by one paragraph would put "a model wrote this" beside the
 * author's own sentence — precisely the confident-but-wrong signal this whole
 * feature exists to prevent. A count that is always right beats a position that
 * is usually right.
 *
 * It appears only when there is something to say.
 */
function AuthorshipStrip() {
  const path = useApp((s) => s.project?.path)
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const setTab = useApp((s) => s.setInspectorTab)
  const showInspector = useApp((s) => s.showInspector)
  const toggleInspector = useApp((s) => s.toggleInspector)

  const summary = useAsync(
    () => lineage.summary(path!, doc!.id),
    [path, doc?.id, doc?.updatedAt],
    isTauri && !!path && !!doc,
  )

  const s = summary.data
  if (!s || s.blocks === 0 || (s.ai === 0 && s.mixed === 0 && s.unknown === 0)) return null

  const parts = [
    s.ai > 0 ? `${s.ai} model-written` : null,
    s.mixed > 0 ? `${s.mixed} mixed` : null,
    s.unknown > 0 ? `${s.unknown} with no record` : null,
  ].filter(Boolean)

  return (
    <button
      onClick={() => {
        setTab('lineage')
        if (!showInspector) toggleInspector()
      }}
      title="Open the lineage of this document"
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        width: '100%',
        textAlign: 'left',
        padding: '6px 10px',
        marginBottom: 10,
        cursor: 'pointer',
        borderRadius: radius.md,
        background: color.surfaceSoft,
        border: `1px solid ${color.lineSoft}`,
        fontSize: 11.5,
        color: color.ink3,
      }}
    >
      <span style={{ width: 7, height: 7, borderRadius: 3, background: color.purple, flexShrink: 0 }} />
      <span style={{ flexGrow: 1 }}>
        Of {s.blocks} paragraphs: {parts.join(', ')}.
      </span>
      <span style={{ color: color.accent }}>Lineage</span>
    </button>
  )
}

export function Editor({
  markdown,
  columnWidth,
  onChange,
  onSaveRequest,
}: {
  markdown: string
  columnWidth: number
  onChange: (markdown: string) => void
  onSaveRequest: () => void
}) {
  const editor = useEditor({
    extensions: [StarterKit, Markdown],
    content: markdown,
    contentType: 'markdown',
    autofocus: 'start',
    editorProps: {
      attributes: { 'aria-label': 'Manuscript text' },
    },
    onUpdate: ({ editor }) => onChange(editor.getMarkdown()),
  })

  // Cmd/Ctrl-S saves immediately rather than waiting for the autosave timer.
  // The handler reads the latest callback from a ref so the listener is
  // attached once rather than on every render.
  const saveRef = useRef(onSaveRequest)
  saveRef.current = onSaveRequest
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 's') {
        e.preventDefault()
        saveRef.current()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  const empty = markdown.trim().length === 0

  return (
    <div style={{ position: 'relative', width: columnWidth }}>
      {empty && (
        <div style={{ position: 'absolute', inset: 0, pointerEvents: 'none' }}>
          <div style={{ fontFamily: font.prose, fontSize: 18.5, lineHeight: 1.78, color: color.disabled }}>
            Start writing. This scene is already a Markdown file on disk.
          </div>
          <div style={{ marginTop: 26, display: 'flex', gap: 10, pointerEvents: 'auto' }}>
            <Button
              icon="sparkle"
              title="Open the assist panel and continue from what is here"
              onClick={() => useApp.getState().setAssist(true)}
            >
              Continue scene
            </Button>
            <Button
              variant="ghost"
              title="Bring in a folder of Markdown you already have"
              onClick={() => void useApp.getState().pickImportFolder()}
            >
              Import Markdown
            </Button>
          </div>
        </div>
      )}
      <AuthorshipStrip />
      <div className="aegis-editor">
        <EditorContent editor={editor} />
      </div>
    </div>
  )
}
