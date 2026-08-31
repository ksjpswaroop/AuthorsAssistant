import { Group, Panel, Separator } from 'react-resizable-panels'
import { useApp, useLiveWordCount } from '../lib/store'
import { Binder } from '../components/Binder'
import { Editor } from '../components/Editor'
import { Corkboard } from '../components/Corkboard'
import { Outliner } from '../components/Outliner'
import { Inspector } from '../components/Inspector'
import { MonoStatus, OfflineStatus, OllamaStatus, StatusBar, StatusItem, TitleBar, Divider } from '../components/Chrome'
import { Button, Dot, EmptyState, IconButton, Kbd, Segmented } from '../components/ui'
import { Icon } from '../components/Icon'
import { chrome, color, font } from '../lib/tokens'
import { relativeTime, tildePath, words } from '../lib/format'

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
              {doc.kind === 'scene' ? 'Scene' : doc.kind === 'sheet' ? 'Character sheet' : 'Document'} · edited{' '}
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

      <div style={{ flexGrow: 1, minHeight: 0 }}>
        <Group orientation="horizontal" id="aegis-studio">
          {showBinder && (
            <>
              <Panel id="binder" defaultSize="18" minSize="12" maxSize="34">
                <div style={{ height: '100%', borderRight: `1px solid ${color.line}` }}>
                  <Binder />
                </div>
              </Panel>
              <Separator style={{ width: 1, background: color.line, cursor: 'col-resize' }} />
            </>
          )}

          <Panel id="center" minSize="30">
            <CenterPane />
          </Panel>

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
