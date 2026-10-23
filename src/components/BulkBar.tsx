import { useState } from 'react'
import { useApp } from '../lib/store'
import { Button, Chip } from './ui'
import { Icon } from './Icon'
import { color, radius } from '../lib/tokens'

const STATUSES = ['Empty', 'Draft', 'Done']
const SECTION_TYPES = ['Part Heading', 'Chapter Heading', 'Scene', 'Front Matter', 'Back Matter']

/**
 * Bulk metadata bar (Phase 6). Appears only when documents are marked, and says
 * exactly how many will change — a batch edit that does not tell you its blast
 * radius is how people lose an afternoon.
 */
export function BulkBar() {
  const marked = useApp((s) => s.marked)
  const clearMarks = useApp((s) => s.clearMarks)
  const bulkSet = useApp((s) => s.bulkSet)
  const [status, setStatus] = useState('')
  const [sectionType, setSectionType] = useState('')
  const [busy, setBusy] = useState(false)

  if (marked.length === 0) return null

  const pending = status !== '' || sectionType !== ''

  const apply = async () => {
    setBusy(true)
    try {
      await bulkSet(marked, {
        status: status || undefined,
        sectionType: sectionType || undefined,
      })
      setStatus('')
      setSectionType('')
      clearMarks()
    } finally {
      setBusy(false)
    }
  }

  const dropdown = (
    value: string,
    onChange: (v: string) => void,
    placeholder: string,
    options: string[],
  ) => (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      style={{
        width: 150,
        height: 30,
        padding: '0 8px',
        fontSize: 12.5,
        color: '#FFFFFF',
        background: 'transparent',
        border: '1px solid rgba(255,255,255,0.2)',
        borderRadius: radius.md,
      }}
    >
      <option value="" style={{ color: color.ink }}>
        {placeholder}
      </option>
      {options.map((o) => (
        <option key={o} value={o} style={{ color: color.ink }}>
          {o}
        </option>
      ))}
    </select>
  )

  return (
    <div
      style={{
        position: 'absolute',
        left: '50%',
        bottom: 22,
        transform: 'translateX(-50%)',
        zIndex: 20,
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '10px 12px',
        background: color.ink,
        color: '#FFFFFF',
        borderRadius: radius.xl + 1,
        boxShadow: '0 12px 32px rgba(15,25,32,0.3)',
      }}
    >
      <Chip fg="#FFFFFF" bg="rgba(255,255,255,0.16)">
        {marked.length} selected
      </Chip>
      {dropdown(status, setStatus, 'Set status…', STATUSES)}
      {dropdown(sectionType, setSectionType, 'Set section type…', SECTION_TYPES)}
      <Button
        height={30}
        icon="checklist"
        disabled={!pending || busy}
        title={pending ? `Change ${marked.length} documents` : 'Pick something to set first'}
        onClick={() => void apply()}
      >
        {busy ? 'Applying…' : `Apply to ${marked.length}`}
      </Button>
      <button
        onClick={clearMarks}
        aria-label="Clear selection"
        style={{ display: 'flex', color: '#A9BAC6', cursor: 'pointer', padding: 4 }}
      >
        <Icon name="close" size={15} strokeWidth={2.2} />
      </button>
    </div>
  )
}
