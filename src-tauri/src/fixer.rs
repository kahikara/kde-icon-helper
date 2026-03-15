use crate::checker;
use crate::desktop::set_icon_value;
use crate::models::FixResult;
use crate::paths::{iconhelper_backup_dir, iconhelper_icons_dir};
use anyhow::{bail, Context, Result};
use chrono::Local;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const ORIGINAL_ICON_KEY: &str = "X-KdeIconHelperOriginalIcon";
const ORIGINAL_ICON_EMPTY_SENTINEL: &str = "__EMPTY__";

fn safe_file_stem(path: &Path) -> String {
    let base = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "launcher".into());

    let mut out = String::new();
    for ch in base.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }

    let cleaned = out.trim_matches('_').to_string();
    if cleaned.is_empty() {
        "launcher".into()
    } else {
        cleaned
    }
}

fn ensure_dirs() -> Result<()> {
    fs::create_dir_all(iconhelper_icons_dir())?;
    fs::create_dir_all(iconhelper_backup_dir())?;
    Ok(())
}

fn backup_desktop_file(path: &Path) -> Result<PathBuf> {
    ensure_dirs()?;

    let stamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "launcher.desktop".into());

    let backup = iconhelper_backup_dir().join(format!("{}_{}", stamp, filename));
    fs::copy(path, &backup)
        .with_context(|| format!("Konnte Backup fuer {} nicht anlegen", path.display()))?;

    Ok(backup)
}

fn move_desktop_item_to_backup(path: &Path) -> Result<PathBuf> {
    ensure_dirs()?;

    let stamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "desktop_item".into());

    let backup = iconhelper_backup_dir().join(format!("{}_{}", stamp, filename));
    fs::rename(path, &backup)
        .with_context(|| format!("Konnte {} nicht ins Backup verschieben", path.display()))?;

    Ok(backup)
}

fn biggest_png_in(dir: &Path) -> Result<Option<PathBuf>> {
    let mut best: Option<(u64, PathBuf)> = None;

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();

        let is_png = path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("png"))
            .unwrap_or(false);

        if !is_png {
            continue;
        }

        let size = fs::metadata(&path)?.len();

        match &best {
            Some((best_size, _)) if *best_size >= size => {}
            _ => best = Some((size, path)),
        }
    }

    Ok(best.map(|(_, path)| path))
}

fn extract_or_fallback_exe_icon(exe_path: &Path) -> Result<PathBuf> {
    ensure_dirs()?;

    let base_name = safe_file_stem(exe_path);
    let output_png = iconhelper_icons_dir().join(format!("{base_name}.png"));
    let temp_dir = std::env::temp_dir().join(format!("kde-icon-helper-{}", std::process::id()));

    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir)?;

    let ico_path = temp_dir.join("app.ico");

    let wrestool_output = Command::new("wrestool")
        .arg("-x")
        .arg("-t14")
        .arg(exe_path)
        .output()
        .context("wrestool fehlt oder kann nicht gestartet werden")?;

    if wrestool_output.status.success() && !wrestool_output.stdout.is_empty() {
        fs::write(&ico_path, wrestool_output.stdout)?;

        let icotool_status = Command::new("icotool")
            .arg("-x")
            .arg(&ico_path)
            .arg("-o")
            .arg(&temp_dir)
            .status()
            .context("icotool fehlt oder kann nicht gestartet werden")?;

        if icotool_status.success() {
            if let Some(found_png) = biggest_png_in(&temp_dir)? {
                fs::copy(&found_png, &output_png)?;
                let _ = fs::remove_dir_all(&temp_dir);
                return Ok(output_png);
            }
        }
    }

    let fallback =
        PathBuf::from("/usr/share/icons/breeze/mimetypes/128/application-x-ms-dos-executable.svg");

    if fallback.exists() {
        let magick_status = Command::new("magick")
            .arg(&fallback)
            .arg("-resize")
            .arg("256x256")
            .arg(&output_png)
            .status()
            .context("ImageMagick fehlt oder kann nicht gestartet werden")?;

        if magick_status.success() {
            let _ = fs::remove_dir_all(&temp_dir);
            return Ok(output_png);
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
    bail!("Kein EXE Icon extrahierbar und kein Fallback Icon konvertierbar")
}

fn write_direct_exe_launcher(
    destination: &Path,
    display_name: &str,
    target_exe: &Path,
    icon_png: &Path,
) -> Result<()> {
    let escaped_name = display_name.replace('\n', " ");
    let escaped_target = target_exe.to_string_lossy().replace('"', "\\\"");
    let icon_value = icon_png.to_string_lossy();

    let content = format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName={}\nExec=xdg-open \"{}\"\nIcon={}\nTerminal=false\nStartupNotify=false\n",
        escaped_name, escaped_target, icon_value
    );

    fs::write(destination, content)
        .with_context(|| format!("Konnte {} nicht schreiben", destination.display()))?;

    let mut perms = fs::metadata(destination)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(destination, perms)?;

    Ok(())
}

