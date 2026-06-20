use std::process::Command;

fn new_command(program: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new(program);
        cmd.creation_flags(0x08000000);
        cmd
    }
    #[cfg(not(windows))]
    {
        Command::new(program)
    }
}

pub fn find_player() -> Option<String> {
    for name in &["mpv", "vlc", "celluloid"] {
        if binary_exists(name) {
            return Some(name.to_string());
        }
    }

    #[cfg(windows)]
    {
        let common_paths = &[
            r"C:\Program Files\VideoLAN\VLC\vlc.exe",
            r"C:\Program Files (x86)\VideoLAN\VLC\vlc.exe",
        ];
        for path in common_paths {
            if std::path::Path::new(path).exists() {
                return Some(path.to_string());
            }
        }
    }

    None
}

fn binary_exists(name: &str) -> bool {
    let (cmd, arg) = if cfg!(windows) { ("where", name) } else { ("which", name) };
    new_command(cmd)
        .arg(arg)
        .output()
        .ok()
        .is_some_and(|o| o.status.success())
}

pub fn launch_player(player: &str, urls: &[String]) -> Result<(), String> {
    let mut cmd = new_command(player);
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
    let status = new_command("xdg-open").arg(url).status();
    #[cfg(target_os = "macos")]
    let status = new_command("open").arg(url).status();
    #[cfg(target_os = "windows")]
    let status = new_command("cmd").args(["/c", "start", "", url]).status();
    status.map_err(|e| format!("Failed to open URL: {e}"))?;
    Ok(())
}

pub fn open_in_default_player(path: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    let status = new_command("xdg-open").arg(path).status();
    #[cfg(target_os = "macos")]
    let status = new_command("open").arg(path).status();
    #[cfg(target_os = "windows")]
    let status = new_command("cmd").args(["/c", "start", "", path]).status();
    status.map_err(|e| format!("Failed to open file: {e}"))?;
    Ok(())
}

// Creates a Command with console window suppression on Windows
pub fn silent(program: impl AsRef<std::path::Path>) -> Command {
    new_command(program.as_ref().to_str().unwrap_or_default())
}
