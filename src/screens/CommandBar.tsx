import { useState } from 'react'
import { Overlay } from '../components/Overlay'
import { Icon } from '../components/Icon'
import { useApp } from '../lib/store'
import { useAsync } from '../lib/useAsync'
import {
  ai,
  agentLoop,
  generationModels,
  isCloudModel,
  type AgentRun,
  type Hop,
  type Pending,
} from '../lib/phases'
import { isTauri } from '../lib/api'
import { Button, Callout, Chip, Kbd, SectionLabel, Toggle } from '../components/ui'
import { color, font, radius } from '../lib/tokens'

const EXAMPLES: Record<string, string[]> = {
  controller: [
    'Create a folder for Act III and add three empty chapters',
    'Create a character sheet for the antagonist',
    'Mark every scene in Act I as Draft',
  ],
  continuity: [
    'Check whether the timeline in this chapter matches the rest of the book',
    'Find every place a character’s injury is mentioned',
  ],
  integrity: [
    'Check this chapter for passages I have reused from elsewhere',
    'Explain what the measurements on this chapter mean',
  ],
  worldbuilder: [
    'Write the lore for the desert they cross and file it under Worldbuilding',
    'Build a character sheet for the water-seller and link it to this scene',
  ],
}

function HopRow({ hop, index }: { hop: Hop; index: number }) {
  const [open, setOpen] = useState(false)
  return (
    <div style={{ border: `1px solid ${color.lineSoft}`, borderRadius: radius.lg, marginBottom: 4 }}>
      <button
        onClick={() => setOpen(!open)}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 11,
          width: '100%',
          textAlign: 'left',
          padding: '9px 12px',
          cursor: 'pointer',
        }}
      >
        <span
          style={{
            width: 20,
            height: 20,
            borderRadius: '50%',
            flexShrink: 0,
            background: hop.ok ? color.greenTint : color.orangeTint,
            color: hop.ok ? color.greenInk : color.orangeInk,
            fontSize: 11,
            fontWeight: 700,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          {index + 1}
        </span>
        <span style={{ fontFamily: font.mono, fontSize: 12, color: color.ink2, flexGrow: 1 }}>
          {hop.tool}
        </span>
        {!hop.ok && (
          <Chip fg={color.orangeInk} bg={color.orangeTint}>
            failed
          </Chip>
        )}
        <Icon name={open ? 'chevronDown' : 'chevronRight'} size={13} strokeWidth={2} style={{ color: color.ink4 }} />
      </button>
      {open && (
        <pre
          style={{
            margin: 0,
            padding: '0 12px 10px 43px',
            fontFamily: font.mono,
            fontSize: 11,
            lineHeight: 1.6,
            color: color.ink3,
            whiteSpace: 'pre-wrap',
            maxHeight: 200,
            overflow: 'auto',
          }}
        >
          {JSON.stringify(hop.args, null, 2)}
          {'\n\n→ '}
          {JSON.stringify(hop.result, null, 2).slice(0, 900)}
        </pre>
      )}
    </div>
  )
}

/**
 * Phase 5: the command bar drives a real tool-calling loop.
 *
 * What it replaced was string matching in `planner.ts` — it looked for the word
 * "folder" and a number, and could not do anything it had not been taught in
 * advance. Now the model is given the tool catalogue with real JSON schemas and
 * decides for itself, one hop at a time, reading each result before the next.
 *
 * The guardrail did not move and is not softened here: when the model asks for
 * a guarded tool the run stops with nothing executed, and this screen asks the
 * author. Declining sends the refusal back as the tool's result, so the model
 * can say what it will do instead — a "no" is a turn in the conversation, not a
 * dead end.
 */
