use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn home_dir() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

fn normalize_user_dir_path(raw: &str) -> Option<PathBuf> {
    let value = raw.trim().trim_matches('"');
    if value.is_empty() {
        return None;
    }

    if value == "$HOME" {
        return Some(home_dir());
    }

    if let Some(rest) = value.strip_prefix("$HOME/") {
        return Some(home_dir().join(rest));
    }

    let path = PathBuf::from(value);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(home_dir().join(path))
    }
}

fn desktop_dir_from_xdg_user_dir() -> Option<PathBuf> {
    let output = Command::new("xdg-user-dir").arg("DESKTOP").output().ok()?;
    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    normalize_user_dir_path(&raw)
}

fn desktop_dir_from_config() -> Option<PathBuf> {
    let config_path = home_dir().join(".config/user-dirs.dirs");
    let raw = fs::read_to_string(config_path).ok()?;

    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("XDG_DESKTOP_DIR=") {
            return normalize_user_dir_path(value);
        }
    }

    None
}

pub fn desktop_dir() -> PathBuf {
    let discovered = desktop_dir_from_xdg_user_dir()
        .or_else(desktop_dir_from_config)
        .unwrap_or_else(|| home_dir().join("Desktop"));

    if discovered.exists() {
        discovered
    } else {
        home_dir().join("Desktop")
    }
}

#[allow(dead_code)]
pub fn applications_dir() -> PathBuf {
    home_dir().join(".local/share/applications")
}

pub fn iconhelper_icons_dir() -> PathBuf {
    home_dir().join(".local/share/icons/iconhelper")
}

pub fn iconhelper_backup_dir() -> PathBuf {
    home_dir().join(".local/share/iconhelper/backups")
}
