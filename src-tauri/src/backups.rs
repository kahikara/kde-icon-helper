use crate::paths::iconhelper_backup_dir;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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
    pub original_path: Option<String>,
    pub restore_available: bool,
    pub restore_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRestoreResult {
    pub ok: bool,
    pub message: String,
    pub restored_path: Option<String>,
    pub safety_backup_path: Option<String>,
}

fn backup_sidecar_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "backup".to_string());

    path.with_file_name(format!("{file_name}.meta"))
}

fn read_original_path_from_sidecar(path: &Path) -> Option<String> {
    let sidecar = backup_sidecar_path(path);
    let text = fs::read_to_string(sidecar).ok()?;

    for line in text.lines() {
        if let Some(value) = line.strip_prefix("original_path=") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    None
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

fn backup_stamp() -> String {
    Local::now().format("%Y%m%d_%H%M%S_%3f").to_string()
}

fn unique_path_suffix(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    format!("{:08x}", hasher.finish() as u32)
}

fn create_restore_safety_backup(target: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(iconhelper_backup_dir()).map_err(|e| e.to_string())?;

    let stamp = backup_stamp();
    let suffix = unique_path_suffix(target);
    let filename = target
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "launcher.desktop".to_string());

    let backup = iconhelper_backup_dir().join(format!(
        "{}_{}_restore_safety_{}",
        stamp, suffix, filename
    ));

    fs::copy(target, &backup).map_err(|e| {
        format!(
            "Could not create restore safety backup from {} to {}: {}",
            target.display(),
            backup.display(),
            e
        )
    })?;

    let mut meta = fs::File::create(backup_sidecar_path(&backup)).map_err(|e| e.to_string())?;
    writeln!(meta, "original_path={}", target.to_string_lossy()).map_err(|e| e.to_string())?;

    Ok(backup)
}

pub fn list_backups() -> Vec<BackupEntry> {
    let backup_dir = iconhelper_backup_dir();

    let Ok(read_dir) = fs::read_dir(&backup_dir) else {
        return Vec::new();
    };

    let mut items: Vec<BackupEntry> = read_dir
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_file())
        .filter(|path| {
            path.file_name()
                .map(|name| !name.to_string_lossy().ends_with(".meta"))
                .unwrap_or(true)
        })
        .filter_map(|path| {
            let meta = fs::metadata(&path).ok()?;
            let modified = meta.modified().unwrap_or(UNIX_EPOCH);
            let kind = file_kind(&path);
            let original_path = read_original_path_from_sidecar(&path);

            let (restore_available, restore_reason) = if kind == "desktop" {
                if original_path.is_some() {
                    (true, None)
                } else {
                    (
                        false,
                        Some(
                            "No original path metadata. Only newer backups can be restored from the app."
                                .to_string(),
                        ),
                    )
                }
            } else {
                (
                    false,
                    Some("Only .desktop backups can be restored from the app.".to_string()),
                )
            };

            Some(BackupEntry {
                name: path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "backup".to_string()),
                path: path.to_string_lossy().to_string(),
                size_bytes: meta.len(),
                modified_unix_ms: modified_unix_ms(modified),
                modified_display: modified_display(modified),
                file_kind: kind,
                original_path,
                restore_available,
                restore_reason,
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

pub fn restore_backup(backup_path: String) -> Result<BackupRestoreResult, String> {
    let backup = PathBuf::from(&backup_path);

    if !backup.exists() || !backup.is_file() {
        return Err(format!("Backup not found: {}", backup.display()));
    }

    let is_desktop = backup
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("desktop"))
        .unwrap_or(false);

    if !is_desktop {
        return Err("Only .desktop backups can be restored from the app.".to_string());
    }

    let original_path = read_original_path_from_sidecar(&backup).ok_or_else(|| {
        "This backup has no original path metadata. It was likely created before restore support was added."
            .to_string()
    })?;

    let target = PathBuf::from(&original_path);

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Could not create restore target directory {}: {}",
                parent.display(),
                e
            )
        })?;
    }

    let safety_backup_path = if target.exists() {
        Some(create_restore_safety_backup(&target)?)
    } else {
        None
    };

    fs::copy(&backup, &target).map_err(|e| {
        format!(
            "Could not restore backup from {} to {}: {}",
            backup.display(),
            target.display(),
            e
        )
    })?;

    let mut perms = fs::metadata(&target)
        .map_err(|e| format!("Could not read restored file metadata {}: {}", target.display(), e))?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&target, perms)
        .map_err(|e| format!("Could not set permissions on {}: {}", target.display(), e))?;

    Ok(BackupRestoreResult {
        ok: true,
        message: match &safety_backup_path {
            Some(safety) => format!(
                "Backup restored to {}. Safety backup created at {}.",
                target.display(),
                safety.display()
            ),
            None => format!("Backup restored to {}.", target.display()),
        },
        restored_path: Some(target.to_string_lossy().to_string()),
        safety_backup_path: safety_backup_path.map(|p| p.to_string_lossy().to_string()),
    })
}
