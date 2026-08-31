/** Mirrors src-tauri/src/model.rs. Keep the two in step. */

export type DocKind = 'folder' | 'scene' | 'sheet' | 'lore' | 'research'

export interface DocumentRow {
  id: string
  parentId: string | null
  title: string
  kind: DocKind
  /** Relative to the project root; null for folders. */
  path: string | null
  position: number
  synopsis: string | null
  label: string | null
  status: string | null
  wordCount: number
  updatedAt: number
}

export interface ProjectSummary {
  path: string
  name: string
  kind: string
  wordCount: number
  documentCount: number
  openedAt: number
}

export interface RecentProject {
  path: string
  name: string
  kind: string
  wordCount: number
  openedAt: number
  exists: boolean
}

export interface SaveReceipt {
  id: string
  wordCount: number
  updatedAt: number
}

export interface MetaPatch {
  synopsis?: string
  label?: string
  status?: string
}
