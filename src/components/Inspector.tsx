import { useEffect, useState } from 'react'
import { useApp } from '../lib/store'
import { color, labels, radius, statuses, type LabelName, type StatusName } from '../lib/tokens'
import { Callout, Chip, Dot, ProgressBar, SectionLabel, SelectLook, Tabs } from './ui'
import { Icon } from './Icon'
import { relativeTime, tildePath, words } from '../lib/format'

const LABEL_CYCLE: LabelName[] = ['None', 'Plot', 'Character', 'Setup']
const STATUS_CYCLE: StatusName[] = ['Empty', 'Draft', 'Done']

function SynopsisTab() {
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const patchMeta = useApp((s) => s.patchMeta)
  const [draft, setDraft] = useState(doc?.synopsis ?? '')

  useEffect(() => setDraft(doc?.synopsis ?? ''), [doc?.id, doc?.synopsis])

  if (!doc) return null

  const label: LabelName = doc.label && doc.label in labels ? (doc.label as LabelName) : 'None'
  const status: StatusName = doc.status && doc.status in statuses ? (doc.status as StatusName) : 'Empty'
  const cycle = <T,>(list: T[], current: T): T => list[(list.indexOf(current) + 1) % list.length]

  return (
    <div>
      <SectionLabel style={{ marginBottom: 8 }}>SYNOPSIS</SectionLabel>
      <textarea
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => draft !== (doc.synopsis ?? '') && void patchMeta(doc.id, { synopsis: draft })}
        placeholder="What happens in this scene?"
        style={{
          width: '100%',
          minHeight: 96,
          border: `1px solid ${color.line}`,
          borderRadius: radius.lg,
          padding: '11px 12px',
          fontSize: 13,
          lineHeight: 1.62,
          color: color.inkProse,
          background: color.surfaceSoft,
          outline: 'none',
        }}
      />

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: 12, marginTop: 18 }}>
        <div>
          <SectionLabel style={{ marginBottom: 7 }}>LABEL</SectionLabel>
          <SelectLook height={32} onClick={() => void patchMeta(doc.id, { label: cycle(LABEL_CYCLE, label) })}>
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: 7 }}>
              <span style={{ width: 9, height: 9, borderRadius: 3, background: labels[label].dot }} />
              {label}
            </span>
          </SelectLook>
        </div>
        <div>
          <SectionLabel style={{ marginBottom: 7 }}>STATUS</SectionLabel>
          <SelectLook height={32} onClick={() => void patchMeta(doc.id, { status: cycle(STATUS_CYCLE, status) })}>
            {status}
          </SelectLook>
        </div>
      </div>

      <div style={{ marginTop: 20, paddingTop: 18, borderTop: `1px solid ${color.lineSoft}` }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 9 }}>
          <SectionLabel>LENGTH</SectionLabel>
          <div style={{ flexGrow: 1 }} />
          <span style={{ fontSize: 12, color: color.ink2, fontVariantNumeric: 'tabular-nums' }}>
            {words(doc.wordCount)} words
          </span>
        </div>
        <ProgressBar pct={Math.min(100, (doc.wordCount / 2000) * 100)} />
        <div style={{ fontSize: 11.5, color: color.ink5, marginTop: 7 }}>
          Against a 2,000-word default. Per-document targets arrive in Phase 2.
        </div>
      </div>

      <div style={{ marginTop: 20, paddingTop: 18, borderTop: `1px solid ${color.lineSoft}` }}>
        <SectionLabel style={{ marginBottom: 9 }}>ON DISK</SectionLabel>
        <div style={{ fontSize: 12.5, lineHeight: 1.7, color: color.ink2, wordBreak: 'break-all' }}>
          {doc.path ?? 'This is a folder — it has no file.'}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: color.ink4, marginTop: 8 }}>
          <Icon name="clock" size={13} strokeWidth={1.9} />
          Saved {relativeTime(doc.updatedAt)}
        </div>
      </div>
    </div>
  )
}

function NotesTab() {
  return (
    <div>
      <SectionLabel style={{ marginBottom: 8 }}>DOCUMENT NOTES</SectionLabel>
      <Callout tone="quiet" icon="info">
        Notes are stored per document from Phase 2, alongside keywords and snapshots. The synopsis on the first tab is
        saved today.
      </Callout>
    </div>
  )
}

function AgentTab() {
  const path = useApp((s) => s.project?.path)
  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
        <SectionLabel>AGENT LOG</SectionLabel>
        <div style={{ flexGrow: 1 }} />
        <Chip fg={color.ink3} bg={color.lineSoft}>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 5 }}>
            <Dot tone={color.disabled} size={6} />
            No runtime
          </span>
        </Chip>
      </div>

      <Callout tone="quiet" icon="sparkle">
        Hermes agents land in Phase 5, on top of the Ollama runtime from Phase 3. When they run, their plans and tool
        calls are written into the project folder and shown here — nothing is sent anywhere.
      </Callout>

      {path && (
        <div style={{ marginTop: 16, fontSize: 12, lineHeight: 1.6, color: color.ink4 }}>
          Traces will live under <span style={{ color: color.ink2 }}>{tildePath(path)}/traces</span>.
        </div>
      )}
    </div>
  )
}

export function Inspector() {
  const tab = useApp((s) => s.inspectorTab)
  const setTab = useApp((s) => s.setInspectorTab)
  const selected = useApp((s) => s.docs.find((d) => d.id === s.selectedId))

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: color.surface }}>
      <Tabs
        value={tab}
        onChange={setTab}
        options={[
          { id: 'synopsis', label: 'Synopsis' },
          { id: 'notes', label: 'Notes' },
          { id: 'agent', label: 'Agent' },
        ]}
      />
      <div style={{ flexGrow: 1, overflowY: 'auto', padding: '16px 16px 24px 16px' }}>
        {!selected ? (
          <div style={{ fontSize: 12.5, color: color.ink4, lineHeight: 1.6 }}>
            Select a document in the binder to see its details.
          </div>
        ) : tab === 'synopsis' ? (
          <SynopsisTab />
        ) : tab === 'notes' ? (
          <NotesTab />
        ) : (
          <AgentTab />
        )}
      </div>
    </div>
  )
}
