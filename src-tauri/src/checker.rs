use crate::desktop::{desktop_extract_value, parse_desktop_file, DesktopFile};
use crate::models::{IconVariant, LauncherEntry};
use crate::paths::iconhelper_icons_dir;
use crate::tools::command_exists;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use walkdir::WalkDir;

const ORIGINAL_ICON_KEY: &str = "X-KdeIconHelperOriginalIcon";

#[derive(Debug, Clone)]
struct LauncherSourceInfo {
    label: String,
    detail: String,
    lookup_terms: Vec<String>,
}

#[derive(Debug, Clone)]
struct RankedVariantCandidate {
    path: PathBuf,
    source: String,
    score: i32,
    reason: String,
}

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

fn extract_appimage_path(exec: &str) -> Option<String> {
    let re =
        Regex::new(r#"(?i)(?:"([^"]+\.appimage)"|'([^']+\.appimage)'|([^\s]+\.appimage))"#).ok()?;
    let captures = re.captures(exec)?;

    for idx in 1..=3 {
        if let Some(m) = captures.get(idx) {
            let value = m.as_str().trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

fn extract_steam_app_id(path: &Path, file: &DesktopFile) -> Option<String> {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    let re_stem = Regex::new(r"(?i)steam_app_(\d+)").ok()?;
    if let Some(caps) = re_stem.captures(&stem) {
        if let Some(value) = caps.get(1) {
            return Some(value.as_str().to_string());
        }
    }

    let re_exec = Regex::new(r"steam://rungameid/(\d+)").ok()?;
    if let Some(caps) = re_exec.captures(&file.exec) {
        if let Some(value) = caps.get(1) {
            return Some(value.as_str().to_string());
        }
    }

    desktop_extract_value(&file.raw, "X-Steam-AppId")
}

fn path_basename_without_ext(raw: &str) -> Option<String> {
    let path = Path::new(raw.trim());
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.trim().is_empty())
}

fn push_lookup_term(terms: &mut Vec<String>, seen: &mut HashSet<String>, raw: &str) {
    let value = raw.trim().trim_matches('"').trim_matches('\'');
    if value.is_empty() {
        return;
    }

    let mut candidates = Vec::new();
    candidates.push(value.to_string());

    let path = Path::new(value);

    if let Some(name) = path.file_name().map(|s| s.to_string_lossy().to_string()) {
        candidates.push(name);
    }

    if let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) {
        candidates.push(stem);
    }

    if let Some(last_segment) = value.split('.').next_back() {
        if !last_segment.trim().is_empty() && last_segment != value {
            candidates.push(last_segment.to_string());
        }
    }

    for candidate in candidates {
        let normalized = candidate.trim().to_lowercase();
        if normalized.is_empty() {
            continue;
        }

        for variant in [
            normalized.clone(),
            normalized.replace(' ', "-"),
            normalized.replace(' ', "_"),
        ] {
            let trimmed = variant.trim().to_string();
            if !trimmed.is_empty() && seen.insert(trimmed.clone()) {
                terms.push(trimmed);
            }
        }
    }
}

fn build_launcher_source_info(
    path: &Path,
    file: &DesktopFile,
    target_path: Option<&str>,
) -> LauncherSourceInfo {
    let mut terms = Vec::new();
    let mut seen = HashSet::new();

    if let Some(icon) = file.icon.as_deref() {
        push_lookup_term(&mut terms, &mut seen, icon);
    }

    push_lookup_term(&mut terms, &mut seen, &file.name);

    if let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) {
        push_lookup_term(&mut terms, &mut seen, &stem);
    }

    let flatpak_id = desktop_extract_value(&file.raw, "X-Flatpak");
    if let Some(app_id) = flatpak_id.as_deref() {
        push_lookup_term(&mut terms, &mut seen, app_id);
        return LauncherSourceInfo {
            label: "Flatpak".to_string(),
            detail: format!("Flatpak app id {app_id}. Prefer Hicolor and matching app id icons."),
            lookup_terms: terms,
        };
    }

    if file.exec.to_lowercase().contains("flatpak run ") {
        if let Some(flatpak_app) = file
            .exec
            .split_whitespace()
            .find(|part| part.contains('.') && !part.starts_with('-'))
        {
            push_lookup_term(&mut terms, &mut seen, flatpak_app);
            return LauncherSourceInfo {
                label: "Flatpak".to_string(),
                detail: format!(
                    "Flatpak launcher using {flatpak_app}. Prefer Hicolor and matching app id icons."
                ),
                lookup_terms: terms,
            };
        }

        return LauncherSourceInfo {
            label: "Flatpak".to_string(),
            detail: "Flatpak launcher. Prefer Hicolor and matching app id icons.".to_string(),
            lookup_terms: terms,
        };
    }

    if let Some(app_id) = extract_steam_app_id(path, file) {
        push_lookup_term(&mut terms, &mut seen, &app_id);
        push_lookup_term(&mut terms, &mut seen, &format!("steam_icon_{app_id}"));
        return LauncherSourceInfo {
            label: "Steam".to_string(),
            detail: format!(
                "Steam launcher for app id {app_id}. Prefer steam specific or pixmaps style icons."
            ),
            lookup_terms: terms,
        };
    }

    if let Some(exe) = target_path {
        if let Some(exe_name) = path_basename_without_ext(exe) {
            push_lookup_term(&mut terms, &mut seen, &exe_name);
            return LauncherSourceInfo {
                label: "Wine".to_string(),
                detail: format!(
                    "Windows executable target {exe_name}. Prefer generated, local or app specific icons."
                ),
                lookup_terms: terms,
            };
        }

        return LauncherSourceInfo {
            label: "Wine".to_string(),
            detail: "Windows executable launcher. Prefer generated, local or app specific icons."
                .to_string(),
            lookup_terms: terms,
        };
    }

    if file.exec.to_lowercase().contains("wine")
        || file.exec.to_lowercase().contains("proton")
        || path.to_string_lossy().to_lowercase().contains("/wine/")
    {
        return LauncherSourceInfo {
            label: "Wine".to_string(),
            detail: "Wine style launcher. Prefer generated, local or app specific icons."
                .to_string(),
            lookup_terms: terms,
        };
    }

    if let Some(appimage_path) = extract_appimage_path(&file.exec) {
        if let Some(app_name) = path_basename_without_ext(&appimage_path) {
            push_lookup_term(&mut terms, &mut seen, &app_name);
            return LauncherSourceInfo {
                label: "AppImage".to_string(),
                detail: format!(
                    "AppImage target {app_name}. Prefer local, generated or bundled looking icons."
                ),
                lookup_terms: terms,
            };
        }

        return LauncherSourceInfo {
            label: "AppImage".to_string(),
            detail: "AppImage launcher. Prefer local, generated or bundled looking icons."
                .to_string(),
            lookup_terms: terms,
        };
    }

    LauncherSourceInfo {
        label: "Native KDE".to_string(),
        detail: "Standard KDE desktop launcher. Prefer Hicolor or Breeze family app icons."
            .to_string(),
        lookup_terms: terms,
    }
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
        score += 95;
    }
    if lower.contains("/usr/share/pixmaps/") {
        score += 80;
    }
    if lower.contains("breeze-dark") {
        score += 70;
    }
    if lower.contains("breeze") {
        score += 65;
    }
    if lower.contains("/apps/") {
        score += 28;
    }
    if lower.contains("/app/") {
        score += 20;
    }
    if lower.contains("/mimetypes/") {
        score -= 18;
    }
    if lower.contains("/actions/") {
        score -= 28;
    }

    if lower.contains("symbolic") {
        score -= 55;
    }
    if lower.contains("char-white") {
        score -= 60;
    }
    if lower.contains("char-black") {
        score -= 60;
    }

    if lower.contains("/16/") || lower.contains("16x16") {
        score -= 55;
    }
    if lower.contains("/22/") || lower.contains("22x22") {
        score -= 38;
    }
    if lower.contains("/24/") || lower.contains("24x24") {
        score -= 30;
    }
    if lower.contains("/32/") || lower.contains("32x32") {
        score -= 18;
    }
    if lower.contains("/48/") || lower.contains("48x48") {
        score += 6;
    }
    if lower.contains("/64/") || lower.contains("64x64") {
        score += 11;
    }
    if lower.contains("/96/") || lower.contains("96x96") {
        score += 15;
    }
    if lower.contains("/128/") || lower.contains("128x128") {
        score += 22;
    }
    if lower.contains("/256/") || lower.contains("256x256") {
        score += 34;
    }
    if lower.contains("/512/") || lower.contains("512x512") {
        score += 38;
    }
    if lower.contains("scalable") {
        score += 12;
    }

    if lower.ends_with(".png") {
        score += 14;
    }
    if lower.ends_with(".svg") {
        score += 8;
    }
    if lower.ends_with(".xpm") {
        score += 2;
    }
    if lower.ends_with(".ico") {
        score += 1;
    }

    score
}

