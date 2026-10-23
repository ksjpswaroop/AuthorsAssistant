//! AegisDraft — Tauri commands.
//!
//! The Rust core owns file I/O and SQLite (see docs/ARCHITECTURE.md). The
//! webview never touches the filesystem directly; it asks for these commands,
//! which keeps one place responsible for the manuscript.

mod error;
pub mod bookmarks;
pub mod blocks;
pub mod collections;
pub mod compile;
pub mod import;
pub mod integrity;
mod menu;
pub mod model;
pub mod ollama;
pub mod project;
pub mod provenance;
pub mod rag;
pub mod scrivenings;
pub mod sidecar;
pub mod snapshots;
pub mod store;
pub mod style;
pub mod templates;
pub mod websearch;

use error::{AppError, AppResult};
use model::{
    Collection, CollectionKind, CompileFilters, CompilePreset, DocKind, DocumentRow, ExportFormat,
    OllamaStatus, ProjectSummary, RecentProject, RetrievedChunk, RubricScore, SaveReceipt,
    SimilarityMatch, Snapshot, WritingSignals,
};
use tauri::Manager;

fn config_dir(app: &tauri::AppHandle) -> AppResult<std::path::PathBuf> {
    app.path()
        .app_config_dir()
        .map_err(|e| AppError(format!("no config directory: {e}")))
}

#[tauri::command]
fn create_project(
    app: tauri::AppHandle,
    parent: String,
    name: String,
    kind: String,
) -> AppResult<ProjectSummary> {
    let summary = project::create(&project::to_path(&parent), &name, &kind)?;
    let _ = project::remember(&config_dir(&app)?, &summary);
    Ok(summary)
}

#[tauri::command]
fn open_project(app: tauri::AppHandle, path: String) -> AppResult<ProjectSummary> {
    let asked = project::to_path(&path);
    // A recent project carries a security-scoped bookmark; claiming it is what
    // makes the folder readable under the sandbox, and it also finds the folder
    // again if it has since been moved.
    let root = match config_dir(&app) {
        Ok(dir) => project::claim(&dir, &asked),
        Err(_) => asked,
    };
    let summary = project::open(&root)?;
    // Anything edited outside AegisDraft is reconciled now, as External and
    // Inferred. A failure here must not stop a project opening.
    let _ = project::reconcile_all(&root);
    let _ = project::remember(&config_dir(&app)?, &summary);
    Ok(summary)
}

#[tauri::command]
fn list_documents(path: String) -> AppResult<Vec<DocumentRow>> {
    project::documents(&project::to_path(&path))
}

#[tauri::command]
fn read_document(path: String, id: String) -> AppResult<String> {
    project::read_body(&project::to_path(&path), &id)
}

#[tauri::command]
fn write_document(path: String, id: String, body: String) -> AppResult<SaveReceipt> {
    project::write_body(&project::to_path(&path), &id, &body)
}

#[tauri::command]
fn create_document(
    path: String,
    parent_id: Option<String>,
    title: String,
    kind: String,
) -> AppResult<DocumentRow> {
    project::create_document(
        &project::to_path(&path),
        parent_id.as_deref(),
        &title,
        DocKind::parse(&kind),
    )
}

#[tauri::command]
fn rename_document(path: String, id: String, title: String) -> AppResult<DocumentRow> {
    project::rename_document(&project::to_path(&path), &id, &title)
}

#[tauri::command]
fn update_document_meta(
    path: String,
    id: String,
    synopsis: Option<String>,
    label: Option<String>,
    status: Option<String>,
) -> AppResult<DocumentRow> {
    project::update_meta(
        &project::to_path(&path),
        &id,
        synopsis.as_deref(),
        label.as_deref(),
        status.as_deref(),
    )
}

#[tauri::command]
fn delete_document(path: String, id: String) -> AppResult<()> {
    project::delete_document(&project::to_path(&path), &id)
}

// --- Phase 2: structure -----------------------------------------------------

