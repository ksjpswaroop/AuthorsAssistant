import { useState } from 'react'
import { PhaseScreen, SidePanel } from '../components/PhaseScreen'
import { Button, Callout, Chip, SectionLabel } from '../components/ui'
import { Icon } from '../components/Icon'
import { useApp } from '../lib/store'
import { useAsync } from '../lib/useAsync'
import { ai, embeddingModels, type RetrievedChunk } from '../lib/phases'
import { isTauri } from '../lib/api'
import { color, font, radius } from '../lib/tokens'
import { words } from '../lib/format'

/**
 * Book-wide search (Phase 3). This is the screen the retrieval index existed
 * for — until now the command was tested but unreachable, which meant the
 * feature effectively did not exist for an author.
 */
export function Search() {
  const path = useApp((s) => s.project?.path)
  const docs = useApp((s) => s.docs)
  const select = useApp((s) => s.select)
  const go = useApp((s) => s.go)

  const status = useAsync(() => ai.status(), [], isTauri)
  const stats = useAsync(() => ai.indexStats(path!), [path], Boolean(path && isTauri))
  const embedders = status.data ? embeddingModels(status.data) : []
  const model = embedders[0]?.name ?? null

  const [query, setQuery] = useState('')
  const [hits, setHits] = useState<RetrievedChunk[] | null>(null)
  const [busy, setBusy] = useState<'index' | 'search' | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [indexed, setIndexed] = useState<string | null>(null)

  const [chunkCount, docCount] = stats.data ?? [0, 0]
  const withText = docs.filter((d) => d.wordCount > 0).length
  const stale = docCount > 0 && docCount < withText

  const runIndex = async () => {
    if (!path || !model) return
    setBusy('index')
    setError(null)
    try {
      const [d, c] = await ai.index(path, model)
      setIndexed(`Indexed ${d} document${d === 1 ? '' : 's'} into ${c} chunk${c === 1 ? '' : 's'}.`)
      stats.reload()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(null)
    }
  }

  const runSearch = async () => {
    if (!path || !model || !query.trim()) return
    setBusy('search')
    setError(null)
    try {
      setHits(await ai.search(path, model, query.trim(), 8))
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(null)
    }
  }

  const titleOf = (id: string) => docs.find((d) => d.id === id)?.title ?? 'Unknown document'

  return (
    <PhaseScreen
      title="Search"
      phase="Phase 3"
      explains="Searches meaning, not words: your prose is embedded on this machine and compared against the question. Nothing is uploaded."
      statusRight={<span>{chunkCount > 0 ? `${words(chunkCount)} chunks indexed` : 'not indexed yet'}</span>}
    >
      <div style={{ flexGrow: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <div style={{ padding: '18px 24px 0 24px' }}>
          {!isTauri ? (
            <Callout tone="quiet">Search needs the desktop app.</Callout>
          ) : !status.data?.reachable ? (
            <Callout tone="warn" icon="alert">{status.data?.message ?? 'Checking for Ollama…'}</Callout>
          ) : !model ? (
            <Callout tone="warn" icon="alert">
              No embedding model installed. Run <span style={{ fontFamily: font.mono }}>ollama pull nomic-embed-text</span> —
              a chat model cannot do this job.
            </Callout>
          ) : (
            <>
              <div style={{ display: 'flex', gap: 10 }}>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 10,
                    flexGrow: 1,
                    height: 40,
                    padding: '0 14px',
                    border: `1px solid ${color.line}`,
                    borderRadius: radius.lg + 1,
                  }}
                >
                  <Icon name="search" size={16} strokeWidth={2} style={{ color: color.ink4 }} />
                  <input
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && void runSearch()}
                    placeholder="Ask about the book — “who was injured, and how was it treated?”"
                    style={{ flexGrow: 1, border: 'none', outline: 'none', fontSize: 14, background: 'transparent' }}
                  />
                </div>
                <Button height={40} disabled={busy !== null || !query.trim() || chunkCount === 0} onClick={() => void runSearch()}>
                  {busy === 'search' ? 'Searching…' : 'Search'}
                </Button>
              </div>

              {chunkCount === 0 && (
                <Callout tone="info" style={{ marginTop: 14 }} action={
                  <Button height={30} disabled={busy !== null} onClick={() => void runIndex()}>
                    {busy === 'index' ? 'Indexing…' : 'Index now'}
                  </Button>
                }>
                  Nothing is indexed yet. Indexing reads every document once and embeds it locally with{' '}
                  <span style={{ fontFamily: font.mono }}>{model}</span>.
                </Callout>
              )}

              {stale && (
                <Callout tone="warn" style={{ marginTop: 14 }} action={
                  <Button height={30} disabled={busy !== null} onClick={() => void runIndex()}>
                    Re-index
                  </Button>
                }>
                  {docCount} of {withText} documents are indexed. Anything written since will not be found.
                </Callout>
              )}

              {indexed && <Callout tone="good" icon="check" style={{ marginTop: 14 }}>{indexed}</Callout>}
              {error && <Callout tone="warn" icon="alert" style={{ marginTop: 14 }}>{error}</Callout>}
            </>
          )}
        </div>

        <div style={{ flexGrow: 1, overflowY: 'auto', padding: '18px 24px 40px 24px' }}>
          {hits?.length === 0 && (
            <div style={{ fontSize: 13, color: color.ink4 }}>Nothing in the book came close to that.</div>
          )}
          {hits?.map((h) => (
            <button
              key={h.id}
              onClick={() => {
                void select(h.documentId)
                go('studio')
              }}
              className="card"
              style={{
                display: 'block',
                width: '100%',
                textAlign: 'left',
                border: `1px solid ${color.line}`,
                borderRadius: radius.xl,
                padding: '14px 16px',
                marginBottom: 10,
                cursor: 'pointer',
                background: color.surface,
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 8 }}>
                <Icon name="doc" size={14} strokeWidth={1.8} style={{ color: color.ink4 }} />
                <span style={{ fontSize: 13.5, fontWeight: 600 }}>{titleOf(h.documentId)}</span>
                <div style={{ flexGrow: 1 }} />
                <Chip fg={color.blueDeep} bg={color.blueTint}>{Math.round(h.score * 100)}% match</Chip>
              </div>
              <div style={{ fontFamily: font.prose, fontSize: 14.5, lineHeight: 1.65, color: color.inkProse }}>
                {h.text.length > 320 ? `${h.text.slice(0, 320)}…` : h.text}
              </div>
            </button>
          ))}
        </div>
      </div>

      <SidePanel width={300}>
        <div style={{ padding: '18px 16px', overflowY: 'auto' }}>
          <SectionLabel style={{ marginBottom: 10 }}>THE INDEX</SectionLabel>
          <div style={{ fontSize: 12.5, lineHeight: 1.7, color: color.ink2 }}>
            {chunkCount > 0
              ? `${words(chunkCount)} chunks from ${docCount} document${docCount === 1 ? '' : 's'}.`
              : 'No index yet — build one to search the whole book.'}
          </div>
          {model && (
            <div style={{ fontFamily: font.mono, fontSize: 11.5, color: color.ink4, marginTop: 8 }}>{model}</div>
          )}
          <Button
            variant="ghost"
            height={31}
            disabled={busy !== null || !model}
            onClick={() => void runIndex()}
            style={{ width: '100%', justifyContent: 'center', marginTop: 12 }}
          >
            {busy === 'index' ? 'Indexing…' : chunkCount > 0 ? 'Re-index' : 'Build index'}
          </Button>

          <Callout tone="quiet" style={{ marginTop: 18 }}>
            The index lives in the project's own SQLite file. Deleting the project deletes it, and no separate database
            server is involved.
          </Callout>

          <SectionLabel style={{ margin: '20px 0 9px 0' }}>WHY NOT KEYWORDS</SectionLabel>
          <div style={{ fontSize: 12.5, lineHeight: 1.65, color: color.ink3 }}>
            Asking “who was injured” finds a scene that says “the descent took the leg”, which no keyword search would
            return. That is the whole point of embedding the book.
          </div>
        </div>
      </SidePanel>
    </PhaseScreen>
  )
}
