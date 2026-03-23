mod checker;
mod desktop;
mod fixer;
mod models;
mod paths;
mod scanner;

use base64::Engine;
use models::{FixResult, LauncherEntry};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, Manager, WebviewWindow};

#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|value| value.eq_ignore_ascii_case("wayland"))
            .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn is_wayland_session() -> bool {
    false
}

#[derive(Debug, Serialize)]
struct LinuxWindowMode {
    wayland_undecorated: bool,
}

#[derive(Debug, Serialize)]
struct AppMeta {
    app_version: String,
}

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
fn get_linux_window_mode() -> LinuxWindowMode {
    LinuxWindowMode {
        wayland_undecorated: cfg!(target_os = "linux") && is_wayland_session(),
    }
}

#[tauri::command]
fn get_app_meta(app: AppHandle) -> AppMeta {
    AppMeta {
        app_version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
fn window_is_maximized(window: WebviewWindow) -> Result<bool, String> {
    window
        .is_maximized()
        .map_err(|e| format!("Could not read maximize state: {e}"))
}

#[tauri::command]
fn window_minimize(window: WebviewWindow) -> Result<(), String> {
    window
        .minimize()
        .map_err(|e| format!("Could not minimize window: {e}"))
}

#[tauri::command]
fn window_toggle_maximize(window: WebviewWindow) -> Result<bool, String> {
    let is_maximized = window
        .is_maximized()
        .map_err(|e| format!("Could not read maximize state: {e}"))?;

    if is_maximized {
        window
            .unmaximize()
            .map_err(|e| format!("Could not unmaximize window: {e}"))?;
    } else {
        window
            .maximize()
            .map_err(|e| format!("Could not maximize window: {e}"))?;
    }

    window
        .is_maximized()
        .map_err(|e| format!("Could not read maximize state: {e}"))
}

#[tauri::command]
fn window_start_dragging(window: WebviewWindow) -> Result<(), String> {
    window
        .start_dragging()
        .map_err(|e| format!("Could not start dragging window: {e}"))
}

#[tauri::command]
fn window_close_main(app: AppHandle) -> Result<(), String> {
    let Some(window) = app.get_webview_window("main") else {
        return Err("Main window not found".to_string());
    };

    window
        .close()
        .map_err(|e| format!("Could not close main window: {e}"))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_window_state::Builder::default().build())?;

            #[cfg(target_os = "linux")]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(!is_wayland_session());
            }

            #[cfg(not(target_os = "linux"))]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(true);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_launchers,
            check_launcher,
            fix_launcher_icon,
            fix_all_launchers,
            set_launcher_icon_manual,
            restore_launcher_icon_default,
            load_icon_preview,
            get_linux_window_mode,
            get_app_meta,
            window_is_maximized,
            window_minimize,
            window_toggle_maximize,
            window_start_dragging,
            window_close_main
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
