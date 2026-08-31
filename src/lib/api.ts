/**
 * The only place the frontend talks to the outside world.
 *
 * Inside Tauri every call goes to a Rust command (see src-tauri/src/lib.rs),
 * which owns file I/O and SQLite. Opened in a plain browser — `npm run dev`
 * without the desktop shell — it falls back to an in-memory project seeded with
 * the sample manuscript from the design canvas, so the UI stays reviewable
 * without a Rust toolchain. The demo backend never touches disk.
 */

import type { DocKind, DocumentRow, MetaPatch, ProjectSummary, RecentProject, SaveReceipt } from './types'
import { countWords } from './format'
import { DEMO_DOCS, DEMO_BODIES } from './demoProject'

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export interface Backend {
  createProject(parent: string, name: string, kind: string): Promise<ProjectSummary>
  openProject(path: string): Promise<ProjectSummary>
  listDocuments(path: string): Promise<DocumentRow[]>
  readDocument(path: string, id: string): Promise<string>
  writeDocument(path: string, id: string, body: string): Promise<SaveReceipt>
  createDocument(path: string, parentId: string | null, title: string, kind: DocKind): Promise<DocumentRow>
  renameDocument(path: string, id: string, title: string): Promise<DocumentRow>
  updateDocumentMeta(path: string, id: string, patch: MetaPatch): Promise<DocumentRow>
  deleteDocument(path: string, id: string): Promise<void>
  recentProjects(): Promise<RecentProject[]>
  pickFolder(title: string): Promise<string | null>
  /** Dev only: a project to open immediately, from AEGISDRAFT_OPEN. */
  startupProject(): Promise<string | null>
}

/* -------------------------------------------------------------------------- */
/* Tauri                                                                       */
/* -------------------------------------------------------------------------- */

const tauriBackend: Backend = {
  async createProject(parent, name, kind) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('create_project', { parent, name, kind })
  },
  async openProject(path) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('open_project', { path })
  },
  async listDocuments(path) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('list_documents', { path })
  },
  async readDocument(path, id) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('read_document', { path, id })
  },
  async writeDocument(path, id, body) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('write_document', { path, id, body })
  },
  async createDocument(path, parentId, title, kind) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('create_document', { path, parentId, title, kind })
  },
  async renameDocument(path, id, title) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('rename_document', { path, id, title })
  },
  async updateDocumentMeta(path, id, patch) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('update_document_meta', {
      path,
      id,
      synopsis: patch.synopsis ?? null,
      label: patch.label ?? null,
      status: patch.status ?? null,
    })
  },
  async deleteDocument(path, id) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('delete_document', { path, id })
  },
  async recentProjects() {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('recent_projects')
  },
  async pickFolder(title) {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const picked = await open({ directory: true, multiple: false, title })
    return typeof picked === 'string' ? picked : null
  },
  async startupProject() {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('startup_project')
  },
}

/* -------------------------------------------------------------------------- */
/* Browser demo                                                                */
/* -------------------------------------------------------------------------- */

const DEMO_PATH = '/demo/The Salt Road.aegis'

function makeDemoBackend(): Backend {
  const docs: DocumentRow[] = DEMO_DOCS.map((d) => ({ ...d }))
  const bodies = new Map<string, string>(Object.entries(DEMO_BODIES))
  let seq = 0

  const summary = (): ProjectSummary => ({
    path: DEMO_PATH,
    name: 'The Salt Road',
    kind: 'novel',
    wordCount: docs.reduce((n, d) => n + d.wordCount, 0),
    documentCount: docs.filter((d) => d.kind !== 'folder').length,
    openedAt: Date.now(),
  })

  const find = (id: string) => docs.find((d) => d.id === id)

  return {
    async createProject(_parent, name) {
      return { ...summary(), name }
    },
    async openProject() {
      return summary()
    },
    async listDocuments() {
      return docs.map((d) => ({ ...d }))
    },
    async readDocument(_path, id) {
      return bodies.get(id) ?? ''
    },
    async writeDocument(_path, id, body) {
      bodies.set(id, body)
      const doc = find(id)
      const wordCount = countWords(body)
      const updatedAt = Date.now()
      if (doc) {
        doc.wordCount = wordCount
        doc.updatedAt = updatedAt
      }
      return { id, wordCount, updatedAt }
    },
    async createDocument(_path, parentId, title, kind) {
      const doc: DocumentRow = {
        id: `demo-${++seq}-${Date.now()}`,
        parentId,
        title,
        kind,
        path: kind === 'folder' ? null : `manuscript/scenes/demo-${seq}.md`,
        position: docs.filter((d) => d.parentId === parentId).length,
        synopsis: null,
        label: null,
        status: null,
        wordCount: 0,
        updatedAt: Date.now(),
      }
      docs.push(doc)
      bodies.set(doc.id, '')
      return { ...doc }
    },
    async renameDocument(_path, id, title) {
      const doc = find(id)
      if (!doc) throw new Error(`no document ${id}`)
      doc.title = title
      doc.updatedAt = Date.now()
      return { ...doc }
    },
    async updateDocumentMeta(_path, id, patch) {
      const doc = find(id)
      if (!doc) throw new Error(`no document ${id}`)
      if (patch.synopsis !== undefined) doc.synopsis = patch.synopsis
      if (patch.label !== undefined) doc.label = patch.label
      if (patch.status !== undefined) doc.status = patch.status
      doc.updatedAt = Date.now()
      return { ...doc }
    },
    async deleteDocument(_path, id) {
      const i = docs.findIndex((d) => d.id === id)
      if (i >= 0) docs.splice(i, 1)
      bodies.delete(id)
    },
    async recentProjects() {
      return [
        {
          path: DEMO_PATH,
          name: 'The Salt Road',
          kind: 'novel',
          wordCount: summary().wordCount,
          openedAt: Date.now() - 120_000,
          exists: true,
        },
      ]
    },
    async pickFolder() {
      return null
    },
    async startupProject() {
      return null
    },
  }
}

export const api: Backend = isTauri ? tauriBackend : makeDemoBackend()

/** Where the demo project lives, so screens can explain themselves. */
export const DEMO_PROJECT_PATH = DEMO_PATH
