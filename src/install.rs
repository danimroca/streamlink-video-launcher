use crate::platform;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Report {
    pub ytdlp: Option<String>,
    pub streamlink: Option<String>,
    pub bin_dir: PathBuf,
}

pub fn install_ytdlp() -> Result<String, String> {
    let dir = bin_dir()?;
    let dest = if cfg!(windows) {
        dir.join("yt-dlp.exe")
    } else {
        dir.join("yt-dlp")
    };

    if dest.exists() {
        return Ok(format!("yt-dlp already at {}", dest.display()));
    }

    let url = ytdlp_download_url();
    download_file(url, &dest)?;

    #[cfg(not(target_os = "windows"))]
    set_executable(&dest)?;

    Ok(format!("Installed yt-dlp to {}", dest.display()))
}

const STREAMLINK_INSTALL_URL: &str = "https://streamlink.github.io/install.html";

fn has_python() -> bool {
    ["python3", "python"].iter().any(|cmd| {
        platform::silent(cmd)
            .arg("--version")
            .output()
            .ok()
            .is_some_and(|o| o.status.success())
    })
}

pub fn install_streamlink() -> Result<String, String> {
    if !has_python() {
        let _ = crate::platform::open_url(STREAMLINK_INSTALL_URL);
        return Ok("Python not found. Opening browser to streamlink download page.".to_string());
    }

    for pip in &["pip3", "pip"] {
        let output = platform::silent(pip)
            .args(["install", "--user", "streamlink"])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let last = stdout.lines().rev().find(|l| !l.is_empty()).unwrap_or("Done");
                return Ok(format!("streamlink ({last})"));
            }
            _ => continue,
        }
    }

    let _ = crate::platform::open_url(STREAMLINK_INSTALL_URL);
    Ok("pip install failed. Opening browser to streamlink download page.".to_string())
}

pub fn bin_dir() -> Result<PathBuf, String> {
    if cfg!(windows) {
        let appdata =
            std::env::var("LOCALAPPDATA").map_err(|_| "LOCALAPPDATA not set".to_string())?;
        let dir = PathBuf::from(appdata)
            .join("streamlink-video-launcher")
            .join("bin");
        std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create directory: {e}"))?;
        Ok(dir)
    } else {
        let home = dirs::home_dir().ok_or("Cannot find home directory".to_string())?;
        let dir = home.join(".local").join("bin");
        std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create {dir:?}: {e}"))?;
        Ok(dir)
    }
}

pub fn tool_path(name: &str) -> Option<PathBuf> {
    bin_dir().ok().map(|d| d.join(name))
}

fn ytdlp_download_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"
    } else {
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp"
    }
}

fn download_file(url: &str, dest: &PathBuf) -> Result<(), String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    let mut body: Vec<u8> = Vec::new();
    let mut reader = response.into_reader();
    std::io::Read::read_to_end(&mut reader, &mut body)
        .map_err(|e| format!("Read response failed: {e}"))?;
    std::fs::write(dest, &body).map_err(|e| format!("Write to {} failed: {e}", dest.display()))?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_executable(path: &PathBuf) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| format!("Cannot set executable: {e}"))
}
