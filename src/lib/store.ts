import { create } from 'zustand'
import { api } from './api'
import type {
  DocKind,
  DocumentRow,
  ExtendedPatch,
  ImportReport,
  MetaPatch,
  ProjectSummary,
  RecentProject,
  Snapshot,
} from './types'
import { countWords } from './format'
import { bulkSetMetadata, project6 } from './phases'

export type Route =
  | 'welcome'
  | 'studio'
  | 'integrity'
  | 'research'
  | 'library'
  | 'paper'
  | 'techbook'
  | 'settings'
  | 'story'
  | 'insights'
  | 'search'
export type StudioView = 'editor' | 'corkboard' | 'outliner' | 'scrivenings'
export type InspectorTab = 'synopsis' | 'notes' | 'lineage'

/** How long typing has to pause before the buffer is written to disk. */
const AUTOSAVE_MS = 800

interface AppState {
  route: Route
  commandBarOpen: boolean
  compileOpen: boolean
  /** Phase 6 project dialogs, one at a time. */
  dialog: null | 'templates' | 'goals' | 'snapshot' | 'import' | 'sheetTemplates'

  project: ProjectSummary | null
  recents: RecentProject[]
  docs: DocumentRow[]
  selectedId: string | null

  /** The open document's body. The editor is the owner while `dirty` is true. */
  body: string
  bodyLoadedFor: string | null
  dirty: boolean
  saving: boolean
  lastSavedAt: number | null

  view: StudioView
  /** Distraction-free writing: hides every pane but the manuscript column. */
  composition: boolean
  /** Multi-select for bulk metadata (Phase 6). Empty means single-selection. */
  marked: string[]
  showBinder: boolean
  showInspector: boolean
  inspectorTab: InspectorTab
  busy: boolean
  error: string | null

  // --- Phase 2 ---
  /** Second document shown beside the editor. Null means a single pane. */
  splitId: string | null
  snapshots: Snapshot[]
  snapshotsFor: string | null
  snapshotBusy: boolean
  /** Phase 3 assist panel, beside the editor. */
  assistOpen: boolean

  /** What an import would bring in, surveyed before anything is created. */
  importPlan: { source: string; name: string; folders: number; files: number } | null
  /** What the last import actually did. Null until one has run. */
  importReport: ImportReport | null
  importBusy: boolean

  go: (route: Route) => void
  setCommandBar: (open: boolean) => void
  setCompile: (open: boolean) => void
  setDialog: (dialog: null | 'templates' | 'goals' | 'snapshot' | 'import' | 'sheetTemplates') => void
  setView: (view: StudioView) => void
  setComposition: (on: boolean) => void
  toggleMark: (id: string) => void
  clearMarks: () => void
  setInspectorTab: (tab: InspectorTab) => void
  toggleBinder: () => void
  toggleInspector: () => void
  dismissError: () => void

  loadRecents: () => Promise<void>
  openStartupProject: () => Promise<void>
  newProject: () => Promise<void>
  openProjectAt: (path: string) => Promise<void>
  openProjectViaDialog: () => Promise<void>
  closeProject: () => void

  pickImportFolder: () => Promise<void>
  runImport: () => Promise<void>
  dismissImport: () => void

  select: (id: string) => Promise<void>
  edit: (body: string) => void
  saveNow: () => Promise<void>
  addDocument: (parentId: string | null, title: string, kind: DocKind) => Promise<void>
  rename: (id: string, title: string) => Promise<void>
  patchMeta: (id: string, patch: MetaPatch) => Promise<void>
  remove: (id: string) => Promise<void>

  setSplit: (id: string | null) => void
  setAssist: (open: boolean) => void
  moveDocument: (id: string, newParent: string | null, index: number) => Promise<void>
  patchExtended: (id: string, patch: ExtendedPatch) => Promise<void>
  setKeywords: (id: string, keywords: string[]) => Promise<void>
  /** Place a freeform corkboard card, or pass nulls to return it to the grid. */
  placeCard: (id: string, x: number | null, y: number | null) => Promise<void>
  /** Re-read the binder after something outside the main editor changed a file. */
  refreshDocs: () => Promise<void>
  setIcon: (id: string, icon: string | null) => Promise<void>
  /** Set metadata across documents. One id is the single-document case. */
  bulkSet: (
    ids: string[],
    fields: { status?: string; label?: string; sectionType?: string; pov?: string },
  ) => Promise<void>
  createSheet: (name: string, kind: DocKind, linkTo: string | null) => Promise<void>
  loadSnapshots: (docPath: string | null) => Promise<void>
  takeSnapshot: (message: string) => Promise<void>
  restoreSnapshot: (commit: string, docPath: string) => Promise<void>
}

