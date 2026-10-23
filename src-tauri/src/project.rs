//! Project folders on disk. A project is a directory, not a binary file:
//! `MyBook.aegis/project.sqlite` plus Markdown under `manuscript/` and
//! `research/`. If this app disappears, the prose is still readable.

use crate::error::{err, AppResult};
use crate::bookmarks;
use crate::model::{DocKind, DocumentRow, ProjectSummary, RecentProject, SaveReceipt};
use crate::store;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROJECT_EXT: &str = "aegis";
const SUBDIRS: [&str; 7] = [
    "manuscript/scenes",
    "manuscript/characters",
    "manuscript/locations",
    "manuscript/worldbuilding",
    "manuscript/canon",
    "research",
    "assets",
];

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn count_words(text: &str) -> i64 {
    text.split_whitespace().filter(|w| w.chars().any(char::is_alphanumeric)).count() as i64
}

/// Lowercase, hyphenated, ASCII-only. Used for file names, never for identity.
pub fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "untitled".to_string()
    } else {
        trimmed.chars().take(60).collect()
    }
}

fn validate_root(root: &Path) -> AppResult<()> {
    if !root.is_dir() {
        return err(format!("{} is not a folder", root.display()));
    }
    if !root.join("project.sqlite").exists() {
        return err(format!(
            "{} does not look like an AegisDraft project (no project.sqlite)",
            root.display()
        ));
    }
    Ok(())
}

fn project_name(root: &Path) -> String {
    root.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

fn summarise(root: &Path) -> AppResult<ProjectSummary> {
    let conn = store::open(root)?;
    let (word_count, document_count) = store::totals(&conn)?;
    let kind = store::get_meta(&conn, "project_kind")?.unwrap_or_else(|| "novel".to_string());
    Ok(ProjectSummary {
        path: root.to_string_lossy().to_string(),
        name: project_name(root),
        kind,
        word_count,
        document_count,
        opened_at: now_ms(),
    })
}

/// Creates `<parent>/<name>.aegis`, seeds the folder tree, the database and one
/// empty opening scene so the binder is never empty on first run.
pub fn create(parent: &Path, name: &str, kind: &str) -> AppResult<ProjectSummary> {
    create_from_template(parent, name, kind, kind)
}

/// A starter folder tree. A template only makes folders and empty scenes — it
/// never writes prose, because a project that starts with words the author did
/// not write is a project they have to delete before they can begin.
///
/// Each entry is (title, children). A child with `true` is an empty scene.
fn template_tree(template: &str) -> Vec<(&'static str, Vec<(&'static str, bool)>)> {
    match template {
        "nonfiction" => vec![
            ("Manuscript", vec![("Introduction", true)]),
            ("Sources", vec![]),
            ("Interviews", vec![]),
            ("Figures", vec![]),
            ("Back matter", vec![]),
        ],
        "academic" => vec![
            (
                "Paper",
                vec![
                    ("Abstract", true),
                    ("1. Introduction", true),
                    ("2. Related work", true),
                    ("3. Method", true),
                    ("4. Results", true),
                    ("5. Discussion", true),
                ],
            ),
            ("References", vec![]),
            ("Figures", vec![]),
            ("Appendices", vec![]),
        ],
        "techbook" => vec![
            ("Manuscript", vec![("Preface", true)]),
            ("Listings", vec![]),
            ("Figures", vec![]),
            ("Glossary", vec![]),
            ("Research", vec![]),
        ],
        // The novel tree, and the fallback for an unknown template name — an
        // unrecognised template gives the default project, never an empty one.
        _ => vec![
            ("Manuscript", vec![("Opening scene", true)]),
            ("Characters", vec![]),
            ("Locations", vec![]),
            ("Worldbuilding", vec![]),
            ("Canon", vec![]),
            ("Research", vec![]),
        ],
    }
}

pub fn templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("novel", "Manuscript with an opening scene, plus Characters, Locations, Worldbuilding, Canon and Research."),
        ("nonfiction", "Manuscript, Sources, Interviews, Figures and Back matter."),
        ("academic", "IMRaD sections, References, Figures and Appendices."),
        ("techbook", "Manuscript, Listings, Figures, Glossary and Research."),
    ]
}

pub fn create_from_template(
    parent: &Path,
    name: &str,
    kind: &str,
    template: &str,
) -> AppResult<ProjectSummary> {
    if !parent.is_dir() {
        return err(format!("{} is not a folder", parent.display()));
    }
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return err("A project needs a name");
    }
    let root = parent.join(format!("{}.{PROJECT_EXT}", trimmed));
    if root.exists() {
        return err(format!("{} already exists", root.display()));
    }

    fs::create_dir_all(&root)?;
    for dir in SUBDIRS {
        fs::create_dir_all(root.join(dir))?;
    }

    let conn = store::open(&root)?;
    let now = now_ms();
    store::set_meta(&conn, "project_kind", kind)?;
    store::set_meta(&conn, "created_at", &now.to_string())?;

    store::set_meta(&conn, "template", template)?;

    let mut sequence = 0;
    for (pos, (folder, children)) in template_tree(template).into_iter().enumerate() {
        let folder_id = new_id();
        store::insert_document(
            &conn,
            &folder_id,
            None,
            folder,
            DocKind::Folder,
            None,
            pos as i64,
            now,
        )?;
        for (child_pos, (title, is_scene)) in children.into_iter().enumerate() {
            if !is_scene {
                continue;
            }
            sequence += 1;
            let rel = format!("{}/{sequence:04}-{}.md", DocKind::Scene.dir(), slugify(title));
            fs::write(root.join(&rel), "")?;
            store::insert_document(
                &conn,
                &new_id(),
                Some(&folder_id),
                title,
                DocKind::Scene,
                Some(&rel),
                child_pos as i64,
                now,
            )?;
        }
    }

    summarise(&root)
}

