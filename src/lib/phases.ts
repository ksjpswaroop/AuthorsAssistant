/**
 * The Tauri command surface for the studio: collections, scrivenings, templates,
 * goals, and the paragraph ledger. Later phases add their commands in this file.
 *
 * Kept apart from api.ts (which is the project lifecycle) so the two can be read
 * independently. Everything here degrades to a clear "not available outside the
 * desktop app" rather than pretending in the browser preview.
 */

import { isTauri } from './api'
import type { DocKind, DocumentRow, ProjectSummary, StyleDistance, StyleProfile } from './types'

async function call<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  if (!isTauri) throw new Error('This needs the desktop app.')
  const { invoke } = await import('@tauri-apps/api/core')
  return invoke<T>(command, args)
}

/* --- Phase 6: collections and scrivenings --------------------------------- */

export type CollectionKind = 'manual' | 'smart' | 'search'

export interface Collection {
  id: string
  name: string
  kind: CollectionKind
  filters: string | null
  position: number
  count: number
}

export interface SmartFilter {
  status?: string
  label?: string
  pov?: string
  sectionType?: string
  keyword?: string
  minWords?: number
  maxWords?: number
  text?: string
}

export const collections = {
  list: (path: string) => call<Collection[]>('collections_list', { path }),
  create: (path: string, name: string, kind: CollectionKind, filters?: SmartFilter) =>
    call<string>('collection_create', { path, name, kind, filters: filters ? JSON.stringify(filters) : null }),
  remove: (path: string, id: string) => call<void>('collection_delete', { path, id }),
  members: (path: string, id: string) => call<unknown[]>('collection_members', { path, id }),
  add: (path: string, id: string, documentId: string) => call<void>('collection_add', { path, id, documentId }),
  removeItem: (path: string, id: string, documentId: string) =>
    call<void>('collection_remove', { path, id, documentId }),
}

export const bulkSetMetadata = (
  path: string,
  ids: string[],
  fields: { status?: string; label?: string; sectionType?: string; pov?: string },
) =>
  call<number>('bulk_set_metadata', {
    path,
    ids,
    status: fields.status ?? null,
    label: fields.label ?? null,
    sectionType: fields.sectionType ?? null,
    pov: fields.pov ?? null,
  })

export const scrivenings = {
  open: (path: string, parentId: string) => call<[string, string[]]>('scrivenings_open', { path, parentId }),
  save: (path: string, combined: string, ids: string[]) =>
    call<number>('scrivenings_save', { path, combined, ids }),
}


/* --- Phase 7: compile ------------------------------------------------------ */

export type ExportFormat = 'markdown' | 'html' | 'pdf' | 'epub' | 'docx' | 'fountain'

export interface CompileFilters {
  root?: string | null
  collection?: string | null
  statuses?: string[] | null
  excludeStatuses?: string[] | null
  excludeLabels?: string[] | null
  excludeSectionTypes?: string[] | null
  includeResearch: boolean
  includeSheets: boolean
  skipEmpty: boolean
}

export const defaultFilters = (): CompileFilters => ({
  root: null,
  collection: null,
  statuses: null,
  excludeStatuses: null,
  excludeLabels: null,
  excludeSectionTypes: null,
  includeResearch: false,
  includeSheets: false,
  skipEmpty: true,
})

export const compile = {
  preview: (path: string, preset: string, filters: CompileFilters) =>
    call<string>('compile_preview', { path, preset, filters }),
  export: (
    path: string,
    preset: string,
    format: ExportFormat,
    filters: CompileFilters,
    destination: string,
    title?: string,
    author?: string,
  ) =>
    call<string>('compile_export', {
      path,
      preset,
      format,
      filters,
      destination,
      title: title ?? null,
      author: author ?? null,
    }),
}

/* --- Phase 3: local models and retrieval ---------------------------------- */

export interface ModelInfo {
  name: string
  size: number
}

export interface OllamaStatus {
  reachable: boolean
  host: string
  models: ModelInfo[]
  message: string | null
}

export interface RetrievedChunk {
  id: string
  documentId: string
  ordinal: number
  text: string
  score: number
}

