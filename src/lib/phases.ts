/**
 * The Tauri command surface for the studio: collections, scrivenings, templates,
 * goals, and the paragraph ledger. Later phases add their commands in this file.
 *
 * Kept apart from api.ts (which is the project lifecycle) so the two can be read
 * independently. Everything here degrades to a clear "not available outside the
 * desktop app" rather than pretending in the browser preview.
 */

import { isTauri } from './api'
import type { DocKind, DocumentRow, ProjectSummary } from './types'

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
