use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct VideoInfo {
    pub title: Option<String>,
}

fn binary() -> PathBuf {
    let name = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };
    if Command::new(name)
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
    Command::new(path)
        .arg("--version")
        .output()
        .ok()
        .is_some_and(|o| o.status.success())
}

pub fn resolve(url: &str) -> Result<VideoInfo, String> {
    let output = Command::new(binary())
        .args(["--dump-json", url])
        .output()
        .map_err(|e| format!("Failed to run yt-dlp: {e}"))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse yt-dlp output: {e}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.trim().to_string())
    }
}

pub fn stream_urls(url: &str, format: &str) -> Result<Vec<String>, String> {
    let output = Command::new(binary())
        .args(["-g", "-f", format, url])
        .output()
        .map_err(|e| format!("Failed to get stream URL: {e}"))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let urls: Vec<String> = stdout.lines().map(|s| s.trim().to_string()).collect();
        if urls.is_empty() {
            Err("No stream URL returned".to_string())
        } else {
            Ok(urls)
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.trim().to_string())
    }
}

pub fn download(url: &str, format: &str, output: &str) -> Result<(), String> {
    Command::new(binary())
        .args(["-f", format, "-o", output, url])
        .spawn()
        .map_err(|e| format!("Failed to start download: {e}"))?;
    Ok(())
}

pub fn format_for_quality(label: &str) -> String {
    match label {
        "best" => "bestvideo+bestaudio/best".to_string(),
        "1080p" => "bestvideo[height<=1080]+bestaudio/best[height<=1080]".to_string(),
        "720p" => "bestvideo[height<=720]+bestaudio/best[height<=720]".to_string(),
        "480p" => "bestvideo[height<=480]+bestaudio/best[height<=480]".to_string(),
        "360p" => "bestvideo[height<=360]+bestaudio/best[height<=360]".to_string(),
        "audio-only" => "bestaudio/best".to_string(),
        "worst" => "worst".to_string(),
        other => other.to_string(),
    }
}
