use crate::desktop::{desktop_extract_value, parse_desktop_file};
use crate::models::LauncherEntry;
use regex::Regex;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use walkdir::WalkDir;

const ORIGINAL_ICON_KEY: &str = "X-KdeIconHelperOriginalIcon";

fn normalize_icon(icon: Option<String>) -> Option<String> {
    icon.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn extract_exe_path(exec: &str) -> Option<String> {
    let re = Regex::new(r#"(?i)(?:"([^"]+\.exe)"|'([^']+\.exe)'|([^\s]+\.exe))"#).ok()?;
    let captures = re.captures(exec)?;

    for idx in 1..=3 {
        if let Some(m) = captures.get(idx) {
            let value = m.as_str().trim();
            if !value.is_empty() {
                return Some(value.replace("\\\\", "\\"));
            }
        }
    }

    None
}

fn is_broken_icon_path(icon: &str) -> bool {
    let looks_like_path = icon.starts_with('/')
        || icon.starts_with("./")
        || icon.starts_with("../")
        || icon.contains('/');

    looks_like_path && !Path::new(icon).exists()
}

fn has_fixed_icon_file(icon: Option<&str>) -> bool {
    icon.map(|value| {
        let path = Path::new(value);
        path.is_absolute() && path.exists()
    })
    .unwrap_or(false)
}

fn icon_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(home) = env::var("HOME") {
        roots.push(PathBuf::from(&home).join(".local/share/icons"));
        roots.push(PathBuf::from(&home).join(".icons"));
    }

    roots.push(PathBuf::from("/usr/share/icons"));
    roots.push(PathBuf::from("/usr/local/share/icons"));
    roots.push(PathBuf::from("/usr/share/pixmaps"));

    roots
}

fn candidate_score(path: &Path) -> i32 {
    let lower = path.to_string_lossy().to_lowercase();
    let mut score = 0;

    if lower.contains("/hicolor/") {
        score += 80;
    }
    if lower.contains("/usr/share/pixmaps/") {
        score += 55;
    }
    if lower.contains("breeze-dark") {
        score += 35;
    }
    if lower.contains("breeze") {
        score += 30;
    }
    if lower.contains("/apps/") {
        score += 20;
    }
    if lower.contains("/app/") {
        score += 15;
    }

    if lower.contains("symbolic") {
        score -= 40;
    }
    if lower.contains("char-white") {
        score -= 60;
    }
    if lower.contains("char-black") {
        score -= 60;
    }

    if lower.contains("/16/") || lower.contains("16x16") {
        score -= 50;
    }
    if lower.contains("/22/") || lower.contains("22x22") {
        score -= 35;
    }
    if lower.contains("/24/") || lower.contains("24x24") {
        score -= 30;
    }
    if lower.contains("/32/") || lower.contains("32x32") {
        score -= 15;
    }
    if lower.contains("/48/") || lower.contains("48x48") {
        score += 5;
    }
    if lower.contains("/64/") || lower.contains("64x64") {
        score += 10;
    }
    if lower.contains("/96/") || lower.contains("96x96") {
        score += 14;
    }
    if lower.contains("/128/") || lower.contains("128x128") {
        score += 20;
    }
    if lower.contains("/256/") || lower.contains("256x256") {
        score += 30;
    }
    if lower.contains("/512/") || lower.contains("512x512") {
        score += 35;
    }
    if lower.contains("scalable") {
        score += 8;
    }

    if lower.ends_with(".png") {
        score += 14;
    }
    if lower.ends_with(".svg") {
        score += 6;
    }
    if lower.ends_with(".xpm") {
        score += 2;
    }
    if lower.ends_with(".ico") {
        score += 1;
    }

    score
}

fn try_absolute_icon_candidates(icon: &str) -> Option<String> {
    let path = Path::new(icon);

    if path.is_absolute() {
        if path.exists() {
            return Some(
                fs::canonicalize(path)
                    .unwrap_or_else(|_| path.to_path_buf())
                    .to_string_lossy()
                    .to_string(),
            );
        }

        for ext in ["png", "svg", "xpm", "ico"] {
            let candidate = PathBuf::from(format!("{icon}.{ext}"));
            if candidate.exists() {
                return Some(
                    fs::canonicalize(&candidate)
                        .unwrap_or(candidate)
                        .to_string_lossy()
                        .to_string(),
                );
            }
        }
    }

    None
}

fn system_theme_icon_lookup(icon: &str) -> Option<String> {
    let script = r#"
from xdg.IconTheme import getIconPath
import sys

name = sys.argv[1].strip()
if not name:
    raise SystemExit(1)

path = None
for size in (256, 128, 96, 64, 48, 32, 24, 22, 16):
    path = getIconPath(name, size=size, extensions=['png', 'svg', 'xpm', 'ico'])
    if path:
        print(path)
        break
"#;

    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(icon)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        return None;
    }

    let path = PathBuf::from(value);
    if path.exists() {
        Some(
            fs::canonicalize(&path)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string(),
        )
    } else {
        None
    }
}