pub fn open(root: &Path) -> AppResult<ProjectSummary> {
    validate_root(root)?;
    summarise(root)
}

pub fn documents(root: &Path) -> AppResult<Vec<DocumentRow>> {
    validate_root(root)?;
    let conn = store::open(root)?;
    store::list_documents(&conn)
}

pub fn read_body(root: &Path, id: &str) -> AppResult<String> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let doc = match store::get_document(&conn, id)? {
        Some(d) => d,
        None => return err(format!("no document {id}")),
    };
    let Some(rel) = doc.path else {
        return Ok(String::new()); // folders have no body
    };
    let file = root.join(&rel);
    if !file.exists() {
        // The row outlived its file. Report empty rather than failing the open;
        // the next save re-creates the file.
        return Ok(String::new());
    }
    Ok(fs::read_to_string(file)?)
}

/// Writes the Markdown body, then records the new word count. The file is
/// written first and atomically, so a crash mid-save can never leave the prose
/// shorter than what the database claims.
pub fn write_body(root: &Path, id: &str, body: &str) -> AppResult<SaveReceipt> {
    write_body_as(root, id, body, crate::provenance::Actor::human())
}

/// Save a document, recording **who** wrote it.
///
/// The actor is not optional and has no default. An agent's edit recorded as
/// `Actor::human()` would launder AI writing as the author's own work, which is
/// the worst thing this ledger could do — people would trust it. Every caller
/// therefore says who is writing, and `write_body` is the one that says "a
/// person, at the keyboard".
pub fn write_body_as(
    root: &Path,
    id: &str,
    body: &str,
    actor: crate::provenance::Actor,
) -> AppResult<SaveReceipt> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let doc = match store::get_document(&conn, id)? {
        Some(d) => d,
        None => return err(format!("no document {id}")),
    };
    let Some(rel) = doc.path else {
        return err("folders have no body to save");
    };

    let file = root.join(&rel);
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = file.with_extension("md.tmp");
    fs::write(&tmp, body)?;
    fs::rename(&tmp, &file)?;

    let words = count_words(body);
    let now = now_ms();
    store::touch_body(&conn, id, words, now)?;
    // Observed: this write went through the app, so we know what changed and
    // who changed it. Reconciliation never fails a save — losing the prose
    // because the ledger stumbled would be the wrong trade.
    let _ = crate::blocks::reconcile(
        &conn,
        id,
        body,
        actor,
        crate::provenance::Confidence::Observed,
        now,
    );
    Ok(SaveReceipt { id: id.to_string(), word_count: words, updated_at: now })
}

/// Insert a model's proposal into the manuscript, recording that it did.
///
/// This closes the one real hole in the ledger. The Assist panel deliberately
/// never inserted generated prose — the author copied it across by hand, which
/// is good for trust and means **AI-written text entered the manuscript with no
/// record whatsoever**. The agent loop was traced; the writing loop was not.
///
/// The two reconciles are not redundant. The first captures anything the author
/// has typed since the last save as *their* work; only then is the proposal
/// appended and reconciled as the model's. Doing it in one pass would attribute
/// the author's own pending edits to the model.
///
/// The resulting chain is `create(ai_model)` then `accept(human)`, which
/// classifies as "AI-generated text with human review" — the truth, and the
/// thing an author can point at later.
pub fn accept_proposal(
    root: &Path,
    id: &str,
    text: &str,
    model: &str,
    params: serde_json::Value,
) -> AppResult<crate::blocks::Block> {
    validate_root(root)?;
    if text.trim().is_empty() {
        return err("there is nothing to accept");
    }
    let conn = store::open(root)?;
    let doc = match store::get_document(&conn, id)? {
        Some(d) => d,
        None => return err(format!("no document {id}")),
    };
    let Some(rel) = doc.path.clone() else {
        return err("folders have no body to add to");
    };

    let existing = fs::read_to_string(root.join(&rel)).unwrap_or_default();
    let now = now_ms();

    // Everything already there is the author's, whatever happens next.
    crate::blocks::reconcile(
        &conn,
        id,
        &existing,
        crate::provenance::Actor::human(),
        crate::provenance::Confidence::Observed,
        now,
    )?;
    let before: std::collections::HashSet<String> =
        crate::blocks::list(&conn, id)?.into_iter().map(|b| b.id).collect();

    let joined = if existing.trim().is_empty() {
        text.trim().to_string()
    } else {
        format!("{}\n\n{}", existing.trim_end(), text.trim())
    };
    write_body_as(
        root,
        id,
        &joined,
        crate::provenance::Actor::model(model, params.clone()),
    )?;

    // The block the model just wrote, and the author's acceptance of it.
    let conn = store::open(root)?;
    let added = crate::blocks::list(&conn, id)?
        .into_iter()
        .find(|b| !before.contains(&b.id))
        .ok_or_else(|| {
            crate::error::AppError(
                "the proposal produced no new paragraph — it may be identical to one already \
                 in the document"
                    .into(),
            )
        })?;

    crate::provenance::append(
        &conn,
        crate::provenance::NewEvent::block(
            &added.id,
            crate::provenance::Actor::human(),
            crate::provenance::Action::Accept,
            crate::provenance::Confidence::Observed,
        )
        .hashes(None, Some(added.content_hash.clone()))
        .note(&format!("The author took a proposal from {model} into the manuscript.")),
        now_ms(),
    )?;

    Ok(added)
}

