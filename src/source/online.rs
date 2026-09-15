use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Clone, Default)]
pub struct OnlineResult {
    pub id: String,
    pub title: String,
    pub uploader: String,
}

fn command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Without this every call flashes a console window over the UI.
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

pub fn available() -> bool {
    command("yt-dlp")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tlk-tune")
        .join("stream")
}

pub fn search(query: &str, count: usize) -> Vec<OnlineResult> {
    let output = command("yt-dlp")
        .args([
            "-4",
            "--no-warnings",
            "--flat-playlist",
            "-j",
            &format!("ytsearch{}:{}", count, query),
        ])
        .stderr(Stdio::null())
        .output();

    let Ok(output) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);

    text.lines()
        .filter_map(|line| {
            let v: serde_json::Value = serde_json::from_str(line).ok()?;
            let id = v.get("id")?.as_str()?.to_string();
            let title = v.get("title")?.as_str()?.to_string();
            let uploader = v
                .get("uploader")
                .or_else(|| v.get("channel"))
                .and_then(|u| u.as_str())
                .unwrap_or("")
                .to_string();
            Some(OnlineResult {
                id,
                title,
                uploader,
            })
        })
        .collect()
}

/// Downloads the best audio-only stream into the cache and returns its path.
/// Playing from a local file keeps the decoder and seek logic identical to
/// local tracks.
pub fn resolve(id: &str) -> Option<PathBuf> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).ok()?;

    for entry in std::fs::read_dir(&dir).ok()?.flatten() {
        let path = entry.path();
        if path.file_stem().and_then(|s| s.to_str()) == Some(id) {
            return Some(path);
        }
    }

    let template = dir.join(format!("{}.%(ext)s", id));
    let status = command("yt-dlp")
        .args([
            "-4",
            "--no-warnings",
            "--no-playlist",
            "-f",
            "bestaudio[ext=m4a]/bestaudio",
            "-o",
        ])
        .arg(&template)
        .arg(format!("https://www.youtube.com/watch?v={}", id))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()?;
    if !status.success() {
        return None;
    }

    std::fs::read_dir(&dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.file_stem().and_then(|s| s.to_str()) == Some(id))
}

/// Saves a track to the user's music folder under its real title.
pub fn download(id: &str, target_dir: &PathBuf) -> Option<PathBuf> {
    std::fs::create_dir_all(target_dir).ok()?;
    let template = target_dir.join("%(title)s.%(ext)s");
    let output = command("yt-dlp")
        .args([
            "-4",
            "--no-warnings",
            "--no-playlist",
            "-x",
            "--audio-format",
            "mp3",
            "--audio-quality",
            "0",
            "--print",
            "after_move:filepath",
            "-o",
        ])
        .arg(&template)
        .arg(format!("https://www.youtube.com/watch?v={}", id))
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}
