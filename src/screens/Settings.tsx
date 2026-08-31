import { useState } from 'react'
import { PhaseScreen, Rail, SampleNote } from '../components/PhaseScreen'
import { Icon, type IconName } from '../components/Icon'
import { Button, Callout, Chip, SectionLabel, SelectLook, Toggle } from '../components/ui'
import { color, font, radius, tint } from '../lib/tokens'
import { tildePath } from '../lib/format'
import { useApp } from '../lib/store'
import { isTauri } from '../lib/api'

const NAV: { id: string; label: string; icon: IconName }[] = [
  { id: 'general', label: 'General', icon: 'settings' },
  { id: 'models', label: 'Models', icon: 'sparkle' },
  { id: 'privacy', label: 'Privacy', icon: 'shield' },
  { id: 'compile', label: 'Compile', icon: 'download' },
]

const ROLES = [
  { role: 'Agents', sub: 'tool calls, strict JSON', value: 'hermes3:8b' },
  { role: 'Drafting', sub: 'continue, rewrite, tone', value: 'llama3.1:8b' },
  { role: 'Embeddings', sub: 'book-wide memory, similarity', value: 'nomic-embed-text' },
  { role: 'AI detection', sub: 'not an Ollama model', value: 'binoculars · onnxruntime' },
]

