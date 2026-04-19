use crate::checker;
use crate::desktop::{
    desktop_extract_value, desktop_remove_key, desktop_upsert_value, set_icon_value,
};
use crate::models::FixResult;
use crate::paths::{iconhelper_backup_dir, iconhelper_icons_dir};
use crate::tools::{command_exists, tool_path_string};
use anyhow::{anyhow, bail, Context, Result};
use chrono::Local;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const ORIGINAL_ICON_KEY: &str = "X-KdeIconHelperOriginalIcon";
const ORIGINAL_ICON_EMPTY_SENTINEL: &str = "__EMPTY__";


fn backup_sidecar_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "backup".to_string());

    path.with_file_name(format!("{file_name}.meta"))
}

fn write_backup_metadata(backup: &Path, original_path: &Path) -> Result<()> {
    let sidecar = backup_sidecar_path(backup);
    fs::write(&sidecar, format!("original_path={}\n", original_path.to_string_lossy()))
        .with_context(|| format!("Could not write backup metadata {}", sidecar.display()))?;
    Ok(())
}

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

fn unique_path_suffix(path: &Path) -> String {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    format!("{:08x}", hasher.finish() as u32)
}

fn backup_stamp() -> String {
    Local::now().format("%Y%m%d_%H%M%S_%3f").to_string()
}

fn ensure_dirs() -> Result<()> {
    fs::create_dir_all(iconhelper_icons_dir())?;
    fs::create_dir_all(iconhelper_backup_dir())?;
    Ok(())
}

fn backup_desktop_file(path: &Path) -> Result<PathBuf> {
    ensure_dirs()?;

    let stamp = backup_stamp();
    let suffix = unique_path_suffix(path);
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "launcher.desktop".into());

    let backup = iconhelper_backup_dir().join(format!("{}_{}_{}", stamp, suffix, filename));
    fs::copy(path, &backup)
        .with_context(|| format!("Konnte Backup fuer {} nicht anlegen", path.display()))?;
    write_backup_metadata(&backup, path)?;

    Ok(backup)
}

fn move_desktop_item_to_backup(path: &Path) -> Result<PathBuf> {
    ensure_dirs()?;

    let stamp = backup_stamp();
    let suffix = unique_path_suffix(path);
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "desktop_item".into());

    let backup = iconhelper_backup_dir().join(format!("{}_{}_{}", stamp, suffix, filename));
    fs::rename(path, &backup)
        .with_context(|| format!("Konnte {} nicht ins Backup verschieben", path.display()))?;
    write_backup_metadata(&backup, path)?;

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

fn file_has_content(path: &Path) -> bool {
    fs::metadata(path).map(|meta| meta.len() > 0).unwrap_or(false)
}

fn command_failure_reason(name: &str) -> String {
    if let Some(path) = tool_path_string(name) {
        format!("{name} failed from {path}")
    } else {
        format!("{name} missing")
    }
}

fn known_exe_fallback_icon_candidates() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/usr/share/icons/hicolor/256x256/mimetypes/application-x-ms-dos-executable.png"),
        PathBuf::from("/usr/share/icons/hicolor/128x128/mimetypes/application-x-ms-dos-executable.png"),
        PathBuf::from("/usr/share/icons/hicolor/64x64/mimetypes/application-x-ms-dos-executable.png"),
        PathBuf::from("/usr/share/icons/hicolor/48x48/mimetypes/application-x-ms-dos-executable.png"),
        PathBuf::from("/usr/share/icons/hicolor/scalable/mimetypes/application-x-ms-dos-executable.svg"),
        PathBuf::from("/usr/share/icons/breeze/mimetypes/128/application-x-ms-dos-executable.svg"),
        PathBuf::from("/usr/share/icons/breeze-dark/mimetypes/128/application-x-ms-dos-executable.svg"),
        PathBuf::from("/usr/share/icons/hicolor/256x256/apps/wine.png"),
        PathBuf::from("/usr/share/icons/hicolor/128x128/apps/wine.png"),
        PathBuf::from("/usr/share/pixmaps/wine.xpm"),
    ]
}

fn find_known_exe_fallback_icon() -> Option<PathBuf> {
    known_exe_fallback_icon_candidates()
        .into_iter()
        .find(|path| path.exists())
}

fn copy_or_convert_fallback_icon(source: &Path, output_png: &Path) -> Result<()> {
    let ext = source
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "png" => {
            fs::copy(source, output_png).with_context(|| {
                format!(
                    "Could not copy fallback icon from {} to {}",
                    source.display(),
                    output_png.display()
                )
            })?;
            Ok(())
        }
        "svg" | "xpm" | "ico" => {
            if !command_exists("magick") {
                bail!(
                    "Fallback icon {} found, but ImageMagick 'magick' is missing for conversion",
                    source.display()
                );
            }

            let status = Command::new("magick")
                .arg(source)
                .arg("-resize")
                .arg("256x256")
                .arg(output_png)
                .status()
                .context("ImageMagick konnte nicht gestartet werden")?;

            if !status.success() {
                bail!(
                    "ImageMagick could not convert fallback icon {} into {}",
                    source.display(),
                    output_png.display()
                );
            }

            Ok(())
        }
        _ => bail!("Unsupported fallback icon format: {}", source.display()),
    }
}

