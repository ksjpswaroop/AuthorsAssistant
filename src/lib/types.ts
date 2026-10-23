/** Mirrors src-tauri/src/model.rs. Keep the two in step. */

export type DocKind = 'folder' | 'scene' | 'sheet' | 'location' | 'lore' | 'research' | 'canon' | 'canon'

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

  // --- Phase 2 ---
  notes: string | null
  /** Per-document word target. */
  target: number | null
  pov: string | null
  /** Foreshadowing ledger: ids planted here, and ids paid off here. */
  plants: string | null
  payoffs: string | null
  keywords: string[]
  links: DocumentLink[]

  // --- Phase 6 ---
  /** Chapter Heading, Scene, Part Heading, Front Matter, Back Matter. */
  sectionType: string | null
  /** Binder icon override, by icon name. Null uses the kind's default. */
  icon: string | null
  /** Freeform corkboard position. Null means never placed — which is not the
   *  same as being placed at (0, 0), a position the author chose. */
  x: number | null
  y: number | null
}

export interface DocumentLink {
  fromId: string
  toId: string
  relation: string
}

export interface Snapshot {
  id: string
  message: string
  at: number
  /** Paths this commit touched, relative to the project root. */
  files: string[]
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

export interface ExtendedPatch {
  notes?: string
  target?: number
  pov?: string
  plants?: string
  payoffs?: string
}

/** What an import brought in, and — just as important — what it passed over. */
export interface ImportReport {
  folders: number
  documents: number
  words: number
  /** Each entry names a file and why it was skipped. Never a silent loss. */
  skipped: string[]
  rootId: string
}

/** Measurable properties of prose, shared by integrity signals and the style profile. */
export interface WritingSignals {
  wordCount: number
  sentenceCount: number
  meanSentenceLength: number
  sentenceLengthVariance: number
  typeTokenRatio: number
  abstractDensity: number
}

/** The author's own measured voice, built from prose they marked Done. */
export interface StyleProfile {
  builtAt: number
  sampleDocuments: number
  sampleWords: number
  /** False means it was built from drafts because nothing was marked Done. */
  fromFinishedWork: boolean
  signals: WritingSignals
  vocabularyRange: number
  dialogueDensity: number
  /** A model's reading of the voice. Absent when built without one. */
  voice: string | null
  voiceModel: string | null
}

export interface StyleNote {
  dimension: string
  yours: number
  profile: number
  /** 0–100, where 100 is indistinguishable from the profile. */
  closeness: number
  note: string
}

export interface StyleDistance {
  overall: number
  notes: StyleNote[]
  /** True when the passage was too short to measure honestly. */
  tooShort: boolean
}