#[tauri::command]
fn move_document(path: String, id: String, new_parent: Option<String>, index: i64) -> AppResult<()> {
    project::move_document(&project::to_path(&path), &id, new_parent.as_deref(), index)
}

#[tauri::command]
fn create_sheet(
    path: String,
    name: String,
    kind: String,
    parent_id: Option<String>,
    link_to: Option<String>,
) -> AppResult<DocumentRow> {
    project::create_sheet(
        &project::to_path(&path),
        &name,
        DocKind::parse(&kind),
        parent_id.as_deref(),
        link_to.as_deref(),
    )
}

#[tauri::command]
fn append_canon_entry(path: String, id: String, fact: String, source: String) -> AppResult<SaveReceipt> {
    project::append_canon_entry(&project::to_path(&path), &id, &fact, &source)
}

// --- Phase 2: metadata ------------------------------------------------------

#[tauri::command]
fn update_document_extended(
    path: String,
    id: String,
    notes: Option<String>,
    target: Option<i64>,
    pov: Option<String>,
    plants: Option<String>,
    payoffs: Option<String>,
) -> AppResult<DocumentRow> {
    project::update_extended(
        &project::to_path(&path),
        &id,
        notes.as_deref(),
        target,
        pov.as_deref(),
        plants.as_deref(),
        payoffs.as_deref(),
    )
}

#[tauri::command]
fn set_keywords(path: String, id: String, keywords: Vec<String>) -> AppResult<DocumentRow> {
    project::set_keywords(&project::to_path(&path), &id, &keywords)
}

#[tauri::command]
fn project_keywords(path: String) -> AppResult<Vec<(String, i64)>> {
    project::project_keywords(&project::to_path(&path))
}

#[tauri::command]
fn link_documents(path: String, from_id: String, to_id: String, relation: String) -> AppResult<()> {
    project::link(&project::to_path(&path), &from_id, &to_id, &relation)
}

#[tauri::command]
fn unlink_documents(path: String, from_id: String, to_id: String, relation: String) -> AppResult<()> {
    project::unlink(&project::to_path(&path), &from_id, &to_id, &relation)
}

// --- Phase 2: snapshots -----------------------------------------------------

#[tauri::command]
fn snapshot_create(path: String, message: String) -> AppResult<Option<Snapshot>> {
    snapshots::create(&project::to_path(&path), &message)
}

#[tauri::command]
fn snapshot_list(path: String, doc_path: Option<String>, limit: Option<usize>) -> AppResult<Vec<Snapshot>> {
    snapshots::list(&project::to_path(&path), doc_path.as_deref(), limit.unwrap_or(50))
}

#[tauri::command]
fn snapshot_read_file(path: String, commit: String, doc_path: String) -> AppResult<String> {
    snapshots::read_file_at(&project::to_path(&path), &commit, &doc_path)
}

#[tauri::command]
fn snapshot_restore_file(path: String, commit: String, doc_path: String) -> AppResult<()> {
    snapshots::restore_file(&project::to_path(&path), &commit, &doc_path)
}

#[tauri::command]
fn snapshot_is_dirty(path: String) -> AppResult<bool> {
    snapshots::is_dirty(&project::to_path(&path))
}

// --- Phase 6: collections, scrivenings, structure ---------------------------

#[tauri::command]
fn collections_list(path: String) -> AppResult<Vec<Collection>> {
    let conn = store::open(&project::to_path(&path))?;
    collections::list(&conn)
}

#[tauri::command]
fn collection_create(path: String, name: String, kind: String, filters: Option<String>) -> AppResult<String> {
    let conn = store::open(&project::to_path(&path))?;
    let id = project::new_id();
    collections::create(&conn, &id, &name, CollectionKind::parse(&kind), filters.as_deref(), project::now_ms())?;
    Ok(id)
}

#[tauri::command]
fn collection_delete(path: String, id: String) -> AppResult<()> {
    collections::delete(&store::open(&project::to_path(&path))?, &id)
}

