//! AegisDraft — Tauri commands.
//!
//! The Rust core owns file I/O and SQLite (see docs/ARCHITECTURE.md). The
//! webview never touches the filesystem directly; it asks for these commands,
//! which keeps one place responsible for the manuscript.

mod error;
pub mod bookmarks;
pub mod blocks;
pub mod collections;
pub mod import;
pub mod integrity;
mod menu;
pub mod model;
pub mod project;
pub mod provenance;
pub mod scrivenings;
pub mod snapshots;
pub mod store;
pub mod templates;
pub mod websearch;

use error::{AppError, AppResult};
use model::{
    Collection, CollectionKind, DocKind, DocumentRow, ProjectSummary, RecentProject, SaveReceipt,
    Snapshot,
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
            import_survey,
            import_folder,
            recent_projects,
            frontend_ready,
            startup_project,
            startup_route
        ])
        .run(tauri::generate_context!())
        .expect("error while running AegisDraft");
}
