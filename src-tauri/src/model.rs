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
}

impl DocKind {
    pub fn as_str(self) -> &'static str {
        match self {
            DocKind::Folder => "folder",
            DocKind::Scene => "scene",
            DocKind::Sheet => "sheet",
            DocKind::Lore => "lore",
            DocKind::Research => "research",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "folder" => DocKind::Folder,
            "sheet" => DocKind::Sheet,
            "lore" => DocKind::Lore,
            "research" => DocKind::Research,
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveReceipt {
    pub id: String,
    pub word_count: i64,
    pub updated_at: i64,
}