export function CommandBar() {
  const close = () => useApp.getState().setCommandBar(false)
  const path = useApp((s) => s.project?.path)
  const reload = useApp((s) => s.openProjectAt)

  const status = useAsync(() => ai.status(), [], isTauri)
  const profiles = useAsync(() => agentLoop.profiles(), [], isTauri)
  const models = status.data ? generationModels(status.data) : []

  const [profileId, setProfileId] = useState('controller')
  const [model, setModel] = useState<string | null>(null)
  const [allowCloud, setAllowCloud] = useState(false)
  const [input, setInput] = useState('')
  const [run, setRun] = useState<AgentRun | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const chosen = model ?? models[0]?.name ?? null
  const profile = profiles.data?.find((p) => p.id === profileId)
  const pending: Pending | null =
    run?.outcome.kind === 'needsApproval' ? run.outcome.pending : null

  const start = async () => {
    if (!path || !chosen || !input.trim()) return
    setBusy(true)
    setError(null)
    setRun(null)
    try {
      const out = await agentLoop.run(path, chosen, profileId, input.trim(), allowCloud)
      setRun(out)
      await reload(path)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  const answer = async (approved: boolean) => {
    if (!path || !pending) return
    setBusy(true)
    setError(null)
    try {
      const out = await agentLoop.resume(path, pending, approved)
      setRun(out)
      await reload(path)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <Overlay onClose={close} label="Command bar">
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 13,
          padding: '17px 19px',
          borderBottom: `1px solid ${color.lineSoft}`,
        }}
      >
        <Icon name="sparkle" size={19} strokeWidth={1.9} style={{ color: color.accent }} />
        <input
          autoFocus
          value={input}
          onChange={(e) => {
            setInput(e.target.value)
            setRun(null)
          }}
          onKeyDown={(e) => e.key === 'Enter' && !busy && void start()}
          placeholder={EXAMPLES[profileId]?.[0] ?? 'Ask for something'}
          style={{ flexGrow: 1, fontSize: 16.5, border: 'none', outline: 'none', background: 'transparent' }}
        />
        <Kbd>esc</Kbd>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          padding: '8px 19px',
          background: color.rail,
          borderBottom: `1px solid ${color.lineSoft}`,
          flexWrap: 'wrap',
        }}
      >
        {(profiles.data ?? []).map((p) => {
          const on = p.id === profileId
          return (
            <button
              key={p.id}
              onClick={() => {
                setProfileId(p.id)
                setRun(null)
              }}
              title={p.blurb}
              style={{
                padding: '4px 10px',
                borderRadius: 999,
                fontSize: 12,
                fontWeight: on ? 650 : 500,
                cursor: 'pointer',
                color: on ? color.surface : color.ink2,
                background: on ? color.accent : color.surface,
                border: `1px solid ${on ? color.accent : color.line}`,
              }}
            >
              {p.name}
            </button>
          )
        })}
        <div style={{ flexGrow: 1 }} />
        {models.length > 0 && (
          <select
            value={chosen ?? ''}
            onChange={(e) => setModel(e.target.value)}
            style={{
              font: 'inherit',
              fontSize: 11.5,
              color: color.ink2,
              background: color.surface,
              border: `1px solid ${color.line}`,
              borderRadius: 999,
              padding: '3px 8px',
            }}
          >
            {models.map((m) => (
              <option key={m.name} value={m.name}>
                {m.name}
              </option>
            ))}
          </select>
        )}
      </div>

      <div style={{ overflowY: 'auto', padding: '16px 19px 18px 19px' }}>
        {profile && (
          <div style={{ fontSize: 12.5, color: color.ink3, lineHeight: 1.6, marginBottom: 14 }}>
            {profile.blurb}{' '}
            <span style={{ color: color.ink4 }}>
              It can call {profile.tools.length} of the gateway&rsquo;s tools and nothing else.
            </span>
          </div>
        )}

        {!status.data?.reachable && (
          <Callout tone="warn" icon="alert" style={{ marginBottom: 14 }}>
            Ollama is not running, so there is no model to plan with. Start it and reopen this.
          </Callout>
        )}
        {models.length === 0 && status.data?.reachable && (
          <Callout tone="warn" icon="alert" style={{ marginBottom: 14 }}>
            Ollama is running but has no model that can write. Pull one —{' '}
            <span style={{ fontFamily: font.mono }}>llama3.2:3b</span> is a good small start.
          </Callout>
        )}

        {chosen && isCloudModel(chosen) && (
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              padding: '10px 12px',
              borderRadius: radius.lg,
              background: color.orangeTint,
              border: `1px solid ${color.orangeMark}`,
              marginBottom: 14,
            }}
          >
            <div style={{ flexGrow: 1, fontSize: 12.5, lineHeight: 1.55, color: color.inkProse }}>
              <strong style={{ fontWeight: 650 }}>{chosen}</strong> is a remote model. Your
              instruction, the tool results and pieces of your manuscript go to someone else&rsquo;s
              computer for this run.
            </div>
            <Toggle on={allowCloud} onChange={() => setAllowCloud(!allowCloud)} tone={color.orangeInk} />
          </div>
        )}

        {!run && !busy && (
          <>
            <SectionLabel style={{ marginBottom: 9 }}>TRY</SectionLabel>
            {(EXAMPLES[profileId] ?? []).map((eg) => (
              <button
                key={eg}
                onClick={() => setInput(eg)}
                className="row"
                style={{
                  display: 'block',
                  width: '100%',
                  textAlign: 'left',
                  padding: '9px 11px',
                  borderRadius: radius.lg,
                  cursor: 'pointer',
                  fontSize: 13,
                  color: color.ink2,
                }}
              >
                {eg}
              </button>
            ))}
            <div style={{ display: 'flex', gap: 9, marginTop: 15 }}>
              <div style={{ flexGrow: 1 }} />
              <Button icon="play" disabled={!input.trim() || !chosen || !path} onClick={() => void start()}>
                Run
              </Button>
            </div>
          </>
        )}

        {busy && (
          <div style={{ fontSize: 13, color: color.ink3, padding: '8px 0' }}>
            Working… each step runs against your project as it is decided.
          </div>
        )}

        {run && (
          <>
            {run.hops.length > 0 && (
              <>
                <SectionLabel style={{ marginBottom: 10 }}>
                  WHAT IT DID · {run.hops.length} {run.hops.length === 1 ? 'STEP' : 'STEPS'}
                </SectionLabel>
                {run.hops.map((h, i) => (
                  <HopRow key={i} hop={h} index={i} />
                ))}
              </>
            )}

            {pending && (
              <div
                style={{
                  marginTop: 14,
                  padding: '12px 14px',
                  borderRadius: radius.lg,
                  background: color.orangeTint,
                  border: `1px solid ${color.orangeMark}`,
                }}
              >
                <div style={{ fontSize: 13, lineHeight: 1.6, color: color.inkProse }}>{pending.why}</div>
                <pre
                  style={{
                    margin: '8px 0 0 0',
                    fontFamily: font.mono,
                    fontSize: 11.5,
                    color: color.ink2,
                    whiteSpace: 'pre-wrap',
                  }}
                >
                  {pending.tool}({JSON.stringify(pending.args)})
                </pre>
                <div style={{ display: 'flex', gap: 9, marginTop: 12 }}>
                  <Button disabled={busy} onClick={() => void answer(true)}>
                    Yes, run it
                  </Button>
                  <Button variant="ghost" disabled={busy} onClick={() => void answer(false)}>
                    No
                  </Button>
                  <div style={{ flexGrow: 1, fontSize: 11.5, color: color.ink3, alignSelf: 'center' }}>
                    Saying no tells the agent, and it carries on.
                  </div>
                </div>
              </div>
            )}

            {run.outcome.kind === 'finished' && (
              <Callout tone="good" icon="check" style={{ marginTop: 14 }}>
                {run.outcome.answer}
              </Callout>
            )}

            {run.outcome.kind === 'exhausted' && (
              <Callout tone="warn" icon="alert" style={{ marginTop: 14 }}>
                Stopped after {run.outcome.hops} steps without finishing. The steps above did run —
                check them before asking again.
              </Callout>
            )}

            <div style={{ display: 'flex', gap: 9, marginTop: 14, alignItems: 'center' }}>
              <span style={{ fontSize: 11.5, color: color.ink4 }}>
                Recorded in <span style={{ fontFamily: font.mono }}>traces/</span> in your project.
              </span>
              <div style={{ flexGrow: 1 }} />
              {!pending && <Button onClick={close}>Done</Button>}
            </div>
          </>
        )}

        {error && (
          <Callout tone="warn" icon="alert" style={{ marginTop: 12 }}>
            {error}
          </Callout>
        )}
      </div>
    </Overlay>
  )
}