let saveTimer: ReturnType<typeof setTimeout> | null = null

function fail(set: (p: Partial<AppState>) => void, e: unknown) {
  set({ error: e instanceof Error ? e.message : String(e), busy: false, saving: false })
}

export const useApp = create<AppState>((set, get) => ({
  route: 'welcome',
  commandBarOpen: false,
  compileOpen: false,
  dialog: null,
  importPlan: null,
  importReport: null,
  importBusy: false,

  project: null,
  recents: [],
  docs: [],
  selectedId: null,

  body: '',
  bodyLoadedFor: null,
  dirty: false,
  saving: false,
  lastSavedAt: null,

  view: 'editor',
  composition: false,
  marked: [],
  showBinder: true,
  showInspector: true,
  inspectorTab: 'synopsis',
  busy: false,
  error: null,

  splitId: null,
  snapshots: [],
  snapshotsFor: null,
  snapshotBusy: false,
  assistOpen: false,

  go: (route) => set({ route }),
  setCommandBar: (commandBarOpen) => set({ commandBarOpen }),
  setCompile: (compileOpen) => set({ compileOpen }),
  setDialog: (dialog) => set({ dialog }),
  setView: (view) => set({ view }),
  setComposition: (composition) => set({ composition }),
  toggleMark: (id) =>
    set((s) => ({ marked: s.marked.includes(id) ? s.marked.filter((m) => m !== id) : [...s.marked, id] })),
  clearMarks: () => set({ marked: [] }),
  setInspectorTab: (inspectorTab) => set({ inspectorTab }),
  toggleBinder: () => set({ showBinder: !get().showBinder }),
  toggleInspector: () => set({ showInspector: !get().showInspector }),
  dismissError: () => set({ error: null }),

  async loadRecents() {
    try {
      set({ recents: await api.recentProjects() })
    } catch (e) {
      fail(set, e)
    }
  },

  /** Dev only: honour AEGISDRAFT_OPEN so a screen can be tested directly. */
  async openStartupProject() {
    try {
      const path = await api.startupProject()
      if (path) await get().openProjectAt(path)
      const route = await api.startupRoute()
      if (route && path) set({ route: route as Route })
    } catch {
      // Not being able to auto-open is never worth an error toast.
    }
  },

  async newProject() {
    try {
      const parent = await api.pickFolder('Where should the project live?')
      if (!parent) return
      set({ busy: true, error: null })
      const name = `Untitled Book ${new Date().toLocaleDateString('en-GB')}`
      const project = await api.createProject(parent, name, 'novel')
      await get().openProjectAt(project.path)
    } catch (e) {
      fail(set, e)
    }
  },

  async openProjectViaDialog() {
    try {
      const path = await api.pickFolder('Open an AegisDraft project')
      if (!path) return
      await get().openProjectAt(path)
    } catch (e) {
      fail(set, e)
    }
  },

  // Import runs in two steps on purpose. The survey happens first and creates
  // nothing, so the author sees how many documents are about to appear — and
  // hears about a folder that cannot be imported — before committing to it.
  async pickImportFolder() {
    try {
      const path = await api.pickFolder('Import a folder of Markdown')
      if (!path) return
      set({ importBusy: true, error: null })
      const [folders, files] = await api.importSurvey(path)
      const name = path.split('/').filter(Boolean).pop() ?? path
      set({
        importPlan: { source: path, name, folders, files },
        importReport: null,
        importBusy: false,
        dialog: 'import',
      })
    } catch (e) {
      set({ importBusy: false, importPlan: null })
      fail(set, e)
    }
  },

  async runImport() {
    const { project, importPlan } = get()
    if (!project || !importPlan) return
    try {
      set({ importBusy: true, error: null })
      const report = await api.importFolder(project.path, importPlan.source, null, 'scene')
      const docs = await api.listDocuments(project.path)
      set({ docs, importReport: report, importPlan: null, importBusy: false })
    } catch (e) {
      set({ importBusy: false })
      fail(set, e)
    }
  },

  dismissImport: () => set({ dialog: null, importPlan: null, importReport: null }),

  async openProjectAt(path) {
    try {
      set({ busy: true, error: null })
      const project = await api.openProject(path)
      const docs = await api.listDocuments(path)
      const first = docs.find((d) => d.kind !== 'folder') ?? null
      set({
        project,
        docs,
        route: 'studio',
        busy: false,
        selectedId: null,
        body: '',
        bodyLoadedFor: null,
        dirty: false,
        lastSavedAt: null,
      })
      if (first) await get().select(first.id)
      await get().loadRecents()
    } catch (e) {
      fail(set, e)
    }
  },

  closeProject() {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = null
    set({
      project: null,
      docs: [],
      selectedId: null,
      body: '',
      bodyLoadedFor: null,
      dirty: false,
      route: 'welcome',
    })
  },

  async select(id) {
    const { project, dirty, selectedId } = get()
    if (!project || id === selectedId) return
    try {
      // Never lose the buffer by switching away from it.
      if (dirty) await get().saveNow()
      const doc = get().docs.find((d) => d.id === id)
      if (!doc || doc.kind === 'folder') {
        set({ selectedId: id, body: '', bodyLoadedFor: id, dirty: false })
        return
      }
      const body = await api.readDocument(project.path, id)
      set({ selectedId: id, body, bodyLoadedFor: id, dirty: false })
    } catch (e) {
      fail(set, e)
    }
  },

  edit(body) {
    set({ body, dirty: true })
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(() => {
      void get().saveNow()
    }, AUTOSAVE_MS)
  },

  async saveNow() {
    const { project, selectedId, body, dirty } = get()
    if (!project || !selectedId || !dirty) return
    const doc = get().docs.find((d) => d.id === selectedId)
    if (!doc || doc.kind === 'folder') return
    if (saveTimer) {
      clearTimeout(saveTimer)
      saveTimer = null
    }
    try {
      set({ saving: true })
      const receipt = await api.writeDocument(project.path, selectedId, body)
      set((s) => ({
        saving: false,
        // Only clear the dirty flag if nothing was typed while the write was in
        // flight; otherwise the newer keystrokes would be silently dropped.
        dirty: s.body !== body,
        lastSavedAt: receipt.updatedAt,
        docs: s.docs.map((d) =>
          d.id === receipt.id ? { ...d, wordCount: receipt.wordCount, updatedAt: receipt.updatedAt } : d,
        ),
        project: s.project
          ? { ...s.project, wordCount: s.docs.reduce((n, d) => n + (d.id === receipt.id ? receipt.wordCount : d.wordCount), 0) }
          : s.project,
      }))
    } catch (e) {
      fail(set, e)
    }
  },

  async addDocument(parentId, title, kind) {
    const { project } = get()
    if (!project) return
    try {
      const doc = await api.createDocument(project.path, parentId, title, kind)
      set((s) => ({ docs: [...s.docs, doc] }))
      if (kind !== 'folder') await get().select(doc.id)
    } catch (e) {
      fail(set, e)
    }
  },

  async rename(id, title) {
    const { project } = get()
    if (!project || !title.trim()) return
    try {
      const doc = await api.renameDocument(project.path, id, title.trim())
      set((s) => ({ docs: s.docs.map((d) => (d.id === id ? doc : d)) }))
    } catch (e) {
      fail(set, e)
    }
  },

  async patchMeta(id, patch) {
    const { project } = get()
    if (!project) return
    try {
      const doc = await api.updateDocumentMeta(project.path, id, patch)
      set((s) => ({ docs: s.docs.map((d) => (d.id === id ? doc : d)) }))
    } catch (e) {
      fail(set, e)
    }
  },

  setSplit: (splitId) => set({ splitId }),
  setAssist: (assistOpen) => set({ assistOpen }),

  async moveDocument(id, newParent, index) {
    const { project } = get()
    if (!project) return
    try {
      await api.moveDocument(project.path, id, newParent, index)
      set({ docs: await api.listDocuments(project.path) })
    } catch (e) {
      fail(set, e)
    }
  },

  async patchExtended(id, patch) {
    const { project } = get()
    if (!project) return
    try {
      const doc = await api.updateExtended(project.path, id, patch)
      set((s) => ({ docs: s.docs.map((d) => (d.id === id ? doc : d)) }))
    } catch (e) {
      fail(set, e)
    }
  },

  async setKeywords(id, keywords) {
    const { project } = get()
    if (!project) return
    try {
      const doc = await api.setKeywords(project.path, id, keywords)
      set((s) => ({ docs: s.docs.map((d) => (d.id === id ? doc : d)) }))
    } catch (e) {
      fail(set, e)
    }
  },

  // Freeform placement writes through immediately: a board the author has to
  // remember to save is a board that loses work.
  async bulkSet(ids, fields) {
    const { project } = get()
    if (!project || ids.length === 0) return
    try {
      await bulkSetMetadata(project.path, ids, fields)
      await get().refreshDocs()
    } catch (e) {
      fail(set, e)
    }
  },

  async refreshDocs() {
    const { project } = get()
    if (!project) return
    try {
      set({ docs: await api.listDocuments(project.path) })
    } catch (e) {
      fail(set, e)
    }
  },

  async placeCard(id, x, y) {
    const { project } = get()
    if (!project) return
    try {
      await project6.setPosition(project.path, id, x, y)
      set((s) => ({ docs: s.docs.map((d) => (d.id === id ? { ...d, x, y } : d)) }))
    } catch (e) {
      fail(set, e)
    }
  },

  async setIcon(id, icon) {
    const { project } = get()
    if (!project) return
    try {
      const doc = await project6.setIcon(project.path, id, icon)
      set((s) => ({ docs: s.docs.map((d) => (d.id === id ? doc : d)) }))
    } catch (e) {
      fail(set, e)
    }
  },

  async createSheet(name, kind, linkTo) {
    const { project } = get()
    if (!project) return
    try {
      await api.createSheet(project.path, name, kind, null, linkTo)
      set({ docs: await api.listDocuments(project.path) })
    } catch (e) {
      fail(set, e)
    }
  },

  async loadSnapshots(docPath) {
    const { project } = get()
    if (!project) return
    try {
      const snapshots = await api.snapshotList(project.path, docPath, 50)
      set({ snapshots, snapshotsFor: docPath })
    } catch (e) {
      fail(set, e)
    }
  },

  async takeSnapshot(message) {
    const { project, dirty } = get()
    if (!project) return
    try {
      set({ snapshotBusy: true })
      // Anything in the editor buffer must reach disk before it is committed.
      if (dirty) await get().saveNow()
      const snap = await api.snapshotCreate(project.path, message)
      set({ snapshotBusy: false, error: snap ? null : 'Nothing has changed since the last snapshot.' })
      await get().loadSnapshots(get().snapshotsFor)
    } catch (e) {
      fail(set, e)
    }
  },

  async restoreSnapshot(commit, docPath) {
    const { project, selectedId } = get()
    if (!project) return
    try {
      set({ snapshotBusy: true })
      await api.snapshotRestoreFile(project.path, commit, docPath)
      set({ snapshotBusy: false, docs: await api.listDocuments(project.path) })
      // Reload the open document so the editor shows what is now on disk.
      if (selectedId) {
        const body = await api.readDocument(project.path, selectedId)
        set({ body, bodyLoadedFor: `${selectedId}:${Date.now()}`, dirty: false })
      }
      await get().loadSnapshots(docPath)
    } catch (e) {
      fail(set, e)
    }
  },

  async remove(id) {
    const { project } = get()
    if (!project) return
    try {
      await api.deleteDocument(project.path, id)
      set((s) => {
        // Deleting a folder removes everything under it, at any depth.
        const doomed = new Set<string>([id])
        let grew = true
        while (grew) {
          grew = false
          for (const d of s.docs) {
            if (d.parentId && doomed.has(d.parentId) && !doomed.has(d.id)) {
              doomed.add(d.id)
              grew = true
            }
          }
        }
        const docs = s.docs.filter((d) => !doomed.has(d.id))
        const stillThere = docs.some((d) => d.id === s.selectedId)
        return {
          docs,
          selectedId: stillThere ? s.selectedId : null,
          body: stillThere ? s.body : '',
          bodyLoadedFor: stillThere ? s.bodyLoadedFor : null,
          dirty: false,
        }
      })
    } catch (e) {
      fail(set, e)
    }
  },
}))

/** Word count of the live buffer, which may be ahead of what is on disk. */
export function useLiveWordCount(): number {
  return useApp((s) => (s.dirty ? countWords(s.body) : (s.docs.find((d) => d.id === s.selectedId)?.wordCount ?? 0)))
}

/** Children of a binder row, in order. */
export function childrenOf(docs: DocumentRow[], parentId: string | null): DocumentRow[] {
  return docs.filter((d) => d.parentId === parentId).sort((a, b) => a.position - b.position)
}

/** Flush pending work when the window goes away — the last line of defence. */
export function installSaveGuards() {
  const flush = () => {
    const s = useApp.getState()
    if (s.dirty) void s.saveNow()
  }
  window.addEventListener('blur', flush)
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') flush()
  })
  window.addEventListener('beforeunload', flush)
}