export function Settings() {
  const [section, setSection] = useState('models')
  const [web, setWeb] = useState(false)
  const [traces, setTraces] = useState(true)
  const project = useApp((s) => s.project)

  return (
    <PhaseScreen
      title="Settings"
      phase="Phase 3"
      explains="Ollama is not wired up yet. Model roles and the installed list are the designed shell; the network switches below are real but not yet persisted."
    >
      <Rail width={208}>
        <div style={{ padding: '14px 10px' }}>
          {NAV.map((n) => {
            const on = n.id === section
            return (
              <button
                key={n.id}
                className={`row ${on ? 'on' : ''}`}
                onClick={() => setSection(n.id)}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  width: '100%',
                  height: 32,
                  padding: '0 11px',
                  borderRadius: radius.lg - 1,
                  cursor: 'pointer',
                  fontSize: 13,
                  marginBottom: 2,
                  background: on ? tint(color.accent, 0.09) : 'transparent',
                  color: on ? color.accent : color.ink2,
                  fontWeight: on ? 600 : 500,
                }}
              >
                <Icon name={n.icon} size={15} />
                {n.label}
              </button>
            )
          })}
        </div>
        <div style={{ flexGrow: 1 }} />
        <div style={{ padding: 16, fontSize: 11.5, lineHeight: 1.6, color: color.ink4 }}>
          AegisDraft 0.1 · Tauri 2
          <br />
          {isTauri ? 'Desktop build' : 'Browser preview'}
        </div>
      </Rail>

      <div style={{ flexGrow: 1, overflowY: 'auto', padding: '22px 26px 30px 26px', minWidth: 0 }}>
        {section === 'models' && (
          <>
            <div style={{ fontSize: 17, fontWeight: 700, letterSpacing: '-0.012em' }}>Models</div>
            <div style={{ fontSize: 13, color: color.ink3, marginTop: 3 }}>
              Everything will run through your local Ollama. There is no cloud provider to configure — by design, there
              is nowhere to put an API key.
            </div>

            <div
              style={{
                marginTop: 18,
                border: `1px solid ${color.line}`,
                borderRadius: 12,
                padding: '15px 16px',
                display: 'flex',
                alignItems: 'center',
                gap: 13,
              }}
            >
              <span style={{ width: 9, height: 9, borderRadius: '50%', background: color.disabled, flexShrink: 0 }} />
              <div style={{ flexGrow: 1, minWidth: 0 }}>
                <div style={{ fontSize: 13.5, fontWeight: 600 }}>Not connected to Ollama</div>
                <div style={{ fontSize: 12, color: color.ink3, marginTop: 2, fontFamily: font.mono }}>
                  http://localhost:11434 · no client in Phase 1
                </div>
              </div>
              <Button variant="ghost" height={31} disabled title="Connection testing arrives in Phase 3">
                Test
              </Button>
            </div>

            <SectionLabel style={{ margin: '24px 0 10px 0' }}>WHAT EACH MODEL WILL DO</SectionLabel>
            <div style={{ border: `1px solid ${color.line}`, borderRadius: 12, overflow: 'hidden' }}>
              {ROLES.map((r, i) => (
                <div
                  key={r.role}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 14,
                    padding: '13px 16px',
                    borderBottom: i < ROLES.length - 1 ? `1px solid ${color.lineSoft}` : 'none',
                  }}
                >
                  <div style={{ width: 168, flexShrink: 0 }}>
                    <div style={{ fontSize: 13.5, fontWeight: 600 }}>{r.role}</div>
                    <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 1 }}>{r.sub}</div>
                  </div>
                  <div style={{ flexGrow: 1, opacity: 0.55 }}>
                    <SelectLook>
                      <span style={{ fontFamily: font.mono, fontSize: 12.5 }}>{r.value}</span>
                    </SelectLook>
                  </div>
                </div>
              ))}
            </div>
            <SampleNote>Model names are the plan recorded in docs/TECH-STACK.md, not a live list from your machine.</SampleNote>

            <SectionLabel style={{ margin: '24px 0 10px 0' }}>NETWORK</SectionLabel>
            <div style={{ border: `1px solid ${color.line}`, borderRadius: 12, overflow: 'hidden' }}>
              {[
                {
                  on: web,
                  set: setWeb,
                  title: 'Web similarity check',
                  body: 'Sends only the phrases you approve, one at a time. The manuscript is never uploaded.',
                },
                {
                  on: traces,
                  set: setTraces,
                  title: 'Keep agent traces in the project',
                  body: 'Plans and tool calls are written to the project folder so you can read what an agent did.',
                },
              ].map((row, i) => (
                <div
                  key={row.title}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 14,
                    padding: '14px 16px',
                    borderBottom: i === 0 ? `1px solid ${color.lineSoft}` : 'none',
                  }}
                >
                  <div style={{ flexGrow: 1 }}>
                    <div style={{ fontSize: 13.5, fontWeight: 600 }}>{row.title}</div>
                    <div style={{ fontSize: 12, color: color.ink3, marginTop: 2, lineHeight: 1.55 }}>{row.body}</div>
                  </div>
                  <Toggle on={row.on} onChange={() => row.set(!row.on)} />
                </div>
              ))}
            </div>

            <Callout tone={web ? 'warn' : 'good'} style={{ marginTop: 14 }}>
              {web
                ? 'Web similarity would be on. Each phrase still needs confirmation before it is sent — and none of this runs until Phase 4.'
                : 'Nothing in this app makes a network request. Phase 1 has no network client at all.'}
            </Callout>
          </>
        )}

        {section === 'general' && (
          <>
            <div style={{ fontSize: 17, fontWeight: 700 }}>General</div>
            <SectionLabel style={{ margin: '20px 0 9px 0' }}>OPEN PROJECT</SectionLabel>
            <div style={{ fontSize: 13, lineHeight: 1.7, color: color.ink2, fontFamily: font.mono, wordBreak: 'break-all' }}>
              {project ? tildePath(project.path) : 'No project open'}
            </div>
            <Callout tone="quiet" style={{ marginTop: 18 }}>
              Autosave writes the open document 800ms after you stop typing, and again when the window loses focus.
              Cmd-S saves immediately.
            </Callout>
          </>
        )}

        {section === 'privacy' && (
          <>
            <div style={{ fontSize: 17, fontWeight: 700 }}>Privacy</div>
            <div style={{ marginTop: 18, display: 'flex', flexDirection: 'column', gap: 12 }}>
              {[
                ['No network client is compiled into this build', true],
                ['Scenes are plain Markdown you can read without this app', true],
                ['Structure lives in a SQLite file inside the project folder', true],
                ['Deleted documents move to the project’s trash folder, not away', true],
                ['Agent traces stay in the project', false],
                ['Opt-in web similarity', false],
              ].map(([text, done]) => (
                <div key={text as string} style={{ display: 'flex', alignItems: 'center', gap: 11, fontSize: 13.5 }}>
                  <span style={{ color: done ? color.green : color.disabled, display: 'flex' }}>
                    <Icon name={done ? 'check' : 'circle'} size={16} strokeWidth={2.2} />
                  </span>
                  <span style={{ color: done ? color.inkProse : color.ink4 }}>{text}</span>
                  {!done && <Chip fg={color.purpleDeep} bg={color.purpleTint}>later</Chip>}
                </div>
              ))}
            </div>
          </>
        )}

        {section === 'compile' && (
          <>
            <div style={{ fontSize: 17, fontWeight: 700 }}>Compile</div>
            <Callout tone="quiet" style={{ marginTop: 18 }}>
              Layout presets, separators and export formats arrive in Phase 2. The compile dialog is reachable from the
              studio title bar and already counts the real documents it would include.
            </Callout>
          </>
        )}
      </div>
    </PhaseScreen>
  )
}