fn resolve_icon_path_uncached(icon: &str) -> Option<String> {
    if let Some(found) = try_absolute_icon_candidates(icon) {
        return Some(found);
    }

    if let Some(found) = system_theme_icon_lookup(icon) {
        return Some(found);
    }

    let icon_path = Path::new(icon);

    let icon_name = icon_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| icon.to_string());

    let icon_stem = icon_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| icon_name.clone());

    let mut wanted = vec![icon_name.clone(), icon_stem.clone()];
    wanted.sort();
    wanted.dedup();

    let mut best: Option<(i32, PathBuf)> = None;

    for root in icon_search_roots() {
        if !root.exists() {
            continue;
        }

        for entry in WalkDir::new(&root)
            .max_depth(8)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase());

            let supported =
                matches!(ext.as_deref(), Some("png") | Some("svg") | Some("xpm") | Some("ico"));
            if !supported {
                continue;
            }

            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let file_stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            let matches_name = wanted.iter().any(|wanted_name| {
                file_name.eq_ignore_ascii_case(wanted_name)
                    || file_stem.eq_ignore_ascii_case(wanted_name)
            });

            if !matches_name {
                continue;
            }

            let score = candidate_score(path);
            match &best {
                Some((best_score, _)) if *best_score >= score => {}
                _ => best = Some((score, path.to_path_buf())),
            }
        }
    }

    best.map(|(_, path)| {
        fs::canonicalize(&path)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string()
    })
}

fn icon_resolve_cache() -> &'static Mutex<HashMap<String, Option<String>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn resolve_icon_path(icon: Option<&str>) -> Option<String> {
    let icon = icon?.trim();
    if icon.is_empty() {
        return None;
    }

    if let Ok(cache) = icon_resolve_cache().lock() {
        if let Some(value) = cache.get(icon).cloned() {
            return value;
        }
    }

    let resolved = resolve_icon_path_uncached(icon);

    if let Ok(mut cache) = icon_resolve_cache().lock() {
        cache.insert(icon.to_string(), resolved.clone());
    }

    resolved
}

fn build_message(status: &str, icon: Option<&str>, target_path: Option<&str>) -> Option<String> {
    let message = match status {
        "ok" => "Launcher looks healthy.",
        "missing_icon" => "Launcher has no icon.",
        "broken_icon_path" => "Icon path is set, but the file does not exist.",
        "exe_detected_needs_fixed_icon" => "Windows EXE target detected. Ready for icon repair.",
        "missing_exec_target" => "Windows EXE target detected, but the file does not exist.",
        "invalid_desktop_file" => "Desktop file is invalid or unreadable.",
        "unsupported_exec" => "Exec is currently outside the main repair flow.",
        "direct_exe_link" => {
            "Desktop item is a direct link to a Windows EXE, not a .desktop launcher."
        }
        _ => "Unknown status.",
    };

    let mut full = String::from(message);

    if let Some(icon) = icon {
        if !icon.is_empty() {
            full.push_str(" Icon=");
            full.push_str(icon);
            full.push('.');
        }
    }

    if let Some(target_path) = target_path {
        if !target_path.is_empty() {
            full.push_str(" Target=");
            full.push_str(target_path);
            full.push('.');
        }
    }

    Some(full)
}

