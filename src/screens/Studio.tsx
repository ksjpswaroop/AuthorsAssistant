import { Group, Panel, Separator } from 'react-resizable-panels'
import { useApp, useLiveWordCount } from '../lib/store'
import { Binder } from '../components/Binder'
import { Editor } from '../components/Editor'
import { Corkboard } from '../components/Corkboard'
import { Outliner } from '../components/Outliner'
import { Inspector } from '../components/Inspector'
import { Scrivenings } from '../components/Scrivenings'
import { CollectionsRail } from '../components/Collections'
import { BulkBar } from '../components/BulkBar'
import { MonoStatus, OfflineStatus, OllamaStatus, StatusBar, StatusItem, TitleBar, Divider } from '../components/Chrome'
import { useCallback, useEffect, useState } from 'react'
import { Button, Dot, EmptyState, IconButton, Kbd, Segmented } from '../components/ui'
import { Icon } from '../components/Icon'
import { api } from '../lib/api'
import { chrome, color, font } from '../lib/tokens'
import { kindLabel, relativeTime, tildePath, words } from '../lib/format'

function SaveIndicator() {
  const dirty = useApp((s) => s.dirty)
  const saving = useApp((s) => s.saving)
  const lastSavedAt = useApp((s) => s.lastSavedAt)

  if (saving) return <StatusItem tone={color.ink3}>Saving…</StatusItem>
  if (dirty)
    return (
      <StatusItem tone={color.yellowInk}>
        <Dot tone={color.yellow} size={6} />
        Unsaved changes
      </StatusItem>
    )
  if (lastSavedAt)
    return (
      <StatusItem tone={color.ink3}>
        <Dot tone={color.green} size={6} />
        Saved {relativeTime(lastSavedAt)}
      </StatusItem>
    )
  return null
}

function EditorToolbar() {
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const project = useApp((s) => s.project)
  const live = useLiveWordCount()

  return (
    <div
      style={{
        height: chrome.toolBar,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '0 18px',
        borderBottom: `1px solid ${color.lineSoft}`,
      }}
    >
      <span style={{ fontSize: 12.5, color: color.ink4 }}>{project?.name}</span>
      <Icon name="chevronRight" size={13} strokeWidth={2} style={{ color: color.disabled }} />
      <span style={{ fontSize: 12.5, fontWeight: 600, color: color.ink }}>{doc?.title ?? 'Nothing open'}</span>
      <div style={{ flexGrow: 1 }} />
      <span style={{ fontSize: 12, color: color.ink3, fontVariantNumeric: 'tabular-nums' }}>{words(live)} words</span>
      <SaveIndicator />
    </div>
  )
}

/**
 * Phase 6: the second pane is editable.
 *
 * It opens read-only, which is right for the common case of keeping a research
 * note or an earlier chapter in view while writing. Turning on Edit makes it a
 * real second editor with its own save — useful for moving a paragraph between
 * two scenes without leaving either one.
 *
 * It saves on a timer and on blur rather than through the main editor's store,
 * because two panes sharing one `body` would have them overwrite each other.
 */
