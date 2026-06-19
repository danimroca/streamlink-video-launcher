use serde::Deserialize;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct StreamInfo {
    pub title: Option<String>,
}

pub fn exists() -> bool {
    if Command::new("streamlink")
        .arg("--version")
        .output()
        .ok()
        .is_some_and(|o| o.status.success())
    {
        return true;
    }
    if let Some(path) = crate::install::tool_path("streamlink") {
        if path.exists() {
            return true;
        }
    }
    false
}

pub fn resolve(url: &str) -> Result<StreamInfo, String> {
    let output = Command::new("streamlink")
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
    Command::new("streamlink")
        .args(["--player", player, url, quality])
        .spawn()
        .map_err(|e| format!("Failed to launch streamlink: {e}"))?;
    Ok(())
}

pub fn download(url: &str, quality: &str, output: &str) -> Result<(), String> {
    Command::new("streamlink")
        .args(["-o", output, url, quality])
        .spawn()
        .map_err(|e| format!("Failed to start download: {e}"))?;
    Ok(())
}
