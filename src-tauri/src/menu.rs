//! The native menu bar.
//!
//! Menu items do not act on the project directly — they emit a `menu` event
//! that the webview handles, so a menu command and its in-app button always run
//! exactly the same code path. Items whose phase is not built yet are disabled
//! here rather than silently doing nothing.

use tauri::menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Payload the frontend listens for. One string id per command.
pub const MENU_EVENT: &str = "menu";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    let pkg = app.package_info();

    let about = AboutMetadata {
        name: Some("AegisDraft".into()),
        version: Some(pkg.version.to_string()),
        copyright: Some("© 2026 AegisDraft".into()),
        comments: Some("A local-first book studio. Your manuscript stays on this machine.".into()),
        ..Default::default()
    };

    let item = |id: &str, label: &str, accel: Option<&str>, enabled: bool| {
        let mut b = MenuItemBuilder::new(label).id(id).enabled(enabled);
        if let Some(a) = accel {
            b = b.accelerator(a);
        }
        b.build(app)
    };

    let app_menu = SubmenuBuilder::new(app, "AegisDraft")
        .item(&PredefinedMenuItem::about(app, Some("About AegisDraft"), Some(about))?)
        .separator()
        .item(&item("settings", "Settings…", Some("CmdOrCtrl+,"), true)?)
        .separator()
        .item(&PredefinedMenuItem::hide(app, None)?)
        .item(&PredefinedMenuItem::hide_others(app, None)?)
        .item(&PredefinedMenuItem::show_all(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::quit(app, None)?)
        .build()?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&item("new_project", "New Project…", Some("CmdOrCtrl+N"), true)?)
        .item(&item("open_project", "Open Project…", Some("CmdOrCtrl+O"), true)?)
        .item(&item("import_markdown", "Import Markdown Folder…", Some("CmdOrCtrl+Shift+I"), true)?)
        .separator()
        .item(&item("new_scene", "New Scene", Some("CmdOrCtrl+Shift+N"), true)?)
        .item(&item("new_folder", "New Folder", Some("CmdOrCtrl+Alt+N"), true)?)
        .item(&item("new_character", "New Character Sheet", None, true)?)
        .item(&item("new_location", "New Location Sheet", None, true)?)
        .item(&item("sheet_templates", "Sheet Templates…", None, true)?)
        .separator()
        .item(&item("save", "Save", Some("CmdOrCtrl+S"), true)?)
        .item(&item("snapshot", "Take Snapshot", Some("CmdOrCtrl+Shift+S"), true)?)
        .separator()
        .item(&item("compile", "Compile…", Some("CmdOrCtrl+Shift+E"), true)?)
        .separator()
        .item(&item("close_project", "Close Project", Some("CmdOrCtrl+W"), true)?)
        .build()?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .item(&PredefinedMenuItem::undo(app, None)?)
        .item(&PredefinedMenuItem::redo(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::cut(app, None)?)
        .item(&PredefinedMenuItem::copy(app, None)?)
        .item(&PredefinedMenuItem::paste(app, None)?)
        .item(&PredefinedMenuItem::select_all(app, None)?)
        .separator()
        .item(&item("find", "Search the Book…", Some("CmdOrCtrl+Shift+F"), true)?)
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&item("view_editor", "Editor", Some("CmdOrCtrl+1"), true)?)
        .item(&item("view_corkboard", "Corkboard", Some("CmdOrCtrl+2"), true)?)
        .item(&item("view_outliner", "Outliner", Some("CmdOrCtrl+3"), true)?)
        .item(&item("view_scrivenings", "Scrivenings", Some("CmdOrCtrl+4"), true)?)
        .separator()
        .item(&item("toggle_binder", "Show Binder", Some("CmdOrCtrl+B"), true)?)
        .item(&item("toggle_inspector", "Show Inspector", Some("CmdOrCtrl+I"), true)?)
        .item(&item("toggle_split", "Reference Pane", Some("CmdOrCtrl+Alt+S"), true)?)
        .separator()
        .item(&item("composition", "Composition Mode", Some("CmdOrCtrl+Shift+C"), false)?)
        .separator()
        .item(&PredefinedMenuItem::fullscreen(app, None)?)
        .build()?;

    let project_menu = SubmenuBuilder::new(app, "Project")
        .item(&item("command_bar", "Ask…", Some("CmdOrCtrl+K"), true)?)
        .separator()
        .item(&item("integrity", "Integrity Report", None, true)?)
        .item(&item("story", "Story Visualization", None, true)?)
        .item(&item("insights", "Insights and Analytics", None, true)?)
        .separator()
        .item(&item("collections", "New Collection…", None, false)?)
        .item(&item("goals", "Writing Goals…", None, false)?)
        .build()?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .item(&PredefinedMenuItem::minimize(app, None)?)
        .item(&PredefinedMenuItem::maximize(app, None)?)
        .separator()
        .item(&PredefinedMenuItem::close_window(app, None)?)
        .build()?;

    let help_menu = SubmenuBuilder::new(app, "Help")
        .item(&item("help_docs", "AegisDraft Documentation", None, true)?)
        .item(&item("help_privacy", "Where My Work Is Stored", None, true)?)
        .build()?;

    MenuBuilder::new(app)
        .items(&[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &project_menu,
            &window_menu,
            &help_menu,
        ])
        .build()
}

/// Forwards a menu click to the webview. Predefined items (copy, quit, …) are
/// handled by the OS and never reach here.
pub fn on_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(MENU_EVENT, id.to_string());
    }
}
