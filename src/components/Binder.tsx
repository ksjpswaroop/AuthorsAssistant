import { useState } from 'react'
import { Icon, type IconName } from './Icon'
import { IconButton, SectionLabel } from './ui'
import { childrenOf, useApp } from '../lib/store'
import type { DocKind, DocumentRow } from '../lib/types'
import { chrome, color, radius, tint } from '../lib/tokens'

const KIND_ICON: Record<DocKind, IconName> = {
  folder: 'folder',
  scene: 'doc',
  sheet: 'person',
  lore: 'globe',
  research: 'doc',
}

function Row({ doc, depth }: { doc: DocumentRow; depth: number }) {
  const selectedId = useApp((s) => s.selectedId)
  const select = useApp((s) => s.select)
  const rename = useApp((s) => s.rename)
  const docs = useApp((s) => s.docs)
  const [collapsed, setCollapsed] = useState(false)
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(doc.title)

  const kids = childrenOf(docs, doc.id)
  const isFolder = doc.kind === 'folder'
  const on = selectedId === doc.id

  const commit = () => {
    setEditing(false)
    if (draft.trim() && draft !== doc.title) void rename(doc.id, draft)
    else setDraft(doc.title)
  }

  return (
    <>
      <div
        className={`row ${on ? 'on' : ''}`}
        onClick={() => void select(doc.id)}
        onDoubleClick={() => {
          setDraft(doc.title)
          setEditing(true)
        }}
        title={`${doc.title} — double-click to rename`}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 7,
          height: 27,
          paddingLeft: 8 + depth * 15,
          paddingRight: 8,
          borderRadius: radius.md - 1,
          cursor: 'pointer',
          fontSize: 12.5,
          background: on ? tint(color.accent, 0.1) : 'transparent',
          color: on ? color.accent : color.ink2,
          fontWeight: on || isFolder ? 600 : 450,
        }}
      >
        {isFolder ? (
          <button
            onClick={(e) => {
              e.stopPropagation()
              setCollapsed((c) => !c)
            }}
            aria-label={collapsed ? 'Expand' : 'Collapse'}
            style={{ display: 'flex', color: color.ink4, cursor: 'pointer' }}
          >
            <Icon
              name="chevronDown"
              size={13}
              strokeWidth={2.4}
              style={{ transform: collapsed ? 'rotate(-90deg)' : 'none', transition: 'transform 120ms ease' }}
            />
          </button>
        ) : (
          <span style={{ width: 13, flexShrink: 0 }} />
        )}

        <Icon name={KIND_ICON[doc.kind]} size={13} strokeWidth={1.7} opacity={0.65} />

        {editing ? (
          <input
            autoFocus
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              if (e.key === 'Enter') commit()
              if (e.key === 'Escape') {
                setDraft(doc.title)
                setEditing(false)
              }
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

        {doc.status === 'Draft' && !editing && (
          <span style={{ width: 6, height: 6, borderRadius: '50%', background: color.orange, flexShrink: 0 }} />
        )}
      </div>

      {isFolder && !collapsed && kids.map((k) => <Row key={k.id} doc={k} depth={depth + 1} />)}
    </>
  )
}

export function Binder() {
  const docs = useApp((s) => s.docs)
  const selectedId = useApp((s) => s.selectedId)
  const addDocument = useApp((s) => s.addDocument)
  const remove = useApp((s) => s.remove)
  const roots = childrenOf(docs, null)

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

      <div style={{ flexGrow: 1, overflowY: 'auto', padding: '0 8px 16px 8px' }}>
        {roots.map((d) => (
          <Row key={d.id} doc={d} depth={0} />
        ))}
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
