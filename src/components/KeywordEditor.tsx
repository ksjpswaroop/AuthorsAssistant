import { useEffect, useState } from 'react'
import { useApp } from '../lib/store'
import { api } from '../lib/api'
import { SectionLabel } from './ui'
import { Icon } from './Icon'
import { color, radius } from '../lib/tokens'

/**
 * Keywords for the open document, backed by a project-wide vocabulary table.
 * Suggesting what the project already uses is what stops three spellings of the
 * same idea accumulating.
 */
export function KeywordEditor() {
  const doc = useApp((s) => s.docs.find((d) => d.id === s.selectedId))
  const projectPath = useApp((s) => s.project?.path)
  const setKeywords = useApp((s) => s.setKeywords)
  const [draft, setDraft] = useState('')
  const [vocabulary, setVocabulary] = useState<[string, number][]>([])

  useEffect(() => {
    if (!projectPath) return
    void api.projectKeywords(projectPath).then(setVocabulary).catch(() => setVocabulary([]))
  }, [projectPath, doc?.keywords])

  if (!doc) return null
  const current = doc.keywords ?? []

  const add = (name: string) => {
    const clean = name.trim()
    if (!clean || current.some((k) => k.toLowerCase() === clean.toLowerCase())) {
      setDraft('')
      return
    }
    void setKeywords(doc.id, [...current, clean])
    setDraft('')
  }

  const suggestions = vocabulary
    .filter(([name]) => !current.some((k) => k.toLowerCase() === name.toLowerCase()))
    .filter(([name]) => (draft ? name.toLowerCase().includes(draft.toLowerCase()) : true))
    .slice(0, 6)

  return (
    <div>
      <SectionLabel style={{ marginBottom: 8 }}>KEYWORDS</SectionLabel>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
        {current.map((k) => (
          <span
            key={k}
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 5,
              fontSize: 11.5,
              fontWeight: 500,
              color: color.ink2,
              background: color.lineSoft,
              borderRadius: radius.sm + 1,
              padding: '4px 6px 4px 9px',
            }}
          >
            {k}
            <button
              onClick={() => void setKeywords(doc.id, current.filter((x) => x !== k))}
              aria-label={`Remove ${k}`}
              style={{ display: 'flex', color: color.ink4, cursor: 'pointer' }}
            >
              <Icon name="close" size={11} strokeWidth={2.6} />
            </button>
          </span>
        ))}
        <input
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ',') {
              e.preventDefault()
              add(draft)
            }
            if (e.key === 'Backspace' && !draft && current.length) {
              void setKeywords(doc.id, current.slice(0, -1))
            }
          }}
          onBlur={() => draft && add(draft)}
          placeholder={current.length ? 'Add…' : 'Add a keyword'}
          style={{
            flexGrow: 1,
            minWidth: 90,
            border: `1px dashed ${color.disabled}`,
            borderRadius: radius.sm + 1,
            padding: '4px 9px',
            fontSize: 11.5,
            outline: 'none',
            background: 'transparent',
          }}
        />
      </div>

      {suggestions.length > 0 && (
        <div style={{ marginTop: 9 }}>
          <div style={{ fontSize: 11, color: color.ink5, marginBottom: 6 }}>Already used in this project</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 5 }}>
            {suggestions.map(([name, n]) => (
              <button
                key={name}
                onClick={() => add(name)}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 5,
                  fontSize: 11.5,
                  color: color.ink3,
                  background: 'transparent',
                  border: `1px solid ${color.line}`,
                  borderRadius: radius.sm + 1,
                  padding: '3px 8px',
                  cursor: 'pointer',
                }}
              >
                {name}
                <span style={{ color: color.ink5 }}>{n}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}