#[tauri::command]
fn collection_members(path: String, id: String) -> AppResult<Vec<DocumentRow>> {
    collections::members(&store::open(&project::to_path(&path))?, &id)
}

#[tauri::command]
fn collection_add(path: String, id: String, document_id: String) -> AppResult<()> {
    collections::add_item(&store::open(&project::to_path(&path))?, &id, &document_id)
}

#[tauri::command]
fn collection_remove(path: String, id: String, document_id: String) -> AppResult<()> {
    collections::remove_item(&store::open(&project::to_path(&path))?, &id, &document_id)
}

#[tauri::command]
fn bulk_set_metadata(
    path: String,
    ids: Vec<String>,
    status: Option<String>,
    label: Option<String>,
    section_type: Option<String>,
    pov: Option<String>,
) -> AppResult<usize> {
    project::bulk_set(
        &project::to_path(&path),
        &ids,
        status.as_deref(),
        label.as_deref(),
        section_type.as_deref(),
        pov.as_deref(),
    )
}

#[tauri::command]
fn scrivenings_open(path: String, parent_id: String) -> AppResult<(String, Vec<String>)> {
    let root = project::to_path(&path);
    let docs = project::documents(&root)?;
    let mut kids: Vec<_> = docs
        .into_iter()
        .filter(|d| d.parent_id.as_deref() == Some(parent_id.as_str()) && d.kind != DocKind::Folder)
        .collect();
    kids.sort_by_key(|d| d.position);

    let mut pieces = Vec::new();
    for k in &kids {
        pieces.push(scrivenings::Piece { id: k.id.clone(), body: project::read_body(&root, &k.id)? });
    }
    let ids = pieces.iter().map(|p| p.id.clone()).collect();
    Ok((scrivenings::stitch(&pieces), ids))
}

#[tauri::command]
fn scrivenings_save(path: String, combined: String, ids: Vec<String>) -> AppResult<usize> {
    let root = project::to_path(&path);
    // Split first: if the boundaries no longer line up, nothing is written.
    let pieces = scrivenings::split(&combined, &ids)?;
    for p in &pieces {
        project::write_body(&root, &p.id, &p.body)?;
    }
    Ok(pieces.len())
}

// --- Phase 7: compile -------------------------------------------------------

#[tauri::command]
fn compile_preview(path: String, preset: String, filters: CompileFilters) -> AppResult<String> {
    let root = project::to_path(&path);
    let items = compile_items(&root, &filters)?;
    Ok(compile::to_markdown(&items, &CompilePreset::named(&preset)))
}

fn compile_items(root: &std::path::Path, filters: &CompileFilters) -> AppResult<Vec<compile::Item>> {
    let docs = if let Some(cid) = &filters.collection {
        collections::members(&store::open(root)?, cid)?
    } else {
        let mut d = project::documents(root)?;
        if let Some(rid) = &filters.root {
            let keep: Vec<String> = descendants(&d, rid);
            d.retain(|x| keep.contains(&x.id));
        }
        d.sort_by_key(|x| x.position);
        d
    };
    let mut items = Vec::new();
    for doc in docs {
        if !compile::included(&doc, filters) {
            continue;
        }
        let body = project::read_body(root, &doc.id)?;
        items.push(compile::Item { doc, body });
    }
    Ok(items)
}

fn descendants(docs: &[DocumentRow], root_id: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut frontier = vec![root_id.to_string()];
    while let Some(id) = frontier.pop() {
        out.push(id.clone());
        for child in docs.iter().filter(|d| d.parent_id.as_deref() == Some(id.as_str())) {
            frontier.push(child.id.clone());
        }
    }
    out
}

#[tauri::command]
fn compile_export(
    path: String,
    preset: String,
    format: String,
    filters: CompileFilters,
    destination: String,
    title: Option<String>,
    author: Option<String>,
) -> AppResult<String> {
    let root = project::to_path(&path);
    let items = compile_items(&root, &filters)?;
    let mut p = CompilePreset::named(&preset);
    p.title_page = title;
    p.author = author;
    let fmt = ExportFormat::parse(&format);
    let bytes = compile::render(&items, &p, fmt)?;

    let dest = std::path::PathBuf::from(&destination);
    if dest.extension().is_none() {
        return err_str("the destination needs a file name");
    }
    std::fs::write(&dest, bytes)?;
    Ok(dest.to_string_lossy().to_string())
}

