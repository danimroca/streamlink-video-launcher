use std::process::Command;

pub fn find_player() -> Option<String> {
    for name in &["mpv", "vlc", "celluloid"] {
        if binary_exists(name) {
            return Some(name.to_string());
        }
    }
    None
}

fn binary_exists(name: &str) -> bool {
    let (cmd, arg) = if cfg!(windows) { ("where", name) } else { ("which", name) };
    Command::new(cmd)
        .arg(arg)
        .output()
        .ok()
        .is_some_and(|o| o.status.success())
}

pub fn launch_player(player: &str, urls: &[String]) -> Result<(), String> {
    let mut cmd = Command::new(player);
    for url in urls {
        cmd.arg(url);
    }
    cmd.spawn()
        .map_err(|e| format!("Failed to launch player: {e}"))?;
    Ok(())
}

pub fn is_youtube_url(url: &str) -> bool {
    url.contains("youtube.com") || url.contains("youtu.be")
}

pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let status = Command::new("xdg-open").arg(url).status();
    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(url).status();
    #[cfg(target_os = "windows")]
    let status = Command::new("cmd").args(["/c", "start", "", url]).status();
    status.map_err(|e| format!("Failed to open URL: {e}"))?;
    Ok(())
}

pub fn open_in_default_player(path: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let status = Command::new("xdg-open").arg(path).status();
    #[cfg(target_os = "macos")]
    let status = Command::new("open").arg(path).status();
    #[cfg(target_os = "windows")]
    let status = Command::new("cmd").args(["/c", "start", "", path]).status();
    status.map_err(|e| format!("Failed to open file: {e}"))?;
    Ok(())
}
