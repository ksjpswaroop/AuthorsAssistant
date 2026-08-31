import { useEffect } from 'react'
import { useApp } from './lib/store'
import { Welcome } from './screens/Welcome'
import { Studio } from './screens/Studio'
import { Integrity } from './screens/Integrity'
import { Settings } from './screens/Settings'
import { Research } from './screens/Research'
import { Library } from './screens/Library'
import { Paper } from './screens/Paper'
import { TechBook } from './screens/TechBook'
import { CommandBar } from './screens/CommandBar'
import { CompileDialog } from './screens/CompileDialog'
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
      <ErrorToast />
    </>
  )
}