function SplitPane() {
  const splitId = useApp((s) => s.splitId)
  const setSplit = useApp((s) => s.setSplit)
  const doc = useApp((s) => s.docs.find((d) => d.id === s.splitId))
  const project = useApp((s) => s.project)
  const [body, setBody] = useState('')
  const [editing, setEditing] = useState(false)
  const [dirty, setDirty] = useState(false)
  const [savedAt, setSavedAt] = useState<number | null>(null)

  useEffect(() => {
    if (!splitId || !project) return
    let live = true
    setEditing(false)
    setDirty(false)
    void api.readDocument(project.path, splitId).then((t) => {
      if (live) setBody(t)
    })
    return () => {
      live = false
    }
  }, [splitId, project])

  const save = useCallback(async () => {
    if (!splitId || !project || !dirty) return
    await api.writeDocument(project.path, splitId, body)
    setDirty(false)
    setSavedAt(Date.now())
    // Refresh the binder so the word count and the outliner agree with the file.
    await useApp.getState().refreshDocs()
  }, [splitId, project, body, dirty])

  useEffect(() => {
    if (!dirty) return
    const t = setTimeout(() => void save(), 1200)
    return () => clearTimeout(t)
  }, [dirty, save])

  // A pane closed or switched away from must not silently drop an edit.
  useEffect(() => () => void save(), [save])

  if (!splitId || !doc) return null

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', borderLeft: `1px solid ${color.line}`, background: color.surfaceSoft }}>
      <div
        style={{
          height: chrome.toolBar,
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          gap: 9,
          padding: '0 14px',
          borderBottom: `1px solid ${color.lineSoft}`,
        }}
      >
        <Icon name="split" size={14} strokeWidth={1.9} style={{ color: color.ink4 }} />
        <span style={{ fontSize: 12.5, fontWeight: 600, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
          {doc.title}
        </span>
        <span style={{ fontSize: 11.5, color: color.ink5 }}>
          {editing ? (dirty ? 'unsaved' : savedAt ? 'saved' : 'editing') : 'reference'}
        </span>
        <div style={{ flexGrow: 1 }} />
        <IconButton
          icon={editing ? 'eye' : 'wand'}
          size={26}
          title={editing ? 'Back to reading' : 'Edit this pane'}
          active={editing}
          onClick={() => {
            if (editing) void save()
            setEditing(!editing)
          }}
        />
        <IconButton
          icon="close"
          size={26}
          title="Close the reference pane"
          onClick={() => {
            void save()
            setSplit(null)
          }}
        />
      </div>
      {editing ? (
        <textarea
          value={body}
          onChange={(e) => {
            setBody(e.target.value)
            setDirty(true)
          }}
          onBlur={() => void save()}
          spellCheck
          style={{
            flexGrow: 1,
            resize: 'none',
            border: 'none',
            outline: 'none',
            background: 'transparent',
            padding: '26px 26px 60px 26px',
            fontFamily: font.prose,
            fontSize: 15.5,
            lineHeight: 1.72,
            color: color.inkProse,
          }}
        />
      ) : (
        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '26px 26px 60px 26px' }}>
          <div style={{ fontFamily: font.prose, fontSize: 15.5, lineHeight: 1.72, color: color.ink2, whiteSpace: 'pre-wrap' }}>
            {body.trim() || 'This document is empty.'}
          </div>
        </div>
      )}
    </div>
  )
}

function CompositionMode() {
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const body = useApp((s) => s.body)
  const bodyLoadedFor = useApp((s) => s.bodyLoadedFor)
  const edit = useApp((s) => s.edit)
  const saveNow = useApp((s) => s.saveNow)
  const setComposition = useApp((s) => s.setComposition)
  const live = useLiveWordCount()

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setComposition(false)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [setComposition])

  return (
    <div style={{ position: 'fixed', inset: 0, background: color.surface, zIndex: 60, display: 'flex', flexDirection: 'column' }}>
      <div
        className="drag-region"
        style={{ height: 38, flexShrink: 0, display: 'flex', alignItems: 'center', justifyContent: 'flex-end', gap: 14, padding: '0 16px' }}
      >
        <span style={{ fontSize: 12, color: color.ink5, fontVariantNumeric: 'tabular-nums' }}>{words(live)} words</span>
        <button
          className="no-drag ghost"
          onClick={() => setComposition(false)}
          style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 12.5, color: color.ink3, cursor: 'pointer', padding: '4px 8px', borderRadius: 7 }}
        >
          <Icon name="close" size={14} strokeWidth={2.2} />
          Leave composition
        </button>
      </div>
      <div style={{ flexGrow: 1, overflowY: 'auto', display: 'flex', justifyContent: 'center' }}>
        <div style={{ padding: '10vh 0 40vh 0' }}>
          {doc && doc.kind !== 'folder' ? (
            <Editor
              key={`comp-${bodyLoadedFor ?? doc.id}`}
              markdown={body}
              columnWidth={720}
              onChange={edit}
              onSaveRequest={() => void saveNow()}
            />
          ) : (
            <EmptyState icon="doc" title="Open a document first" />
          )}
        </div>
      </div>
    </div>
  )
}

