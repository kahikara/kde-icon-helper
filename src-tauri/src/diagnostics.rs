use crate::paths::desktop_dir;
use crate::tools::{find_in_path, run_version_command};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
            Some("Optional but helpful for theme icon lookup via xdg.IconTheme."),
            &["theme icon resolution"],
        ),
        build_tool_diagnostic(
            "wrestool",
            Some(&["--version"]),
            Some("Preferred EXE icon resource extractor."),
            &["EXE icon extraction"],
        ),
        build_tool_diagnostic(
            "icotool",
            Some(&["--version"]),
            Some("Preferred ICO unpacker after EXE resource extraction."),
            &["EXE icon extraction"],
        ),
        build_tool_diagnostic(
            "magick",
            Some(&["--version"]),
            Some("Used when SVG, XPM, or ICO fallback icons must be converted into PNG."),
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
