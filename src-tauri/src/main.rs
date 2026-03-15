mod checker;
mod desktop;
mod fixer;
mod models;
mod paths;
mod scanner;

use base64::Engine;
use models::{FixResult, LauncherEntry};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
fn apply_linux_x11_backend() {
    if std::env::var_os("GDK_BACKEND").is_none() {
        std::env::set_var("GDK_BACKEND", "x11");
    }

    if std::env::var_os("WINIT_UNIX_BACKEND").is_none() {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
    }
}

#[cfg(not(target_os = "linux"))]
fn apply_linux_x11_backend() {}

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
fn fix_all_launchers() -> Vec<FixResult> {
    fixer::fix_all_launchers()
}

#[tauri::command]
fn set_launcher_icon_manual(path: String, source_icon_path: String) -> FixResult {
    fixer::set_launcher_icon_manual(path, source_icon_path)
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
        _ => return Ok(None),
    };

    let bytes = std::fs::read(&path_buf)
        .map_err(|e| format!("Could not read icon preview {}: {}", path_buf.display(), e))?;

    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(Some(format!("data:{};base64,{}", mime, encoded)))
}

fn main() {
    apply_linux_x11_backend();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_window_state::Builder::default().build())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_launchers,
            check_launcher,
            fix_launcher_icon,
            fix_all_launchers,
            set_launcher_icon_manual,
            load_icon_preview
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
