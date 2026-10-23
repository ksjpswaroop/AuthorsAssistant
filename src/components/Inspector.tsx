import { useEffect, useState } from 'react'
import { useApp } from '../lib/store'
import { color, font, labels, radius, statuses, type LabelName, type StatusName } from '../lib/tokens'
import { Callout, Chip, Dot, ProgressBar, SectionLabel, SelectLook, Tabs } from './ui'
import { Icon, type IconName } from './Icon'
import { Snapshots } from './Snapshots'
import { KeywordEditor } from './KeywordEditor'
import { relativeTime, words } from '../lib/format'
import { useAsync } from '../lib/useAsync'
import { lineage, type Origin } from '../lib/phases'
import { api, isTauri } from '../lib/api'

const LABEL_CYCLE: LabelName[] = ['None', 'Plot', 'Character', 'Setup']

/** The five compile understands (Phase 6 / Phase 7). */
const SECTION_TYPES = ['Part Heading', 'Chapter Heading', 'Scene', 'Front Matter', 'Back Matter']

/** A small, deliberate set — a full icon grid is a decision the author has to
 *  make about every document, which is worse than a default that just works. */
const BINDER_ICONS: IconName[] = [
  'doc', 'book', 'person', 'globe', 'shield', 'target', 'bookmark',
  'timeline', 'network', 'film', 'database', 'wand',
]
const STATUS_CYCLE: StatusName[] = ['Empty', 'Draft', 'Done']

