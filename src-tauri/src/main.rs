mod checker;
mod desktop;
mod fixer;
mod models;
mod paths;
mod scanner;

use base64::Engine;
use models::{FixResult, LauncherEntry};
use std::path::PathBuf;
use tauri::Manager;

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

#[cfg(target_os = "linux")]
fn maybe_relaunch_appimage_with_wayland_preload() {
    use std::path::Path;
    use std::process::Command;

    if !is_wayland_session() {
        return;
    }

    let appimage = match std::env::var("APPIMAGE") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return,
    };

    if std::env::var_os("KDEICONHELPER_APPIMAGE_RELAUNCHED").is_some() {
        return;
    }

    if std::env::var_os("LD_PRELOAD").is_some() {
        return;
    }

    let candidates = [
        "/usr/lib/libwayland-client.so",
        "/usr/lib64/libwayland-client.so",
        "/lib/x86_64-linux-gnu/libwayland-client.so.0",
        "/usr/lib/x86_64-linux-gnu/libwayland-client.so.0",
        "/lib64/libwayland-client.so.0",
    ];

    let preload = candidates
        .iter()
        .find(|candidate| Path::new(candidate).exists())
        .map(|candidate| (*candidate).to_string());

    let Some(preload) = preload else {
        return;
    };

    let args: Vec<String> = std::env::args().skip(1).collect();

    let spawn_result = Command::new(&appimage)
        .args(args)
        .env("LD_PRELOAD", &preload)
        .env("KDEICONHELPER_APPIMAGE_RELAUNCHED", "1")
        .spawn();

    if spawn_result.is_ok() {
        std::process::exit(0);
    }
}

#[cfg(not(target_os = "linux"))]
fn maybe_relaunch_appimage_with_wayland_preload() {}

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

fn main() {
    maybe_relaunch_appimage_with_wayland_preload();

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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_launchers,
            check_launcher,
            fix_launcher_icon,
            fix_all_launchers,
            set_launcher_icon_manual,
            restore_launcher_icon_default,
            load_icon_preview
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