fn err_str<T>(m: &str) -> AppResult<T> {
    Err(error::AppError(m.to_string()))
}

// --- Phase 3: local AI and retrieval ---------------------------------------

#[tauri::command]
fn ollama_status() -> OllamaStatus {
    ollama::status()
}

#[tauri::command]
fn index_project(path: String, model: String) -> AppResult<(usize, usize)> {
    let root = project::to_path(&path);
    let docs = project::documents(&root)?;
    let mut conn = store::open(&root)?;
    let mut chunk_total = 0usize;
    let mut doc_total = 0usize;
    for doc in docs.iter().filter(|d| d.path.is_some() && d.word_count > 0) {
        let body = project::read_body(&root, &doc.id)?;
        let texts = rag::chunk(&body, 220, 40)?;
        if texts.is_empty() {
            continue;
        }
        let vectors = ollama::embed(&model, &texts)?;
        let pairs: Vec<(String, Vec<f32>)> = texts.into_iter().zip(vectors).collect();
        chunk_total += rag::index_document(&mut conn, &doc.id, &pairs, &model, project::now_ms())?;
        doc_total += 1;
    }
    Ok((doc_total, chunk_total))
}

#[tauri::command]
fn search_index(path: String, model: String, query: String, k: usize, exclude: Option<String>) -> AppResult<Vec<RetrievedChunk>> {
    let root = project::to_path(&path);
    let q = ollama::embed(&model, &[query])?;
    let conn = store::open(&root)?;
    rag::search(&conn, &q[0], k, exclude.as_deref())
}

#[tauri::command]
fn index_stats(path: String) -> AppResult<(i64, i64)> {
    rag::index_stats(&store::open(&project::to_path(&path))?)
}

#[tauri::command]
fn assist(
    path: String,
    model: String,
    id: String,
    mode: String,
    instruction: Option<String>,
    allow_cloud: Option<bool>,
) -> AppResult<String> {
    let root = project::to_path(&path);
    let body = project::read_body(&root, &id)?;
    if body.trim().is_empty() && mode != "continue" {
        return err_str("there is nothing to work with in this document yet");
    }
    let (system, prompt) = match mode.as_str() {
        "rewrite" => (
            "You are an editor. Rewrite the passage in the author's own voice. Return only the rewritten prose.".to_string(),
            format!("{}\n\nPassage:\n{body}", instruction.unwrap_or_else(|| "Tighten this.".into())),
        ),
        "critique" => (
            "You are a demanding reader. Name concrete problems. Do not rewrite anything.".to_string(),
            format!("Critique this passage:\n{body}"),
        ),
        _ => (
            "You are a novelist continuing an existing scene. Match the voice exactly. Return only prose.".to_string(),
            format!("Continue this scene:\n{body}"),
        ),
    };
    // "In the author's own voice" is a label unless the model is told what that
    // voice measures. When a style profile exists it goes into the system
    // prompt for the two modes that write prose; critique is left alone, since
    // a critic should react to the page, not to a target.
    let system = match (mode.as_str(), style::load(&root)) {
        ("critique", _) | (_, None) => system,
        (_, Some(profile)) => format!("{system}\n\n{}", style::prompt_fragment(&profile)),
    };
    ollama::generate_guarded(&model, &prompt, Some(&system), 900, 0.7, allow_cloud.unwrap_or(false))
}

/// Splits the installed models into what each can actually do, so the UI never
/// has to guess and never offers a model for the wrong job.
#[tauri::command]
fn model_pools() -> AppResult<(Vec<model::ModelInfo>, Vec<model::ModelInfo>, Vec<model::ModelInfo>)> {
    let s = ollama::status();
    let cloud: Vec<model::ModelInfo> =
        s.models.iter().filter(|m| ollama::is_cloud_model(&m.name)).cloned().collect();
    Ok((ollama::generation_models(&s), ollama::embedding_models(&s), cloud))
}