function SynopsisTab() {
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const patchMeta = useApp((s) => s.patchMeta)
  const patchExtended = useApp((s) => s.patchExtended)
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

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: 12, marginTop: 18 }}>
        <div>
          <SectionLabel style={{ marginBottom: 7 }}>POV</SectionLabel>
          <input
            defaultValue={doc.pov ?? ''}
            key={`pov-${doc.id}`}
            placeholder="Whose eyes"
            onBlur={(e) => e.target.value !== (doc.pov ?? '') && void patchExtended(doc.id, { pov: e.target.value })}
            style={{
              width: '100%',
              height: 32,
              padding: '0 10px',
              border: `1px solid ${color.line}`,
              borderRadius: radius.lg - 1,
              fontSize: 13,
              outline: 'none',
            }}
          />
        </div>
        <div>
          <SectionLabel style={{ marginBottom: 7 }}>TARGET</SectionLabel>
          <input
            type="number"
            min={0}
            step={100}
            key={`target-${doc.id}`}
            defaultValue={doc.target ?? ''}
            placeholder="words"
            onBlur={(e) => {
              const n = e.target.value === '' ? 0 : Number(e.target.value)
              if (Number.isFinite(n) && n !== (doc.target ?? 0)) void patchExtended(doc.id, { target: n })
            }}
            style={{
              width: '100%',
              height: 32,
              padding: '0 10px',
              border: `1px solid ${color.line}`,
              borderRadius: radius.lg - 1,
              fontSize: 13,
              outline: 'none',
            }}
          />
        </div>
      </div>

      <div style={{ marginTop: 20, paddingTop: 18, borderTop: `1px solid ${color.lineSoft}` }}>
        <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, marginBottom: 9 }}>
          <SectionLabel>LENGTH</SectionLabel>
          <div style={{ flexGrow: 1 }} />
          <span style={{ fontSize: 12, color: color.ink2, fontVariantNumeric: 'tabular-nums' }}>
            {words(doc.wordCount)}
            {doc.target ? ` / ${words(doc.target)}` : ''} words
          </span>
        </div>
        <ProgressBar pct={Math.min(100, (doc.wordCount / (doc.target || 2000)) * 100)} />
        <div style={{ fontSize: 11.5, color: color.ink5, marginTop: 7 }}>
          {doc.target ? 'Against this document\u2019s target.' : 'No target set — shown against a 2,000-word default.'}
        </div>
      </div>

      <div style={{ marginTop: 20, paddingTop: 18, borderTop: `1px solid ${color.lineSoft}` }}>
        <KeywordEditor />
      </div>

      <div style={{ marginTop: 18 }}>
        <SectionLabel style={{ marginBottom: 7 }}>SECTION TYPE</SectionLabel>
        <select
          value={doc.sectionType ?? ''}
          onChange={(e) => {
            const value = e.target.value
            void useApp
              .getState()
              .bulkSet([doc.id], { sectionType: value === '' ? undefined : value })
          }}
          style={{
            width: '100%',
            height: 32,
            padding: '0 9px',
            fontSize: 12.5,
            color: color.ink2,
            background: color.surface,
            border: `1px solid ${color.line}`,
            borderRadius: radius.md,
          }}
        >
          <option value="">Not set</option>
          {SECTION_TYPES.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <div style={{ fontSize: 11.5, lineHeight: 1.55, color: color.ink5, marginTop: 6 }}>
          Section type is what stops compile treating a chapter heading and a scene the same way.
        </div>
      </div>

      <div style={{ marginTop: 18 }}>
        <SectionLabel style={{ marginBottom: 7 }}>BINDER ICON</SectionLabel>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
          {BINDER_ICONS.map((name) => {
            const on = doc.icon === name
            return (
              <button
                key={name}
                title={name}
                onClick={() => void useApp.getState().setIcon(doc.id, on ? null : name)}
                style={{
                  width: 28,
                  height: 28,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  cursor: 'pointer',
                  borderRadius: radius.md,
                  color: on ? color.surface : color.ink3,
                  background: on ? color.accent : color.surfaceSoft,
                  border: `1px solid ${on ? color.accent : color.lineSoft}`,
                }}
              >
                <Icon name={name} size={15} strokeWidth={1.9} />
              </button>
            )
          })}
        </div>
        <div style={{ fontSize: 11.5, lineHeight: 1.55, color: color.ink5, marginTop: 6 }}>
          {doc.icon ? 'Click it again to go back to the default for this kind.' : 'Optional. The kind’s own icon is used when none is set.'}
        </div>
      </div>

      <div style={{ marginTop: 20, paddingTop: 18, borderTop: `1px solid ${color.lineSoft}` }}>
        <Snapshots />
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

/**
 * Cross-document links: a scene pointing at the sheet, the note or the earlier
 * chapter it depends on.
 *
 * Bookmarks are stored in the same `document_links` table that character sheets
 * already use, under the `bookmark` relation. Links are shown in **both**
 * directions — a bookmark is only useful if the other end knows about it, and a
 * one-way link is how a reference quietly rots when the other document moves.
 */
function Bookmarks() {
  const path = useApp((s) => s.project?.path)
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const docs = useApp((s) => s.docs)
  const select = useApp((s) => s.select)
  const refreshDocs = useApp((s) => s.refreshDocs)
  const [adding, setAdding] = useState(false)
  const [busy, setBusy] = useState(false)

  if (!doc) return null

  const titleOf = (id: string) => docs.find((d) => d.id === id)?.title ?? 'a document since deleted'

  // Both directions. `links` on a row carries every edge it takes part in.
  const outgoing = doc.links.filter((l) => l.relation === 'bookmark' && l.fromId === doc.id)
  const incoming = doc.links.filter((l) => l.relation === 'bookmark' && l.toId === doc.id)

  const candidates = docs
    .filter(
      (d) =>
        d.id !== doc.id &&
        d.kind !== 'folder' &&
        !outgoing.some((l) => l.toId === d.id) &&
        !incoming.some((l) => l.fromId === d.id),
    )
    .slice(0, 200)

  const link = async (toId: string) => {
    if (!path) return
    setBusy(true)
    try {
      await api.linkDocuments(path, doc.id, toId, 'bookmark')
      await refreshDocs()
      setAdding(false)
    } finally {
      setBusy(false)
    }
  }

  const unlink = async (fromId: string, toId: string) => {
    if (!path) return
    setBusy(true)
    try {
      await api.unlinkDocuments(path, fromId, toId, 'bookmark')
      await refreshDocs()
    } finally {
      setBusy(false)
    }
  }

  const row = (id: string, from: string, to: string, inbound: boolean) => (
    <div
      key={`${from}-${to}`}
      style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '5px 0' }}
    >
      <Icon
        name={inbound ? 'chevronRight' : 'bookmark'}
        size={13}
        strokeWidth={1.9}
        style={{ color: color.ink4, flexShrink: 0, transform: inbound ? 'rotate(180deg)' : undefined }}
      />
      <button
        onClick={() => void select(id)}
        style={{
          flexGrow: 1,
          minWidth: 0,
          textAlign: 'left',
          fontSize: 12.5,
          color: color.accent,
          cursor: 'pointer',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
          whiteSpace: 'nowrap',
        }}
      >
        {titleOf(id)}
      </button>
      {!inbound && (
        <button
          onClick={() => void unlink(from, to)}
          aria-label="Remove this bookmark"
          disabled={busy}
          style={{ display: 'flex', color: color.ink4, cursor: 'pointer', padding: 2 }}
        >
          <Icon name="close" size={12} strokeWidth={2.2} />
        </button>
      )}
    </div>
  )

  return (
    <div style={{ marginTop: 20 }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 9 }}>
        <SectionLabel>BOOKMARKS</SectionLabel>
        <div style={{ flexGrow: 1 }} />
        <button
          onClick={() => setAdding(!adding)}
          aria-label="Add a bookmark"
          style={{ display: 'flex', color: color.ink4, cursor: 'pointer', padding: 2 }}
        >
          <Icon name={adding ? 'close' : 'plus'} size={14} strokeWidth={2.1} />
        </button>
      </div>

      {outgoing.length === 0 && incoming.length === 0 && !adding && (
        <div style={{ fontSize: 12.5, lineHeight: 1.6, color: color.ink4 }}>
          Nothing linked. A bookmark points this document at another — the sheet it depends on, the
          chapter it answers — and shows up on both ends.
        </div>
      )}

      {outgoing.map((l) => row(l.toId, l.fromId, l.toId, false))}

      {incoming.length > 0 && (
        <>
          <div style={{ fontSize: 11, color: color.ink5, marginTop: 8, marginBottom: 2 }}>
            LINKED FROM
          </div>
          {incoming.map((l) => row(l.fromId, l.fromId, l.toId, true))}
        </>
      )}

      {adding && (
        <div style={{ marginTop: 8 }}>
          <SelectLook height={30}>
            <select
              disabled={busy || candidates.length === 0}
              defaultValue=""
              onChange={(e) => e.target.value && void link(e.target.value)}
              style={{ font: 'inherit', color: 'inherit', background: 'transparent', border: 0, width: '100%' }}
            >
              <option value="">
                {candidates.length === 0 ? 'Nothing left to link to' : 'Link to…'}
              </option>
              {candidates.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.title}
                </option>
              ))}
            </select>
          </SelectLook>
        </div>
      )}
    </div>
  )
}

