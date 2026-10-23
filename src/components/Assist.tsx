import { useState } from 'react'
import { useApp } from '../lib/store'
import { useAsync } from '../lib/useAsync'
import { ai, cloudModels, generationModels, isCloudModel, lineage } from '../lib/phases'
import { isTauri } from '../lib/api'
import { Button, Callout, Chip, SectionLabel, Toggle } from './ui'
import { Icon } from './Icon'
import { color, font, radius } from '../lib/tokens'

type Mode = 'continue' | 'rewrite' | 'critique'

const MODES: { id: Mode; label: string; blurb: string }[] = [
  { id: 'continue', label: 'Continue', blurb: 'Writes on from where the scene stops, matching its voice.' },
  { id: 'rewrite', label: 'Rewrite', blurb: 'Returns the same passage, tightened — matched to your style profile when one exists (Inspector → Voice).' },
  { id: 'critique', label: 'Critique', blurb: 'Names problems. Changes nothing.' },
]

/**
 * The Phase 3 writing loop. Generated prose is never inserted automatically —
 * it lands in a panel the author copies from. A tool that silently edits the
 * manuscript is one the author stops trusting.
 */
export function Assist({ onClose }: { onClose: () => void }) {
  const path = useApp((s) => s.project?.path)
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const status = useAsync(() => ai.status(), [], isTauri)
  const models = status.data ? generationModels(status.data) : []
  const cloud = status.data ? cloudModels(status.data) : []
  const [allowCloud, setAllowCloud] = useState(false)
  const [model, setModel] = useState<string | null>(null)
  const [mode, setMode] = useState<Mode>('continue')
  const [output, setOutput] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [accepted, setAccepted] = useState(false)

  const chosen = model ?? models[0]?.name ?? null

  // Taking a proposal is the only path that leaves a record. The copy button
  // beside it does not, and the panel says so rather than letting the author
  // assume the app can tell.
  const accept = async () => {
    if (!path || !doc || !chosen || !output.trim()) return
    setBusy(true)
    setError(null)
    try {
      await lineage.acceptProposal(path, doc.id, output, chosen, { mode, allowCloud })
      setAccepted(true)
      await useApp.getState().refreshDocs()
      await useApp.getState().select(doc.id)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const run = async () => {
    if (!path || !doc || !chosen) return
    setBusy(true)
    setError(null)
    setOutput('')
    try {
      setAccepted(false)
      setOutput(await ai.assist(path, chosen, doc.id, mode, { allowCloud }))
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: color.surface }}>
      <div style={{ height: 44, flexShrink: 0, display: 'flex', alignItems: 'center', gap: 9, padding: '0 14px', borderBottom: `1px solid ${color.lineSoft}` }}>
        <Icon name="sparkle" size={15} strokeWidth={1.9} style={{ color: color.accent }} />
        <span style={{ fontSize: 12.5, fontWeight: 600 }}>Assist</span>
        <div style={{ flexGrow: 1 }} />
        <button onClick={onClose} aria-label="Close" style={{ display: 'flex', color: color.ink4, cursor: 'pointer' }}>
          <Icon name="close" size={15} strokeWidth={2.2} />
        </button>
      </div>

      <div style={{ flexGrow: 1, overflowY: 'auto', padding: 16 }}>
        {!isTauri ? (
          <Callout tone="quiet">Assist needs the desktop app.</Callout>
        ) : !status.data?.reachable ? (
          <Callout tone="warn" icon="alert">
            {status.data?.message ?? 'Checking for Ollama…'}
          </Callout>
        ) : models.length === 0 ? (
          <Callout tone="warn" icon="alert">
            Ollama is running but has no model that can write prose. Pull one — <span style={{ fontFamily: font.mono }}>llama3.2:3b</span> is a good small start. Embedding models cannot generate text.
          </Callout>
        ) : (
          <>
            <SectionLabel style={{ marginBottom: 8 }}>ON THIS MACHINE</SectionLabel>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginBottom: 14 }}>
              {models.map((m) => (
                <button
                  key={m.name}
                  onClick={() => setModel(m.name)}
                  style={{
                    fontFamily: font.mono, fontSize: 11.5, padding: '4px 9px', borderRadius: radius.sm + 1,
                    cursor: 'pointer',
                    border: `1px solid ${chosen === m.name ? color.accent : color.line}`,
                    color: chosen === m.name ? color.accent : color.ink2,
                    background: chosen === m.name ? color.blueTint : 'transparent',
                  }}
                >
                  {m.name}
                </button>
              ))}
            </div>

            {cloud.length > 0 && (
              <div style={{ marginBottom: 16 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                  <SectionLabel>SOMEWHERE ELSE</SectionLabel>
                  <div style={{ flexGrow: 1 }} />
                  <Toggle on={allowCloud} onChange={() => setAllowCloud((v) => !v)} tone={color.orange} />
                </div>
                <div style={{ fontSize: 11.5, lineHeight: 1.55, color: allowCloud ? color.orangeInk : color.ink4, marginBottom: 8 }}>
                  {allowCloud
                    ? 'On. Choosing one of these sends the scene you are working on to a remote service.'
                    : 'These run on someone else’s computer. Turn this on to use one.'}
                </div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
                  {cloud.map((m) => (
                    <button
                      key={m.name}
                      onClick={() => allowCloud && setModel(m.name)}
                      disabled={!allowCloud}
                      title={allowCloud ? 'Sends your text off this machine' : 'Turn on remote models first'}
                      style={{
                        fontFamily: font.mono, fontSize: 11.5, padding: '4px 9px', borderRadius: radius.sm + 1,
                        cursor: allowCloud ? 'pointer' : 'not-allowed',
                        opacity: allowCloud ? 1 : 0.5,
                        border: `1px solid ${chosen === m.name ? color.orange : color.line}`,
                        color: chosen === m.name ? color.orangeInk : color.ink2,
                        background: chosen === m.name ? color.orangeTint : 'transparent',
                      }}
                    >
                      {m.name}
                    </button>
                  ))}
                </div>
              </div>
            )}

            {chosen && isCloudModel(chosen) && (
              <Callout tone="warn" icon="globe" style={{ marginBottom: 14 }}>
                <b style={{ fontWeight: 700 }}>{chosen}</b> runs remotely. Running this will send the text of this
                document to a service outside your machine.
              </Callout>
            )}

            <SectionLabel style={{ marginBottom: 8 }}>WHAT TO DO</SectionLabel>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 3, marginBottom: 14 }}>
              {MODES.map((m) => (
                <button
                  key={m.id}
                  onClick={() => setMode(m.id)}
                  className={`row ${mode === m.id ? 'on' : ''}`}
                  style={{
                    display: 'block', width: '100%', textAlign: 'left', padding: '9px 11px',
                    borderRadius: radius.lg, cursor: 'pointer',
                    background: mode === m.id ? color.blueTint : 'transparent',
                  }}
                >
                  <div style={{ fontSize: 13, fontWeight: 600, color: mode === m.id ? color.accent : color.ink }}>{m.label}</div>
                  <div style={{ fontSize: 11.5, color: color.ink3, marginTop: 2 }}>{m.blurb}</div>
                </button>
              ))}
            </div>

            <Button icon="sparkle" disabled={busy || !doc} onClick={() => void run()} style={{ width: '100%', justifyContent: 'center' }}>
              {busy ? 'Thinking…' : chosen && isCloudModel(chosen) ? `Send to ${chosen}` : `Run on “${doc?.title ?? 'nothing'}”`}
            </Button>

            {error && <Callout tone="warn" icon="alert" style={{ marginTop: 14 }}>{error}</Callout>}

            {output && (
              <div style={{ marginTop: 16 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                  <SectionLabel>RESULT</SectionLabel>
                  <div style={{ flexGrow: 1 }} />
                  <Chip fg={accepted ? color.purpleDeep : color.ink3} bg={accepted ? color.purpleTint : color.lineSoft}>
                    {accepted ? 'in your manuscript, recorded' : 'not inserted'}
                  </Chip>
                </div>
                <div
                  style={{
                    fontFamily: mode === 'critique' ? font.ui : font.prose,
                    fontSize: mode === 'critique' ? 13 : 15,
                    lineHeight: 1.7,
                    color: color.inkProse,
                    background: color.surfaceSoft,
                    border: `1px solid ${color.lineFaint}`,
                    borderRadius: radius.xl,
                    padding: '12px 14px',
                    whiteSpace: 'pre-wrap',
                  }}
                >
                  {output}
                </div>
                <div style={{ display: 'flex', gap: 8, marginTop: 10, alignItems: 'center' }}>
                  {mode !== 'critique' && (
                    <Button
                      height={30}
                      icon="check"
                      disabled={busy || accepted || !doc}
                      title="Add this to the document and record that a model wrote it"
                      onClick={() => void accept()}
                    >
                      {accepted ? 'Added' : 'Use this'}
                    </Button>
                  )}
                  <Button
                    variant="ghost"
                    height={30}
                    icon="copy"
                    onClick={() => void navigator.clipboard.writeText(output)}
                  >
                    Copy
                  </Button>
                </div>
                <Callout tone="quiet" style={{ marginTop: 12 }}>
                  {mode === 'critique' ? (
                    <>A critique changes nothing. It is a reading of the page, not prose for it.</>
                  ) : accepted ? (
                    <>
                      Added to the end of this document and recorded as{' '}
                      <strong style={{ fontWeight: 650 }}>{chosen}</strong>&rsquo;s writing that you
                      accepted. Inspector → Lineage shows the chain.
                    </>
                  ) : (
                    <>
                      <strong style={{ fontWeight: 650 }}>Use this</strong> adds the text and
                      records which model wrote it. Copying it by hand works too, but AegisDraft
                      cannot see that happen — pasted text is indistinguishable from prose you
                      typed, and the lineage will say you wrote it.
                    </>
                  )}
                </Callout>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  )
}
