use crate::paths::desktop_dir;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDiagnostic {
    pub name: String,
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub note: Option<String>,
    pub required_for: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDiagnostics {
    pub desktop_dir: String,
    pub desktop_dir_exists: bool,
    pub tools: Vec<ToolDiagnostic>,
}

fn find_in_path(command: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn first_non_empty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.to_string())
}

fn run_version_command(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    first_non_empty_line(&stdout).or_else(|| first_non_empty_line(&stderr))
}

fn build_tool_diagnostic(
    name: &str,
    version_args: Option<&[&str]>,
    note: Option<&str>,
    required_for: &[&str],
) -> ToolDiagnostic {
    let found_path = find_in_path(name);
    let version = if found_path.is_some() {
        version_args.and_then(|args| run_version_command(name, args))
    } else {
        None
    };

    ToolDiagnostic {
        name: name.to_string(),
        found: found_path.is_some(),
        path: found_path.map(|p| p.to_string_lossy().to_string()),
        version,
        note: note.map(|v| v.to_string()),
        required_for: required_for.iter().map(|v| v.to_string()).collect(),
    }
}

pub fn get_runtime_diagnostics() -> RuntimeDiagnostics {
    let desktop = desktop_dir();

    let tools = vec![
        build_tool_diagnostic(
            "python3",
            Some(&["--version"]),
            Some("Used for theme icon lookup via xdg.IconTheme."),
            &["theme icon resolution"],
        ),
        build_tool_diagnostic(
            "wrestool",
            Some(&["--version"]),
            Some("Used to extract icon resources from Windows EXE files."),
            &["EXE icon extraction"],
        ),
        build_tool_diagnostic(
            "icotool",
            Some(&["--version"]),
            Some("Used to unpack ICO files extracted from EXEs."),
            &["EXE icon extraction"],
        ),
        build_tool_diagnostic(
            "magick",
            Some(&["--version"]),
            Some("Used to convert the fallback EXE icon into PNG."),
            &["fallback icon conversion"],
        ),
        build_tool_diagnostic(
            "xdg-user-dir",
            None,
            Some("Used to discover the configured XDG Desktop directory."),
            &["desktop path detection"],
        ),
    ];

    RuntimeDiagnostics {
        desktop_dir: desktop.to_string_lossy().to_string(),
        desktop_dir_exists: Path::new(&desktop).exists(),
        tools,
    }
}
