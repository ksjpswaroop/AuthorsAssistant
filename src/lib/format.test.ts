import { describe, expect, it } from 'vitest'
import { countWords, relativeTime, tildePath, words } from './format'

describe('countWords', () => {
  it('agrees with the Rust implementation on what a word is', () => {
    // Both sides split on whitespace and require an alphanumeric character,
    // so a stray dash is not a word. If these drift, the editor's live count
    // disagrees with the count that gets saved.
    expect(countWords('one two three')).toBe(3)
    expect(countWords('one — two')).toBe(2)
    expect(countWords('   ')).toBe(0)
    expect(countWords('')).toBe(0)
    expect(countWords('hello, world!')).toBe(2)
  })

  it('counts numbers and accented words', () => {
    expect(countWords('42 café naïve')).toBe(3)
  })
})

describe('relativeTime', () => {
  it('describes recent moments without a clock', () => {
    const now = Date.now()
    expect(relativeTime(now)).toBe('just now')
    expect(relativeTime(now - 30_000)).toMatch(/30s ago/)
    expect(relativeTime(now - 5 * 60_000)).toBe('5m ago')
    expect(relativeTime(now - 3 * 3_600_000)).toBe('3h ago')
    expect(relativeTime(now - 86_400_000)).toBe('yesterday')
  })

  it('says never rather than showing the epoch', () => {
    expect(relativeTime(null)).toBe('never')
    expect(relativeTime(undefined)).toBe('never')
    expect(relativeTime(0)).toBe('never')
  })
})

describe('tildePath', () => {
  it('shortens a home path and leaves others alone', () => {
    expect(tildePath('/Users/ada/Books/A.aegis')).toBe('~/Books/A.aegis')
    expect(tildePath('/home/ada/Books/A.aegis')).toBe('~/Books/A.aegis')
    expect(tildePath('/tmp/A.aegis')).toBe('/tmp/A.aegis')
  })
})

describe('words', () => {
  it('groups thousands so long manuscripts stay readable', () => {
    expect(words(1214)).toBe('1,214')
    expect(words(0)).toBe('0')
    expect(words(61480)).toBe('61,480')
  })
})