function CenterPane() {
  const view = useApp((s) => s.view)
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const body = useApp((s) => s.body)
  const bodyLoadedFor = useApp((s) => s.bodyLoadedFor)
  const edit = useApp((s) => s.edit)
  const saveNow = useApp((s) => s.saveNow)
  const showBinder = useApp((s) => s.showBinder)
  const showInspector = useApp((s) => s.showInspector)

  const columnWidth = !showBinder && !showInspector ? 740 : 680

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', minWidth: 0, background: color.surface }}>
      <EditorToolbar />

      {view === 'corkboard' ? (
        <Corkboard />
      ) : view === 'outliner' ? (
        <Outliner />
      ) : view === 'scrivenings' ? (
        <Scrivenings />
      ) : !doc ? (
        <EmptyState icon="doc" title="Nothing open" body="Pick a document in the binder, or add one with the + button." />
      ) : doc.kind === 'folder' ? (
        <EmptyState
          icon="folder"
          title={doc.title}
          body="Folders group documents. Select a scene inside it, or switch to the corkboard to see everything at once."
        />
      ) : (
        <div style={{ flexGrow: 1, overflowY: 'auto', display: 'flex', justifyContent: 'center' }}>
          <div style={{ padding: '54px 0 220px 0' }}>
            <h1
              style={{
                margin: '0 0 6px 0',
                width: columnWidth,
                fontFamily: font.prose,
                fontSize: 33,
                fontWeight: 600,
                letterSpacing: '-0.015em',
                lineHeight: 1.2,
                color: color.ink,
              }}
            >
              {doc.title}
            </h1>
            <div style={{ fontSize: 12, color: color.ink5, marginBottom: 34, letterSpacing: '0.02em' }}>
              {kindLabel(doc.kind)} · edited{' '}
              {relativeTime(doc.updatedAt)}
            </div>
            {/* Keyed on the document so each one gets a fresh editor. */}
            <Editor
              key={bodyLoadedFor ?? doc.id}
              markdown={body}
              columnWidth={columnWidth}
              onChange={edit}
              onSaveRequest={() => void saveNow()}
            />
          </div>
        </div>
      )}
    </div>
  )
}

