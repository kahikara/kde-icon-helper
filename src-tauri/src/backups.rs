use crate::paths::iconhelper_backup_dir;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntry {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified_unix_ms: i64,
    pub modified_display: String,
    pub file_kind: String,
}

fn file_kind(path: &Path) -> String {
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("desktop"))
        .unwrap_or(false)
    {
        "desktop".to_string()
    } else {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .filter(|ext| !ext.is_empty())
            .unwrap_or_else(|| "file".to_string())
    }
}

fn modified_unix_ms(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn modified_display(time: SystemTime) -> String {
    let dt: DateTime<Local> = DateTime::<Local>::from(time);
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn list_backups() -> Vec<BackupEntry> {
    let backup_dir = iconhelper_backup_dir();

    let Ok(read_dir) = fs::read_dir(&backup_dir) else {
        return Vec::new();
    };

    let mut items: Vec<BackupEntry> = read_dir
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let meta = fs::metadata(&path).ok()?;
            let modified = meta.modified().unwrap_or(UNIX_EPOCH);

            Some(BackupEntry {
                name: path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "backup".to_string()),
                path: path.to_string_lossy().to_string(),
                size_bytes: meta.len(),
                modified_unix_ms: modified_unix_ms(modified),
                modified_display: modified_display(modified),
                file_kind: file_kind(&path),
            })
        })
        .collect();

    items.sort_by(|a, b| {
        b.modified_unix_ms
            .cmp(&a.modified_unix_ms)
            .then_with(|| a.name.cmp(&b.name))
    });

    items
}