// --- Phase 4: integrity -----------------------------------------------------

#[tauri::command]
fn integrity_similarity(path: String, id: String, threshold: f32) -> AppResult<Vec<SimilarityMatch>> {
    let root = project::to_path(&path);
    let subject = project::read_body(&root, &id)?;
    let docs = project::documents(&root)?;
    let mut sources = Vec::new();
    for d in docs.iter().filter(|d| d.id != id && d.path.is_some()) {
        sources.push((d.id.clone(), d.title.clone(), project::read_body(&root, &d.id)?));
    }
    Ok(integrity::internal_similarity(&id, &subject, &sources, threshold, 5))
}

#[tauri::command]
fn integrity_signals(path: String, id: String) -> AppResult<WritingSignals> {
    Ok(integrity::writing_signals(&project::read_body(&project::to_path(&path), &id)?))
}

#[tauri::command]
fn integrity_rubric(path: String, id: String) -> AppResult<Vec<RubricScore>> {
    Ok(integrity::rubric(&project::read_body(&project::to_path(&path), &id)?))
}

/// How wigolo could be started here, and whether it is running now.
#[tauri::command]
fn wigolo_sidecar_status() -> sidecar::SidecarStatus {
    sidecar::status()
}

/// Start wigolo, or adopt one that is already answering.
///
/// Deliberately not called on launch. Starting wigolo starts something that
/// reaches the internet on the author's behalf, and per the golden rule that is
/// a choice they make, not a step the app takes for them.
#[tauri::command]
fn wigolo_sidecar_start() -> AppResult<sidecar::SidecarStatus> {
    sidecar::start()
}

/// Stop the wigolo this app started. One it did not start is left alone.
#[tauri::command]
fn wigolo_sidecar_stop() -> AppResult<sidecar::SidecarStatus> {
    sidecar::stop()
}

/// Is the author's wigolo daemon running? Never an error — absent is a state.
#[tauri::command]
fn wigolo_status() -> websearch::WigoloStatus {
    websearch::status()
}

/// Check one phrase against the web, through wigolo.
///
/// `confirmed` is not a formality. This is the only path in the app that sends
/// the author's words off the machine, so it refuses to run without an explicit
/// yes from a person, exactly like `delete_document` and `compile_manuscript`.
#[tauri::command]
fn web_similarity(
    path: String,
    phrase: String,
    max_results: Option<usize>,
    confirmed: bool,
) -> AppResult<websearch::WebSearchResult> {
    if !confirmed {
        return err_str(
            "a web check sends this phrase off your machine, through your own wigolo. It needs your explicit yes.",
        );
    }
    let root = project::to_path(&path);
    let found = websearch::search(&phrase, max_results.unwrap_or(5))?;
    websearch::record(
        &root,
        &websearch::TraceEntry {
            at: project::now_ms(),
            query: found.query.clone(),
            word_count: found.query.split_whitespace().count() as i64,
            base: websearch::base(),
            result_count: found.matches.len() as i64,
            urls: found.matches.iter().map(|m| m.url.clone()).collect(),
        },
    )?;
    Ok(found)
}

/// Every phrase this project has ever sent off the machine.
#[tauri::command]
fn research_trace(path: String) -> Vec<websearch::TraceEntry> {
    websearch::trace(&project::to_path(&path))
}