export const ai = {
  status: () => call<OllamaStatus>('ollama_status'),
  index: (path: string, model: string) => call<[number, number]>('index_project', { path, model }),
  indexStats: (path: string) => call<[number, number]>('index_stats', { path }),
  search: (path: string, model: string, query: string, k = 5, exclude?: string) =>
    call<RetrievedChunk[]>('search_index', { path, model, query, k, exclude: exclude ?? null }),
  assist: (
    path: string,
    model: string,
    id: string,
    mode: 'continue' | 'rewrite' | 'critique',
    opts: { instruction?: string; allowCloud?: boolean } = {},
  ) =>
    call<string>('assist', {
      path,
      model,
      id,
      mode,
      instruction: opts.instruction ?? null,
      allowCloud: opts.allowCloud ?? false,
    }),
  pools: () => call<[ModelInfo[], ModelInfo[], ModelInfo[]]>('model_pools'),

  /** The stored style profile, or null when none has been built. */
  styleLoad: (path: string) => call<StyleProfile | null>('style_load', { path }),
  /** Rebuilds and saves it. Without a model only the measured half is built. */
  styleBuild: (path: string, model: string | null, allowCloud = false) =>
    call<StyleProfile>('style_build', { path, model, allowCloud }),
  /** How far one document sits from the profile. Null when there is no profile. */
  styleDistance: (path: string, id: string) =>
    call<StyleDistance | null>('style_distance', { path, id }),
}

/**
 * A model that can actually run here. `:cloud` tags are remote models Ollama
 * proxies — they report a tiny size and produce nothing locally, so offering
 * one would look like a working choice that silently fails.
 */
export function localModels(status: OllamaStatus): ModelInfo[] {
  return status.models.filter((m) => !m.name.includes(':cloud') && m.size >= 100_000_000)
}

/**
 * Embedding models and generation models are not interchangeable, and picking
 * the wrong one fails in a way that looks like the feature is broken: an
 * embedding model returns an empty string when asked for prose, and a chat
 * model refuses to embed with a message that blames the server. Ollama exposes
 * no flag for this, so the distinction is made by name.
 */
const EMBEDDING_HINTS = ['embed', 'bge', 'gte', 'e5-', 'minilm']

export function isEmbeddingModel(name: string): boolean {
  const n = name.toLowerCase()
  return EMBEDDING_HINTS.some((h) => n.includes(h))
}

/** Models that can write prose. Sorted cheapest first. */
export function generationModels(status: OllamaStatus): ModelInfo[] {
  return localModels(status)
    .filter((m) => !isEmbeddingModel(m.name))
    .sort((a, b) => a.size - b.size)
}

/** Models that can produce embeddings, for indexing and search. */
/**
 * A `:cloud` tag is proxied to a remote service. Using one sends the prompt —
 * and therefore the manuscript — off the machine, which is the single thing
 * this product promises not to do. Never a default; always labelled.
 */
export function isCloudModel(name: string): boolean {
  return name.includes(':cloud') || name.endsWith('-cloud')
}

export function cloudModels(status: OllamaStatus): ModelInfo[] {
  return status.models.filter((m) => isCloudModel(m.name))
}

export function embeddingModels(status: OllamaStatus): ModelInfo[] {
  return status.models
    .filter((m) => !m.name.includes(':cloud') && isEmbeddingModel(m.name))
    .sort((a, b) => a.size - b.size)
}

/* --- Phase 4: integrity ---------------------------------------------------- */

export interface SimilarityMatch {
  sourceId: string
  sourceTitle: string
  excerpt: string
  score: number
}

export interface WritingSignals {
  wordCount: number
  sentenceCount: number
  meanSentenceLength: number
  sentenceLengthVariance: number
  typeTokenRatio: number
  abstractDensity: number
}

export interface RubricScore {
  criterion: string
  score: number
  note: string
}

/** Is the author's wigolo daemon reachable? Absent is a state, not an error. */
export interface WigoloStatus {
  running: boolean
  base: string
  health: string | null
  detail: string | null
  authenticated: boolean
}

export interface WebMatch {
  title: string
  url: string
  excerpt: string
  /** wigolo's citation id, when its version sends one. */
  citationId: string | null
  sourceSpan: [number, number] | null
  /** wigolo's confidence in the *result*. Not a similarity score. */
  evidenceScore: number | null
  /** Measured here: how much of the sent phrase appears in the excerpt. */
  overlap: number
}

export interface WebSearchResult {
  query: string
  matches: WebMatch[]
  engines: string[]
  published: string | null
  warning: string | null
}

/** One phrase that left the machine, and what came back. */
export interface TraceEntry {
  at: number
  query: string
  wordCount: number
  base: string
  resultCount: number
  urls: string[]
}

export interface IntegrityPacket {
  generatedAt: number
  project: string
  documentId: string
  documentTitle: string
  wordCount: number
  internalMatches: SimilarityMatch[]
  threshold: number
  signals: WritingSignals
  rubric: RubricScore[]
  webChecks: TraceEntry[]
  /** What the numbers are, and what they are not. */
  method: string[]
}