fn best_term_match_score(path: &Path, terms: &[String]) -> i32 {
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let file_stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let path_lower = path.to_string_lossy().to_lowercase();
    let mut best = 0;

    for term in terms {
        if term.is_empty() {
            continue;
        }

        let score = if file_stem == *term {
            42
        } else if file_name == *term {
            34
        } else if file_name.starts_with(term) || file_stem.starts_with(term) {
            22
        } else if path_lower.contains(term) {
            10
        } else {
            0
        };

        if score > best {
            best = score;
        }
    }

    best
}

fn source_runtime_bonus(path: &Path, launcher_source: &str) -> i32 {
    let lower = path.to_string_lossy().to_lowercase();

    match launcher_source {
        "Flatpak" => {
            if lower.contains("/hicolor/") {
                24
            } else if lower.contains("breeze-dark") {
                12
            } else if lower.contains("breeze") {
                9
            } else {
                0
            }
        }
        "Steam" => {
            if lower.contains("steam_icon_") {
                32
            } else if lower.contains("/usr/share/pixmaps/") {
                16
            } else if lower.contains("/hicolor/") {
                10
            } else {
                0
            }
        }
        "Wine" => {
            if lower.contains("/iconhelper/") {
                28
            } else if lower.contains("/usr/share/pixmaps/") {
                10
            } else {
                0
            }
        }
        "AppImage" => {
            if lower.contains("/iconhelper/") {
                24
            } else if lower.contains("/usr/share/pixmaps/") {
                12
            } else {
                0
            }
        }
        _ => {
            if lower.contains("/hicolor/") {
                18
            } else if lower.contains("breeze-dark") {
                12
            } else if lower.contains("breeze") {
                10
            } else {
                0
            }
        }
    }
}