function NotesTab() {
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const patchExtended = useApp((s) => s.patchExtended)
  const [draft, setDraft] = useState(doc?.notes ?? '')

  useEffect(() => setDraft(doc?.notes ?? ''), [doc?.id, doc?.notes])
  if (!doc) return null

  return (
    <div>
      <SectionLabel style={{ marginBottom: 8 }}>DOCUMENT NOTES</SectionLabel>
      <textarea
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => draft !== (doc.notes ?? '') && void patchExtended(doc.id, { notes: draft })}
        placeholder="Anything about this document that is not the document."
        style={{
          width: '100%',
          minHeight: 150,
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

      <Bookmarks />

      <div style={{ display: 'flex', alignItems: 'center', gap: 8, margin: '20px 0 9px 0' }}>
        <SectionLabel>FORESHADOWING</SectionLabel>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, minmax(0, 1fr))', gap: 12 }}>
        {([
          ['plants', 'Plants', 'ids set up here'],
          ['payoffs', 'Pays off', 'ids resolved here'],
        ] as const).map(([field, label, hint]) => (
          <div key={field}>
            <div style={{ fontSize: 11.5, color: color.ink3, marginBottom: 5 }}>{label}</div>
            <input
              key={`${field}-${doc.id}`}
              defaultValue={doc[field] ?? ''}
              placeholder={hint}
              onBlur={(e) =>
                e.target.value !== (doc[field] ?? '') && void patchExtended(doc.id, { [field]: e.target.value })
              }
              style={{
                width: '100%',
                height: 32,
                padding: '0 10px',
                border: `1px solid ${color.line}`,
                borderRadius: radius.lg - 1,
                fontSize: 12.5,
                outline: 'none',
              }}
            />
          </div>
        ))}
      </div>
      <div style={{ fontSize: 11.5, lineHeight: 1.55, color: color.ink5, marginTop: 7 }}>
        Comma-separated ids that tie a setup to its payoff. The ledger that reads them arrives with the continuity
        agent.
      </div>
    </div>
  )
}

/**
 * Phase 5: what the agent actually did, read back from the project.
 *
 * This used to describe a runtime that was not built yet. It now reads
 * `traces/` — real runs, real tool calls, real results. An agent whose
 * reasoning is invisible is not more trustworthy for being local.
 */