/** How wigolo could be launched on this machine, or why it cannot be. */
export type WigoloLauncher =
  | { via: 'explicit'; path: string }
  | { via: 'onPath'; path: string }
  | { via: 'npx'; npx: string; node: string }

export interface WigoloResolution {
  launcher: WigoloLauncher | null
  /** What was found, or what is missing and how to fix it. Never bare. */
  detail: string
}

export interface SidecarStatus {
  running: boolean
  /** True only when AegisDraft started it — and it stops only what it started. */
  managed: boolean
  base: string
  health: string | null
  resolution: WigoloResolution
  detail: string | null
}

export const sidecar = {
  status: () => call<SidecarStatus>('wigolo_sidecar_status'),
  /** Starts wigolo, or adopts one already answering. Never called on launch. */
  start: () => call<SidecarStatus>('wigolo_sidecar_start'),
  stop: () => call<SidecarStatus>('wigolo_sidecar_stop'),
}

export const integrity = {
  similarity: (path: string, id: string, threshold = 0.5) =>
    call<SimilarityMatch[]>('integrity_similarity', { path, id, threshold }),
  signals: (path: string, id: string) => call<WritingSignals>('integrity_signals', { path, id }),
  rubric: (path: string, id: string) => call<RubricScore[]>('integrity_rubric', { path, id }),

  wigolo: () => call<WigoloStatus>('wigolo_status'),
  /**
   * Sends one phrase off the machine, through the author's own wigolo.
   * `confirmed` must be a real answer from a person — the Rust side refuses
   * without it, so passing `true` from code that did not ask would be a lie
   * told to the only guard there is.
   */
  webSimilarity: (path: string, phrase: string, confirmed: boolean, maxResults = 5) =>
    call<WebSearchResult>('web_similarity', { path, phrase, maxResults, confirmed }),
  trace: (path: string) => call<TraceEntry[]>('research_trace', { path }),
  packet: (path: string, id: string, threshold = 0.5) =>
    call<IntegrityPacket>('integrity_packet', { path, id, threshold }),
  /** Writes the packet as a JSON + HTML pair. Returns both paths. */
  exportPacket: (path: string, id: string, threshold: number, outPath: string) =>
    call<[string, string]>('integrity_export', { path, id, threshold, outPath }),
}

/* --- Phases 8 and 9: analysis ---------------------------------------------- */

export interface Beat {
  documentId: string
  title: string
  position: number
  label: string
  confidence: number
}

export interface TimelineEntry {
  documentId: string
  title: string
  narrativeOrder: number
  storyDay: number | null
  outOfOrder: boolean
}

export interface GraphNode {
  id: string
  label: string
  weight: number
  kind: string
}

export interface GraphEdge {
  from: string
  to: string
  weight: number
  relation: string
  /** Scenes since these two last shared one. Null means they still do. A
   *  number, never a warning: only the author knows if a gap is deliberate. */
  scenesSince: number | null
}

export interface StoryGraph {
  nodes: GraphNode[]
  edges: GraphEdge[]
  /** The labels this graph can be filtered by, taken from the scenes. */
  labels: string[]
}

export interface PacingPoint {
  index: number
  documentId: string
  title: string
  wordCount: number
  tempo: number
  variance: number
}

export interface ClusterProposal {
  name: string
  documentIds: string[]
  titles: string[]
  score: number
}

export const story = {
  beats: (path: string, method = 'three_act') => call<Beat[]>('story_beats', { path, method }),
  timeline: (path: string, chronological: boolean) =>
    call<TimelineEntry[]>('story_timeline', { path, chronological }),
  /** `label` narrows the graph to scenes carrying it. Null is the whole book. */
  graph: (path: string, label: string | null = null) =>
    call<StoryGraph>('story_graph', { path, label }),
  pacing: (path: string) => call<PacingPoint[]>('story_pacing', { path }),
  clusters: (path: string, threshold = 0.6) => call<ClusterProposal[]>('story_clusters', { path, threshold }),
}

/** A note the author put on the board. Never a document. */
export interface CanvasNote {
  id: string
  x: number
  y: number
  text: string
  tone: string | null
}

export interface CanvasArrow {
  id: string
  from: string
  to: string
  label: string | null
}

export interface CanvasCard {
  id: string
  x: number
  y: number
  group: string | null
}

export interface Canvas {
  cards: CanvasCard[]
  /** Notes and arrows are not part of the reading order and never commit. */
  notes: CanvasNote[]
  arrows: CanvasArrow[]
  updatedAt: number
}

export interface MoveStep {
  id: string
  title: string
  from: number
  to: number
}