fn canonical_existing(path: &Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }

    Some(fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
}

fn absolute_icon_candidates(icon: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let path = Path::new(icon);

    if path.is_absolute() {
        if let Some(found) = canonical_existing(path) {
            out.push(found);
        }

        for ext in ["png", "svg", "xpm", "ico"] {
            let candidate = PathBuf::from(format!("{icon}.{ext}"));
            if let Some(found) = canonical_existing(&candidate) {
                out.push(found);
            }
        }
    }

    let mut seen = HashSet::new();
    out.retain(|path| seen.insert(path.to_string_lossy().to_string()));
    out
}

fn system_theme_icon_lookup(icon: &str) -> Option<String> {
    if !command_exists("python3") {
        return None;
    }

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

fn collect_icon_candidate_paths_uncached(icon: &str) -> Vec<PathBuf> {
    let trimmed = icon.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    collect_icon_candidate_paths_for_terms(&[trimmed.to_lowercase()])
}

fn collect_icon_candidate_paths_for_terms(terms: &[String]) -> Vec<PathBuf> {
    if terms.is_empty() {
        return Vec::new();
    }

    let mut wanted: Vec<String> = terms
        .iter()
        .map(|value| value.trim().to_lowercase())
        .filter(|value| !value.is_empty())
        .collect();

    wanted.sort();
    wanted.dedup();

    let mut scored: Vec<(i32, PathBuf)> = Vec::new();

    for term in &wanted {
        for path in absolute_icon_candidates(term) {
            scored.push((10_000 + best_term_match_score(&path, &wanted), path));
        }

        if let Some(found) = system_theme_icon_lookup(term) {
            let found_path = PathBuf::from(found);
            scored.push((9_000 + best_term_match_score(&found_path, &wanted), found_path));
        }
    }

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
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            let file_stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            let matches_name = wanted.iter().any(|wanted_name| {
                file_name == *wanted_name
                    || file_stem == *wanted_name
                    || file_name.starts_with(wanted_name)
                    || file_stem.starts_with(wanted_name)
            });

            if !matches_name {
                continue;
            }

            scored.push((candidate_score(path) + best_term_match_score(path, &wanted), path.to_path_buf()));
        }
    }

    scored.sort_by(|a, b| {
        b.0.cmp(&a.0).then_with(|| {
            a.1.to_string_lossy()
                .to_lowercase()
                .cmp(&b.1.to_string_lossy().to_lowercase())
        })
    });

    let mut out = Vec::new();
    let mut seen = HashSet::new();

    for (_, path) in scored {
        let canonical = fs::canonicalize(&path).unwrap_or(path);
        let key = canonical.to_string_lossy().to_string();
        if seen.insert(key) {
            out.push(canonical);
        }
        if out.len() >= 20 {
            break;
        }
    }

    out
}