/// Build the integrity packet for one document.
#[tauri::command]
fn integrity_packet(
    path: String,
    id: String,
    threshold: f32,
) -> AppResult<integrity::IntegrityPacket> {
    let root = project::to_path(&path);
    let subject = project::read_body(&root, &id)?;
    let docs = project::documents(&root)?;
    let doc = docs
        .iter()
        .find(|d| d.id == id)
        .ok_or_else(|| error::AppError(format!("no document {id}")))?;

    let mut sources = Vec::new();
    for other in docs.iter().filter(|d| d.id != id && d.path.is_some()) {
        sources.push((other.id.clone(), other.title.clone(), project::read_body(&root, &other.id)?));
    }

    Ok(integrity::IntegrityPacket {
        generated_at: project::now_ms(),
        project: root.display().to_string(),
        document_id: id.clone(),
        document_title: doc.title.clone(),
        word_count: project::count_words(&subject),
        internal_matches: integrity::internal_similarity(&id, &subject, &sources, threshold, 5),
        threshold,
        signals: integrity::writing_signals(&subject),
        rubric: integrity::rubric(&subject),
        web_checks: websearch::trace(&root),
        method: integrity::method_notes(),
    })
}

/// Write the packet as a JSON + HTML pair beside each other.
#[tauri::command]
fn integrity_export(
    path: String,
    id: String,
    threshold: f32,
    out_path: String,
) -> AppResult<(String, String)> {
    let packet = integrity_packet(path, id, threshold)?;
    let json_path = std::path::PathBuf::from(&out_path);
    let html_path = json_path.with_extension("html");
    std::fs::write(&json_path, serde_json::to_string_pretty(&packet)?)?;
    std::fs::write(&html_path, integrity::packet_html(&packet))?;
    Ok((json_path.display().to_string(), html_path.display().to_string()))
}

// --- Knowledge to Publication: blocks, provenance, sources, research --------

#[tauri::command]
fn blocks_list(path: String, document_id: String) -> AppResult<Vec<blocks::Block>> {
    let conn = store::open(&project::to_path(&path))?;
    blocks::list(&conn, &document_id)
}

#[tauri::command]
fn block_history(path: String, block_id: String) -> AppResult<Vec<provenance::Event>> {
    let conn = store::open(&project::to_path(&path))?;
    provenance::history(&conn, &block_id)
}

#[tauri::command]
fn block_authorship(path: String, block_id: String) -> AppResult<provenance::Authorship> {
    let conn = store::open(&project::to_path(&path))?;
    Ok(provenance::classify(&provenance::history(&conn, &block_id)?))
}

#[tauri::command]
fn provenance_summary(
    path: String,
    document_id: Option<String>,
) -> AppResult<provenance::AuthorshipSummary> {
    let conn = store::open(&project::to_path(&path))?;
    provenance::summarise(&conn, document_id.as_deref())
}

/// Is the ledger intact? `None` means nothing has been altered since it was
/// recorded. Tamper-evident, not tamper-proof — see docs/BLOCK-IDENTITY.md §7.
#[tauri::command]
fn provenance_verify(path: String) -> AppResult<Option<provenance::ChainBreak>> {
    let conn = store::open(&project::to_path(&path))?;
    provenance::verify_chain(&conn)
}

/// Bring blocks up to date with files edited outside AegisDraft.
#[tauri::command]
fn provenance_reconcile(path: String) -> AppResult<usize> {
    project::reconcile_all(&project::to_path(&path))
}

/// Take a model's proposal into the manuscript, recording that it did.
///
/// The Assist panel's alternative is the author copying the text by hand, which
/// AegisDraft cannot observe. This is the path that leaves a record.
#[tauri::command]
fn assist_accept(
    path: String,
    id: String,
    text: String,
    model: String,
    params: Option<serde_json::Value>,
) -> AppResult<blocks::Block> {
    project::accept_proposal(
        &project::to_path(&path),
        &id,
        &text,
        &model,
        params.unwrap_or_else(|| serde_json::json!({})),
    )
}

// --- Sheet templates --------------------------------------------------------

/// Every sheet template: the shipped shapes plus the author's own.
#[tauri::command]
fn sheet_templates(path: String) -> Vec<templates::Template> {
    templates::list(&project::to_path(&path))
}

/// Save a template as a Markdown file the author can also edit directly.
#[tauri::command]
fn sheet_template_save(path: String, id: String, body: String) -> AppResult<templates::Template> {
    templates::save(&project::to_path(&path), &id, &body)
}