/** What committing the canvas would do to the binder. */
export interface CommitPlan {
  parentId: string | null
  parentTitle: string
  /** Only the documents that would actually move. */
  moves: MoveStep[]
  /** Placed cards that live in another folder — named, never dragged in. */
  orphans: string[]
  /** Documents never placed. They keep their order and follow. */
  unplaced: string[]
}

export type DiagramKind = 'mind_map' | 'flowchart' | 'chrono_flowchart' | 'infographic'

export interface DiagramNode {
  id: string
  label: string
  group: string | null
  /** The binder document this node is about, resolved against the real binder. */
  documentId: string | null
}

export interface DiagramEdge {
  from: string
  to: string
  label: string | null
}

export interface Diagram {
  kind: DiagramKind
  title: string
  nodes: DiagramNode[]
  edges: DiagramEdge[]
  model: string
  generatedAt: number
  /** What validation threw away, each with the reason. Never a silent loss. */
  dropped: string[]
}

export const diagrams = {
  /** Asks a model for a graph. Fails loudly when the reply is not one. */
  generate: (path: string, model: string, kind: DiagramKind, query: string, allowCloud = false) =>
    call<Diagram>('diagram_generate', { path, model, kind, query, allowCloud }),
  /** Saves it into the binder as Markdown with a Mermaid block. */
  save: (path: string, diagram: Diagram) => call<DocumentRow>('diagram_save', { path, diagram }),
}

export interface Entity {
  name: string
  kind: 'character' | 'location' | 'date' | 'unknown'
  documentId: string | null
  mentions: number
}

export interface CanonCandidate {
  fact: string
  source: string
  passageExcerpt: string
  overlap: number
}

export interface CanonCheck {
  candidates: CanonCandidate[]
  canonEntries: number
  /** What was examined and what was not. Retrieval, never a verdict. */
  method: string
}

export interface Thread {
  id: string
  plantedIn: [string, string][]
  paidOffIn: [string, string][]
}

export interface LedgerReport {
  /** Planted, never paid off — a promise the book has not kept. */
  unpaid: Thread[]
  /** Paid off with nothing planted. */
  unplanted: Thread[]
  complete: Thread[]
}

export interface Debt {
  id: string
  gap: string
  foundIn: string
  foundBy: string
  at: number
  resolvedAt: number | null
}

export interface ContinuityReport {
  documentId: string
  entities: Entity[]
  canon: CanonCheck
  ledger: LedgerReport
  openDebts: Debt[]
}

export const continuity = {
  /** Everything the check gathers about one document. No model involved. */
  report: (path: string, id: string) => call<ContinuityReport>('continuity_report', { path, id }),
  ledger: (path: string) => call<LedgerReport>('foreshadowing_ledger', { path }),
  debts: (path: string) => call<Debt[]>('debts_list', { path }),
  /** Record a hole in the world rather than inventing something to fill it. */
  logDebt: (path: string, gap: string, foundIn: string, foundBy?: string) =>
    call<Debt>('debt_log', { path, gap, foundIn, foundBy: foundBy ?? null }),
  resolveDebt: (path: string, id: string) => call<void>('debt_resolve', { path, id }),
}

export const canvas = {
  load: (path: string) => call<Canvas>('canvas_load', { path }),
  place: (path: string, id: string, x: number, y: number) =>
    call<Canvas>('canvas_place', { path, id, x, y }),
  clear: (path: string) => call<void>('canvas_clear', { path }),
  /** Dry run. Changes nothing — the author sees the reorder before it happens. */
  plan: (path: string, parentId: string | null) =>
    call<CommitPlan>('canvas_plan', { path, parentId }),
  commit: (path: string, parentId: string | null) =>
    call<CommitPlan>('canvas_commit', { path, parentId }),

  note: (path: string, id: string | null, x: number, y: number, text: string, tone: string | null = null) =>
    call<CanvasNote>('canvas_note', { path, id, x, y, text, tone }),
  removeNote: (path: string, id: string) => call<void>('canvas_note_remove', { path, id }),
  arrow: (path: string, from: string, to: string, label: string | null = null) =>
    call<CanvasArrow>('canvas_arrow', { path, from, to, label }),
  removeArrow: (path: string, id: string) => call<void>('canvas_arrow_remove', { path, id }),
}

/** Writing targets, stored with the project rather than in app preferences. */
export interface Goals {
  projectTarget: number | null
  sessionTarget: number | null
  /** The project's word count when the current session began. */
  sessionStartWords: number | null
  sessionStartedAt: number | null
  defaultDocumentTarget: number | null
}