fn try_extract_exe_icon_with_tools(
    exe_path: &Path,
    temp_dir: &Path,
    output_png: &Path,
    reasons: &mut Vec<String>,
) -> Result<Option<PathBuf>> {
    if !command_exists("wrestool") {
        reasons.push(command_failure_reason("wrestool"));
        return Ok(None);
    }

    if !command_exists("icotool") {
        reasons.push(command_failure_reason("icotool"));
        return Ok(None);
    }

    let ico_path = temp_dir.join("app.ico");

    let wrestool_output = Command::new("wrestool")
        .arg("-x")
        .arg("-t14")
        .arg(exe_path)
        .output()
        .context("wrestool konnte nicht gestartet werden")?;

    if !wrestool_output.status.success() {
        let stderr = String::from_utf8_lossy(&wrestool_output.stderr).trim().to_string();
        reasons.push(if stderr.is_empty() {
            "wrestool did not succeed while extracting EXE resources".to_string()
        } else {
            format!("wrestool failed: {}", stderr)
        });
        return Ok(None);
    }

    if wrestool_output.stdout.is_empty() {
        reasons.push("wrestool returned no icon resource data".to_string());
        return Ok(None);
    }

    fs::write(&ico_path, wrestool_output.stdout)
        .with_context(|| format!("Could not write temporary ICO {}", ico_path.display()))?;

    let icotool_status = Command::new("icotool")
        .arg("-x")
        .arg(&ico_path)
        .arg("-o")
        .arg(temp_dir)
        .status()
        .context("icotool konnte nicht gestartet werden")?;

    if !icotool_status.success() {
        reasons.push("icotool failed while unpacking the extracted ICO".to_string());
        return Ok(None);
    }

    if let Some(found_png) = biggest_png_in(temp_dir)? {
        fs::copy(&found_png, output_png).with_context(|| {
            format!(
                "Could not copy extracted PNG from {} to {}",
                found_png.display(),
                output_png.display()
            )
        })?;
        return Ok(Some(output_png.to_path_buf()));
    }

    reasons.push("icotool finished but no PNG frame was produced".to_string());
    Ok(None)
}

fn extract_or_fallback_exe_icon(exe_path: &Path) -> Result<PathBuf> {
    ensure_dirs()?;

    let base_name = safe_file_stem(exe_path);
    let suffix = unique_path_suffix(exe_path);
    let output_png = iconhelper_icons_dir().join(format!("{base_name}_{suffix}.png"));

    if file_has_content(&output_png) {
        return Ok(output_png);
    }

    let temp_dir =
        std::env::temp_dir().join(format!("kde-icon-helper-{}-{}", std::process::id(), suffix));

    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir)?;

    let mut reasons: Vec<String> = Vec::new();

    let result = (|| -> Result<PathBuf> {
        if let Some(extracted) =
            try_extract_exe_icon_with_tools(exe_path, &temp_dir, &output_png, &mut reasons)?
        {
            return Ok(extracted);
        }

        if let Some(fallback_icon) = find_known_exe_fallback_icon() {
            match copy_or_convert_fallback_icon(&fallback_icon, &output_png) {
                Ok(()) => return Ok(output_png.clone()),
                Err(error) => {
                    reasons.push(format!("fallback icon failed: {}", error));
                }
            }
        } else {
            reasons.push("no known system fallback icon was found".to_string());
        }

        let details = if reasons.is_empty() {
            "no further details available".to_string()
        } else {
            reasons.join(" | ")
        };

        Err(anyhow!(
            "Could not produce an EXE icon for {}. {}",
            exe_path.display(),
            details
        ))
    })();

    let _ = fs::remove_dir_all(&temp_dir);
    result
}


fn exec_uses_xdg_open_for_exe(exec: &str) -> bool {
    let lower = exec.to_ascii_lowercase();
    lower.contains("xdg-open") && lower.contains(".exe")
}

fn wine_exec_value_for(target_exe: &Path) -> String {
    let escaped_target = target_exe.to_string_lossy().replace('"', "\\\"");
    format!("wine \"{}\"", escaped_target)
}