fn resolve_icon_path_uncached(icon: &str) -> Option<String> {
    collect_icon_candidate_paths_uncached(icon)
        .into_iter()
        .next()
        .map(|path| path.to_string_lossy().to_string())
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

fn variant_source_label(path: &Path) -> String {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let manual_root = iconhelper_icons_dir().join("manual");
    let generated_root = iconhelper_icons_dir();
    let lower = canonical.to_string_lossy().to_lowercase();

    if canonical.starts_with(&manual_root) {
        return "Manual".to_string();
    }

    if canonical.starts_with(&generated_root) {
        return "Generated".to_string();
    }

    if lower.contains("/usr/share/pixmaps/") {
        return "Pixmaps".to_string();
    }

    if lower.contains("/hicolor/") {
        return "Hicolor".to_string();
    }

    if lower.contains("breeze-dark") {
        return "Breeze Dark".to_string();
    }

    if lower.contains("breeze") {
        return "Breeze".to_string();
    }

    if lower.contains("/usr/share/icons/")
        || lower.contains("/usr/local/share/icons/")
        || lower.contains("/.icons/")
    {
        return "Theme".to_string();
    }

    "Local".to_string()
}

fn variant_label(path: &Path) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Icon".to_string())
}

fn variant_size_reason(path: &Path) -> &'static str {
    let lower = path.to_string_lossy().to_lowercase();

    if lower.contains("/512/") || lower.contains("512x512") {
        "very high resolution"
    } else if lower.contains("/256/") || lower.contains("256x256") {
        "high resolution"
    } else if lower.contains("/128/") || lower.contains("128x128") {
        "good resolution"
    } else if lower.contains("/96/") || lower.contains("96x96") {
        "medium resolution"
    } else if lower.contains("/64/") || lower.contains("64x64") {
        "clean medium size"
    } else if lower.contains("/48/") || lower.contains("48x48") {
        "standard size"
    } else if lower.contains("scalable") {
        "scalable asset"
    } else if lower.contains("/32/") || lower.contains("32x32") {
        "smaller asset"
    } else {
        "usable asset"
    }
}

fn build_ranked_variant(
    path: PathBuf,
    launcher_source: &LauncherSourceInfo,
    lookup_terms: &[String],
) -> RankedVariantCandidate {
    let source = variant_source_label(&path);
    let score = candidate_score(&path)
        + best_term_match_score(&path, lookup_terms)
        + source_runtime_bonus(&path, &launcher_source.label);

    let size_reason = variant_size_reason(&path);

    let reason = match launcher_source.label.as_str() {
        "Flatpak" => format!("{source} candidate with {size_reason}. Preferred for Flatpak style launchers."),
        "Steam" => format!("{source} candidate with {size_reason}. Good match for Steam style launchers."),
        "Wine" => format!("{source} candidate with {size_reason}. Good match for Windows app launchers."),
        "AppImage" => format!("{source} candidate with {size_reason}. Good match for AppImage launchers."),
        _ => format!("{source} candidate with {size_reason}. Good match for a KDE desktop launcher."),
    };

    RankedVariantCandidate {
        path,
        source,
        score,
        reason,
    }
}