export function Studio() {
  const composition = useApp((s) => s.composition)
  const splitId = useApp((s) => s.splitId)
  const selectedId = useApp((s) => s.selectedId)
  const setSplit = useApp((s) => s.setSplit)
  const takeSnapshot = useApp((s) => s.takeSnapshot)
  const project = useApp((s) => s.project)
  const docs = useApp((s) => s.docs)
  const view = useApp((s) => s.view)
  const setView = useApp((s) => s.setView)
  const showBinder = useApp((s) => s.showBinder)
  const showInspector = useApp((s) => s.showInspector)
  const toggleBinder = useApp((s) => s.toggleBinder)
  const toggleInspector = useApp((s) => s.toggleInspector)
  const setCommandBar = useApp((s) => s.setCommandBar)
  const setCompile = useApp((s) => s.setCompile)
  const go = useApp((s) => s.go)

  const total = docs.reduce((n, d) => n + d.wordCount, 0)

  if (composition) return <CompositionMode />

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: color.surface }}>
      <TitleBar
        title={project?.name ?? 'AegisDraft'}
        subtitle={`${project?.kind ?? 'novel'} · ${words(total)} words`}
        center={
          <Segmented
            value={view}
            onChange={setView}
            options={[
              { id: 'editor', label: 'Editor', icon: 'lines' },
              { id: 'corkboard', label: 'Corkboard', icon: 'grid' },
              { id: 'outliner', label: 'Outliner', icon: 'list' },
              { id: 'scrivenings', label: 'Scrivenings', icon: 'stack' },
            ]}
          />
        }
        right={
          <>
            <IconButton icon="panelLeft" title="Show or hide the binder" active={showBinder} onClick={toggleBinder} />
            <IconButton
              icon="panelRight"
              title="Show or hide the inspector"
              active={showInspector}
              onClick={toggleInspector}
            />
            <Divider />
            <IconButton
              icon="split"
              title={splitId ? 'Close the reference pane' : 'Open the selected document beside the editor'}
              active={!!splitId}
              onClick={() => setSplit(splitId ? null : selectedId)}
            />
            <IconButton
              icon="clock"
              title="Take a snapshot of the project"
              onClick={() => void takeSnapshot('Snapshot')}
            />
            <IconButton icon="database" title="Research" onClick={() => go('research')} />
            <IconButton icon="shield" title="Integrity" onClick={() => go('integrity')} />
            <IconButton icon="settings" title="Settings" onClick={() => go('settings')} />
            <button
              className="ghost"
              onClick={() => setCommandBar(true)}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 7,
                height: 28,
                padding: '0 10px 0 9px',
                borderRadius: 7,
                cursor: 'pointer',
                color: color.ink2,
                fontSize: 12.5,
                fontWeight: 500,
              }}
            >
              <Icon name="sparkle" size={14} strokeWidth={2} />
              Ask
              <Kbd>⌘K</Kbd>
            </button>
            <Button height={28} onClick={() => setCompile(true)}>
              Compile
            </Button>
          </>
        }
      />

      <div style={{ flexGrow: 1, minHeight: 0, position: 'relative' }}>
        <BulkBar />
        <Group orientation="horizontal" id="aegis-studio">
          {showBinder && (
            <>
              <Panel id="binder" defaultSize="18" minSize="12" maxSize="34">
                <div
                  style={{
                    height: '100%',
                    borderRight: `1px solid ${color.line}`,
                    display: 'flex',
                    flexDirection: 'column',
                    background: color.rail,
                  }}
                >
                  <div style={{ flexGrow: 1, minHeight: 0 }}>
                    <Binder />
                  </div>
                  <div style={{ flexShrink: 0, borderTop: `1px solid ${color.lineFaint}`, padding: '0 8px 8px 8px', maxHeight: 260, overflowY: 'auto' }}>
                    <CollectionsRail />
                  </div>
                </div>
              </Panel>
              <Separator style={{ width: 1, background: color.line, cursor: 'col-resize' }} />
            </>
          )}

          <Panel id="center" minSize="30">
            <CenterPane />
          </Panel>

          {splitId && (
            <>
              <Separator style={{ width: 1, background: color.line, cursor: 'col-resize' }} />
              <Panel id="split" defaultSize="28" minSize="18" maxSize="46">
                <SplitPane />
              </Panel>
            </>
          )}

          {showInspector && (
            <>
              <Separator style={{ width: 1, background: color.line, cursor: 'col-resize' }} />
              <Panel id="inspector" defaultSize="23" minSize="16" maxSize="40">
                <div style={{ height: '100%' }}>
                  <Inspector />
                </div>
              </Panel>
            </>
          )}
        </Group>
      </div>

      <StatusBar
        left={
          <>
            <OllamaStatus />
            <OfflineStatus />
            {project && <MonoStatus>{tildePath(project.path)}</MonoStatus>}
          </>
        }
        right={
          <>
            <span>{docs.filter((d) => d.kind !== 'folder').length} documents</span>
            <span style={{ fontVariantNumeric: 'tabular-nums' }}>{words(total)} words</span>
          </>
        }
      />
    </div>
  )
}