fn maybe_repair_exe_launcher_exec(path: &Path, target_exe: &Path) -> Result<bool> {
    let original_text = fs::read_to_string(path)
        .with_context(|| format!("Could not read desktop file {}", path.display()))?;

    let current_exec = desktop_extract_value(&original_text, "Exec").unwrap_or_default();
    if !exec_uses_xdg_open_for_exe(&current_exec) {
        return Ok(false);
    }

    let mut patched = desktop_upsert_value(&original_text, "Exec", &wine_exec_value_for(target_exe));

    if let Some(parent) = target_exe.parent() {
        patched = desktop_upsert_value(&patched, "Path", &parent.to_string_lossy());
    }

    fs::write(path, patched)
        .with_context(|| format!("Could not write desktop file {}", path.display()))?;

    Ok(true)
}

fn write_direct_exe_launcher(
    destination: &Path,
    display_name: &str,
    target_exe: &Path,
    icon_png: &Path,
) -> Result<()> {
    let escaped_name = display_name.replace('\n', " ");
    let icon_value = icon_png.to_string_lossy();
    let exec_value = wine_exec_value_for(target_exe);
    let path_value = target_exe
        .parent()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();

    let content = format!(
        "[Desktop Entry]\nType=Application\nVersion=1.0\nName={}\nExec={}\nPath={}\nIcon={}\nTerminal=false\nStartupNotify=false\n",
        escaped_name, exec_value, path_value, icon_value
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
        .ok_or_else(|| anyhow!("Desktop item hat keinen Parent Ordner"))?;

    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Desktop item hat keinen Dateinamen"))?;

    let candidate = parent.join(format!("{}.desktop", filename));

    if candidate.exists() {
        bail!("Ziel Launcher existiert bereits: {}", candidate.display());
    }

    Ok(candidate)
}

fn preserve_original_icon_value(path: &Path) -> Result<()> {
    let original_text = fs::read_to_string(path)
        .with_context(|| format!("Could not read desktop file {}", path.display()))?;

    if desktop_extract_value(&original_text, ORIGINAL_ICON_KEY).is_some() {
        return Ok(());
    }

    let current_icon = desktop_extract_value(&original_text, "Icon");
    let stored = current_icon
        .as_deref()
        .unwrap_or(ORIGINAL_ICON_EMPTY_SENTINEL);

    let patched = desktop_upsert_value(&original_text, ORIGINAL_ICON_KEY, stored);

    fs::write(path, patched)
        .with_context(|| format!("Could not write desktop file {}", path.display()))?;

    Ok(())
}

fn fix_desktop_launcher_internal(path: &Path) -> Result<FixResult> {
    let current = checker::check_launcher(path.to_string_lossy().to_string());

    let target = current
        .target_path
        .clone()
        .ok_or_else(|| anyhow!("Kein EXE Ziel gefunden"))?;

    let exe_path = PathBuf::from(&target);

    if !exe_path.exists() {
        bail!("EXE Ziel existiert nicht: {}", exe_path.display());
    }

    let backup = backup_desktop_file(path)?;
    let icon_png = extract_or_fallback_exe_icon(&exe_path)?;
    preserve_original_icon_value(path)?;
    set_icon_value(path, &icon_png)?;
    let exec_repaired = maybe_repair_exe_launcher_exec(path, &exe_path)?;

    let mut updated = checker::check_launcher(path.to_string_lossy().to_string());
    updated.backup_path = Some(backup.to_string_lossy().to_string());

    let repair_note = if exec_repaired {
        format!(" Exec=wine and Path={} updated.", exe_path.parent().map(|p| p.display().to_string()).unwrap_or_default())
    } else {
        String::new()
    };

    Ok(FixResult {
        ok: true,
        path: path.to_string_lossy().to_string(),
        message: format!(
            "Launcher repariert. PNG={} Backup={}{}",
            icon_png.display(),
            backup.display(),
            repair_note
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
        _ => Err(anyhow!(format!(
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

fn import_manual_icon(source_icon: &Path, launcher_path: &Path) -> Result<PathBuf> {
    ensure_dirs()?;

    if !source_icon.exists() {
        bail!(
            "Selected icon file does not exist: {}",
            source_icon.display()
        );
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
    let suffix = unique_path_suffix(launcher_path);
    let destination = manual_dir.join(format!("{}_{}_manual.{}", base, suffix, ext));

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

        let patched =
            desktop_upsert_value(&with_original, "Icon", &imported_icon.to_string_lossy());

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
            message: "Restore default icon currently supports only .desktop launchers."
                .to_string(),
            updated_entry: Some(updated),
        };
    }

    let result: Result<FixResult> = (|| {
        let original_text = fs::read_to_string(&path_buf)
            .with_context(|| format!("Could not read desktop file {}", path_buf.display()))?;

        let stored = desktop_extract_value(&original_text, ORIGINAL_ICON_KEY)
            .ok_or_else(|| anyhow!("No stored default icon found for this launcher"))?;

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
