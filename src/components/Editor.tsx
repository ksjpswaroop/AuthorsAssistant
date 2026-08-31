import { useEditor, EditorContent } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { Markdown } from '@tiptap/markdown'
import { useEffect, useRef } from 'react'
import { Button } from './ui'
import { color, font } from '../lib/tokens'

/**
 * TipTap over Markdown.
 *
 * The component is keyed on the document id by its parent, so every document
 * gets a fresh editor initialised from its own Markdown. That avoids the
 * setContent/onUpdate feedback loop entirely — there is no path by which
 * loading a document can be mistaken for the author typing.
 */
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
            <Button icon="sparkle" disabled title="Scene continuation arrives in Phase 3">
              Continue scene
            </Button>
            <Button variant="ghost" disabled title="Import arrives in Phase 2">
              Import Markdown
            </Button>
          </div>
        </div>
      )}
      <div className="aegis-editor">
        <EditorContent editor={editor} />
      </div>
    </div>
  )
}