/// Bring every document's blocks up to date with what is actually on disk.
///
/// Called on open. Anything that differs was changed outside AegisDraft, so it
/// is recorded as `External` and `Inferred`: we did not watch it happen and are
/// reconstructing it by matching text. A file whose only change was a reflow
/// produces no event at all, because normalisation sees no difference.
pub fn reconcile_all(root: &Path) -> AppResult<usize> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let now = now_ms();
    let mut touched = 0;
    for doc in store::list_documents(&conn)? {
        let Some(rel) = doc.path.as_deref() else { continue };
        let Ok(body) = fs::read_to_string(root.join(rel)) else { continue };
        let before: Option<String> = conn
            .query_row("SELECT body_hash FROM documents WHERE id = ?1", [&doc.id], |r| r.get(0))
            .unwrap_or(None);
        if before.as_deref() == Some(crate::blocks::body_hash(&body).as_str()) {
            continue;
        }
        crate::blocks::reconcile(
            &conn,
            &doc.id,
            &body,
            crate::provenance::Actor::external(),
            crate::provenance::Confidence::Inferred,
            now,
        )?;
        touched += 1;
    }
    Ok(touched)
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn create_document(
    root: &Path,
    parent_id: Option<&str>,
    title: &str,
    kind: DocKind,
) -> AppResult<DocumentRow> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let now = now_ms();
    let position = store::next_position(&conn, parent_id)?;
    let id = new_id();

    let path = if kind == DocKind::Folder {
        None
    } else {
        let rel = unique_path(root, kind, title)?;
        fs::create_dir_all(root.join(kind.dir()))?;
        fs::write(root.join(&rel), "")?;
        Some(rel)
    };

    store::insert_document(&conn, &id, parent_id, title, kind, path.as_deref(), position, now)?;
    match store::get_document(&conn, &id)? {
        Some(d) => Ok(d),
        None => err("document vanished immediately after being created"),
    }
}

/// `manuscript/scenes/0007-nightfall-at-the-rim.md`.
///
/// The counter is the next free sequence number in that directory, not just a
/// collision breaker — it is what makes the folder read in creation order when
/// someone opens it in Finder or a text editor, which is the whole point of
/// keeping the prose as files.
fn unique_path(root: &Path, kind: DocKind, title: &str) -> AppResult<String> {
    let dir = kind.dir();
    let slug = slugify(title);
    let dir_path = root.join(dir);

    let existing = fs::read_dir(&dir_path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|x| x == "md"))
                .count()
        })
        .unwrap_or(0);

    for seq in (existing + 1..).take(10_000) {
        let rel = format!("{dir}/{seq:04}-{slug}.md");
        if !root.join(&rel).exists() {
            return Ok(rel);
        }
    }
    err("could not find a free file name")
}

pub fn rename_document(root: &Path, id: &str, title: &str) -> AppResult<DocumentRow> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let doc = match store::get_document(&conn, id)? {
        Some(d) => d,
        None => return err(format!("no document {id}")),
    };
    let now = now_ms();

    // Keep the file name in step with the title so the folder stays readable.
    // If the rename fails for any reason, the old path is kept — the title is
    // what the author sees, and losing the file would be worse.
    let mut new_rel: Option<String> = None;
    if let Some(old_rel) = doc.path.as_deref() {
        if let Ok(candidate) = unique_path(root, doc.kind, title) {
            if fs::rename(root.join(old_rel), root.join(&candidate)).is_ok() {
                new_rel = Some(candidate);
            }
        }
    }

    store::rename(&conn, id, title, new_rel.as_deref(), now)?;
    match store::get_document(&conn, id)? {
        Some(d) => Ok(d),
        None => err("document vanished during rename"),
    }
}

pub fn update_meta(
    root: &Path,
    id: &str,
    synopsis: Option<&str>,
    label: Option<&str>,
    status: Option<&str>,
) -> AppResult<DocumentRow> {
    validate_root(root)?;
    let conn = store::open(root)?;
    store::update_meta_fields(&conn, id, synopsis, label, status, now_ms())?;
    match store::get_document(&conn, id)? {
        Some(d) => Ok(d),
        None => err(format!("no document {id}")),
    }
}

