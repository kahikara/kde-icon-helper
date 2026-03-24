use std::env;
use std::path::PathBuf;
use std::process::Command;

pub fn find_in_path(command: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn command_exists(command: &str) -> bool {
    find_in_path(command).is_some()
}

pub fn tool_path_string(command: &str) -> Option<String> {
    find_in_path(command).map(|p| p.to_string_lossy().to_string())
}

pub fn first_non_empty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.to_string())
}

pub fn run_version_command(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    first_non_empty_line(&stdout).or_else(|| first_non_empty_line(&stderr))
}
