use serde::{Deserialize, Serialize};

/// What kind of thing a binder row is. `Folder` rows have no file on disk;
/// everything else owns exactly one Markdown file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocKind {
    Folder,
    Scene,
    Sheet,
    Lore,
    Research,
    Canon,
    Location,
}

impl DocKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DocKind::Folder => "folder",
            DocKind::Scene => "scene",
            DocKind::Sheet => "sheet",
            DocKind::Lore => "lore",
            DocKind::Research => "research",
            DocKind::Canon => "canon",
            DocKind::Location => "location",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "folder" => DocKind::Folder,
            "sheet" => DocKind::Sheet,
            "lore" => DocKind::Lore,
            "research" => DocKind::Research,
            "canon" => DocKind::Canon,
            "location" => DocKind::Location,
            _ => DocKind::Scene,
        }
    }

    /// Directory, relative to the project root, where this kind keeps its files.
    pub fn dir(self) -> &'static str {
        match self {
            DocKind::Folder => "",
            DocKind::Scene => "manuscript/scenes",
            DocKind::Sheet => "manuscript/characters",
            DocKind::Lore => "manuscript/worldbuilding",
            DocKind::Research => "research",
            DocKind::Canon => "manuscript/canon",
            DocKind::Location => "manuscript/locations",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRow {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub kind: DocKind,
    /// Relative to the project root. `None` for folders.
    pub path: Option<String>,
    pub position: i64,
    pub synopsis: Option<String>,
    pub label: Option<String>,
    pub status: Option<String>,
    pub word_count: i64,
    pub updated_at: i64,

    // --- Phase 2 ---
    pub notes: Option<String>,
    /// Per-document word target.
    pub target: Option<i64>,
    pub pov: Option<String>,
    /// Foreshadowing ledger: ids this document plants, and ones it pays off.
    pub plants: Option<String>,
    pub payoffs: Option<String>,
    pub keywords: Vec<String>,
    pub links: Vec<DocumentLink>,

    // --- Phase 6 ---
    /// Chapter Heading, Scene, Part Heading, Front Matter, Back Matter.
    pub section_type: Option<String>,
    /// Binder icon override, by icon name.
    pub icon: Option<String>,
    /// Freeform corkboard position. `None` means the card has never been
    /// placed, so it falls back to the grid instead of stacking at the origin.
    pub x: Option<f32>,
    pub y: Option<f32>,
}

/// A link between two binder documents — a character sheet and the scene it
/// appears in, for example. Stored once and surfaced on both ends.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLink {
    pub from_id: String,
    pub to_id: String,
    pub relation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CollectionKind {
    Manual,
    Smart,
    Search,
}

impl CollectionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CollectionKind::Manual => "manual",
            CollectionKind::Smart => "smart",
            CollectionKind::Search => "search",
        }
    }
    pub fn parse(s: &str) -> Self {
        match s {
            "smart" => CollectionKind::Smart,
            "search" => CollectionKind::Search,
            _ => CollectionKind::Manual,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub kind: CollectionKind,
    /// JSON `SmartFilter` for smart and search collections.
    pub filters: Option<String>,
    pub position: i64,
    /// Members, recomputed on read.
    pub count: i64,
}

/// A smart collection's query. Every set field must match, so adding a
/// criterion always narrows the result.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartFilter {
    pub status: Option<String>,
    pub label: Option<String>,
    pub pov: Option<String>,
    pub section_type: Option<String>,
    pub keyword: Option<String>,
    pub min_words: Option<i64>,
    pub max_words: Option<i64>,
    /// Free text matched against title, synopsis and notes.
    pub text: Option<String>,
}

/// One LibGit2 commit in the project repository.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub id: String,
    pub message: String,
    pub at: i64,
    /// Paths touched by this commit, relative to the project root.
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    /// Absolute path to the `.aegis` folder.
    pub path: String,
    pub name: String,
    pub kind: String,
    pub word_count: i64,
    pub document_count: i64,
    pub opened_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    pub kind: String,
    pub word_count: i64,
    pub opened_at: i64,
    /// False when the folder has since been moved or deleted.
    pub exists: bool,
    /// A security-scoped bookmark for the folder. `None` on platforms without
    /// one, and in lists written before bookmarks existed — the plain `path` is
    /// the fallback, which is all an unsandboxed build ever needed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bookmark: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReceipt {
    pub id: String,
    pub word_count: i64,
    pub updated_at: i64,
}
// ---------------------------------------------------------------------------
// Phase 4: integrity
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarityMatch {
    pub source_id: String,
    pub source_title: String,
    pub excerpt: String,
    /// Share of this passage's phrasing that also appears in the source.
    pub score: f32,
}

/// Measurable properties of prose. Explicitly not a verdict about authorship —
/// a real AI detector needs model weights this build does not ship.
// `Deserialize` too: a style profile stores these numbers in a file the author
// can read, and reads them back on the next launch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingSignals {
    pub word_count: i64,
    pub sentence_count: i64,
    pub mean_sentence_length: f32,
    pub sentence_length_variance: f32,
    pub type_token_ratio: f32,
    pub abstract_density: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RubricScore {
    pub criterion: String,
    pub score: i64,
    pub note: String,
}


