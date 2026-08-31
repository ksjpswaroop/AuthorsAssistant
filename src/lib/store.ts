import { create } from 'zustand'
import { api } from './api'
import type { DocKind, DocumentRow, MetaPatch, ProjectSummary, RecentProject } from './types'
import { countWords } from './format'

export type Route = 'welcome' | 'studio' | 'integrity' | 'research' | 'library' | 'paper' | 'techbook' | 'settings'
export type StudioView = 'editor' | 'corkboard' | 'outliner'
export type InspectorTab = 'synopsis' | 'notes' | 'agent'

/** How long typing has to pause before the buffer is written to disk. */
const AUTOSAVE_MS = 800

interface AppState {
  route: Route
  commandBarOpen: boolean
  compileOpen: boolean

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
  showBinder: boolean
  showInspector: boolean
  inspectorTab: InspectorTab
  busy: boolean
  error: string | null

  go: (route: Route) => void
  setCommandBar: (open: boolean) => void
  setCompile: (open: boolean) => void
  setView: (view: StudioView) => void
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

  select: (id: string) => Promise<void>
  edit: (body: string) => void
  saveNow: () => Promise<void>
  addDocument: (parentId: string | null, title: string, kind: DocKind) => Promise<void>
  rename: (id: string, title: string) => Promise<void>
  patchMeta: (id: string, patch: MetaPatch) => Promise<void>
  remove: (id: string) => Promise<void>
}

let saveTimer: ReturnType<typeof setTimeout> | null = null

function fail(set: (p: Partial<AppState>) => void, e: unknown) {
  set({ error: e instanceof Error ? e.message : String(e), busy: false, saving: false })
}

export const useApp = create<AppState>((set, get) => ({
  route: 'welcome',
  commandBarOpen: false,
  compileOpen: false,

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
  showBinder: true,
  showInspector: true,
  inspectorTab: 'synopsis',
  busy: false,
  error: null,

  go: (route) => set({ route }),
  setCommandBar: (commandBarOpen) => set({ commandBarOpen }),
  setCompile: (compileOpen) => set({ compileOpen }),
  setView: (view) => set({ view }),
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
