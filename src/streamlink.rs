use crate::platform;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct StreamInfo {
    pub title: Option<String>,
}

fn binary() -> PathBuf {
    let name = if cfg!(windows) { "streamlink.exe" } else { "streamlink" };
    if platform::silent(name)
        .arg("--version")
        .output()
        .ok()
        .is_some_and(|o| o.status.success())
    {
        return PathBuf::from(name);
    }
    crate::install::tool_path(name).unwrap_or_else(|| PathBuf::from(name))
}

pub fn exists() -> bool {
    let path = binary();
    platform::silent(&path)
        .arg("--version")
        .output()
        .ok()
        .is_some_and(|o| o.status.success())
}

pub fn resolve(url: &str) -> Result<StreamInfo, String> {
    let output = platform::silent(binary())
        .args(["--json", url])
        .output()
        .map_err(|e| format!("Failed to run streamlink: {e}"))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse streamlink output: {e}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.trim().to_string())
    }
}

pub fn play(url: &str, quality: &str, player: &str) -> Result<(), String> {
    platform::silent(binary())
        .args(["--player", player, url, quality])
        .spawn()
        .map_err(|e| format!("Failed to launch streamlink: {e}"))?;
    Ok(())
}

pub fn download(url: &str, quality: &str, output: &str) -> Result<(), String> {
    platform::silent(binary())
        .args(["-o", output, url, quality])
        .spawn()
        .map_err(|e| format!("Failed to start download: {e}"))?;
    Ok(())
}
