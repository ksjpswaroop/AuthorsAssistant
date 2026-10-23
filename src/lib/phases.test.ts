import { describe, expect, it } from 'vitest'
import {
  defaultFilters,
  embeddingModels,
  generationModels,
  isEmbeddingModel,
  isGuarded,
  localModels,
  type OllamaStatus,
} from './phases'

describe('localModels', () => {
  const status = (models: { name: string; size: number }[]): OllamaStatus => ({
    reachable: true,
    host: 'http://127.0.0.1:11434',
    models,
    message: null,
  })

  it('hides cloud-proxied tags, which cannot run locally', () => {
    // A :cloud tag reports a tiny size and returns nothing when called. It
    // would sort first by size and look like the obvious default.
    const models = localModels(
      status([
        { name: 'qwen3.5:cloud', size: 1_000 },
        { name: 'llama3.2:3b', size: 2_000_000_000 },
      ]),
    )
    expect(models.map((m) => m.name)).toEqual(['llama3.2:3b'])
  })

  it('hides anything too small to be real weights', () => {
    const models = localModels(status([{ name: 'stub:latest', size: 4_096 }]))
    expect(models).toHaveLength(0)
  })

  it('returns nothing when Ollama has nothing', () => {
    expect(localModels(status([]))).toHaveLength(0)
  })
})

describe('compile filters', () => {
  it('default to excluding working material from a manuscript', () => {
    const f = defaultFilters()
    expect(f.includeResearch).toBe(false)
    expect(f.includeSheets).toBe(false)
    expect(f.skipEmpty).toBe(true)
  })
})

describe('guarded tools', () => {
  it('cover everything destructive or networked', () => {
    expect(isGuarded('delete_document')).toBe(true)
    expect(isGuarded('search_web')).toBe(true)
    expect(isGuarded('compile_manuscript')).toBe(true)
    expect(isGuarded('restore_snapshot')).toBe(true)
  })

  it('leave read-only tools unguarded', () => {
    expect(isGuarded('list_documents')).toBe(false)
    expect(isGuarded('search_binder')).toBe(false)
  })
})

describe('model pools', () => {
  const status = (models: { name: string; size: number }[]): OllamaStatus => ({
    reachable: true,
    host: 'http://127.0.0.1:11434',
    models,
    message: null,
  })

  const installed = status([
    { name: 'xagi/qwen3-embedding:0.6b-q4_k_m-1024', size: 400_000_000 },
    { name: 'llama3.2:3b', size: 2_020_000_000 },
    { name: 'qwen2.5-coder:7b', size: 4_680_000_000 },
    { name: 'qwen3.5:cloud', size: 1_000 },
  ])

  it('recognises embedding models by name', () => {
    expect(isEmbeddingModel('nomic-embed-text')).toBe(true)
    expect(isEmbeddingModel('xagi/qwen3-embedding:0.6b-q4_k_m-1024')).toBe(true)
    expect(isEmbeddingModel('bge-m3')).toBe(true)
    expect(isEmbeddingModel('llama3.2:3b')).toBe(false)
    expect(isEmbeddingModel('qwen2.5-coder:7b')).toBe(false)
  })

  it('never offers an embedding model for writing prose', () => {
    // An embedding model is small, so it sorts first by size and would become
    // the default. Asked to generate, it returns an empty string — which reads
    // as "the feature is broken".
    const gen = generationModels(installed).map((m) => m.name)
    expect(gen[0]).toBe('llama3.2:3b')
    expect(gen).not.toContain('xagi/qwen3-embedding:0.6b-q4_k_m-1024')
    expect(gen).not.toContain('qwen3.5:cloud')
  })

  it('never offers a chat model for embedding', () => {
    const emb = embeddingModels(installed).map((m) => m.name)
    expect(emb).toEqual(['xagi/qwen3-embedding:0.6b-q4_k_m-1024'])
  })

  it('keeps the two pools disjoint', () => {
    const gen = generationModels(installed).map((m) => m.name)
    const emb = embeddingModels(installed).map((m) => m.name)
    expect(gen.filter((m) => emb.includes(m))).toEqual([])
  })

  it('reports an empty pool rather than substituting the wrong kind', () => {
    const onlyChat = status([{ name: 'llama3.2:3b', size: 2_020_000_000 }])
    expect(embeddingModels(onlyChat)).toHaveLength(0)
    const onlyEmbed = status([{ name: 'nomic-embed-text', size: 274_000_000 }])
    expect(generationModels(onlyEmbed)).toHaveLength(0)
  })
})
