import { useState } from 'react'
import { Overlay } from '../components/Overlay'
import { useApp } from '../lib/store'
import { Button, Callout, ProgressBar, SectionLabel } from '../components/ui'
import { Icon, type IconName } from '../components/Icon'
import { color, font, radius, tint } from '../lib/tokens'
import { relativeTime, words } from '../lib/format'
import { useAsync } from '../lib/useAsync'
import { api, isTauri } from '../lib/api'
import { project6, sheetTemplates, type Goals } from '../lib/phases'

const close = () => useApp.getState().setDialog(null)

/* -------------------------------------------------------------------------- */
/* Project and document templates (Phase 6)                                    */
/* -------------------------------------------------------------------------- */

const PROJECT_TEMPLATES: { id: string; name: string; icon: IconName; body: string; structure: string[] }[] = [
  {
    id: 'novel',
    name: 'Novel',
    icon: 'book',
    body: 'A manuscript with an opening scene, plus folders for characters, locations, worldbuilding, canon and research.',
    structure: ['Manuscript', 'Characters', 'Locations', 'Worldbuilding', 'Canon', 'Research'],
  },
  {
    id: 'nonfiction',
    name: 'Nonfiction',
    icon: 'doc',
    body: 'A manuscript with an introduction, plus source, interview, figure and back-matter folders.',
    structure: ['Manuscript', 'Sources', 'Interviews', 'Figures', 'Back matter'],
  },
  {
    id: 'academic',
    name: 'Academic paper',
    icon: 'latex',
    body: 'Abstract and five numbered sections as empty files, plus references, figures and appendices.',
    structure: ['Paper / Abstract and sections 1–5', 'References', 'Figures', 'Appendices'],
  },
  {
    id: 'techbook',
    name: 'Technical book',
    icon: 'markdown',
    body: 'A manuscript with a preface, plus listings, figures, glossary and research folders.',
    structure: ['Manuscript', 'Listings', 'Figures', 'Glossary', 'Research'],
  },
]

function Templates() {
  const [pick, setPick] = useState('novel')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const current = PROJECT_TEMPLATES.find((t) => t.id === pick)!

  const create = async () => {
    setBusy(true)
    setError(null)
    try {
      const parent = await api.pickFolder('Where should the project live?')
      if (!parent) return
      // The project kind stays "novel"-style metadata; the template only
      // decides the starting folder tree. Keeping them separate means an
      // academic paper can still use every feature a novel can.
      const summary = await project6.createFromTemplate(parent, current.name, pick, pick)
      close()
      await useApp.getState().openProjectAt(summary.path)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Overlay onClose={close} label="Project templates" width={760} top={90}>
      <div style={{ padding: '18px 20px 0 20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <Icon name="template" size={18} strokeWidth={1.9} style={{ color: color.accent }} />
          <div style={{ fontSize: 15, fontWeight: 700, letterSpacing: '-0.01em' }}>Start from a template</div>
        </div>
        <div style={{ fontSize: 12.5, color: color.ink3, marginTop: 4 }}>
          A template only creates folders and empty files. It never writes prose for you.
        </div>
      </div>

      <div style={{ display: 'flex', gap: 16, padding: 20, minHeight: 0 }}>
        <div style={{ width: 260, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 4 }}>
          {PROJECT_TEMPLATES.map((t) => {
            const on = t.id === pick
            return (
              <button
                key={t.id}
                className={`row ${on ? 'on' : ''}`}
                onClick={() => setPick(t.id)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 11,
                  padding: '11px 12px',
                  borderRadius: radius.lg + 1,
                  cursor: 'pointer',
                  background: on ? tint(color.accent, 0.09) : 'transparent',
                  boxShadow: on ? `inset 0 0 0 1px ${color.accent}` : 'none',
                }}
              >
                <span
                  style={{
                    width: 32,
                    height: 32,
                    borderRadius: radius.lg,
                    background: on ? color.blueTint : color.lineSoft,
                    color: on ? color.accent : color.ink3,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                  }}
                >
                  <Icon name={t.icon} size={16} />
                </span>
                <span style={{ fontSize: 13.5, fontWeight: 600, color: on ? color.accent : color.ink }}>{t.name}</span>
              </button>
            )
          })}
        </div>

        <div style={{ flexGrow: 1, minWidth: 0 }}>
          <div style={{ fontSize: 13.5, lineHeight: 1.68, color: color.inkProse }}>{current.body}</div>
          <SectionLabel style={{ margin: '18px 0 9px 0' }}>CREATES</SectionLabel>
          <div style={{ border: `1px solid ${color.line}`, borderRadius: radius.xl, overflow: 'hidden' }}>
            {current.structure.map((row, i) => (
              <div
                key={row}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  padding: '10px 13px',
                  borderBottom: i < current.structure.length - 1 ? `1px solid ${color.lineSoft}` : 'none',
                  fontSize: 13,
                  color: color.ink2,
                }}
              >
                <Icon name="folder" size={14} strokeWidth={1.8} style={{ color: color.ink4 }} />
                {row}
              </div>
            ))}
          </div>
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '14px 20px',
          borderTop: `1px solid ${color.lineSoft}`,
          background: color.surfaceSoft,
        }}
      >
        <span style={{ fontSize: 12.5, color: error ? color.orangeInk : color.ink3 }}>
          {error ?? 'A template only creates folders and empty files. It writes no prose.'}
        </span>
        <div style={{ flexGrow: 1 }} />
        <Button variant="ghost" onClick={close}>
          Cancel
        </Button>
        <Button disabled={busy || !isTauri} onClick={() => void create()}>
          {busy ? 'Creating…' : 'Create from template'}
        </Button>
      </div>
    </Overlay>
  )
}

