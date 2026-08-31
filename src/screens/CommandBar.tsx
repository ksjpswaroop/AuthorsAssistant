import { useState } from 'react'
import { Overlay } from '../components/Overlay'
import { Icon } from '../components/Icon'
import { useApp } from '../lib/store'
import { Button, Chip, Dot, Kbd, SectionLabel } from '../components/ui'
import { color, font, radius, tint } from '../lib/tokens'

type Step = 'idle' | 'plan' | 'run' | 'done'

const PLAN = [
  { n: 1, text: 'Create the folder Act III under Manuscript', tool: 'create_folder' },
  { n: 2, text: 'Add 3 empty chapters inside it', tool: 'create_chapter ×3' },
  { n: 3, text: 'Create a character sheet named Antagonist', tool: 'create_character_sheet' },
  { n: 4, text: 'Refresh the binder and select Act III', tool: 'binder_changed' },
]

/**
 * The App Controller flow from the design canvas. Phase 5 wires this to Hermes
 * through the sidecar; today it steps through the designed states so the
 * interaction can be reviewed without pretending an agent ran.
 */
export function CommandBar() {
  const close = () => useApp.getState().setCommandBar(false)
  const [step, setStep] = useState<Step>('idle')

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
        <span style={{ color: color.accent, display: 'flex' }}>
          <Icon name="sparkle" size={19} strokeWidth={1.9} />
        </span>
        <div style={{ flexGrow: 1, fontSize: 16.5, color: color.ink }}>
          Create a folder for Act III, add three empty chapters, and start a character sheet for the antagonist
        </div>
        <Kbd>esc</Kbd>
      </div>

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 9,
          padding: '9px 19px',
          background: color.rail,
          borderBottom: `1px solid ${color.lineSoft}`,
        }}
      >
        <Chip fg={color.ink2} bg={color.surface}>
          <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
            <Dot tone={color.disabled} size={6} />
            App Controller
          </span>
        </Chip>
        <span style={{ fontFamily: font.mono, fontSize: 11, color: color.ink3 }}>no model connected</span>
        <div style={{ flexGrow: 1 }} />
        <Chip fg={color.purpleDeep} bg={color.purpleTint}>
          Phase 5
        </Chip>
      </div>

      <div style={{ overflowY: 'auto', padding: '16px 19px 18px 19px' }}>
        {step === 'idle' && (
          <>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 12,
                padding: 11,
                borderRadius: radius.lg + 1,
                background: tint(color.accent, 0.09),
              }}
            >
              <span style={{ color: color.accent, display: 'flex' }}>
                <Icon name="folder" size={17} />
              </span>
              <div style={{ flexGrow: 1 }}>
                <div style={{ fontSize: 13.5, fontWeight: 600 }}>Multi-step binder change · 4 tool calls</div>
                <div style={{ fontSize: 12, color: color.ink3, marginTop: 2 }}>
                  create_folder · create_chapter ×3 · create_character_sheet
                </div>
              </div>
            </div>
            <div style={{ marginTop: 14, fontSize: 12.5, lineHeight: 1.6, color: color.ink3 }}>
              Agents plan and execute in Phase 5. Until then the binder buttons do the same work by hand — the tools
              they will call are the ones already wired in the Rust core.
            </div>
            <div style={{ marginTop: 14, display: 'flex', justifyContent: 'flex-end' }}>
              <Button onClick={() => setStep('plan')}>Preview the plan</Button>
            </div>
          </>
        )}

        {step === 'plan' && (
          <>
            <SectionLabel style={{ marginBottom: 13 }}>PROPOSED PLAN · 4 STEPS</SectionLabel>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              {PLAN.map((p) => (
                <div
                  key={p.n}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 12,
                    padding: '10px 12px',
                    borderRadius: radius.lg,
                    background: color.railSoft,
                  }}
                >
                  <span
                    style={{
                      width: 20,
                      height: 20,
                      borderRadius: '50%',
                      background: color.lineFaint,
                      color: color.ink2,
                      fontSize: 11,
                      fontWeight: 700,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: 0,
                    }}
                  >
                    {p.n}
                  </span>
                  <span style={{ fontSize: 13.5, color: color.inkProse, flexGrow: 1 }}>{p.text}</span>
                  <span style={{ fontFamily: font.mono, fontSize: 11, color: color.ink3 }}>{p.tool}</span>
                </div>
              ))}
            </div>
            <div style={{ marginTop: 15, display: 'flex', gap: 9 }}>
              <Button variant="ghost" onClick={() => setStep('idle')}>
                Back
              </Button>
              <div style={{ flexGrow: 1 }} />
              <Button icon="play" onClick={() => setStep('done')}>
                Run 4 steps
              </Button>
            </div>
          </>
        )}

        {step === 'done' && (
          <>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 11,
                padding: '13px 14px',
                background: color.amberTint,
                border: `1px solid ${color.amberLine}`,
                borderRadius: radius.xl,
              }}
            >
              <span style={{ color: color.yellowInk, display: 'flex' }}>
                <Icon name="alert" size={18} strokeWidth={2.1} />
              </span>
              <div style={{ fontSize: 13, lineHeight: 1.6, color: color.amberInk }}>
                Nothing ran. There is no agent runtime in Phase 1, and this screen will not pretend otherwise — it exists
                so the flow can be reviewed before Phase 5 builds it.
              </div>
            </div>
            <div style={{ marginTop: 15, display: 'flex', gap: 9 }}>
              <Button variant="ghost" onClick={() => setStep('idle')}>
                Start over
              </Button>
              <div style={{ flexGrow: 1 }} />
              <Button onClick={close}>Close</Button>
            </div>
          </>
        )}
      </div>
    </Overlay>
  )
}
