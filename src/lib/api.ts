/**
 * The only place the frontend talks to the outside world.
 *
 * Inside Tauri every call goes to a Rust command (see src-tauri/src/lib.rs),
 * which owns file I/O and SQLite. Opened in a plain browser — `npm run dev`
 * without the desktop shell — it falls back to an in-memory project seeded with
 * the sample manuscript from the design canvas, so the UI stays reviewable
 * without a Rust toolchain. The demo backend never touches disk.
 */

import type {
  DocKind,
  DocumentRow,
  ExtendedPatch,
  ImportReport,
  MetaPatch,
  ProjectSummary,
  RecentProject,
  SaveReceipt,
  Snapshot,
} from './types'
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
  // --- Phase 2 ---
  moveDocument(path: string, id: string, newParent: string | null, index: number): Promise<void>
  createSheet(path: string, name: string, kind: DocKind, parentId: string | null, linkTo: string | null): Promise<DocumentRow>
  appendCanonEntry(path: string, id: string, fact: string, source: string): Promise<SaveReceipt>
  updateExtended(path: string, id: string, patch: ExtendedPatch): Promise<DocumentRow>
  setKeywords(path: string, id: string, keywords: string[]): Promise<DocumentRow>
  projectKeywords(path: string): Promise<[string, number][]>
  linkDocuments(path: string, fromId: string, toId: string, relation: string): Promise<void>
  unlinkDocuments(path: string, fromId: string, toId: string, relation: string): Promise<void>
  snapshotCreate(path: string, message: string): Promise<Snapshot | null>
  snapshotList(path: string, docPath: string | null, limit?: number): Promise<Snapshot[]>
  snapshotReadFile(path: string, commit: string, docPath: string): Promise<string>
  snapshotRestoreFile(path: string, commit: string, docPath: string): Promise<void>
  snapshotIsDirty(path: string): Promise<boolean>

  /** Counts what a folder would bring in, and refuses one that should not be
   *  imported at all — so the author sees the size before committing. */
  importSurvey(source: string): Promise<[number, number]>
  importFolder(path: string, source: string, parentId: string | null, kind: DocKind): Promise<ImportReport>

  recentProjects(): Promise<RecentProject[]>
  pickFolder(title: string): Promise<string | null>
  /** Asks for one file to register as a research source. Null when cancelled. */
  pickFile(title: string): Promise<string | null>
  /** Dev only: a project to open immediately, from AEGISDRAFT_OPEN. */
  startupProject(): Promise<string | null>
  /** Dev only: a screen to open immediately, from AEGISDRAFT_ROUTE. */
  startupRoute(): Promise<string | null>
  /** Dismisses the splash window and reveals the main one. */
  frontendReady(): Promise<void>
  /** Asks where to write an exported file. Null when cancelled. */
  pickSaveFile(title: string, defaultName: string): Promise<string | null>
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
  async moveDocument(path, id, newParent, index) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('move_document', { path, id, newParent, index })
  },
  async createSheet(path, name, kind, parentId, linkTo) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('create_sheet', { path, name, kind, parentId, linkTo })
  },
  async appendCanonEntry(path, id, fact, source) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('append_canon_entry', { path, id, fact, source })
  },
  async updateExtended(path, id, patch) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('update_document_extended', {
      path,
      id,
      notes: patch.notes ?? null,
      target: patch.target ?? null,
      pov: patch.pov ?? null,
      plants: patch.plants ?? null,
      payoffs: patch.payoffs ?? null,
    })
  },
  async setKeywords(path, id, keywords) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('set_keywords', { path, id, keywords })
  },
  async projectKeywords(path) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('project_keywords', { path })
  },
  async linkDocuments(path, fromId, toId, relation) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('link_documents', { path, fromId, toId, relation })
  },
  async unlinkDocuments(path, fromId, toId, relation) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('unlink_documents', { path, fromId, toId, relation })
  },
  async snapshotCreate(path, message) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('snapshot_create', { path, message })
  },
  async snapshotList(path, docPath, limit) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('snapshot_list', { path, docPath, limit: limit ?? null })
  },
  async snapshotReadFile(path, commit, docPath) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('snapshot_read_file', { path, commit, docPath })
  },
  async snapshotRestoreFile(path, commit, docPath) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('snapshot_restore_file', { path, commit, docPath })
  },
  async snapshotIsDirty(path) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('snapshot_is_dirty', { path })
  },
  async importSurvey(source) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('import_survey', { source })
  },
  async importFolder(path, source, parentId, kind) {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('import_folder', { path, source, parentId, kind })
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
  async pickFile(title) {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const picked = await open({
      directory: false,
      multiple: false,
      title,
      filters: [
        { name: 'Readable here', extensions: ['md', 'markdown', 'txt', 'html', 'htm', 'docx', 'pdf'] },
      ],
    })
    return typeof picked === 'string' ? picked : null
  },
  async startupProject() {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('startup_project')
  },
  async startupRoute() {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('startup_route')
  },
  async frontendReady() {
    const { invoke } = await import('@tauri-apps/api/core')
    return invoke('frontend_ready')
  },
  async pickSaveFile(title, defaultName) {
    const { save } = await import('@tauri-apps/plugin-dialog')
    const picked = await save({ title, defaultPath: defaultName })
    return typeof picked === 'string' ? picked : null
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
        notes: null,
        target: null,
        pov: null,
        plants: null,
        payoffs: null,
        keywords: [],
        links: [],
        sectionType: null,
        icon: null,
        x: null,
        y: null,
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
    async moveDocument(_path, id, newParent, index) {
      const doc = find(id)
      if (!doc) throw new Error(`no document ${id}`)
      if (newParent) {
        // Same guard the Rust side enforces: no folder inside itself.
        let cursor: string | null = newParent
        while (cursor) {
          if (cursor === id) throw new Error('a folder cannot be moved inside itself')
          cursor = find(cursor)?.parentId ?? null
        }
      }
      doc.parentId = newParent
      const siblings = docs.filter((d) => d.parentId === newParent && d.id !== id).sort((a, b) => a.position - b.position)
      siblings.splice(Math.max(0, Math.min(index, siblings.length)), 0, doc)
      siblings.forEach((d, i) => {
        d.position = i
      })
    },
    async createSheet(_path, name, kind, parentId, linkTo) {
      const doc = await this.createDocument(_path, parentId, name, kind)
      if (linkTo) doc.links = [{ fromId: doc.id, toId: linkTo, relation: 'appears_in' }]
      const stored = find(doc.id)
      if (stored) stored.links = doc.links
      return doc
    },
    async appendCanonEntry(_path, id, fact, source) {
      const next = `${(bodies.get(id) ?? '').trimEnd()}\n\n- ${fact}  \n  *established in:* ${source}\n`.trim()
      return this.writeDocument(_path, id, next)
    },
    async updateExtended(_path, id, patch) {
      const doc = find(id)
      if (!doc) throw new Error(`no document ${id}`)
      Object.assign(doc, patch)
      doc.updatedAt = Date.now()
      return { ...doc }
    },
    async setKeywords(_path, id, keywords) {
      const doc = find(id)
      if (!doc) throw new Error(`no document ${id}`)
      doc.keywords = keywords.map((k) => k.trim()).filter(Boolean)
      return { ...doc }
    },
    async projectKeywords() {
      const counts = new Map<string, number>()
      for (const d of docs) for (const k of d.keywords) counts.set(k, (counts.get(k) ?? 0) + 1)
      return [...counts.entries()].sort((a, b) => a[0].localeCompare(b[0]))
    },
    async linkDocuments(_path, fromId, toId, relation) {
      const link = { fromId, toId, relation }
      for (const d of docs) {
        if ((d.id === fromId || d.id === toId) && !d.links.some((l) => l.fromId === fromId && l.toId === toId)) {
          d.links.push(link)
        }
      }
    },
    async unlinkDocuments(_path, fromId, toId, relation) {
      for (const d of docs) {
        d.links = d.links.filter((l) => !(l.fromId === fromId && l.toId === toId && l.relation === relation))
      }
    },
    async snapshotCreate() {
      // No repository in the browser preview; say so rather than fake a commit.
      return null
    },
    async snapshotList() {
      return []
    },
    async snapshotReadFile() {
      return ''
    },
    async snapshotRestoreFile() {
      throw new Error('Snapshots need the desktop app')
    },
    async snapshotIsDirty() {
      return false
    },
    async importSurvey() {
      throw new Error('Importing a folder needs the desktop app')
    },
    async importFolder() {
      throw new Error('Importing a folder needs the desktop app')
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
    async pickFile() {
      return null
    },
    async startupProject() {
      return null
    },
    async startupRoute() {
      return null
    },
    async frontendReady() {
      // No splash window outside Tauri.
    },
    async pickSaveFile() {
      return null
    },
  }
}

export const api: Backend = isTauri ? tauriBackend : makeDemoBackend()

/** Where the demo project lives, so screens can explain themselves. */
export const DEMO_PROJECT_PATH = DEMO_PATH

/** Subscribes to native menu clicks. Returns an unsubscribe function. */
export async function onMenu(handler: (id: string) => void): Promise<() => void> {
  if (!isTauri) return () => {}
  const { listen } = await import('@tauri-apps/api/event')
  const unlisten = await listen<string>('menu', (e) => handler(e.payload))
  return unlisten
}