/* -------------------------------------------------------------------------- */
/* Writing goals (Phase 6)                                                     */
/* -------------------------------------------------------------------------- */

/**
 * Phase 6: writing goals, stored with the project.
 *
 * A session target only means something if there is a mark to count from, so
 * "Start a session" stamps the project's current word count. Before that the
 * session line says there is no session — not "0 of 2,000", which would imply a
 * session was running and going badly.
 */
function Goals() {
  const path = useApp((s) => s.project?.path)
  const docs = useApp((s) => s.docs)
  const total = docs.reduce((n, d) => n + d.wordCount, 0)

  const stored = useAsync(() => project6.goals(path!), [path], isTauri && !!path)
  const progress = useAsync(
    () => project6.sessionProgress(path!),
    [path, total],
    isTauri && !!path,
  )
  const [draft, setDraft] = useState<Goals | null>(null)
  const [saved, setSaved] = useState(false)
  const goals = draft ?? stored.data

  const field = (key: keyof Goals, label: string, hint: string) => (
    <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '7px 0' }}>
      <div style={{ flexGrow: 1, minWidth: 0 }}>
        <div style={{ fontSize: 13, fontWeight: 600 }}>{label}</div>
        <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 1 }}>{hint}</div>
      </div>
      <input
        type="number"
        min={0}
        value={(goals?.[key] as number | null) ?? ''}
        placeholder="none"
        onChange={(e) => {
          setSaved(false)
          const raw = e.target.value.trim()
          setDraft({
            ...(goals ?? {
              projectTarget: null,
              sessionTarget: null,
              sessionStartWords: null,
              sessionStartedAt: null,
              defaultDocumentTarget: null,
            }),
            [key]: raw === '' ? null : Math.max(0, Number(raw)),
          })
        }}
        style={{
          width: 110,
          textAlign: 'right',
          fontSize: 13,
          fontVariantNumeric: 'tabular-nums',
          padding: '5px 8px',
          borderRadius: radius.md,
          border: `1px solid ${color.line}`,
          background: color.surface,
        }}
      />
    </div>
  )

  const save = async () => {
    if (!path || !goals) return
    await project6.setGoals(path, goals)
    stored.reload()
    setDraft(null)
    setSaved(true)
  }

  const projectTarget = goals?.projectTarget ?? null
  const sessionTarget = goals?.sessionTarget ?? null
  const written = progress.data

  return (
    <Overlay onClose={close} label="Writing goals" width={560} top={110}>
      <div style={{ padding: '18px 20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <Icon name="target" size={18} strokeWidth={1.9} style={{ color: color.accent }} />
          <div style={{ fontSize: 15, fontWeight: 700 }}>Writing goals</div>
        </div>

        <div style={{ marginTop: 20 }}>
          <div style={{ display: 'flex', alignItems: 'baseline', marginBottom: 8 }}>
            <SectionLabel>PROJECT</SectionLabel>
            <div style={{ flexGrow: 1 }} />
            <span style={{ fontSize: 12.5, color: color.ink2, fontVariantNumeric: 'tabular-nums' }}>
              {projectTarget ? `${words(total)} of ${words(projectTarget)}` : words(total)}
            </span>
          </div>
          {projectTarget ? (
            <ProgressBar pct={(total / projectTarget) * 100} height={9} />
          ) : (
            <div style={{ fontSize: 11.5, color: color.ink4 }}>
              No project target set. The count above is real either way.
            </div>
          )}
        </div>

        <div style={{ marginTop: 22 }}>
          <div style={{ display: 'flex', alignItems: 'baseline', marginBottom: 8 }}>
            <SectionLabel>THIS SESSION</SectionLabel>
            <div style={{ flexGrow: 1 }} />
            <span style={{ fontSize: 12.5, color: color.ink2, fontVariantNumeric: 'tabular-nums' }}>
              {written === null
                ? 'not started'
                : sessionTarget
                  ? `${words(written)} of ${words(sessionTarget)}`
                  : words(written)}
            </span>
          </div>
          {written !== null && sessionTarget ? (
            <ProgressBar pct={(written / sessionTarget) * 100} height={9} tone={color.green} />
          ) : (
            <div style={{ fontSize: 11.5, color: color.ink4 }}>
              {written === null
                ? 'Start a session to mark where today’s count begins.'
                : 'No session target set.'}
              {goals?.sessionStartedAt ? ` Began ${relativeTime(goals.sessionStartedAt)}.` : ''}
            </div>
          )}
        </div>

        <div style={{ marginTop: 22, borderTop: `1px solid ${color.lineSoft}`, paddingTop: 6 }}>
          {field('projectTarget', 'Project target', 'Words for the whole book.')}
          {field('sessionTarget', 'Session target', 'Words for one sitting.')}
          {field('defaultDocumentTarget', 'Default scene target', 'Applied to new scenes.')}
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '14px 20px',
          borderTop: `1px solid ${color.lineSoft}`,
          background: color.surfaceSoft,
        }}
      >
        <Button
          variant="ghost"
          disabled={!path}
          onClick={async () => {
            if (!path) return
            await project6.startSession(path)
            stored.reload()
            progress.reload()
          }}
        >
          {written === null ? 'Start a session' : 'Restart session'}
        </Button>
        <span style={{ fontSize: 11.5, color: color.ink4 }}>
          {saved ? 'Saved with the project.' : 'Stored in the project, not in app settings.'}
        </span>
        <div style={{ flexGrow: 1 }} />
        <Button variant="ghost" onClick={close}>
          Close
        </Button>
        <Button disabled={!draft} onClick={() => void save()}>
          Set goals
        </Button>
      </div>
    </Overlay>
  )
}