pub fn build_direct_exe_link(path: &Path, target: &Path) -> LauncherEntry {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown".to_string());

    let path_string = path.to_string_lossy().to_string();
    let target_string = target.to_string_lossy().to_string();

    LauncherEntry {
        name,
        path: path_string,
        exec: target_string.clone(),
        icon: None,
        resolved_icon_path: None,
        status: "direct_exe_link".to_string(),
        target_path: Some(target_string.clone()),
        message: build_message("direct_exe_link", None, Some(&target_string)),
        backup_path: None,
        can_restore_default_icon: false,
    }
}

pub fn build_launcher_from_path(path: &Path) -> LauncherEntry {
    let path_string = path.to_string_lossy().to_string();

    match parse_desktop_file(path) {
        Ok(file) => {
            let icon = normalize_icon(file.icon);
            let resolved_icon_path = resolve_icon_path(icon.as_deref());
            let target_path = extract_exe_path(&file.exec);
            let can_restore_default_icon =
                desktop_extract_value(&file.raw, ORIGINAL_ICON_KEY).is_some();

            let missing_exec_target = target_path
                .as_deref()
                .map(|p| p.starts_with('/') && !Path::new(p).exists())
                .unwrap_or(false);

            let status = if file.exec.trim().is_empty() {
                "invalid_desktop_file"
            } else if let Some(icon_value) = icon.as_deref() {
                if is_broken_icon_path(icon_value) {
                    "broken_icon_path"
                } else if target_path.is_some() {
                    if missing_exec_target {
                        "missing_exec_target"
                    } else if has_fixed_icon_file(icon.as_deref()) || resolved_icon_path.is_some() {
                        "ok"
                    } else {
                        "exe_detected_needs_fixed_icon"
                    }
                } else {
                    "ok"
                }
            } else if target_path.is_some() {
                if missing_exec_target {
                    "missing_exec_target"
                } else {
                    "exe_detected_needs_fixed_icon"
                }
            } else {
                "missing_icon"
            };

            LauncherEntry {
                name: file.name,
                path: path_string,
                exec: file.exec,
                icon: icon.clone(),
                resolved_icon_path,
                status: status.to_string(),
                target_path: target_path.clone(),
                message: build_message(status, icon.as_deref(), target_path.as_deref()),
                backup_path: None,
                can_restore_default_icon,
            }
        }
        Err(error) => LauncherEntry {
            name: path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Unknown".to_string()),
            path: path_string,
            exec: String::new(),
            icon: None,
            resolved_icon_path: None,
            status: "invalid_desktop_file".to_string(),
            target_path: None,
            message: Some(format!("Desktop file could not be read: {}", error)),
            backup_path: None,
            can_restore_default_icon: false,
        },
    }
}

pub fn check_launcher(path: String) -> LauncherEntry {
    let path_ref = Path::new(&path);

    let is_desktop = path_ref
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("desktop"))
        .unwrap_or(false);

    if is_desktop {
        return build_launcher_from_path(path_ref);
    }

    match std::fs::canonicalize(path_ref) {
        Ok(target) => {
            let is_exe = target
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("exe"))
                .unwrap_or(false);

            if is_exe {
                build_direct_exe_link(path_ref, &target)
            } else {
                LauncherEntry {
                    name: path_ref
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| "Unknown".to_string()),
                    path,
                    exec: String::new(),
                    icon: None,
                    resolved_icon_path: None,
                    status: "unsupported_exec".to_string(),
                    target_path: Some(target.to_string_lossy().to_string()),
                    message: Some(
                        "Desktop item is not a .desktop launcher and not a Windows EXE link."
                            .to_string(),
                    ),
                    backup_path: None,
                    can_restore_default_icon: false,
                }
            }
        }
        Err(_) => LauncherEntry {
            name: path_ref
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "Unknown".to_string()),
            path,
            exec: String::new(),
            icon: None,
            resolved_icon_path: None,
            status: "unsupported_exec".to_string(),
            target_path: None,
            message: Some("Desktop item is not a supported launcher type.".to_string()),
            backup_path: None,
            can_restore_default_icon: false,
        },
    }
}
