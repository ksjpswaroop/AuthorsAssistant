export function words(n: number): string {
  return n.toLocaleString('en-US')
}

export function relativeTime(ms: number | null | undefined): string {
  if (!ms) return 'never'
  const delta = Date.now() - ms
  if (delta < 8_000) return 'just now'
  if (delta < 60_000) return `${Math.round(delta / 1000)}s ago`
  if (delta < 3_600_000) return `${Math.round(delta / 60_000)}m ago`
  if (delta < 86_400_000) return `${Math.round(delta / 3_600_000)}h ago`
  const days = Math.round(delta / 86_400_000)
  if (days === 1) return 'yesterday'
  if (days < 7) return `${days} days ago`
  return new Date(ms).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })
}

/** Shortens a long absolute path for display: ~/Books/TheSaltRoad.aegis */
export function tildePath(path: string): string {
  const home = path.match(/^\/(Users|home)\/[^/]+/)
  return home ? `~${path.slice(home[0].length)}` : path
}

/** Word count used for unsaved buffers; matches the Rust implementation. */
export function countWords(text: string): number {
  return text.split(/\s+/).filter((w) => /[\p{L}\p{N}]/u.test(w)).length
}

const KIND_LABEL: Record<string, string> = {
  folder: 'Folder',
  scene: 'Scene',
  sheet: 'Character sheet',
  lore: 'Lore',
  research: 'Research',
  canon: 'Canon',
}

export function kindLabel(kind: string): string {
  return KIND_LABEL[kind] ?? 'Document'
}