function LineageTab() {
  const path = useApp((s) => s.project?.path)
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const ready = isTauri && !!path && !!doc

  const list = useAsync(() => lineage.blocks(path!, doc!.id), [path, doc?.id, doc?.updatedAt], ready)
  const summary = useAsync(
    () => lineage.summary(path!, doc!.id),
    [path, doc?.id, doc?.updatedAt],
    ready,
  )
  const [open, setOpen] = useState<string | null>(null)
  const chain = useAsync(() => lineage.history(path!, open!), [path, open], ready && !!open)
  const who = useAsync(() => lineage.authorship(path!, open!), [path, open], ready && !!open)

  const tone = (origin: Origin) =>
    origin === 'human'
      ? color.green
      : origin === 'ai'
        ? color.purple
        : origin === 'external'
          ? color.orangeInk
          : color.disabled

  const s = summary.data

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
        <SectionLabel>LINEAGE</SectionLabel>
        <div style={{ flexGrow: 1 }} />
        {s && (
          <Chip fg={s.chainIntact ? color.greenInk : color.orangeInk} bg={color.surfaceSoft}>
            {s.chainIntact ? 'ledger intact' : 'ledger altered'}
          </Chip>
        )}
      </div>

      {s && s.blocks > 0 && (
        <div style={{ marginBottom: 14 }}>
          <div style={{ display: 'flex', gap: 3, height: 8, borderRadius: 4, overflow: 'hidden' }}>
            {(
              [
                ['human', s.human, color.green],
                ['ai', s.ai, color.purple],
                ['external', s.external, color.orangeInk],
                ['mixed', s.mixed, color.blueDeep],
                ['unknown', s.unknown, color.disabled],
              ] as const
            )
              .filter(([, n]) => n > 0)
              .map(([k, n, c]) => (
                <div key={k} style={{ flexGrow: n, background: c }} title={`${k}: ${n}`} />
              ))}
          </div>
          <div style={{ fontSize: 11.5, color: color.ink3, marginTop: 6, lineHeight: 1.6 }}>
            {s.blocks} paragraphs · {s.human} yours · {s.ai} model-written
            {s.external > 0 && ` · ${s.external} imported`}
            {s.unrecorded > 0 && (
              <>
                {' '}
                ·{' '}
                <span style={{ color: color.ink4 }}>
                  {s.unrecorded} with no record — which is not evidence of who wrote them
                </span>
              </>
            )}
          </div>
        </div>
      )}

      {(list.data?.length ?? 0) === 0 ? (
        <Callout tone="quiet" icon="info">
          No paragraphs recorded yet. Saving this document gives each paragraph a stable identity
          that survives rewriting, reordering, and editing the file in another editor.
        </Callout>
      ) : (
        list.data!.map((b, i) => {
          const on = open === b.id
          return (
            <div
              key={b.id}
              style={{
                border: `1px solid ${on ? color.line : color.lineSoft}`,
                borderRadius: radius.md,
                marginBottom: 4,
                overflow: 'hidden',
              }}
            >
              <button
                onClick={() => setOpen(on ? null : b.id)}
                style={{ display: 'block', width: '100%', textAlign: 'left', padding: '8px 10px', cursor: 'pointer' }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <span style={{ fontSize: 11, color: color.ink4, fontFamily: font.mono }}>
                    {i + 1}
                  </span>
                  <span
                    style={{
                      fontSize: 12,
                      color: color.ink2,
                      flexGrow: 1,
                      minWidth: 0,
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {b.text.replace(/^#+\s*/, '').slice(0, 46)}
                  </span>
                  <span style={{ fontSize: 10.5, color: color.ink4 }}>r{b.revision}</span>
                </div>
              </button>

              {on && (
                <div style={{ padding: '0 10px 10px 10px' }}>
                  {who.data && (
                    <div
                      style={{
                        display: 'flex',
                        alignItems: 'flex-start',
                        gap: 8,
                        padding: '7px 9px',
                        borderRadius: radius.md,
                        background: color.surfaceSoft,
                        marginBottom: 8,
                      }}
                    >
                      <Dot tone={tone(who.data.origin)} />
                      <div style={{ fontSize: 12, color: color.ink2, lineHeight: 1.55 }}>
                        {who.data.disclosureLine}
                        {who.data.models.length > 0 && (
                          <div style={{ fontSize: 11, color: color.ink4, marginTop: 3 }}>
                            {who.data.models.join(', ')}
                          </div>
                        )}
                      </div>
                    </div>
                  )}

                  {(chain.data ?? []).map((e) => (
                    <div key={e.id} style={{ fontSize: 11.5, color: color.ink3, padding: '3px 0', lineHeight: 1.5 }}>
                      <span style={{ fontFamily: font.mono, color: color.ink2 }}>{e.action}</span>{' '}
                      by {e.actorId}
                      {e.confidence === 'inferred' && (
                        <span
                          style={{ color: color.orangeInk, marginLeft: 5 }}
                          title="AegisDraft did not watch this happen; it matched the text afterwards"
                        >
                          (reconstructed)
                        </span>
                      )}
                      <span style={{ color: color.ink4 }}> · {relativeTime(e.at)}</span>
                      {e.note && (
                        <div style={{ color: color.ink4, marginLeft: 4 }}>{e.note}</div>
                      )}
                    </div>
                  ))}
                  {(chain.data?.length ?? 0) === 0 && (
                    <div style={{ fontSize: 11.5, color: color.ink4, lineHeight: 1.55 }}>
                      No record kept for this paragraph. That is not evidence that anyone in
                      particular wrote it.
                    </div>
                  )}
                </div>
              )}
            </div>
          )
        })
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
          { id: 'lineage', label: 'Lineage' },
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
          <LineageTab />
        )}
      </div>
    </div>
  )
}
