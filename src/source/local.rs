use std::path::{Path, PathBuf};
use std::time::SystemTime;

const EXTENSIONS: [&str; 10] = [
    "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma", "aiff", "mp4",
];

#[derive(Clone)]
pub struct LocalTrack {
    pub path: PathBuf,
    pub title: String,
    pub folder: String,
    pub modified: SystemTime,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Name,
    Artist,
    Folder,
    Recent,
}

impl SortMode {
    pub fn next(self) -> SortMode {
        match self {
            SortMode::Name => SortMode::Artist,
            SortMode::Artist => SortMode::Folder,
            SortMode::Folder => SortMode::Recent,
            SortMode::Recent => SortMode::Name,
        }
    }
}

pub fn default_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(p) = dirs::audio_dir() {
        out.push(p);
    }
    if let Some(p) = dirs::download_dir() {
        out.push(p);
    }
    out
}

pub fn scan(roots: &[PathBuf]) -> Vec<LocalTrack> {
    let mut found = Vec::new();
    for root in roots {
        walk(root, 0, &mut found);
    }
    found.sort_by_key(|a| a.title.to_lowercase());
    found.dedup_by(|a, b| a.path == b.path);
    found
}

fn walk(dir: &Path, depth: u32, out: &mut Vec<LocalTrack>) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            walk(&path, depth + 1, out);
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) {
            continue;
        }
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let folder = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        out.push(LocalTrack {
            path,
            title,
            folder,
            modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        });
    }
}

pub fn sort(tracks: &mut [LocalTrack], mode: SortMode, artist_of: impl Fn(&LocalTrack) -> String) {
    match mode {
        SortMode::Name => tracks.sort_by_key(|t| t.title.to_lowercase()),
        SortMode::Artist => tracks.sort_by_key(|t| (artist_of(t).to_lowercase(), t.title.to_lowercase())),
        SortMode::Folder => {
            tracks.sort_by_key(|t| (t.folder.to_lowercase(), t.title.to_lowercase()))
        }
        SortMode::Recent => tracks.sort_by_key(|t| std::cmp::Reverse(t.modified)),
    }
}

/// Subsequence-aware score used by the search box: an exact substring beats a
/// prefix, which beats scattered letters. Returns 0 when nothing matches.
pub fn match_score(query: &str, target: &str) -> f64 {
    if query.is_empty() {
        return 1.0;
    }
    let q = query.to_lowercase();
    let t = target.to_lowercase();
    if t == q {
        return 1000.0;
    }
    if let Some(pos) = t.find(&q) {
        return 500.0 - pos as f64 + q.len() as f64;
    }

    let mut chars = q.chars().peekable();
    let mut matched = 0usize;
    let mut first_at = None;
    for (i, c) in t.chars().enumerate() {
        if let Some(want) = chars.peek() {
            if *want == c {
                if first_at.is_none() {
                    first_at = Some(i);
                }
                matched += 1;
                chars.next();
            }
        } else {
            break;
        }
    }
    if matched < q.chars().count() {
        return 0.0;
    }
    100.0 - first_at.unwrap_or(0) as f64 * 0.5 + matched as f64
}

pub fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}