/** One card's placement on the infinite canvas. */
export interface SheetTemplate {
  id: string
  name: string
  kind: DocKind
  body: string
  /** False for the two shipped shapes. */
  custom: boolean
}

export const sheetTemplates = {
  list: (path: string) => call<SheetTemplate[]>('sheet_templates', { path }),
  save: (path: string, id: string, body: string) =>
    call<SheetTemplate>('sheet_template_save', { path, id, body }),
  /** Deleting a custom template restores the shipped one it overrode. */
  forget: (path: string, id: string) => call<void>('sheet_template_forget', { path, id }),
  create: (
    path: string,
    name: string,
    kind: DocKind,
    parentId: string | null,
    linkTo: string | null,
    templateId: string | null,
  ) =>
    call<DocumentRow>('create_sheet_from_template', {
      path,
      name,
      kind,
      parentId,
      linkTo,
      templateId,
    }),
}

/* --- Knowledge to Publication ---------------------------------------------- */

export type BlockKind =
  | 'heading' | 'paragraph' | 'code' | 'quote' | 'list' | 'table' | 'frontmatter' | 'rule'

export interface Block {
  id: string
  documentId: string
  ordinal: number
  kind: BlockKind
  contentHash: string
  text: string
  revision: number
  firstSeen: number
  updatedAt: number
  removedAt: number | null
}

export type ActorType = 'human' | 'ai_model' | 'tool' | 'external'
/** `observed` = AegisDraft watched it happen. `inferred` = reconstructed after
 *  the fact by matching text. Conflating them would make the ledger lie. */
export type Confidence = 'observed' | 'inferred'

export interface ProvEvent {
  id: string
  seq: number
  objectType: string
  objectId: string
  actorType: ActorType
  actorId: string
  action: string
  confidence: Confidence
  model: string | null
  note: string | null
  sourceRefs: string[]
  at: number
  entryHash: string
}

export type Origin = 'human' | 'ai' | 'external' | 'mixed' | 'unknown'

export interface Authorship {
  origin: Origin
  transformation: string | null
  disclosureLine: string
  /** Any event in the chain was inferred rather than observed. */
  uncertain: boolean
  /** No record at all — which is not evidence that a human wrote it. */
  unrecorded: boolean
  events: number
  models: string[]
}

export interface AuthorshipSummary {
  blocks: number
  human: number
  ai: number
  external: number
  mixed: number
  unknown: number
  /** Its own count. Never folded into `human`. */
  unrecorded: number
  uncertain: number
  chainIntact: boolean
  models: string[]
}

export interface ChainBreak {
  seq: number
  expected: string
  found: string
  detail: string
}


export const lineage = {
  /** Takes a model's proposal into the manuscript and records that it did.
   *  The alternative — the author copying it by hand — cannot be observed. */
  acceptProposal: (
    path: string,
    id: string,
    text: string,
    model: string,
    params: Record<string, unknown> = {},
  ) => call<Block>('assist_accept', { path, id, text, model, params }),

  blocks: (path: string, documentId: string) =>
    call<Block[]>('blocks_list', { path, documentId }),
  history: (path: string, blockId: string) =>
    call<ProvEvent[]>('block_history', { path, blockId }),
  authorship: (path: string, blockId: string) =>
    call<Authorship>('block_authorship', { path, blockId }),
  summary: (path: string, documentId: string | null) =>
    call<AuthorshipSummary>('provenance_summary', { path, documentId }),
  /** Null means the ledger is intact. */
  verify: (path: string) => call<ChainBreak | null>('provenance_verify', { path }),
  reconcile: (path: string) => call<number>('provenance_reconcile', { path }),
}

export const project6 = {
  templates: () => call<[string, string][]>('project_templates'),
  createFromTemplate: (parent: string, name: string, kind: string, template: string) =>
    call<ProjectSummary>('create_project_from_template', { parent, name, kind, template }),

  goals: (path: string) => call<Goals>('goals_read', { path }),
  setGoals: (path: string, goals: Goals) => call<Goals>('goals_write', { path, goals }),
  startSession: (path: string) => call<Goals>('goals_start_session', { path }),
  /** Words since the session mark. `null` means no session has begun — which
   *  the UI shows differently from zero. */
  sessionProgress: (path: string) => call<number | null>('goals_session_progress', { path }),

  /** Place a freeform corkboard card, or pass nulls to return it to the grid. */
  setPosition: (path: string, id: string, x: number | null, y: number | null) =>
    call<void>('set_document_position', { path, id, x, y }),
  setIcon: (path: string, id: string, icon: string | null) =>
    call<DocumentRow>('set_document_icon', { path, id, icon }),
}
