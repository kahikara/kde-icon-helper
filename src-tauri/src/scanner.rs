use crate::checker::{build_direct_exe_link, build_launcher_from_path};
use crate::models::LauncherEntry;
use crate::paths::desktop_dir;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn canonical_or_self(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn canonical_dedupe_key(path: &Path) -> String {
    canonical_or_self(path).to_string_lossy().to_string()
}

fn raw_path_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn is_desktop_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("desktop"))
        .unwrap_or(false)
}

fn is_exe_target(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
}

fn should_ignore_entry(entry: &LauncherEntry) -> bool {
    let name = entry.name.trim();
    let path_lower = entry.path.to_lowercase();
    let exec_lower = entry.exec.to_lowercase();

    name.eq_ignore_ascii_case("trash")
        || path_lower.ends_with("/trash.desktop")
        || path_lower.ends_with("/user-trash.desktop")
        || exec_lower.contains("trash:/")
}

pub fn scan_launchers() -> Vec<LauncherEntry> {
    let root = desktop_dir();
    let mut entries = Vec::new();
    let mut seen = BTreeSet::new();

    if !root.exists() {
        return entries;
    }

    let read_dir = match fs::read_dir(&root) {
        Ok(rd) => rd,
        Err(_) => return entries,
    };

    for item in read_dir.flatten() {
        let path = item.path();

        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let ft = meta.file_type();

        if !(ft.is_file() || ft.is_symlink()) {
            continue;
        }

        if is_desktop_file(&path) {
            let key = format!("desktop:{}", canonical_dedupe_key(&path));
            if seen.insert(key) {
                let entry = build_launcher_from_path(&path);
                if !should_ignore_entry(&entry) {
                    entries.push(entry);
                }
            }
            continue;
        }

        if ft.is_symlink() {
            let target = canonical_or_self(&path);
            if is_exe_target(&target) {
                let key = format!("exe:{}", raw_path_key(&path));
                if seen.insert(key) {
                    let entry = build_direct_exe_link(&path, &target);
                    if !should_ignore_entry(&entry) {
                        entries.push(entry);
                    }
                }
            }
        }
    }

    entries.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.path.cmp(&b.path))
    });

    entries
}
