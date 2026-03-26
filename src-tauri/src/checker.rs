use crate::desktop::{desktop_extract_value, parse_desktop_file};
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

    let icon_path = Path::new(trimmed);

    let icon_name = icon_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| trimmed.to_string());

    let icon_stem = icon_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| icon_name.clone());

    let mut wanted = vec![icon_name.clone(), icon_stem.clone()];
    wanted.sort();
    wanted.dedup();

    let mut scored: Vec<(i32, PathBuf)> = Vec::new();

    for path in absolute_icon_candidates(trimmed) {
        scored.push((10_000, path));
    }

    if let Some(found) = system_theme_icon_lookup(trimmed) {
        scored.push((9_000, PathBuf::from(found)));
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

            scored.push((candidate_score(path), path.to_path_buf()));
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
        if out.len() >= 16 {
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

fn variant_source_reason(source: &str) -> &'static str {
    match source {
        "Hicolor" => "best general app icon match",
        "Breeze Dark" => "good KDE dark theme match",
        "Breeze" => "good KDE theme match",
        "Pixmaps" => "classic app icon source",
        "Manual" => "manual override source",
        "Generated" => "generated launcher asset",
        "Theme" => "theme provided icon source",
        "Local" => "local file based source",
        _ => "icon source",
    }
}

fn is_symbolic_variant(path: &Path) -> bool {
    path.to_string_lossy().to_lowercase().contains("symbolic")
}

fn build_variant_reason(source: &str, path: &Path) -> String {
    let base = variant_source_reason(source);
    let size = variant_size_reason(path);

    if is_symbolic_variant(path) {
        format!("{base}, but this one is symbolic and usually less ideal for launcher icons.")
    } else {
        format!("{base} with {size}.")
    }
}

fn build_ranked_variant(path: PathBuf) -> RankedVariantCandidate {
    let source = variant_source_label(&path);
    let score = candidate_score(&path);
    let reason = build_variant_reason(&source, &path);

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
    let current_resolved = entry.resolved_icon_path.clone();

    let icon_value = match entry.icon.as_deref() {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => return Vec::new(),
    };

    let current_source = current_resolved
        .as_deref()
        .map(|current| variant_source_label(Path::new(current)));

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

    let mut ranked_candidates: Vec<RankedVariantCandidate> = collect_icon_candidate_paths_uncached(&icon_value)
        .into_iter()
        .filter(|path| {
            let key = path.to_string_lossy().to_string();
            !seen_paths.contains(&key)
        })
        .map(build_ranked_variant)
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