/// Delete a custom template. The shipped shape comes back — it was overridden,
/// never removed.
#[tauri::command]
fn sheet_template_forget(path: String, id: String) -> AppResult<()> {
    templates::forget(&project::to_path(&path), &id)
}

#[tauri::command]
fn create_sheet_from_template(
    path: String,
    name: String,
    kind: String,
    parent_id: Option<String>,
    link_to: Option<String>,
    template_id: Option<String>,
) -> AppResult<DocumentRow> {
    project::create_sheet_from(
        &project::to_path(&path),
        &name,
        DocKind::parse(&kind),
        parent_id.as_deref(),
        link_to.as_deref(),
        template_id.as_deref(),
    )
}

// --- Phase 5: agents --------------------------------------------------------

// --- Phase 6: templates, goals, freeform board, icons -----------------------

#[tauri::command]
fn project_templates() -> Vec<(String, String)> {
    project::templates().into_iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

#[tauri::command]
fn create_project_from_template(
    app: tauri::AppHandle,
    parent: String,
    name: String,
    kind: String,
    template: String,
) -> AppResult<ProjectSummary> {
    let summary =
        project::create_from_template(&project::to_path(&parent), &name, &kind, &template)?;
    let _ = project::remember(&config_dir(&app)?, &summary);
    Ok(summary)
}

#[tauri::command]
fn goals_read(path: String) -> AppResult<project::Goals> {
    project::goals(&project::to_path(&path))
}

#[tauri::command]
fn goals_write(path: String, goals: project::Goals) -> AppResult<project::Goals> {
    project::set_goals(&project::to_path(&path), &goals)
}

#[tauri::command]
fn goals_start_session(path: String) -> AppResult<project::Goals> {
    project::start_session(&project::to_path(&path))
}

/// Words written since the session mark. `None` means no session has begun,
/// which the UI shows differently from zero.
#[tauri::command]
fn goals_session_progress(path: String) -> AppResult<Option<i64>> {
    project::session_progress(&project::to_path(&path))
}

#[tauri::command]
fn set_document_position(
    path: String,
    id: String,
    x: Option<f32>,
    y: Option<f32>,
) -> AppResult<()> {
    project::set_position(&project::to_path(&path), &id, x, y)
}

#[tauri::command]
fn set_document_icon(path: String, id: String, icon: Option<String>) -> AppResult<DocumentRow> {
    project::set_icon(&project::to_path(&path), &id, icon.as_deref())
}

// --- Phase 3: writing style profile ----------------------------------------

#[tauri::command]
fn style_load(path: String) -> Option<style::StyleProfile> {
    style::load(&project::to_path(&path))
}

#[tauri::command]
fn style_build(
    path: String,
    model: Option<String>,
    allow_cloud: Option<bool>,
) -> AppResult<style::StyleProfile> {
    let root = project::to_path(&path);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let profile = style::build(&root, model.as_deref(), allow_cloud.unwrap_or(false), now)?;
    style::save(&root, &profile)?;
    Ok(profile)
}

/// How far the open document sits from the profile. `None` when no profile has
/// been built — the inspector says so rather than showing a zero.
#[tauri::command]
fn style_distance(path: String, id: String) -> AppResult<Option<style::StyleDistance>> {
    let root = project::to_path(&path);
    let Some(profile) = style::load(&root) else {
        return Ok(None);
    };
    let body = project::read_body(&root, &id)?;
    Ok(Some(style::distance(&profile, &body)))
}

#[tauri::command]
fn import_survey(source: String) -> AppResult<(usize, usize)> {
    import::survey(&project::to_path(&source))
}

#[tauri::command]
fn import_folder(
    path: String,
    source: String,
    parent_id: Option<String>,
    kind: Option<String>,
) -> AppResult<import::ImportReport> {
    import::run(
        &project::to_path(&path),
        &project::to_path(&source),
        parent_id.as_deref(),
        kind.map(|k| DocKind::parse(&k)).unwrap_or(DocKind::Scene),
    )
}

#[tauri::command]
fn recent_projects(app: tauri::AppHandle) -> AppResult<Vec<RecentProject>> {
    Ok(project::read_recents(&config_dir(&app)?))
}

/// Called by the webview once the first paint is done: dismisses the splash
/// window and reveals the main one. A timeout in `run` covers the case where
/// the frontend never gets there, so a failure can never leave a user staring
/// at a splash screen with no way forward.
#[tauri::command]
fn frontend_ready(app: tauri::AppHandle) {
    reveal_main(&app);
}

fn reveal_main(app: &tauri::AppHandle) {
    if let Some(splash) = app.get_webview_window("splashscreen") {
        let _ = splash.close();
    }
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

/// Development convenience: `AEGISDRAFT_OPEN=/path/to/Book.aegis npm run app`
/// opens that project straight away instead of stopping at the launcher.
/// Useful for testing a screen without clicking through the launch flow.
#[tauri::command]
fn startup_project() -> Option<String> {
    std::env::var("AEGISDRAFT_OPEN").ok().filter(|p| !p.is_empty())
}

/// Development convenience: `AEGISDRAFT_ROUTE=story` opens straight onto a
/// screen. Used to capture and review each one without clicking through.
#[tauri::command]
fn startup_route() -> Option<String> {
    std::env::var("AEGISDRAFT_ROUTE").ok().filter(|r| !r.is_empty())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            app.set_menu(menu::build(&handle)?)?;

            // If the webview never reports ready — a build error, a crash on
            // boot — show the window anyway rather than stranding the user on
            // the splash screen.
            let watchdog = handle.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(10));
                if let Some(main) = watchdog.get_webview_window("main") {
                    if !main.is_visible().unwrap_or(false) {
                        reveal_main(&watchdog);
                    }
                }
            });
            Ok(())
        })
        .on_menu_event(|app, event| {
            menu::on_event(app, event.id().as_ref());
        })
        .invoke_handler(tauri::generate_handler![
            create_project,
            open_project,
            list_documents,
            read_document,
            write_document,
            create_document,
            rename_document,
            update_document_meta,
            delete_document,
            move_document,
            create_sheet,
            append_canon_entry,
            update_document_extended,
            set_keywords,
            project_keywords,
            link_documents,
            unlink_documents,
            snapshot_create,
            snapshot_list,
            snapshot_read_file,
            snapshot_restore_file,
            snapshot_is_dirty,
            collections_list,
            collection_create,
            collection_delete,
            collection_members,
            collection_add,
            collection_remove,
            bulk_set_metadata,
            scrivenings_open,
            scrivenings_save,
            compile_preview,
            compile_export,
            ollama_status,
            index_project,
            search_index,
            index_stats,
            assist,
            model_pools,
            integrity_similarity,
            integrity_signals,
            integrity_rubric,
            wigolo_sidecar_status,
            wigolo_sidecar_start,
            wigolo_sidecar_stop,
            wigolo_status,
            web_similarity,
            research_trace,
            integrity_packet,
            integrity_export,
            blocks_list,
            block_history,
            block_authorship,
            provenance_summary,
            provenance_verify,
            provenance_reconcile,
            assist_accept,
            sheet_templates,
            sheet_template_save,
            sheet_template_forget,
            create_sheet_from_template,
            project_templates,
            create_project_from_template,
            goals_read,
            goals_write,
            goals_start_session,
            goals_session_progress,
            set_document_position,
            set_document_icon,
            style_load,
            style_build,
            style_distance,
            import_survey,
            import_folder,
            recent_projects,
            frontend_ready,
            startup_project,
            startup_route
        ])
        .build(tauri::generate_context!())
        .expect("error while running AegisDraft")
        .run(|_app, event| {
            // A daemon this app started must not outlive it. Without this the
            // wigolo sidecar is reparented to init and keeps serving — and
            // keeps holding the port — after the window is gone.
            if let tauri::RunEvent::Exit = event {
                sidecar::shutdown();
            }
        });
}