/// Removes the row and moves its files to `trash/` rather than unlinking them.
/// Phase 1 has no snapshots yet, so deletion must not be final — and deleting a
/// folder must not orphan the files of everything inside it.
pub fn delete_document(root: &Path, id: &str) -> AppResult<()> {
    validate_root(root)?;
    let conn = store::open(root)?;
    let doomed = store::subtree(&conn, id)?;

    let trash = root.join("trash");
    let stamp = now_ms();
    for doc in &doomed {
        let Some(rel) = doc.path.as_deref() else { continue };
        let src = root.join(rel);
        if !src.exists() {
            continue;
        }
        fs::create_dir_all(&trash)?;
        let name = Path::new(rel).file_name().unwrap_or_default();
        let dest = trash.join(format!("{}-{}", stamp, name.to_string_lossy()));
        let _ = fs::rename(&src, &dest);
    }

    // The SQL cascade removes the descendant rows.
    store::delete(&conn, id)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Recent projects — a small JSON file in the app config directory.
// ---------------------------------------------------------------------------

pub fn read_recents(config_dir: &Path) -> Vec<RecentProject> {
    let file = config_dir.join("recent-projects.json");
    let Ok(raw) = fs::read_to_string(file) else {
        return Vec::new();
    };
    let mut list: Vec<RecentProject> = serde_json::from_str(&raw).unwrap_or_default();
    for item in &mut list {
        // The bookmark outranks the stored path: it follows a folder that was
        // renamed or moved, and under the App Sandbox it is the only thing that
        // grants access at all. The path stays as the fallback, which is what
        // every unsandboxed build has always used.
        if let Some(found) = item.bookmark.as_deref().and_then(bookmarks::resolve) {
            if let Some(text) = found.to_str() {
                item.path = text.to_string();
            }
        }
        item.exists = Path::new(&item.path).join("project.sqlite").exists();
    }
    list
}

/// The folder to open for `path`, with sandbox access claimed if a bookmark for
/// it exists. Called before `open` so a signed build can actually read the
/// folder; off the sandbox `hold` returns `None` and the path is used unchanged.
pub fn claim(config_dir: &Path, path: &Path) -> PathBuf {
    let wanted = path.to_string_lossy();
    read_recents(config_dir)
        .into_iter()
        .find(|r| r.path == wanted)
        .and_then(|r| r.bookmark)
        .and_then(|token| bookmarks::hold(&token))
        .unwrap_or_else(|| path.to_path_buf())
}

pub fn remember(config_dir: &Path, summary: &ProjectSummary) -> AppResult<Vec<RecentProject>> {
    fs::create_dir_all(config_dir)?;
    let mut list = read_recents(config_dir);
    list.retain(|r| r.path != summary.path);
    list.insert(
        0,
        RecentProject {
            path: summary.path.clone(),
            name: summary.name.clone(),
            kind: summary.kind.clone(),
            word_count: summary.word_count,
            opened_at: now_ms(),
            exists: true,
            bookmark: bookmarks::create(Path::new(&summary.path)),
        },
    );
    list.truncate(12);
    fs::write(
        config_dir.join("recent-projects.json"),
        serde_json::to_string_pretty(&list)?,
    )?;
    Ok(list)
}

// ---------------------------------------------------------------------------
// Phase 2: structure, sheets, canon, keywords, links
// ---------------------------------------------------------------------------

pub fn move_document(root: &Path, id: &str, new_parent: Option<&str>, index: i64) -> AppResult<()> {
    validate_root(root)?;
    let mut conn = store::open(root)?;
    store::move_document(&mut conn, id, new_parent, index, now_ms())
}

pub fn set_keywords(root: &Path, id: &str, names: &[String]) -> AppResult<DocumentRow> {
    validate_root(root)?;
    let mut conn = store::open(root)?;
    store::set_keywords(&mut conn, id, names, now_ms())?;
    match store::get_document(&conn, id)? {
        Some(d) => Ok(d),
        None => err(format!("no document {id}")),
    }
}

pub fn project_keywords(root: &Path) -> AppResult<Vec<(String, i64)>> {
    validate_root(root)?;
    store::project_keywords(&store::open(root)?)
}

#[allow(clippy::too_many_arguments)]
pub fn update_extended(
    root: &Path,
    id: &str,
    notes: Option<&str>,
    target: Option<i64>,
    pov: Option<&str>,
    plants: Option<&str>,
    payoffs: Option<&str>,
) -> AppResult<DocumentRow> {
    validate_root(root)?;
    let conn = store::open(root)?;
    store::update_extended(&conn, id, notes, target, pov, plants, payoffs, now_ms())?;
    match store::get_document(&conn, id)? {
        Some(d) => Ok(d),
        None => err(format!("no document {id}")),
    }
}

pub fn bulk_set(
    root: &Path,
    ids: &[String],
    status: Option<&str>,
    label: Option<&str>,
    section_type: Option<&str>,
    pov: Option<&str>,
) -> AppResult<usize> {
    validate_root(root)?;
    let mut conn = store::open(root)?;
    store::bulk_set(&mut conn, ids, status, label, section_type, pov, now_ms())
}

pub fn link(root: &Path, from_id: &str, to_id: &str, relation: &str) -> AppResult<()> {
    validate_root(root)?;
    store::link(&store::open(root)?, from_id, to_id, relation)
}

pub fn unlink(root: &Path, from_id: &str, to_id: &str, relation: &str) -> AppResult<()> {
    validate_root(root)?;
    store::unlink(&store::open(root)?, from_id, to_id, relation)
}

/// Starter body for a sheet. A sheet with headings the author will actually
/// fill in beats an empty file they have to invent a shape for.
fn sheet_template(kind: DocKind, name: &str) -> String {
    match kind {
        DocKind::Sheet => format!(
            "# {name}

## Role

## Wants

## Refuses to do

## Continuity facts

- 

## First appears

"
        ),
        DocKind::Location => format!(
            "# {name}

## What it looks like

## Who controls it

## Continuity facts

- 

## Scenes here

"
        ),
        DocKind::Canon => format!(
            "# {name}

<!-- One falsifiable fact per entry, each with where it was established. -->

"
        ),
        _ => String::new(),
    }
}

/// Creates a character or location sheet, seeded with its template and
/// optionally linked to the scene it was created from.
pub fn create_sheet(
    root: &Path,
    name: &str,
    kind: DocKind,
    parent_id: Option<&str>,
    link_to: Option<&str>,
) -> AppResult<DocumentRow> {
    create_sheet_from(root, name, kind, parent_id, link_to, None)
}

/// Creates a sheet from a named template.
///
/// `template_id` picks one of the author's own shapes under `.aegis/templates/`;
/// omitting it uses the shipped shape for the kind. The template decides the
/// kind when one is given, because a template that declares itself a location
/// making a character sheet would be a surprise nobody wants.
pub fn create_sheet_from(
    root: &Path,
    name: &str,
    kind: DocKind,
    parent_id: Option<&str>,
    link_to: Option<&str>,
    template_id: Option<&str>,
) -> AppResult<DocumentRow> {
    // With no template named, fall back to the one for this kind — which is
    // the author's own if they have overridden it. Without this an override of
    // `character` would only apply when explicitly asked for, and the default
    // "New Character Sheet" would keep using the shipped shape, which defeats
    // the point of letting anyone customise it.
    let default_id = match kind {
        DocKind::Location => "location",
        _ => "character",
    };
    // A named template that does not exist falls back to the default for this
    // kind — which is the author's own when they have overridden it. Falling
    // back to the shipped shape instead would quietly ignore their template
    // because of a typo in a name.
    let template = template_id
        .and_then(|id| crate::templates::get(root, id))
        .or_else(|| crate::templates::get(root, default_id));
    // A template names its own kind; an explicitly chosen one therefore decides.
    let kind = match (template_id, &template) {
        (Some(_), Some(t)) => t.kind,
        _ => kind,
    };
    if !matches!(kind, DocKind::Sheet | DocKind::Location) {
        return err("that is not a sheet type");
    }
    // Default into the folder the kind belongs in, so sheets do not scatter.
    let parent = match parent_id {
        Some(p) => Some(p.to_string()),
        None => {
            let conn = store::open(root)?;
            let want = if kind == DocKind::Sheet { "Characters" } else { "Locations" };
            store::list_documents(&conn)?
                .into_iter()
                .find(|d| d.kind == DocKind::Folder && d.title == want)
                .map(|d| d.id)
        }
    };

    let doc = create_document(root, parent.as_deref(), name, kind)?;
    let body = match &template {
        Some(t) => crate::templates::render(&t.body, name, kind, &today()),
        None => sheet_template(kind, name),
    };
    write_body(root, &doc.id, &body)?;
    if let Some(scene) = link_to {
        link(root, &doc.id, scene, "appears_in")?;
    }
    match store::get_document(&store::open(root)?, &doc.id)? {
        Some(d) => Ok(d),
        None => err("sheet vanished after being created"),
    }
}

/// Appends one falsifiable fact to a canon document, with the source that
/// establishes it. A canon entry without a source is just an opinion.
pub fn append_canon_entry(root: &Path, id: &str, fact: &str, source: &str) -> AppResult<SaveReceipt> {
    validate_root(root)?;
    let fact = fact.trim();
    if fact.is_empty() {
        return err("a canon entry needs a fact");
    }
    let source = source.trim();
    if source.is_empty() {
        return err("a canon entry needs the document that establishes it");
    }
    let existing = read_body(root, id)?;
    let mut next = existing.trim_end().to_string();
    if !next.is_empty() {
        next.push_str("\n\n");
    }
    next.push_str(&format!("- {fact}  \n  *established in:* {source}\n"));
    write_body(root, id, &next)
}

/// Today, as `YYYY-MM-DD`, for the `{{date}}` placeholder.
///
/// Computed from the epoch rather than pulling in a date crate for one string:
/// the civil-from-days algorithm is short, exact, and has no dependency.
fn today() -> String {
    let days = now_ms() / 86_400_000;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

pub fn to_path(raw: &str) -> PathBuf {
    PathBuf::from(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_is_filename_safe() {
        assert_eq!(slugify("4. Nightfall at the Rim"), "4-nightfall-at-the-rim");
        assert_eq!(slugify("  ---  "), "untitled");
        assert_eq!(slugify("Salt & Debt"), "salt-debt");
    }

    #[test]
    fn word_count_ignores_punctuation_runs() {
        assert_eq!(count_words("one two three"), 3);
        assert_eq!(count_words("one — two"), 2);
        assert_eq!(count_words("   "), 0);
    }

    #[test]
    fn scene_files_are_numbered_in_creation_order() {
        let tmp = std::env::temp_dir().join(format!("aegis-seq-{}", now_ms()));
        fs::create_dir_all(&tmp).unwrap();
        let summary = create(&tmp, "Sequence Test", "novel").unwrap();
        let root = PathBuf::from(&summary.path);

        // create() already seeded 0001-opening-scene.md.
        let a = create_document(&root, None, "Second scene", DocKind::Scene).unwrap();
        let b = create_document(&root, None, "Third scene", DocKind::Scene).unwrap();

        assert_eq!(a.path.as_deref(), Some("manuscript/scenes/0002-second-scene.md"));
        assert_eq!(b.path.as_deref(), Some("manuscript/scenes/0003-third-scene.md"));

        fs::remove_dir_all(&tmp).ok();
    }

    /// A module can define a migration and simply never be called from
    /// `store::open`. That failure is invisible until a feature is used for the
    /// first time, so it is asserted here instead.
    #[test]
    fn a_new_project_has_every_table_each_feature_depends_on() {
        let tmp = std::env::temp_dir().join(format!("aegis-tables-{}", now_ms()));
        fs::create_dir_all(&tmp).unwrap();
        let summary = create(&tmp, "Tables", "novel").unwrap();
        let root = PathBuf::from(&summary.path);

        let conn = store::open(&root).unwrap();
        let present = store::table_names(&conn).unwrap();
        for want in store::EXPECTED_TABLES {
            assert!(
                present.iter().any(|t| t == want),
                "table `{want}` is missing — its migration is probably not called from store::open. Present: {present:?}"
            );
        }
        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn moving_documents_reorders_nests_and_refuses_cycles() {
        let tmp = std::env::temp_dir().join(format!("aegis-move-{}", now_ms()));
        fs::create_dir_all(&tmp).unwrap();
        let summary = create(&tmp, "Move Test", "novel").unwrap();
        let root = PathBuf::from(&summary.path);

        let act1 = create_document(&root, None, "Act I", DocKind::Folder).unwrap();
        let act2 = create_document(&root, None, "Act II", DocKind::Folder).unwrap();
        let a = create_document(&root, Some(&act1.id), "A", DocKind::Scene).unwrap();
        let b = create_document(&root, Some(&act1.id), "B", DocKind::Scene).unwrap();
        let c = create_document(&root, Some(&act1.id), "C", DocKind::Scene).unwrap();

        let order = |root: &Path, parent: &str| -> Vec<String> {
            let mut kids: Vec<_> = documents(root)
                .unwrap()
                .into_iter()
                .filter(|d| d.parent_id.as_deref() == Some(parent))
                .collect();
            kids.sort_by_key(|d| d.position);
            kids.into_iter().map(|d| d.title).collect()
        };

        assert_eq!(order(&root, &act1.id), vec!["A", "B", "C"]);

        // Reorder within the same folder: C to the front.
        move_document(&root, &c.id, Some(&act1.id), 0).unwrap();
        assert_eq!(order(&root, &act1.id), vec!["C", "A", "B"]);

        // Move across folders, and the source list closes its gap.
        move_document(&root, &a.id, Some(&act2.id), 0).unwrap();
        assert_eq!(order(&root, &act1.id), vec!["C", "B"]);
        assert_eq!(order(&root, &act2.id), vec!["A"]);

        // A folder may not be dropped inside itself or its own child.
        let inner = create_document(&root, Some(&act1.id), "Inner", DocKind::Folder).unwrap();
        assert!(move_document(&root, &act1.id, Some(&inner.id), 0).is_err());
        assert!(move_document(&root, &act1.id, Some(&act1.id), 0).is_err());
        // ...and documents cannot be nested inside a non-folder.
        assert!(move_document(&root, &b.id, Some(&c.id), 0).is_err());

        // The refused moves changed nothing.
        assert_eq!(order(&root, &act1.id), vec!["C", "B", "Inner"]);

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn deleting_a_folder_trashes_the_files_inside_it() {
        let tmp = std::env::temp_dir().join(format!("aegis-del-{}", now_ms()));
        fs::create_dir_all(&tmp).unwrap();
        let summary = create(&tmp, "Delete Test", "novel").unwrap();
        let root = PathBuf::from(&summary.path);

        let folder = create_document(&root, None, "Act I", DocKind::Folder).unwrap();
        let child = create_document(&root, Some(&folder.id), "A scene", DocKind::Scene).unwrap();
        write_body(&root, &child.id, "words on a page").unwrap();

        let child_file = root.join(child.path.as_ref().unwrap());
        assert!(child_file.exists());

        delete_document(&root, &folder.id).unwrap();

        // Row gone, file moved rather than destroyed.
        assert!(documents(&root).unwrap().iter().all(|d| d.id != child.id));
        assert!(!child_file.exists(), "the child file should have left its original path");
        let trashed: Vec<_> = fs::read_dir(root.join("trash")).unwrap().filter_map(Result::ok).collect();
        assert_eq!(trashed.len(), 1, "the child file should be in trash");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn create_open_write_read_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("aegis-test-{}", now_ms()));
        fs::create_dir_all(&tmp).unwrap();

        let summary = create(&tmp, "Test Book", "novel").unwrap();
        assert_eq!(summary.name, "Test Book");

        let root = PathBuf::from(&summary.path);
        let docs = documents(&root).unwrap();
        assert!(
            docs.iter().any(|d| d.title == "Canon" && d.kind == DocKind::Folder),
            "new projects seed a Canon binder folder"
        );

        let scene = docs.iter().find(|d| d.kind == DocKind::Scene).unwrap();

        let receipt = write_body(&root, &scene.id, "The rim of the basin held the heat.").unwrap();
        assert_eq!(receipt.word_count, 8);
        assert_eq!(read_body(&root, &scene.id).unwrap(), "The rim of the basin held the heat.");

        // The prose must be readable without going through the database.
        let on_disk = fs::read_to_string(root.join(scene.path.as_ref().unwrap())).unwrap();
        assert_eq!(on_disk, "The rim of the basin held the heat.");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn a_remembered_project_stores_a_bookmark_and_reopens_by_it() {
        let tmp = std::env::temp_dir().join(format!("aegis-recents-{}", std::process::id()));
        fs::remove_dir_all(&tmp).ok();
        let config = tmp.join("config");
        let parent = tmp.join("projects");
        fs::create_dir_all(&parent).unwrap();
        let summary = create(&parent, "Bookmarked", "novel").unwrap();

        let list = remember(&config, &summary).unwrap();
        assert_eq!(list.len(), 1);
        if cfg!(target_os = "macos") {
            assert!(
                list[0].bookmark.is_some(),
                "macOS should always manage at least a plain bookmark"
            );
        }

        // Read back through the same path a fresh launch would take.
        let back = read_recents(&config);
        assert_eq!(back.len(), 1);
        assert!(back[0].exists, "the project it just made must be findable");
        assert_eq!(
            fs::canonicalize(&back[0].path).unwrap(),
            fs::canonicalize(&summary.path).unwrap()
        );

        // `claim` hands back a real folder whether or not a bookmark resolved.
        let claimed = claim(&config, Path::new(&back[0].path));
        assert!(claimed.join("project.sqlite").exists());

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn a_recents_file_written_before_bookmarks_existed_still_loads() {
        let tmp = std::env::temp_dir().join(format!("aegis-legacy-{}", std::process::id()));
        fs::remove_dir_all(&tmp).ok();
        fs::create_dir_all(&tmp).unwrap();
        // Exactly the shape the shipped app wrote before this change.
        fs::write(
            tmp.join("recent-projects.json"),
            r#"[{"path":"/nowhere/gone","name":"Old","kind":"novel","wordCount":12,"openedAt":1,"exists":true}]"#,
        )
        .unwrap();

        let list = read_recents(&tmp);
        assert_eq!(list.len(), 1, "an old list must not be silently dropped");
        assert_eq!(list[0].name, "Old");
        assert_eq!(list[0].bookmark, None);
        assert!(!list[0].exists, "the folder is gone, and the list must say so");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn a_template_seeds_its_own_tree_and_an_unknown_one_falls_back() {
        let base = std::env::temp_dir().join(format!("aegis-template-{}", std::process::id()));
        fs::remove_dir_all(&base).ok();
        fs::create_dir_all(&base).unwrap();

        let paper = create_from_template(&base, "Paper", "academic", "academic").unwrap();
        let docs = documents(Path::new(&paper.path)).unwrap();
        let titles: Vec<&str> = docs.iter().map(|d| d.title.as_str()).collect();
        assert!(titles.contains(&"References"));
        assert!(titles.contains(&"3. Method"));
        assert!(!titles.contains(&"Canon"), "a paper has no canon folder");

        // Every seeded scene is a real, empty file — a template writes no prose.
        for doc in docs.iter().filter(|d| d.kind != DocKind::Folder) {
            let rel = doc.path.clone().expect("a scene has a file");
            let body = fs::read_to_string(Path::new(&paper.path).join(&rel)).unwrap();
            assert!(body.is_empty(), "{rel} was seeded with words the author did not write");
        }
        // And they do not all collide on one filename.
        let paths: std::collections::HashSet<_> =
            docs.iter().filter_map(|d| d.path.clone()).collect();
        let scenes = docs.iter().filter(|d| d.kind != DocKind::Folder).count();
        assert_eq!(paths.len(), scenes, "seeded scenes share a filename");

        // An unknown template gives the default project, never an empty one.
        let odd = create_from_template(&base, "Odd", "novel", "no-such-template").unwrap();
        let fallback = documents(Path::new(&odd.path)).unwrap();
        assert!(fallback.iter().any(|d| d.title == "Manuscript"));
        assert!(fallback.iter().any(|d| d.title == "Canon"));

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn goals_persist_and_a_session_counts_from_where_it_started() {
        let base = std::env::temp_dir().join(format!("aegis-goals-{}", std::process::id()));
        fs::remove_dir_all(&base).ok();
        fs::create_dir_all(&base).unwrap();
        let summary = create(&base, "Targets", "novel").unwrap();
        let root = Path::new(&summary.path);

        // Nothing set is not the same as zero, and reads back as nothing.
        assert_eq!(goals(root).unwrap().project_target, None);
        assert_eq!(session_progress(root).unwrap(), None, "no session is not zero words");

        set_goals(
            root,
            &Goals { project_target: Some(90_000), session_target: Some(800), ..Default::default() },
        )
        .unwrap();
        assert_eq!(goals(root).unwrap().project_target, Some(90_000));

        // Words written before the session started must not count towards it.
        let doc = create_document(root, None, "Scene", DocKind::Scene).unwrap();
        write_body(root, &doc.id, "one two three four five").unwrap();
        start_session(root).unwrap();
        assert_eq!(session_progress(root).unwrap(), Some(0));

        write_body(root, &doc.id, "one two three four five six seven").unwrap();
        assert_eq!(session_progress(root).unwrap(), Some(2));

        // Starting a session must not lose the targets already set.
        let after = goals(root).unwrap();
        assert_eq!(after.session_target, Some(800));
        assert!(after.session_started_at.is_some());

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn an_unplaced_card_is_distinct_from_one_placed_at_the_origin() {
        let base = std::env::temp_dir().join(format!("aegis-freeform-{}", std::process::id()));
        fs::remove_dir_all(&base).ok();
        fs::create_dir_all(&base).unwrap();
        let summary = create(&base, "Board", "novel").unwrap();
        let root = Path::new(&summary.path);
        let doc = create_document(root, None, "Card", DocKind::Scene).unwrap();

        let placed = |root: &Path, id: &str| {
            documents(root).unwrap().into_iter().find(|d| d.id == id).map(|d| (d.x, d.y)).unwrap()
        };
        assert_eq!(placed(root, &doc.id), (None, None), "a new card is unplaced");

        set_position(root, &doc.id, Some(0.0), Some(0.0)).unwrap();
        assert_eq!(placed(root, &doc.id), (Some(0.0), Some(0.0)), "(0,0) is a real choice");

        set_position(root, &doc.id, Some(140.5), Some(-32.0)).unwrap();
        assert_eq!(placed(root, &doc.id), (Some(140.5), Some(-32.0)));

        set_position(root, &doc.id, None, None).unwrap();
        assert_eq!(placed(root, &doc.id), (None, None), "clearing returns it to the grid");

        // An icon override round-trips and clears back to the kind default.
        assert_eq!(set_icon(root, &doc.id, Some("compass")).unwrap().icon.as_deref(), Some("compass"));
        assert_eq!(set_icon(root, &doc.id, None).unwrap().icon, None);

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn a_sheet_uses_the_authors_own_template_when_there_is_one() {
        let base = std::env::temp_dir().join(format!("aegis-sheettpl-{}", std::process::id()));
        fs::remove_dir_all(&base).ok();
        fs::create_dir_all(&base).unwrap();
        let summary = create(&base, "Sheets", "novel").unwrap();
        let root = Path::new(&summary.path);

        // The shipped shape, first.
        let plain = create_sheet(root, "Tamsin", DocKind::Sheet, None, None).unwrap();
        assert!(read_body(root, &plain.id).unwrap().contains("Refuses to do"));

        // The author writes their own, and it takes over.
        crate::templates::save(
            root,
            "character",
            "# {{name}}\n\n## Sources\n\n## Handedness\n\nCreated {{date}}.\n",
        )
        .unwrap();
        let mine = create_sheet(root, "The Cartographer", DocKind::Sheet, None, None).unwrap();
        let body = read_body(root, &mine.id).unwrap();
        assert!(body.starts_with("# The Cartographer"), "{body}");
        assert!(body.contains("Handedness"));
        assert!(!body.contains("Refuses to do"), "the shipped shape is overridden");
        assert!(!body.contains("{{"), "every placeholder is filled: {body}");
        assert!(body.contains(&today()[..4]), "the date is real, not a placeholder: {body}");

        // A template that declares itself a location makes a location, whatever
        // the caller asked for — otherwise the shape and the kind disagree.
        crate::templates::save(
            root,
            "field-site",
            "<!-- kind: location -->\n# {{name}}\n\n## Coordinates\n",
        )
        .unwrap();
        let site =
            create_sheet_from(root, "Kharsa Basin", DocKind::Sheet, None, None, Some("field-site"))
                .unwrap();
        assert_eq!(site.kind, DocKind::Location);
        let site_body = read_body(root, &site.id).unwrap();
        assert!(site_body.contains("Coordinates"));
        assert!(!site_body.contains("kind:"), "the declaration is not content");

        // An unknown template name falls back rather than failing: a sheet the
        // author asked for should not be lost to a typo.
        let fallback =
            create_sheet_from(root, "Someone", DocKind::Sheet, None, None, Some("nope")).unwrap();
        assert!(read_body(root, &fallback.id).unwrap().contains("Handedness"));

        fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn the_date_placeholder_is_a_real_calendar_date() {
        let iso = today();
        let parts: Vec<i64> = iso.split('-').map(|p| p.parse().unwrap()).collect();
        assert_eq!(parts.len(), 3, "{iso}");
        assert!((2020..2200).contains(&parts[0]), "{iso}");
        assert!((1..=12).contains(&parts[1]), "{iso}");
        assert!((1..=31).contains(&parts[2]), "{iso}");
    }

}

// ---------------------------------------------------------------------------
// Phase 6: goals, freeform positions, icons
// ---------------------------------------------------------------------------

/// Writing targets. Stored in the project's `meta` table, so they travel with
/// the project rather than living in app preferences that a second machine
/// would not have.
///
/// `session_start_words` is what makes a session target meaningful: without a
/// mark at the start, "800 words today" can only be answered by guessing.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Goals {
    /// Words for the whole book. `None` means the author has not set one.
    pub project_target: Option<i64>,
    /// Words for one sitting.
    pub session_target: Option<i64>,
    /// The project's word count when the current session began.
    pub session_start_words: Option<i64>,
    /// When that session began, so a stale one can be noticed rather than
    /// silently counted as today's work.
    pub session_started_at: Option<i64>,
    /// Default target applied to a new scene, if any.
    pub default_document_target: Option<i64>,
}

const GOALS_KEY: &str = "goals";

pub fn goals(root: &Path) -> AppResult<Goals> {
    validate_root(root)?;
    let conn = store::open(root)?;
    Ok(store::get_meta(&conn, GOALS_KEY)?
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default())
}

pub fn set_goals(root: &Path, goals: &Goals) -> AppResult<Goals> {
    validate_root(root)?;
    let conn = store::open(root)?;
    store::set_meta(&conn, GOALS_KEY, &serde_json::to_string(goals)?)?;
    Ok(goals.clone())
}

/// Mark the start of a writing session at the current word count.
pub fn start_session(root: &Path) -> AppResult<Goals> {
    let mut g = goals(root)?;
    g.session_start_words = Some(summarise(root)?.word_count);
    g.session_started_at = Some(now_ms());
    set_goals(root, &g)
}

/// Words written since the session mark. `None` when no session has started —
/// which is different from zero, and shown differently.
pub fn session_progress(root: &Path) -> AppResult<Option<i64>> {
    let g = goals(root)?;
    let Some(start) = g.session_start_words else {
        return Ok(None);
    };
    Ok(Some((summarise(root)?.word_count - start).max(0)))
}

/// Place a card on the freeform corkboard, or unplace it with `None`.
pub fn set_position(root: &Path, id: &str, x: Option<f32>, y: Option<f32>) -> AppResult<()> {
    validate_root(root)?;
    let conn = store::open(root)?;
    store::set_position(&conn, id, x, y, now_ms())
}

/// Set a binder icon override, or clear it back to the kind's default.
pub fn set_icon(root: &Path, id: &str, icon: Option<&str>) -> AppResult<DocumentRow> {
    validate_root(root)?;
    let conn = store::open(root)?;
    store::set_icon(&conn, id, icon, now_ms())?;
    match store::get_document(&conn, id)? {
        Some(d) => Ok(d),
        None => err(format!("no document {id}")),
    }
}