/* -------------------------------------------------------------------------- */
/* Snapshot compare (Phase 6)                                                  */
/* -------------------------------------------------------------------------- */

type Line = { kind: 'same' | 'add' | 'del'; text: string }

/**
 * A paragraph-level diff.
 *
 * Deliberately not a word-level one: prose is read in paragraphs, and a
 * character diff of a rewritten sentence produces a confetti of highlights that
 * is harder to read than the two versions side by side. This finds the longest
 * common subsequence of paragraphs and marks the rest.
 */
export function diffParagraphs(before: string, after: string): Line[] {
  const a = before.split(/\n{2,}/).map((p) => p.trim()).filter(Boolean)
  const b = after.split(/\n{2,}/).map((p) => p.trim()).filter(Boolean)

  // Classic LCS table. Paragraph counts are small enough that the quadratic
  // cost is irrelevant, and the result is stable and easy to reason about.
  const n = a.length
  const m = b.length
  const lcs: number[][] = Array.from({ length: n + 1 }, () => new Array(m + 1).fill(0))
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      lcs[i][j] = a[i] === b[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1])
    }
  }

  const out: Line[] = []
  let i = 0
  let j = 0
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      out.push({ kind: 'same', text: a[i] })
      i++
      j++
    } else if (lcs[i + 1][j] >= lcs[i][j + 1]) {
      out.push({ kind: 'del', text: a[i] })
      i++
    } else {
      out.push({ kind: 'add', text: b[j] })
      j++
    }
  }
  while (i < n) out.push({ kind: 'del', text: a[i++] })
  while (j < m) out.push({ kind: 'add', text: b[j++] })
  return out
}

