import { useEffect } from 'react'
import { useApp } from './lib/store'
import { Welcome } from './screens/Welcome'
import { Studio } from './screens/Studio'
import { Integrity } from './screens/Integrity'
import { Settings } from './screens/Settings'
import { Research } from './screens/Research'
import { Search } from './screens/Search'
import { Story } from './screens/Story'
import { Insights } from './screens/Insights'
import { Library } from './screens/Library'
import { Paper } from './screens/Paper'
import { TechBook } from './screens/TechBook'
import { CommandBar } from './screens/CommandBar'
import { CompileDialog } from './screens/CompileDialog'
import { ProjectDialogs } from './screens/ProjectDialogs'
import { api, onMenu } from './lib/api'
import { Icon } from './components/Icon'
import { color, radius } from './lib/tokens'

function ErrorToast() {
  const error = useApp((s) => s.error)
  const dismiss = useApp((s) => s.dismissError)
  if (!error) return null
  return (
    <div
      role="alert"
      style={{
        position: 'fixed',
        bottom: 44,
        left: '50%',
        transform: 'translateX(-50%)',
        zIndex: 90,
        display: 'flex',
        alignItems: 'center',
        gap: 11,
        maxWidth: 640,
        padding: '12px 14px',
        background: color.ink,
        color: '#FFFFFF',
        borderRadius: radius.xl,
        boxShadow: '0 12px 32px rgba(15,25,32,0.28)',
        fontSize: 13,
      }}
    >
      <span style={{ color: color.yellow, display: 'flex' }}>
        <Icon name="alert" size={16} strokeWidth={2.2} />
      </span>
      <span style={{ flexGrow: 1, lineHeight: 1.5 }}>{error}</span>
      <button onClick={dismiss} aria-label="Dismiss" style={{ display: 'flex', color: '#A9BAC6', cursor: 'pointer' }}>
        <Icon name="close" size={15} strokeWidth={2.2} />
      </button>
    </div>
  )
}

export default function App() {
  const route = useApp((s) => s.route)
  const project = useApp((s) => s.project)
  const commandBarOpen = useApp((s) => s.commandBarOpen)
  const compileOpen = useApp((s) => s.compileOpen)

  useEffect(() => {
    void useApp.getState().openStartupProject()
    // Dismiss the splash window once React has painted something real.
    void api.frontendReady()
  }, [])

  // The native menu bar. Menu items never act directly — they route to the same
  // store actions the in-app controls use, so both paths behave identically.
  useEffect(() => {
    let dispose: (() => void) | undefined
    void onMenu((id) => {
      const s = useApp.getState()
      switch (id) {
        case 'new_project':
          void s.newProject()
          break
        case 'open_project':
          void s.openProjectViaDialog()
          break
        case 'import_markdown':
          if (s.project) void s.pickImportFolder()
          break
        case 'new_scene':
          if (s.project) void s.addDocument(null, 'Untitled scene', 'scene')
          break
        case 'new_folder':
          if (s.project) void s.addDocument(null, 'New folder', 'folder')
          break
        case 'new_character':
          if (s.project) void s.createSheet('New character', 'sheet', s.selectedId)
          break
        case 'new_location':
          if (s.project) void s.createSheet('New location', 'location', s.selectedId)
          break
        case 'sheet_templates':
          if (s.project) s.setDialog('sheetTemplates')
          break
        case 'snapshot':
          if (s.project) void s.takeSnapshot('Snapshot')
          break
        case 'view_scrivenings':
          s.setView('scrivenings')
          break
        case 'toggle_split':
          s.setSplit(s.splitId ? null : s.selectedId)
          break
        case 'save':
          void s.saveNow()
          break
        case 'close_project':
          s.closeProject()
          break
        case 'compile':
          if (s.project) s.setCompile(true)
          break
        case 'command_bar':
          if (s.project) s.setCommandBar(!s.commandBarOpen)
          break
        case 'settings':
          if (s.project) s.go('settings')
          break
        case 'view_editor':
          s.setView('editor')
          break
        case 'view_corkboard':
          s.setView('corkboard')
          break
        case 'view_outliner':
          s.setView('outliner')
          break
        case 'toggle_binder':
          s.toggleBinder()
          break
        case 'toggle_inspector':
          s.toggleInspector()
          break
        case 'research':
          if (s.project) s.go('research')
          break
        case 'story':
          if (s.project) s.go('story')
          break
        case 'insights':
          if (s.project) s.go('insights')
          break
        case 'integrity':
          if (s.project) s.go('integrity')
          break
        case 'help_docs':
        case 'help_privacy':
          if (s.project) s.go('settings')
          break
        default:
          break
      }
    }).then((un) => {
      dispose = un
    })
    return () => dispose?.()
  }, [])

  // Global shortcuts. The editor owns Cmd-S; everything here is navigation.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey
      if (!mod) return
      const s = useApp.getState()
      if (e.key.toLowerCase() === 'k') {
        e.preventDefault()
        if (s.project) s.setCommandBar(!s.commandBarOpen)
      }
      if (e.key.toLowerCase() === 'b' && s.project) {
        e.preventDefault()
        s.toggleBinder()
      }
      if (e.key.toLowerCase() === 'i' && s.project) {
        e.preventDefault()
        s.toggleInspector()
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  // Every route except the launcher needs a project open.
  const screen =
    !project || route === 'welcome' ? (
      <Welcome />
    ) : route === 'studio' ? (
      <Studio />
    ) : route === 'integrity' ? (
      <Integrity />
    ) : route === 'settings' ? (
      <Settings />
    ) : route === 'search' ? (
      <Search />
    ) : route === 'story' ? (
      <Story />
    ) : route === 'insights' ? (
      <Insights />
    ) : route === 'research' ? (
      <Research />
    ) : route === 'library' ? (
      <Library />
    ) : route === 'paper' ? (
      <Paper />
    ) : (
      <TechBook />
    )

  return (
    <>
      {screen}
      {project && commandBarOpen && <CommandBar />}
      {project && compileOpen && <CompileDialog />}
      {project && <ProjectDialogs />}
      <ErrorToast />
    </>
  )
}