fn direct_launcher_path_for(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Desktop item hat keinen Parent Ordner"))?;

    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow::anyhow!("Desktop item hat keinen Dateinamen"))?;

    let candidate = parent.join(format!("{}.desktop", filename));

    if candidate.exists() {
        bail!("Ziel Launcher existiert bereits: {}", candidate.display());
    }

    Ok(candidate)
}

fn desktop_extract_value(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}=");
    text.lines()
        .find_map(|line| line.strip_prefix(&prefix).map(|v| v.trim().to_string()))
}

fn desktop_remove_key(text: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    let mut out = Vec::new();

    for line in text.lines() {
        if line.starts_with(&prefix) {
            continue;
        }
        out.push(line.to_string());
    }

    let mut result = out.join("\n");
    result.push('\n');
    result
}

fn desktop_upsert_value(text: &str, key: &str, value: &str) -> String {
    let prefix = format!("{key}=");
    let mut out = Vec::new();
    let mut replaced = false;

    for line in text.lines() {
        if line.starts_with(&prefix) {
            out.push(format!("{key}={value}"));
            replaced = true;
        } else {
            out.push(line.to_string());
        }
    }

    if !replaced {
        out.push(format!("{key}={value}"));
    }

    let mut result = out.join("\n");
    result.push('\n');
    result
}

fn fix_desktop_launcher_internal(path: &Path) -> Result<FixResult> {
    let current = checker::check_launcher(path.to_string_lossy().to_string());

    let target = current
        .target_path
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Kein EXE Ziel gefunden"))?;

    let exe_path = PathBuf::from(&target);

    if !exe_path.exists() {
        bail!("EXE Ziel existiert nicht: {}", exe_path.display());
    }

    let backup = backup_desktop_file(path)?;
    let icon_png = extract_or_fallback_exe_icon(&exe_path)?;
    set_icon_value(path, &icon_png)?;

    let mut updated = checker::check_launcher(path.to_string_lossy().to_string());
    updated.backup_path = Some(backup.to_string_lossy().to_string());

    Ok(FixResult {
        ok: true,
        path: path.to_string_lossy().to_string(),
        message: format!(
            "Icon repariert. PNG={} Backup={}",
            icon_png.display(),
            backup.display()
        ),
        updated_entry: Some(updated),
    })
}

fn fix_direct_exe_link_internal(path: &Path) -> Result<FixResult> {
    let target_exe = fs::canonicalize(path)
        .with_context(|| format!("Konnte Ziel von {} nicht aufloesen", path.display()))?;

    let is_exe = target_exe
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("exe"))
        .unwrap_or(false);

    if !is_exe {
        bail!("Desktop item zeigt nicht auf eine EXE");
    }

    let icon_png = extract_or_fallback_exe_icon(&target_exe)?;
    let new_launcher = direct_launcher_path_for(path)?;

    let display_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Windows App".to_string());

    write_direct_exe_launcher(&new_launcher, &display_name, &target_exe, &icon_png)?;

    let backup = match move_desktop_item_to_backup(path) {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&new_launcher);
            return Err(error);
        }
    };

    let mut updated = checker::check_launcher(new_launcher.to_string_lossy().to_string());
    updated.backup_path = Some(backup.to_string_lossy().to_string());

    Ok(FixResult {
        ok: true,
        path: new_launcher.to_string_lossy().to_string(),
        message: format!(
            "Direct EXE link converted to launcher. Desktop item={} PNG={} Backup={}",
            new_launcher.display(),
            icon_png.display(),
            backup.display()
        ),
        updated_entry: Some(updated),
    })
}

pub fn fix_launcher_icon(path: String) -> FixResult {
    let path_buf = PathBuf::from(&path);
    let current = checker::check_launcher(path.clone());

    let result = match current.status.as_str() {
        "direct_exe_link" => fix_direct_exe_link_internal(&path_buf),
        "exe_detected_needs_fixed_icon" | "broken_icon_path" => {
            fix_desktop_launcher_internal(&path_buf)
        }
        _ => Err(anyhow::anyhow!(format!(
            "Aktuell nichts Fixbares erkannt. Status: {}",
            current.status
        ))),
    };

    match result {
        Ok(result) => result,
        Err(error) => {
            let mut updated = checker::check_launcher(path.clone());
            updated.backup_path = None;

            FixResult {
                ok: false,
                path,
                message: format!("Fix fehlgeschlagen: {}", error),
                updated_entry: Some(updated),
            }
        }
    }
}

pub fn fix_all_launchers() -> Vec<FixResult> {
    crate::scanner::scan_launchers()
        .into_iter()
        .filter(|entry| {
            matches!(
                entry.status.as_str(),
                "exe_detected_needs_fixed_icon" | "broken_icon_path" | "direct_exe_link"
            )
        })
        .map(|entry| fix_launcher_icon(entry.path))
        .collect()
}

