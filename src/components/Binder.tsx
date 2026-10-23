import { useEffect, useMemo, useRef, useState } from 'react'
import { Tree, type NodeRendererProps, type MoveHandler, type RenameHandler } from 'react-arborist'
import { Icon, isIconName, type IconName } from './Icon'
import { IconButton, SectionLabel } from './ui'
import { useApp } from '../lib/store'
import type { DocKind, DocumentRow } from '../lib/types'
import { chrome, color, radius, tint } from '../lib/tokens'

const KIND_ICON: Record<DocKind, IconName> = {
  folder: 'folder',
  scene: 'doc',
  sheet: 'person',
  location: 'globe',
  lore: 'globe',
  research: 'doc',
  canon: 'shieldCheck',
}

/** react-arborist wants a nested shape; the store keeps a flat parent list. */
interface TreeNode extends DocumentRow {
  children?: TreeNode[]
}

function nest(docs: DocumentRow[], parentId: string | null): TreeNode[] {
  return docs
    .filter((d) => d.parentId === parentId)
    .sort((a, b) => a.position - b.position || a.title.localeCompare(b.title))
    .map((d) => (d.kind === 'folder' ? { ...d, children: nest(docs, d.id) } : { ...d }))
}

/** Tracks a box's height so the virtualized tree can fill it. */
function useHeight<T extends HTMLElement>() {
  const ref = useRef<T | null>(null)
  const [height, setHeight] = useState(320)
  useEffect(() => {
    const el = ref.current
    if (!el) return
    const ro = new ResizeObserver(([entry]) => setHeight(entry.contentRect.height))
    ro.observe(el)
    return () => ro.disconnect()
  }, [])
  return { ref, height }
}

