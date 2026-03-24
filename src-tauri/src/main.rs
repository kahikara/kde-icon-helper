mod checker;
mod desktop;
mod diagnostics;
mod fixer;
mod models;
mod paths;
mod scanner;
mod tools;

use base64::Engine;
use diagnostics::RuntimeDiagnostics;
use models::{FixResult, LauncherEntry};
use std::path::PathBuf;
use tauri::Manager;

#[tauri::command]
fn scan_launchers() -> Vec<LauncherEntry> {
    scanner::scan_launchers()
}

#[tauri::command]
fn check_launcher(path: String) -> LauncherEntry {
    checker::check_launcher(path)
}

#[tauri::command]
fn fix_launcher_icon(path: String) -> FixResult {
    fixer::fix_launcher_icon(path)
}

#[tauri::command]
fn set_launcher_icon_manual(path: String, source_icon_path: String) -> FixResult {
    fixer::set_launcher_icon_manual(path, source_icon_path)
}

#[tauri::command]
fn restore_launcher_icon_default(path: String) -> FixResult {
    fixer::restore_launcher_icon_default(path)
}

#[tauri::command]
fn load_icon_preview(path: String) -> Result<Option<String>, String> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Ok(None);
    }

    let ext = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    let mime = match ext.as_str() {
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "xpm" => "image/x-xpixmap",
        "ico" => "image/x-icon",
        _ => return Ok(None),
    };

    let bytes = std::fs::read(&path_buf)
        .map_err(|e| format!("Could not read icon preview {}: {}", path_buf.display(), e))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(Some(format!("data:{};base64,{}", mime, encoded)))
}

#[tauri::command]
fn get_runtime_diagnostics() -> RuntimeDiagnostics {
    diagnostics::get_runtime_diagnostics()
}

#[tauri::command]
fn reveal_main_window(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    window.show().map_err(|e| e.to_string())?;
    let _ = window.set_focus();

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_window_state::Builder::default().build())?;

            if let Some(window) = app.get_webview_window("main") {
                let app_version = app.package_info().version.to_string();
                let title = format!("KDE Icon Helper v{}", app_version);
                let _ = window.set_title(&title);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_launchers,
            check_launcher,
            fix_launcher_icon,
            set_launcher_icon_manual,
            restore_launcher_icon_default,
            load_icon_preview,
            get_runtime_diagnostics,
            reveal_main_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