fn import_manual_icon(source_icon: &Path, launcher_path: &Path) -> Result<PathBuf> {
    ensure_dirs()?;

    if !source_icon.exists() {
        bail!("Selected icon file does not exist: {}", source_icon.display());
    }

    let ext = source_icon
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if !matches!(ext.as_str(), "png" | "svg" | "xpm" | "ico") {
        bail!("Only PNG, SVG, XPM, and ICO files are supported for manual icons");
    }

    let manual_dir = iconhelper_icons_dir().join("manual");
    fs::create_dir_all(&manual_dir)?;

    let base = safe_file_stem(launcher_path);
    let destination = manual_dir.join(format!("{}_manual.{}", base, ext));

    fs::copy(source_icon, &destination).with_context(|| {
        format!(
            "Could not copy manual icon from {} to {}",
            source_icon.display(),
            destination.display()
        )
    })?;

    Ok(destination)
}

pub fn set_launcher_icon_manual(path: String, source_icon_path: String) -> FixResult {
    let path_buf = PathBuf::from(&path);
    let source_icon = PathBuf::from(&source_icon_path);

    let is_desktop = path_buf
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("desktop"))
        .unwrap_or(false);

    if !is_desktop {
        let updated = checker::check_launcher(path.clone());
        return FixResult {
            ok: false,
            path,
            message: "Manual icon override currently supports only .desktop launchers. Convert direct EXE links first.".to_string(),
            updated_entry: Some(updated),
        };
    }

    let result: Result<FixResult> = (|| {
        let backup = backup_desktop_file(&path_buf)?;
        let imported_icon = import_manual_icon(&source_icon, &path_buf)?;

        let original_text = fs::read_to_string(&path_buf)
            .with_context(|| format!("Could not read desktop file {}", path_buf.display()))?;

        let current_icon = desktop_extract_value(&original_text, "Icon");
        let with_original = if desktop_extract_value(&original_text, ORIGINAL_ICON_KEY).is_none() {
            let stored = current_icon
                .as_deref()
                .unwrap_or(ORIGINAL_ICON_EMPTY_SENTINEL);
            desktop_upsert_value(&original_text, ORIGINAL_ICON_KEY, stored)
        } else {
            original_text
        };

        let patched = desktop_upsert_value(
            &with_original,
            "Icon",
            &imported_icon.to_string_lossy(),
        );

        fs::write(&path_buf, patched)
            .with_context(|| format!("Could not write desktop file {}", path_buf.display()))?;

        let mut updated = checker::check_launcher(path.clone());
        updated.backup_path = Some(backup.to_string_lossy().to_string());

        Ok(FixResult {
            ok: true,
            path: path.clone(),
            message: format!(
                "Manual icon applied. Icon={} Backup={}",
                imported_icon.display(),
                backup.display()
            ),
            updated_entry: Some(updated),
        })
    })();

    match result {
        Ok(result) => result,
        Err(error) => {
            let updated = checker::check_launcher(path.clone());
            FixResult {
                ok: false,
                path,
                message: format!("Manual icon failed: {}", error),
                updated_entry: Some(updated),
            }
        }
    }
}

pub fn restore_launcher_icon_default(path: String) -> FixResult {
    let path_buf = PathBuf::from(&path);

    let is_desktop = path_buf
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("desktop"))
        .unwrap_or(false);

    if !is_desktop {
        let updated = checker::check_launcher(path.clone());
        return FixResult {
            ok: false,
            path,
            message: "Restore default icon currently supports only .desktop launchers.".to_string(),
            updated_entry: Some(updated),
        };
    }

    let result: Result<FixResult> = (|| {
        let original_text = fs::read_to_string(&path_buf)
            .with_context(|| format!("Could not read desktop file {}", path_buf.display()))?;

        let stored = desktop_extract_value(&original_text, ORIGINAL_ICON_KEY)
            .ok_or_else(|| anyhow::anyhow!("No stored default icon found for this launcher"))?;

        let backup = backup_desktop_file(&path_buf)?;
        let without_meta = desktop_remove_key(&original_text, ORIGINAL_ICON_KEY);

        let restored = if stored == ORIGINAL_ICON_EMPTY_SENTINEL {
            desktop_remove_key(&without_meta, "Icon")
        } else {
            desktop_upsert_value(&without_meta, "Icon", &stored)
        };

        fs::write(&path_buf, restored)
            .with_context(|| format!("Could not write desktop file {}", path_buf.display()))?;

        let mut updated = checker::check_launcher(path.clone());
        updated.backup_path = Some(backup.to_string_lossy().to_string());

        Ok(FixResult {
            ok: true,
            path: path.clone(),
            message: format!("Default icon restored. Backup={}", backup.display()),
            updated_entry: Some(updated),
        })
    })();

    match result {
        Ok(result) => result,
        Err(error) => {
            let updated = checker::check_launcher(path.clone());
            FixResult {
                ok: false,
                path,
                message: format!("Restore default icon failed: {}", error),
                updated_entry: Some(updated),
            }
        }
    }
}