function Row({ node, style, dragHandle }: NodeRendererProps<TreeNode>) {
  const selectedId = useApp((s) => s.selectedId)
  const marked = useApp((s) => s.marked)
  const toggleMark = useApp((s) => s.toggleMark)
  const select = useApp((s) => s.select)

  const doc = node.data
  const on = selectedId === doc.id
  const isMarked = marked.includes(doc.id)
  const isFolder = doc.kind === 'folder'

  return (
    <div
      ref={dragHandle}
      className={`row ${on ? 'on' : ''}`}
      style={{
        ...style,
        display: 'flex',
        alignItems: 'center',
        gap: 7,
        height: 27,
        paddingRight: 8,
        borderRadius: radius.md - 1,
        cursor: 'pointer',
        fontSize: 12.5,
        background: on ? tint(color.accent, 0.1) : isMarked ? tint(color.accent, 0.05) : 'transparent',
        color: on ? color.accent : color.ink2,
        fontWeight: on || isFolder ? 600 : 450,
        boxShadow: isMarked && !on ? `inset 0 0 0 1px ${color.line}` : 'none',
      }}
      onClick={(e) => {
        // Cmd/Ctrl-click marks for a bulk edit instead of opening.
        if (e.metaKey || e.ctrlKey) {
          e.preventDefault()
          toggleMark(doc.id)
          return
        }
        void select(doc.id)
      }}
      onDoubleClick={() => node.edit()}
      title={`${doc.title} — double-click to rename, drag to move`}
    >
      {isFolder ? (
        <button
          onClick={(e) => {
            e.stopPropagation()
            node.toggle()
          }}
          aria-label={node.isOpen ? 'Collapse' : 'Expand'}
          style={{ display: 'flex', color: color.ink4, cursor: 'pointer' }}
        >
          <Icon
            name="chevronDown"
            size={13}
            strokeWidth={2.4}
            style={{ transform: node.isOpen ? 'none' : 'rotate(-90deg)', transition: 'transform 120ms ease' }}
          />
        </button>
      ) : (
        <span style={{ width: 13, flexShrink: 0 }} />
      )}

      {/* An author's own icon wins over the kind's default, which is the
          point of setting one. An unknown name falls back rather than
          rendering an empty gap. */}
      <Icon
        name={isIconName(doc.icon) ? doc.icon : (KIND_ICON[doc.kind] ?? 'doc')}
        size={13}
        strokeWidth={1.7}
        opacity={0.65}
      />

      {node.isEditing ? (
        <input
          autoFocus
          defaultValue={doc.title}
          onBlur={(e) => node.submit(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') node.submit(e.currentTarget.value)
            if (e.key === 'Escape') node.reset()
          }}
          onClick={(e) => e.stopPropagation()}
          style={{
            flexGrow: 1,
            minWidth: 0,
            border: `1px solid ${color.accent}`,
            borderRadius: 4,
            padding: '1px 4px',
            fontSize: 12.5,
            background: color.surface,
          }}
        />
      ) : (
        <span style={{ flexGrow: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {doc.title}
        </span>
      )}

      {doc.status === 'Draft' && !node.isEditing && (
        <span style={{ width: 6, height: 6, borderRadius: '50%', background: color.orange, flexShrink: 0 }} />
      )}
    </div>
  )
}

export function Binder() {
  const docs = useApp((s) => s.docs)
  const selectedId = useApp((s) => s.selectedId)
  const addDocument = useApp((s) => s.addDocument)
  const remove = useApp((s) => s.remove)
  const rename = useApp((s) => s.rename)
  const moveDocument = useApp((s) => s.moveDocument)
  const { ref, height } = useHeight<HTMLDivElement>()

  const data = useMemo(() => nest(docs, null), [docs])

  const onMove: MoveHandler<TreeNode> = ({ dragIds, parentId, index }) => {
    // Dragging several rows keeps their relative order at the drop point.
    dragIds.forEach((id, i) => void moveDocument(id, parentId ?? null, index + i))
  }

  const onRename: RenameHandler<TreeNode> = ({ id, name }) => {
    void rename(id, name)
  }

  const addSibling = (kind: DocKind) => {
    const selected = docs.find((d) => d.id === selectedId)
    const parentId = selected ? (selected.kind === 'folder' ? selected.id : selected.parentId) : null
    void addDocument(parentId, kind === 'folder' ? 'New folder' : 'Untitled scene', kind)
  }

  const scenes = docs.filter((d) => d.kind !== 'folder').length

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: color.rail }}>
      <div
        style={{
          height: chrome.railHeader,
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          padding: '0 8px 0 14px',
          gap: 8,
        }}
      >
        <SectionLabel style={{ color: color.ink4 }}>BINDER</SectionLabel>
        <div style={{ flexGrow: 1 }} />
        <IconButton icon="plus" size={24} title="New scene" onClick={() => addSibling('scene')} />
        <IconButton icon="folder" size={24} title="New folder" onClick={() => addSibling('folder')} />
        <IconButton
          icon="trash"
          size={24}
          title="Move the selected document to the project's trash folder"
          onClick={() => selectedId && void remove(selectedId)}
        />
      </div>

      <div ref={ref} style={{ flexGrow: 1, minHeight: 0, padding: '0 8px' }}>
        <Tree<TreeNode>
          data={data}
          idAccessor="id"
          childrenAccessor={(d) => d.children ?? null}
          openByDefault
          width="100%"
          height={height}
          indent={15}
          rowHeight={27}
          onMove={onMove}
          onRename={onRename}
          disableMultiSelection={false}
          aria-label="Binder"
        >
          {Row}
        </Tree>
      </div>

      <div
        style={{
          height: 34,
          flexShrink: 0,
          borderTop: `1px solid ${color.lineFaint}`,
          display: 'flex',
          alignItems: 'center',
          padding: '0 14px',
          gap: 8,
          fontSize: 11.5,
          color: color.ink4,
        }}
      >
        <Icon name="book" size={13} strokeWidth={1.8} />
        {docs.length} documents · {scenes} with text
      </div>
    </div>
  )
}
