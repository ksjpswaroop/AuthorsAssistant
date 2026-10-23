import { useState } from 'react'
import { useAsync } from '../lib/useAsync'
import { ai, agents, lineage, sidecar, localModels, generationModels, embeddingModels } from '../lib/phases'
import { PhaseScreen, Rail, SampleNote } from '../components/PhaseScreen'
import { Icon, type IconName } from '../components/Icon'
import { Button, Callout, Chip, SectionLabel, Toggle } from '../components/ui'
import { color, font, radius, tint } from '../lib/tokens'
import { tildePath } from '../lib/format'
import { useApp } from '../lib/store'
import { isTauri } from '../lib/api'

const NAV: { id: string; label: string; icon: IconName }[] = [
  { id: 'general', label: 'General', icon: 'settings' },
  { id: 'models', label: 'Models', icon: 'sparkle' },
  { id: 'voice', label: 'Voice and skills', icon: 'wand' },
  { id: 'privacy', label: 'Privacy', icon: 'shield' },
  { id: 'compile', label: 'Compile', icon: 'download' },
]

export function Settings() {
  const [section, setSection] = useState('models')
  const ollama = useAsync(() => ai.status(), [], isTauri)
  const agent = useAsync(() => agents.status(), [], isTauri)
  const wigolo = useAsync(() => sidecar.status(), [], isTauri)
  const projectPath = useApp((s) => s.project?.path)
  const chain = useAsync(
    () => lineage.verify(projectPath!),
    [projectPath],
    isTauri && !!projectPath,
  )
  const [wigoloBusy, setWigoloBusy] = useState(false)
  const [wigoloError, setWigoloError] = useState<string | null>(null)

  // Starting wigolo is never automatic: it launches something that reaches the
  // internet on the author's behalf, which the golden rule makes their choice.
  const toggleWigolo = async () => {
    setWigoloBusy(true)
    setWigoloError(null)
    try {
      await (wigolo.data?.managed ? sidecar.stop() : sidecar.start())
      wigolo.reload()
    } catch (e) {
      setWigoloError(e instanceof Error ? e.message : String(e))
    } finally {
      setWigoloBusy(false)
    }
  }
  const installed = ollama.data ? localModels(ollama.data) : []
  const generators = ollama.data ? generationModels(ollama.data) : []
  const embedders = ollama.data ? embeddingModels(ollama.data) : []
  const [web, setWeb] = useState(false)
  const [traces, setTraces] = useState(true)
  const project = useApp((s) => s.project)

  return (
    <PhaseScreen
      title="Settings"
      phase="Phase 3"
      explains="Ollama and the agent runtime are probed live. Model role assignment and the network switches are not persisted yet."
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

            <SectionLabel style={{ margin: '18px 0 10px 0' }}>LOCAL SERVICES</SectionLabel>
            <div style={{ border: `1px solid ${color.line}`, borderRadius: 12, overflow: 'hidden' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 13, padding: '13px 16px', borderBottom: `1px solid ${color.lineSoft}` }}>
                <span style={{ width: 9, height: 9, borderRadius: '50%', flexShrink: 0, background: ollama.data?.reachable ? color.green : color.disabled }} />
                <div style={{ flexGrow: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 13.5, fontWeight: 600 }}>
                    Ollama — {ollama.loading ? 'checking…' : ollama.data?.reachable ? 'connected' : 'not running'}
                  </div>
                  <div style={{ fontSize: 12, color: color.ink3, marginTop: 2, fontFamily: font.mono }}>
                    {ollama.data?.host ?? 'http://127.0.0.1:11434'}
                  </div>
                  {ollama.data?.message && (
                    <div style={{ fontSize: 12, color: color.yellowInk, marginTop: 3 }}>{ollama.data.message}</div>
                  )}
                </div>
                <Button variant="ghost" height={31} onClick={ollama.reload}>Test</Button>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: 13, padding: '13px 16px', borderBottom: `1px solid ${color.lineSoft}` }}>
                <span style={{ width: 9, height: 9, borderRadius: '50%', flexShrink: 0, background: agent.data?.reachable ? color.green : color.disabled }} />
                <div style={{ flexGrow: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 13.5, fontWeight: 600 }}>
                    hermes-agent — {agent.data?.reachable ? 'connected' : 'not running'}
                  </div>
                  <div style={{ fontSize: 12, color: color.ink3, marginTop: 2, fontFamily: font.mono }}>
                    {agent.data?.host ?? 'http://127.0.0.1:18765'}
                  </div>
                  <div style={{ fontSize: 12, color: color.ink4, marginTop: 3 }}>
                    {agent.data ? `${agent.data.tools.length} tools ready for it to call` : 'agent runtime'}
                  </div>
                </div>
                <Button variant="ghost" height={31} onClick={agent.reload}>Test</Button>
              </div>

              <div style={{ display: 'flex', alignItems: 'center', gap: 13, padding: '13px 16px' }}>
                <span style={{ width: 9, height: 9, borderRadius: '50%', flexShrink: 0, background: wigolo.data?.running ? color.green : color.disabled }} />
                <div style={{ flexGrow: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 13.5, fontWeight: 600 }}>
                    wigolo — {wigolo.loading ? 'checking…' : wigolo.data?.running ? (wigolo.data.health ?? 'connected') : 'not running'}
                    {wigolo.data?.running && (
                      <span style={{ fontWeight: 500, color: color.ink4 }}>
                        {wigolo.data.managed ? ' · started by AegisDraft' : ' · yours, not ours to stop'}
                      </span>
                    )}
                  </div>
                  <div style={{ fontSize: 12, color: color.ink3, marginTop: 2, fontFamily: font.mono }}>
                    {wigolo.data?.base ?? 'http://127.0.0.1:3333'} · loopback only
                  </div>
                  <div style={{ fontSize: 12, color: color.ink4, marginTop: 3, lineHeight: 1.5 }}>
                    {wigolo.data?.running
                      ? 'Web similarity checks go through it. It runs on your machine and queries the internet for you — an approved phrase does leave this machine.'
                      : (wigolo.data?.resolution.detail ??
                         'Optional. Without it, Integrity works offline and the Web tab stays unavailable.')}
                  </div>
                  {!wigolo.data?.running && wigolo.data?.resolution.launcher?.via === 'npx' && (
                    <div style={{ fontSize: 11.5, color: color.yellowInk, marginTop: 4, lineHeight: 1.5 }}>
                      First start downloads wigolo, a browser engine and on-device models — around
                      1.5 GB into ~/.wigolo — so it can take a few minutes.
                    </div>
                  )}
                  {wigoloError && (
                    <div style={{ fontSize: 12, color: color.orangeInk, marginTop: 4, lineHeight: 1.5 }}>
                      {wigoloError}
                    </div>
                  )}
                </div>
                <Button
                  variant="ghost"
                  height={31}
                  disabled={wigoloBusy || (!wigolo.data?.running && !wigolo.data?.resolution.launcher)}
                  title={
                    wigolo.data?.managed
                      ? 'Stop the wigolo AegisDraft started'
                      : wigolo.data?.running
                        ? 'You started this one, so AegisDraft will not stop it'
                        : (wigolo.data?.resolution.launcher
                            ? 'Start wigolo on loopback'
                            : wigolo.data?.resolution.detail ?? '')
                  }
                  onClick={() => void (wigolo.data?.running && !wigolo.data.managed ? wigolo.reload() : toggleWigolo())}
                >
                  {wigoloBusy
                    ? (wigolo.data?.managed ? 'Stopping…' : 'Starting…')
                    : wigolo.data?.managed
                      ? 'Stop'
                      : wigolo.data?.running
                        ? 'Test'
                        : 'Start'}
                </Button>
              </div>
            </div>

            <SectionLabel style={{ margin: '24px 0 10px 0' }}>THIS PROJECT&rsquo;S LEDGER</SectionLabel>
            <div style={{ border: `1px solid ${color.line}`, borderRadius: 12, overflow: 'hidden' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 13, padding: '13px 16px' }}>
                <span
                  style={{
                    width: 9,
                    height: 9,
                    borderRadius: '50%',
                    flexShrink: 0,
                    background: !project ? color.disabled : chain.data === null ? color.green : color.orange,
                  }}
                />
                <div style={{ flexGrow: 1, minWidth: 0 }}>
                  <div style={{ fontSize: 13.5, fontWeight: 600 }}>
                    Provenance ledger —{' '}
                    {!project
                      ? 'no project open'
                      : chain.loading
                        ? 'checking…'
                        : chain.data === null
                          ? 'intact'
                          : `altered at entry ${chain.data.seq}`}
                  </div>
                  <div style={{ fontSize: 12, color: color.ink4, marginTop: 3, lineHeight: 1.5 }}>
                    {chain.data
                      ? chain.data.detail
                      : 'Every paragraph carries a record of who or what wrote it, chained so a change made after the fact is detectable.'}
                  </div>
                  <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 4, lineHeight: 1.5 }}>
                    Tamper-<em>evident</em>, not tamper-proof: anyone who can write the project
                    database can rebuild the chain. This tells you something changed, not who.
                  </div>
                </div>
                <Button variant="ghost" height={31} disabled={!project} onClick={chain.reload}>
                  Verify
                </Button>
              </div>
            </div>

            <SectionLabel style={{ margin: '24px 0 10px 0' }}>MODELS ON THIS MACHINE</SectionLabel>
            <div style={{ border: `1px solid ${color.line}`, borderRadius: 12, overflow: 'hidden' }}>
              {installed.length === 0 ? (
                <div style={{ padding: '14px 16px', fontSize: 12.5, lineHeight: 1.6, color: color.ink3 }}>
                  {ollama.data?.reachable
                    ? 'Ollama is running but has no local models. Pull one — llama3.2:3b is a good small start.'
                    : 'Start Ollama to see what is installed.'}
                </div>
              ) : (
                installed.map((m, i) => (
                  <div
                    key={m.name}
                    style={{ display: 'flex', alignItems: 'center', gap: 13, padding: '12px 16px', borderBottom: i < installed.length - 1 ? `1px solid ${color.lineSoft}` : 'none' }}
                  >
                    <span style={{ fontFamily: font.mono, fontSize: 13, flexGrow: 1 }}>{m.name}</span>
                    <span style={{ fontSize: 12, color: color.ink4, fontVariantNumeric: 'tabular-nums' }}>
                      {(m.size / 1e9).toFixed(1)} GB
                    </span>
                    <Chip fg={color.greenInk} bg={color.greenTint}>Ready</Chip>
                  </div>
                ))
              )}
            </div>
            {(ollama.data?.models.length ?? 0) > installed.length && (
              <SampleNote>
                {(ollama.data?.models.length ?? 0) - installed.length} cloud-proxied tag(s) are hidden. They report a
                tiny size and produce nothing locally, so offering one would look like a working choice that silently
                fails.
              </SampleNote>
            )}

            <SectionLabel style={{ margin: '24px 0 10px 0' }}>WHAT EACH MODEL WILL DO</SectionLabel>
            <div style={{ border: `1px solid ${color.line}`, borderRadius: 12, overflow: 'hidden' }}>
              {([
                { role: 'Drafting', sub: 'continue, rewrite, critique', pool: generators, need: 'a generation model' },
                { role: 'Embeddings', sub: 'book-wide memory, search', pool: embedders, need: 'an embedding model' },
                { role: 'Agents', sub: 'tool calls, strict JSON', pool: generators, need: 'a generation model' },
              ] as const).map((r, i) => {
                // Resolve against what is installed, by capability. Naming a
                // model that is absent reads as an assignment, and one that
                // cannot do the job fails in a way that looks like a bug.
                const chosen = r.pool[0]
                return (
                  <div
                    key={r.role}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 14,
                      padding: '13px 16px',
                      borderBottom: i < 2 ? `1px solid ${color.lineSoft}` : 'none',
                    }}
                  >
                    <div style={{ width: 168, flexShrink: 0 }}>
                      <div style={{ fontSize: 13.5, fontWeight: 600 }}>{r.role}</div>
                      <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 1 }}>{r.sub}</div>
                    </div>
                    <span style={{ fontFamily: font.mono, fontSize: 12.5, flexGrow: 1, color: chosen ? color.ink : color.ink4 }}>
                      {chosen ? chosen.name : `no ${r.need} installed`}
                    </span>
                    {chosen ? (
                      <Chip fg={color.greenInk} bg={color.greenTint}>ready</Chip>
                    ) : (
                      <Chip fg={color.yellowInk} bg={color.amberMark}>missing</Chip>
                    )}
                  </div>
                )
              })}
              <div style={{ display: 'flex', alignItems: 'center', gap: 14, padding: '13px 16px', borderTop: `1px solid ${color.lineSoft}` }}>
                <div style={{ width: 168, flexShrink: 0 }}>
                  <div style={{ fontSize: 13.5, fontWeight: 600 }}>AI detection</div>
                  <div style={{ fontSize: 11.5, color: color.ink4, marginTop: 1 }}>deliberately absent</div>
                </div>
                <span style={{ fontSize: 12.5, flexGrow: 1, color: color.ink4, lineHeight: 1.5 }}>
                  Not shipping. The method needs token log-probabilities Ollama does not expose, and a
                  bundled classifier is wrong often enough to ruin someone with no way to appeal.
                </span>
                <Chip fg={color.ink3} bg={color.lineSoft}>cut</Chip>
              </div>
            </div>
            <SampleNote>
              Chosen automatically from what is installed — role assignment is not persisted yet. Embedding and
              generation models are not interchangeable: an embedding model returns nothing when asked for prose, and a
              chat model cannot embed.
            </SampleNote>

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

        {section === 'voice' && (
          <>
            <div style={{ fontSize: 17, fontWeight: 700, letterSpacing: '-0.012em' }}>Voice and skills</div>
            <div style={{ fontSize: 13, color: color.ink3, marginTop: 3 }}>
              What the co-author learns from you, kept in the project folder and nowhere else.
            </div>

            <SectionLabel style={{ margin: '22px 0 10px 0' }}>WRITING STYLE PROFILE</SectionLabel>
            <div style={{ border: `1px solid ${color.line}`, borderRadius: 12, padding: '15px 16px' }}>
              <div style={{ fontSize: 13.5, fontWeight: 600 }}>No profile yet</div>
              <div style={{ fontSize: 12.5, lineHeight: 1.6, color: color.ink3, marginTop: 4 }}>
                The profile is built from prose you accept, and drives “rewrite in my voice”. It is a set of traits in a
                local file — not a fine-tuned model, and never uploaded for training.
              </div>
              <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
                <Button variant="ghost" height={31} disabled title="Style profiles arrive in Phase 3">
                  View traits
                </Button>
                <Button variant="ghost" height={31} disabled title="Style profiles arrive in Phase 3">
                  Reset profile
                </Button>
              </div>
            </div>

            <SectionLabel style={{ margin: '22px 0 10px 0' }}>SKILLS</SectionLabel>
            <div style={{ border: `1px solid ${color.line}`, borderRadius: 12, padding: '15px 16px' }}>
              <div style={{ fontSize: 13.5, fontWeight: 600 }}>No skills installed</div>
              <div style={{ fontSize: 12.5, lineHeight: 1.6, color: color.ink3, marginTop: 4 }}>
                After a workflow succeeds a few times, the runtime proposes it as a reusable skill. You confirm before
                anything is installed, and a skill can only call the same scoped tools the app already exposes.
              </div>
            </div>

            <Callout tone="good" style={{ marginTop: 18 }}>
              Style, skills and agent memory live under the project folder. Deleting the project deletes them with it.
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
                ['Agent traces, memory and style stay in the project', false],
                ['Opt-in web similarity', false],
                ['Agent runtime bound to loopback only', false],
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
