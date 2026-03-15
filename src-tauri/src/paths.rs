use std::env;
use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
}

pub fn desktop_dir() -> PathBuf {
    home_dir().join("Desktop")
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