fn build_message(status: &str, _icon: Option<&str>, _target_path: Option<&str>) -> Option<String> {
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

    Some(message.to_string())
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
        launcher_source: "Direct EXE".to_string(),
        launcher_source_detail: "This item points directly to a Windows executable.".to_string(),
    }
}

pub fn build_launcher_from_path(path: &Path) -> LauncherEntry {
    let path_string = path.to_string_lossy().to_string();

    match parse_desktop_file(path) {
        Ok(file) => {
            let icon = normalize_icon(file.icon.clone());
            let resolved_icon_path = resolve_icon_path(icon.as_deref());
            let target_path = extract_exe_path(&file.exec);
            let can_restore_default_icon =
                desktop_extract_value(&file.raw, ORIGINAL_ICON_KEY).is_some();
            let source_info = build_launcher_source_info(path, &file, target_path.as_deref());

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
                launcher_source: source_info.label,
                launcher_source_detail: source_info.detail,
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
            launcher_source: "Unknown".to_string(),
            launcher_source_detail: "Launcher metadata could not be parsed.".to_string(),
        },
    }
}

pub fn list_icon_variants(path: String) -> Vec<IconVariant> {
    let path_ref = Path::new(&path);

    let is_desktop = path_ref
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("desktop"))
        .unwrap_or(false);

    if !is_desktop {
        return Vec::new();
    }

    let entry = build_launcher_from_path(path_ref);
    let file = match parse_desktop_file(path_ref) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };

    let source_info = build_launcher_source_info(path_ref, &file, entry.target_path.as_deref());
    let current_resolved = entry.resolved_icon_path.clone();

    if source_info.lookup_terms.is_empty() {
        return Vec::new();
    }

    let mut variants = Vec::new();
    let mut seen_paths = HashSet::new();

    if let Some(current_path) = current_resolved.as_deref() {
        let current_buf = PathBuf::from(current_path);
        let current_canonical = fs::canonicalize(&current_buf).unwrap_or(current_buf);
        let current_key = current_canonical.to_string_lossy().to_string();
        let current_source_label = variant_source_label(&current_canonical);

        seen_paths.insert(current_key.clone());

        variants.push(IconVariant {
            key: current_key.clone(),
            label: variant_label(&current_canonical),
            path: current_key,
            source: current_source_label,
            score: candidate_score(&current_canonical),
            recommended: false,
            reason: "Current resolved icon.".to_string(),
            is_current: true,
        });
    }

    let current_source = current_resolved
        .as_deref()
        .map(|current| variant_source_label(Path::new(current)));

    let mut ranked_candidates: Vec<RankedVariantCandidate> =
        collect_icon_candidate_paths_for_terms(&source_info.lookup_terms)
            .into_iter()
            .filter(|path| {
                let key = path.to_string_lossy().to_string();
                !seen_paths.contains(&key)
            })
            .map(|path| build_ranked_variant(path, &source_info, &source_info.lookup_terms))
            .filter(|candidate| {
                current_source
                    .as_deref()
                    .map(|source| source != candidate.source.as_str())
                    .unwrap_or(true)
            })
            .collect();

    ranked_candidates.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| {
                a.path
                    .to_string_lossy()
                    .to_lowercase()
                    .cmp(&b.path.to_string_lossy().to_lowercase())
            })
    });

    let mut chosen_per_source = Vec::new();
    let mut seen_sources = HashSet::new();

    for candidate in ranked_candidates {
        let source_key = candidate.source.to_lowercase();
        if seen_sources.insert(source_key) {
            chosen_per_source.push(candidate);
        }
        if chosen_per_source.len() >= 6 {
            break;
        }
    }

    for (index, candidate) in chosen_per_source.into_iter().enumerate() {
        let path_string = candidate.path.to_string_lossy().to_string();
        variants.push(IconVariant {
            key: path_string.clone(),
            label: variant_label(&candidate.path),
            path: path_string,
            source: candidate.source,
            score: candidate.score,
            recommended: index == 0,
            reason: candidate.reason,
            is_current: false,
        });
    }

    variants
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
                    launcher_source: "Other".to_string(),
                    launcher_source_detail:
                        "Desktop item is not a supported launcher type.".to_string(),
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
            launcher_source: "Other".to_string(),
            launcher_source_detail:
                "Desktop item is not a supported launcher type.".to_string(),
        },
    }
}
