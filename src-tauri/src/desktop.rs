use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DesktopFile {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub raw: String,
}

fn fallback_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn read_key(raw: &str, key: &str) -> Option<String> {
    let mut in_desktop_entry = false;
    let prefix = format!("{key}=");

    for raw_line in raw.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_desktop_entry = line.eq_ignore_ascii_case("[Desktop Entry]");
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some(value) = line.strip_prefix(&prefix) {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

pub fn parse_desktop_file(path: &Path) -> Result<DesktopFile> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("Kann {} nicht lesen", path.display()))?;

    let name = read_key(&raw, "Name").unwrap_or_else(|| fallback_name(path));
    let exec = read_key(&raw, "Exec").unwrap_or_default();
    let icon = read_key(&raw, "Icon");

    Ok(DesktopFile {
        name,
        exec,
        icon,
        raw,
    })
}

fn write_key(raw: &str, key: &str, value: &str) -> String {
    let had_trailing_newline = raw.ends_with('\n');
    let mut out: Vec<String> = Vec::new();
    let mut in_desktop_entry = false;
    let mut replaced = false;
    let key_prefix = format!("{key}=");

    for raw_line in raw.lines() {
        let line = raw_line.to_string();
        let trimmed = raw_line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_desktop_entry && !replaced {
                out.push(format!("{key}={value}"));
                replaced = true;
            }

            in_desktop_entry = trimmed.eq_ignore_ascii_case("[Desktop Entry]");
            out.push(line);
            continue;
        }

        if in_desktop_entry && trimmed.starts_with(&key_prefix) && !replaced {
            out.push(format!("{key}={value}"));
            replaced = true;
            continue;
        }

        out.push(line);
    }

    if in_desktop_entry && !replaced {
        out.push(format!("{key}={value}"));
    }

    let mut updated = out.join("\n");
    if had_trailing_newline {
        updated.push('\n');
    }
    updated
}

pub fn set_icon_value(path: &Path, icon_path: &Path) -> Result<()> {
    let file = parse_desktop_file(path)?;
    let updated = write_key(&file.raw, "Icon", &icon_path.to_string_lossy());
    fs::write(path, updated).with_context(|| format!("Kann {} nicht schreiben", path.display()))?;
    Ok(())
}
