//! AegisDraft — Tauri commands.
//!
//! The Rust core owns file I/O and SQLite (see docs/ARCHITECTURE.md). The
//! webview never touches the filesystem directly; it asks for these commands,
//! which keeps one place responsible for the manuscript.

mod error;
mod menu;
pub mod model;
pub mod project;
pub mod store;

use error::{AppError, AppResult};
use model::{DocKind, DocumentRow, ProjectSummary, RecentProject, SaveReceipt};
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
    let summary = project::open(&project::to_path(&path))?;
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
            recent_projects,
            startup_project,
            frontend_ready,
        ])
        .run(tauri::generate_context!())
        .expect("error while running AegisDraft");
}