function SnapshotCompare() {
  const path = useApp((s) => s.project?.path)
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const body = useApp((s) => s.body)
  const restore = useApp((s) => s.restoreSnapshot)
  const docPath = doc?.path ?? null

  const list = useAsync(
    () => api.snapshotList(path!, docPath, 50),
    [path, docPath],
    isTauri && !!path && !!docPath,
  )
  const [commit, setCommit] = useState<string | null>(null)
  const chosen = commit ?? list.data?.[0]?.id ?? null

  const old = useAsync(
    () => api.snapshotReadFile(path!, chosen!, docPath!),
    [path, chosen, docPath],
    isTauri && !!path && !!chosen && !!docPath,
  )

  const lines = old.data !== null ? diffParagraphs(old.data, body) : []
  const changed = lines.filter((l) => l.kind !== 'same').length
  const snapshot = list.data?.find((s) => s.id === chosen)

  return (
    <Overlay onClose={close} label="Snapshot compare" width={860} top={70}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          padding: '16px 20px',
          borderBottom: `1px solid ${color.lineSoft}`,
        }}
      >
        <Icon name="compare" size={18} strokeWidth={1.9} style={{ color: color.accent }} />
        <div style={{ minWidth: 0 }}>
          <div style={{ fontSize: 15, fontWeight: 700 }}>Compare with a snapshot</div>
          <div style={{ fontSize: 12, color: color.ink4, marginTop: 1 }}>
            {doc ? doc.title : 'No document selected'}
            {snapshot && ` · ${snapshot.message} · ${relativeTime(snapshot.at)}`}
          </div>
        </div>
        <div style={{ flexGrow: 1 }} />
        {(list.data?.length ?? 0) > 1 && (
          <select
            value={chosen ?? ''}
            onChange={(e) => setCommit(e.target.value)}
            style={{
              font: 'inherit',
              fontSize: 12,
              color: color.ink2,
              background: color.surface,
              border: `1px solid ${color.line}`,
              borderRadius: 999,
              padding: '4px 9px',
              maxWidth: 260,
            }}
          >
            {list.data!.map((s) => (
              <option key={s.id} value={s.id}>
                {relativeTime(s.at)} — {s.message}
              </option>
            ))}
          </select>
        )}
      </div>

      {!docPath ? (
        <div style={{ padding: 22 }}>
          <Callout tone="quiet">
            Select a document in the binder to compare it with one of its snapshots.
          </Callout>
        </div>
      ) : (list.data?.length ?? 0) === 0 && !list.loading ? (
        <div style={{ padding: 22 }}>
          <Callout tone="quiet">
            No snapshot has been taken of this document yet. Take one from File → Take Snapshot, and
            it becomes a real Git commit inside your project folder.
          </Callout>
        </div>
      ) : (
        <div style={{ display: 'flex', gap: 14, padding: 20, overflowY: 'auto' }}>
          {(['Snapshot', 'Now'] as const).map((side) => (
            <div key={side} style={{ flex: 1, minWidth: 0 }}>
              <SectionLabel style={{ marginBottom: 9 }}>{side.toUpperCase()}</SectionLabel>
              <div
                style={{
                  border: `1px solid ${color.line}`,
                  borderRadius: radius.xl,
                  padding: 14,
                  fontFamily: font.prose,
                  fontSize: 14,
                  lineHeight: 1.7,
                  background: color.surfaceSoft,
                  minHeight: 120,
                }}
              >
                {old.loading ? (
                  <span style={{ fontSize: 12.5, color: color.ink4 }}>Reading the snapshot…</span>
                ) : old.error ? (
                  <span style={{ fontSize: 12.5, color: color.orangeInk }}>{old.error}</span>
                ) : (
                  lines
                    .filter((d) => (side === 'Snapshot' ? d.kind !== 'add' : d.kind !== 'del'))
                    .map((d, i) => (
                      <p
                        key={i}
                        style={{
                          margin: '0 0 12px 0',
                          padding: d.kind === 'same' ? 0 : '3px 6px',
                          borderRadius: 5,
                          background:
                            d.kind === 'add' ? '#E4F5E9' : d.kind === 'del' ? '#FFEDE0' : 'transparent',
                          color: d.kind === 'same' ? color.ink3 : color.inkProse,
                        }}
                      >
                        {d.text}
                      </p>
                    ))
                )}
              </div>
            </div>
          ))}
        </div>
      )}

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '14px 20px',
          borderTop: `1px solid ${color.lineSoft}`,
          background: color.surfaceSoft,
        }}
      >
        <span style={{ fontSize: 12.5, color: color.ink3 }}>
          {old.data === null
            ? 'Pick a snapshot to compare against.'
            : changed === 0
              ? 'Nothing has changed since this snapshot.'
              : `${changed} paragraph${changed === 1 ? '' : 's'} differ. Restoring replaces the current text — your unsaved work is snapshotted first.`}
        </span>
        <div style={{ flexGrow: 1 }} />
        <Button variant="ghost" onClick={close}>
          Close
        </Button>
        <Button
          disabled={!chosen || !docPath || changed === 0}
          onClick={() => {
            if (chosen && docPath) void restore(chosen, docPath)
            close()
          }}
        >
          Restore snapshot
        </Button>
      </div>
    </Overlay>
  )
}

