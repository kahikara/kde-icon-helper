use crate::paths::{iconhelper_backup_dir, iconhelper_icons_dir};
use crate::scanner;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedAssetStats {
    pub generated_icons_count: usize,
    pub generated_icons_bytes: u64,
    pub manual_icons_count: usize,
    pub manual_icons_bytes: u64,
    pub backups_count: usize,
    pub backups_bytes: u64,
    pub orphan_generated_icons_count: usize,
    pub orphan_generated_icons_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub dry_run: bool,
    pub removed_files_count: usize,
    pub removed_bytes: u64,
    pub removed_paths: Vec<String>,
    pub stats_before: GeneratedAssetStats,
    pub stats_after: GeneratedAssetStats,
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

fn direct_files_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return Vec::new();
    };

    read_dir
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file())
        .collect()
}

fn generated_icon_files() -> Vec<PathBuf> {
    let root = iconhelper_icons_dir();
    direct_files_in(&root)
        .into_iter()
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
        })
        .collect()
}

fn manual_icon_files() -> Vec<PathBuf> {
    direct_files_in(&iconhelper_icons_dir().join("manual"))
}

fn backup_files() -> Vec<PathBuf> {
    direct_files_in(&iconhelper_backup_dir())
}

fn canonical_or_original(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn normalize_if_under(base: &Path, candidate: &str) -> Option<PathBuf> {
    let candidate_path = PathBuf::from(candidate);
    if !candidate_path.is_absolute() {
        return None;
    }

    let base_norm = canonical_or_original(base);
    let candidate_norm = canonical_or_original(&candidate_path);

    if candidate_norm.starts_with(&base_norm) {
        Some(candidate_norm)
    } else {
        None
    }
}

fn referenced_generated_icon_paths() -> BTreeSet<PathBuf> {
    let icons_root = iconhelper_icons_dir();
    let mut referenced = BTreeSet::new();

    for entry in scanner::scan_launchers() {
        if let Some(icon) = entry.icon.as_deref() {
            if let Some(path) = normalize_if_under(&icons_root, icon) {
                referenced.insert(path);
            }
        }

        if let Some(resolved) = entry.resolved_icon_path.as_deref() {
            if let Some(path) = normalize_if_under(&icons_root, resolved) {
                referenced.insert(path);
            }
        }
    }

    referenced
}

fn orphan_generated_icon_files() -> Vec<PathBuf> {
    let referenced = referenced_generated_icon_paths();

    generated_icon_files()
        .into_iter()
        .map(|path| canonical_or_original(&path))
        .filter(|path| !referenced.contains(path))
        .collect()
}

fn sum_bytes(paths: &[PathBuf]) -> u64 {
    paths.iter().map(|path| file_size(path)).sum()
}

pub fn get_generated_asset_stats() -> GeneratedAssetStats {
    let generated = generated_icon_files();
    let manual = manual_icon_files();
    let backups = backup_files();
    let orphaned = orphan_generated_icon_files();

    let generated_icons_bytes = sum_bytes(&generated);
    let manual_icons_bytes = sum_bytes(&manual);
    let backups_bytes = sum_bytes(&backups);
    let orphan_generated_icons_bytes = sum_bytes(&orphaned);

    GeneratedAssetStats {
        generated_icons_count: generated.len(),
        generated_icons_bytes,
        manual_icons_count: manual.len(),
        manual_icons_bytes,
        backups_count: backups.len(),
        backups_bytes,
        orphan_generated_icons_count: orphaned.len(),
        orphan_generated_icons_bytes,
        total_bytes: generated_icons_bytes + manual_icons_bytes + backups_bytes,
    }
}

pub fn cleanup_generated_assets(dry_run: bool) -> CleanupResult {
    let stats_before = get_generated_asset_stats();
    let orphaned = orphan_generated_icon_files();

    let mut removed_paths: Vec<String> = Vec::new();
    let mut removed_bytes: u64 = 0;

    for path in orphaned {
        let size = file_size(&path);
        removed_paths.push(path.to_string_lossy().to_string());

        if !dry_run {
            if fs::remove_file(&path).is_ok() {
                removed_bytes += size;
            }
        } else {
            removed_bytes += size;
        }
    }

    let stats_after = if dry_run {
        stats_before.clone()
    } else {
        get_generated_asset_stats()
    };

    CleanupResult {
        dry_run,
        removed_files_count: removed_paths.len(),
        removed_bytes,
        removed_paths,
        stats_before,
        stats_after,
    }
}
