import { describe, expect, it } from 'vitest'
import { diffParagraphs } from './ProjectDialogs'

describe('diffParagraphs', () => {
  it('marks nothing when the text is unchanged', () => {
    const text = 'One paragraph.\n\nAnd a second.'
    expect(diffParagraphs(text, text).every((l) => l.kind === 'same')).toBe(true)
  })

  it('keeps the paragraphs that survived rather than rewriting the whole thing', () => {
    const before = 'The rim held the heat.\n\nShe counted her paces.\n\nThe salt stayed white.'
    const after = 'The rim held the heat.\n\nShe counted forty paces and stopped.\n\nThe salt stayed white.'
    const lines = diffParagraphs(before, after)

    expect(lines.filter((l) => l.kind === 'same').map((l) => l.text)).toEqual([
      'The rim held the heat.',
      'The salt stayed white.',
    ])
    expect(lines.filter((l) => l.kind === 'del').map((l) => l.text)).toEqual([
      'She counted her paces.',
    ])
    expect(lines.filter((l) => l.kind === 'add').map((l) => l.text)).toEqual([
      'She counted forty paces and stopped.',
    ])
  })

  it('shows an insertion as an addition, not as a rewrite of everything after it', () => {
    const before = 'A.\n\nB.'
    const after = 'A.\n\nNew middle.\n\nB.'
    const lines = diffParagraphs(before, after)
    expect(lines.map((l) => l.kind)).toEqual(['same', 'add', 'same'])
  })

  it('handles an empty side without inventing content', () => {
    expect(diffParagraphs('', 'Only new.').map((l) => l.kind)).toEqual(['add'])
    expect(diffParagraphs('Only old.', '').map((l) => l.kind)).toEqual(['del'])
    expect(diffParagraphs('', '')).toEqual([])
  })

  it('ignores blank-line noise so reformatting is not a change', () => {
    const lines = diffParagraphs('A.\n\nB.', 'A.\n\n\n\nB.\n')
    expect(lines.every((l) => l.kind === 'same')).toBe(true)
  })
})