/* -------------------------------------------------------------------------- */
/* Import a folder of Markdown (Phase 2)                                       */
/* -------------------------------------------------------------------------- */

function ImportDialog() {
  const plan = useApp((s) => s.importPlan)
  const report = useApp((s) => s.importReport)
  const busy = useApp((s) => s.importBusy)
  const dismiss = useApp((s) => s.dismissImport)
  const run = useApp((s) => s.runImport)

  const done = report !== null

  return (
    <Overlay onClose={dismiss} label="Import Markdown" width={560} top={110}>
      <div style={{ padding: '18px 20px 0 20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <Icon name="markdown" size={18} strokeWidth={1.9} style={{ color: color.accent }} />
          <div style={{ fontSize: 15, fontWeight: 700, letterSpacing: '-0.01em' }}>
            {done ? 'Imported' : 'Import Markdown'}
          </div>
        </div>
      </div>

      <div style={{ padding: 20, display: 'flex', flexDirection: 'column', gap: 14 }}>
        {plan && !done && (
          <>
            <div style={{ fontSize: 13, color: color.inkProse, lineHeight: 1.55 }}>
              <strong style={{ fontWeight: 650 }}>{plan.name}</strong> holds{' '}
              <strong style={{ fontWeight: 650 }}>{plan.files}</strong>{' '}
              {plan.files === 1 ? 'Markdown file' : 'Markdown files'}
              {plan.folders > 0 && (
                <>
                  {' '}
                  across <strong style={{ fontWeight: 650 }}>{plan.folders}</strong>{' '}
                  {plan.folders === 1 ? 'sub-folder' : 'sub-folders'}
                </>
              )}
              . The binder will mirror that structure.
            </div>
            <Callout tone="info">
              Every file is copied in as a new document. Nothing already in this project is
              renamed, moved or overwritten — if you import the same folder twice you get two
              copies, not a merge.
            </Callout>
            <div style={{ fontSize: 12, color: color.ink3, fontFamily: font.mono }}>{plan.source}</div>
          </>
        )}

        {done && report && (
          <>
            <div style={{ fontSize: 13, color: color.inkProse, lineHeight: 1.55 }}>
              {report.documents} {report.documents === 1 ? 'document' : 'documents'} and{' '}
              {report.folders} {report.folders === 1 ? 'folder' : 'folders'} added,{' '}
              {words(report.words)} in all.
            </div>
            {report.skipped.length > 0 && (
              <div>
                <SectionLabel>Passed over ({report.skipped.length})</SectionLabel>
                <div
                  style={{
                    marginTop: 6,
                    maxHeight: 180,
                    overflowY: 'auto',
                    border: `1px solid ${color.lineSoft}`,
                    borderRadius: radius.md,
                    background: color.surfaceSoft,
                  }}
                >
                  {report.skipped.map((line) => (
                    <div
                      key={line}
                      style={{
                        padding: '6px 10px',
                        fontSize: 12,
                        color: color.ink3,
                        borderBottom: `1px solid ${color.lineSoft}`,
                      }}
                    >
                      {line}
                    </div>
                  ))}
                </div>
              </div>
            )}
          </>
        )}
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '14px 20px',
          borderTop: `1px solid ${color.lineSoft}`,
          background: color.surfaceSoft,
        }}
      >
        <div style={{ flexGrow: 1 }} />
        <Button variant="ghost" onClick={dismiss}>
          {done ? 'Close' : 'Cancel'}
        </Button>
        {!done && (
          <Button onClick={() => void run()} disabled={busy || !plan}>
            {busy ? 'Importing…' : `Import ${plan?.files ?? 0} files`}
          </Button>
        )}
      </div>
    </Overlay>
  )
}

