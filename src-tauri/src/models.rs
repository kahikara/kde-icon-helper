use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherEntry {
    pub name: String,
    pub path: String,
    pub exec: String,
    pub icon: Option<String>,
    pub resolved_icon_path: Option<String>,
    pub status: String,
    pub target_path: Option<String>,
    pub message: Option<String>,
    pub backup_path: Option<String>,
    pub can_restore_default_icon: bool,
    pub launcher_source: String,
    pub launcher_source_detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixResult {
    pub ok: bool,
    pub path: String,
    pub message: String,
    pub updated_entry: Option<LauncherEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IconVariant {
    pub key: String,
    pub label: String,
    pub path: String,
    pub source: String,
    pub score: i32,
    pub recommended: bool,
    pub reason: String,
    pub is_current: bool,
}
