use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub url: String,
    pub title: String,
    pub timestamp: u64,
}

pub fn load(path: &Path) -> Vec<HistoryEntry> {
    if !path.exists() {
        return Vec::new();
    }
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

pub fn save(path: &Path, entries: &[HistoryEntry]) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(content) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(path, content);
    }
}