/* -------------------------------------------------------------------------- */
/* Sheet templates (Phase 6)                                                   */
/* -------------------------------------------------------------------------- */

/**
 * The shapes a character or location sheet starts from.
 *
 * A template is a Markdown file under `.aegis/templates/` in the project. The
 * author edits it here or in any editor, it travels with the project, and it
 * outlives this app. The two shipped shapes are the starting point and the
 * fallback — overriding one replaces it rather than sitting beside it, and
 * deleting the override brings it back.
 */
function SheetTemplates() {
  const path = useApp((s) => s.project?.path)
  const list = useAsync(() => sheetTemplates.list(path!), [path], isTauri && !!path)
  const [pick, setPick] = useState<string | null>(null)
  const [draft, setDraft] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const chosen = list.data?.find((t) => t.id === (pick ?? list.data?.[0]?.id))
  const body = draft ?? chosen?.body ?? ''

  const save = async () => {
    if (!path || !chosen) return
    setBusy(true)
    setError(null)
    try {
      await sheetTemplates.save(path, chosen.id, body)
      setDraft(null)
      list.reload()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Overlay onClose={close} label="Sheet templates" width={780} top={80}>
      <div style={{ padding: '18px 20px 0 20px' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <Icon name="template" size={18} strokeWidth={1.9} style={{ color: color.accent }} />
          <div style={{ fontSize: 15, fontWeight: 700 }}>Sheet templates</div>
        </div>
        <div style={{ fontSize: 12.5, color: color.ink3, marginTop: 4, lineHeight: 1.55 }}>
          The shape a new character or location sheet starts from. Use{' '}
          <span style={{ fontFamily: font.mono }}>{'{{name}}'}</span>,{' '}
          <span style={{ fontFamily: font.mono }}>{'{{date}}'}</span> and{' '}
          <span style={{ fontFamily: font.mono }}>{'{{kind}}'}</span>; declare a kind with{' '}
          <span style={{ fontFamily: font.mono }}>{'<!-- kind: location -->'}</span> on the first
          line.
        </div>
      </div>

      <div style={{ display: 'flex', gap: 16, padding: 20, minHeight: 0 }}>
        <div style={{ width: 200, flexShrink: 0, display: 'flex', flexDirection: 'column', gap: 3 }}>
          {(list.data ?? []).map((t) => {
            const on = t.id === chosen?.id
            return (
              <button
                key={t.id}
                onClick={() => {
                  setPick(t.id)
                  setDraft(null)
                }}
                className={`row ${on ? 'on' : ''}`}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '8px 10px',
                  borderRadius: radius.lg,
                  cursor: 'pointer',
                  textAlign: 'left',
                  background: on ? color.blueTint : 'transparent',
                }}
              >
                <Icon
                  name={t.kind === 'location' ? 'globe' : 'person'}
                  size={14}
                  strokeWidth={1.9}
                  style={{ color: on ? color.accent : color.ink4 }}
                />
                <span style={{ flexGrow: 1, minWidth: 0, fontSize: 13, fontWeight: on ? 600 : 500 }}>
                  {t.name}
                </span>
                {t.custom && (
                  <span style={{ fontSize: 10, color: color.ink4 }}>yours</span>
                )}
              </button>
            )
          })}
        </div>

        <div style={{ flexGrow: 1, minWidth: 0 }}>
          <textarea
            value={body}
            onChange={(e) => setDraft(e.target.value)}
            spellCheck={false}
            style={{
              width: '100%',
              height: 320,
              resize: 'none',
              padding: '11px 12px',
              borderRadius: radius.lg,
              border: `1px solid ${color.line}`,
              background: color.surfaceSoft,
              fontFamily: font.mono,
              fontSize: 12.5,
              lineHeight: 1.6,
              color: color.inkProse,
              outline: 'none',
            }}
          />
          {error && (
            <Callout tone="warn" icon="alert" style={{ marginTop: 10 }}>
              {error}
            </Callout>
          )}
        </div>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 10,
          padding: '14px 20px',
          borderTop: `1px solid ${color.lineSoft}`,
          background: color.surfaceSoft,
        }}
      >
        <span style={{ fontSize: 12, color: color.ink4 }}>
          {chosen?.custom
            ? 'Yours. Removing it brings the shipped shape back — it was overridden, not deleted.'
            : 'The shipped shape. Editing it makes your own copy.'}
        </span>
        <div style={{ flexGrow: 1 }} />
        {chosen?.custom && (
          <Button
            variant="ghost"
            disabled={busy}
            onClick={async () => {
              if (!path || !chosen) return
              await sheetTemplates.forget(path, chosen.id)
              setDraft(null)
              list.reload()
            }}
          >
            Remove
          </Button>
        )}
        <Button variant="ghost" onClick={close}>
          Close
        </Button>
        <Button disabled={busy || draft === null} onClick={() => void save()}>
          {busy ? 'Saving…' : 'Save template'}
        </Button>
      </div>
    </Overlay>
  )
}

export function ProjectDialogs() {
  const dialog = useApp((s) => s.dialog)
  if (dialog === 'templates') return <Templates />
  if (dialog === 'goals') return <Goals />
  if (dialog === 'snapshot') return <SnapshotCompare />
  if (dialog === 'import') return <ImportDialog />
  if (dialog === 'sheetTemplates') return <SheetTemplates />
  return null
}
