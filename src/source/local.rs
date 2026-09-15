use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::paths;

const EXTENSIONS: [&str; 10] = [
    "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma", "aiff", "mp4",
];
const PLAYLIST_EXTENSIONS: [&str; 3] = ["m3u", "m3u8", "pls"];

#[derive(Clone)]
pub struct LocalTrack {
    pub path: PathBuf,
    pub title: String,
    pub folder: String,
    pub modified: SystemTime,
    pub size: u64,
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

pub fn default_paths() -> Vec<String> {
    let mut out = Vec::new();
    if let Some(p) = dirs::audio_dir() {
        out.push(p.to_string_lossy().to_string());
    }
    if let Some(p) = dirs::download_dir() {
        out.push(p.to_string_lossy().to_string());
    }
    out
}

pub fn is_playlist(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| PLAYLIST_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

/// Each root may be a folder or a playlist file; both are written the same way
/// in the config, so the user never has to know which kind it is.
pub fn scan(roots: &[String]) -> Vec<LocalTrack> {
    let mut found = Vec::new();
    for raw in roots {
        let root = paths::expand(raw);
        if is_playlist(&root) {
            found.extend(read_playlist(&root));
        } else {
            walk(&root, 0, &mut found);
        }
    }
    found.sort_by_key(|t| t.title.to_lowercase());
    found.dedup_by(|a, b| a.path == b.path);
    found
}

fn entry_for(path: PathBuf) -> Option<LocalTrack> {
    let meta = std::fs::metadata(&path).ok()?;
    if !meta.is_file() {
        return None;
    }
    Some(LocalTrack {
        title: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string(),
        folder: path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string(),
        modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
        size: meta.len(),
        path,
    })
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
        if !is_audio(&path) {
            continue;
        }
        out.push(LocalTrack {
            title: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
            folder: path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string(),
            modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            size: meta.len(),
            path,
        });
    }
}

/// Reads m3u, m3u8 and pls. Relative entries resolve against the playlist's own
/// folder, which is how every player that writes them expects it.
pub fn read_playlist(list: &Path) -> Vec<LocalTrack> {
    let Ok(text) = std::fs::read_to_string(list) else {
        return Vec::new();
    };
    let base = list.parent().unwrap_or(Path::new("."));
    let pls = list
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pls"))
        .unwrap_or(false);

    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let entry = if pls {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if !key.trim().to_ascii_lowercase().starts_with("file") {
                continue;
            }
            value.trim()
        } else {
            if line.starts_with('#') {
                continue;
            }
            line
        };

        let expanded = paths::expand(entry);
        let full = if expanded.is_absolute() {
            expanded
        } else {
            base.join(expanded)
        };
        if !is_audio(&full) {
            continue;
        }
        if let Some(track) = entry_for(full) {
            out.push(track);
        }
    }
    out
}

pub fn sort(tracks: &mut [LocalTrack], mode: SortMode, artist_of: impl Fn(&LocalTrack) -> String) {
    match mode {
        SortMode::Name => tracks.sort_by_key(|t| t.title.to_lowercase()),
        SortMode::Artist => {
            tracks.sort_by_key(|t| (artist_of(t).to_lowercase(), t.title.to_lowercase()))
        }
        SortMode::Folder => {
            tracks.sort_by_key(|t| (t.folder.to_lowercase(), t.title.to_lowercase()))
        }
        SortMode::Recent => tracks.sort_by_key(|t| std::cmp::Reverse(t.modified)),
    }
}

/// Letters that a keyboard makes awkward to type, and what they are typed as.
/// Without this a library full of "Oğuzhan" and "Dünya" is unsearchable unless
/// every accent is entered exactly.
const FOLDS: [(&str, char); 21] = [
    ("àáâãäåāăą", 'a'),
    ("æ", 'a'),
    ("çćĉċč", 'c'),
    ("ďđ", 'd'),
    ("èéêëēĕėęě", 'e'),
    ("ĝğġģ", 'g'),
    ("ĥħ", 'h'),
    ("ìíîïĩīĭįı", 'i'),
    ("ĵ", 'j'),
    ("ķ", 'k'),
    ("ĺļľłŀ", 'l'),
    ("ñńņňŉ", 'n'),
    ("òóôõöøōŏő", 'o'),
    ("œ", 'o'),
    ("ŕŗř", 'r'),
    ("śŝşšß", 's'),
    ("ţťŧ", 't'),
    ("ùúûüũūŭůűų", 'u'),
    ("ŵ", 'w'),
    ("ýÿŷ", 'y'),
    ("źżž", 'z'),
];

/// Lowercases and strips accents so typing plain ASCII finds anything.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.to_lowercase().chars() {
        // `İ` lowercases to `i` plus a combining dot; drop the marks.
        if ('\u{0300}'..='\u{036F}').contains(&c) {
            continue;
        }
        match FOLDS.iter().find(|(set, _)| set.contains(c)) {
            Some((_, base)) => out.push(*base),
            None => out.push(c),
        }
    }
    out
}

/// Subsequence-aware score used by the search box: an exact substring beats a
/// prefix, which beats scattered letters. Returns 0 when nothing matches.
pub fn match_score(query: &str, target: &str) -> f64 {
    if query.is_empty() {
        return 1.0;
    }
    let q = fold(query);
    let t = fold(target);
    if t == q {
        return 1000.0;
    }
    if let Some(pos) = t.find(&q) {
        return 500.0 - pos as f64 + q.len() as f64;
    }

    let mut wanted = q.chars().peekable();
    let mut matched = 0usize;
    let mut first_at = None;
    for (i, c) in t.chars().enumerate() {
        let Some(want) = wanted.peek() else { break };
        if *want == c {
            if first_at.is_none() {
                first_at = Some(i);
            }
            matched += 1;
            wanted.next();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join("tlk-tune-tests");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn reads_m3u_with_relative_entries() {
        let track = scratch("in-list.mp3");
        std::fs::write(&track, b"x").unwrap();
        let list = scratch("plain.m3u");
        std::fs::write(&list, "#EXTM3U\n#EXTINF:1,x\nin-list.mp3\nmissing.mp3\n").unwrap();

        let entries = read_playlist(&list);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, track);
    }

    #[test]
    fn reads_pls_file_keys() {
        let track = scratch("in-pls.flac");
        std::fs::write(&track, b"x").unwrap();
        let list = scratch("winamp.pls");
        std::fs::write(
            &list,
            "[playlist]\nNumberOfEntries=1\nFile1=in-pls.flac\nTitle1=x\n",
        )
        .unwrap();

        let entries = read_playlist(&list);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, track);
    }

    #[test]
    fn scan_accepts_a_playlist_as_a_root() {
        let track = scratch("root-entry.wav");
        std::fs::write(&track, b"x").unwrap();
        let list = scratch("root.m3u");
        std::fs::write(&list, "root-entry.wav\n").unwrap();

        let found = scan(&[list.to_string_lossy().to_string()]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "root-entry");
    }

    #[test]
    fn scoring_prefers_a_substring_over_scattered_letters() {
        assert!(match_score("gold", "golden hour") > match_score("gold", "go loud"));
        assert_eq!(match_score("zzz", "abc"), 0.0);
    }

    #[test]
    fn plain_ascii_finds_accented_titles() {
        for (query, title) in [
            ("oguzhan", "004 - Oğuzhan Koç - Ayy"),
            ("dunya", "002 - KADR - Dünya Boştur Lo"),
            ("bostur", "002 - KADR - Dünya Boştur Lo"),
            ("nalbantoglu", "005 - EMRE NALBANTOĞLU"),
            ("sufri", "001 - Eslabon Armado - Jugaste y Sufrí"),
            ("istanbul", "İstanbul Geceleri"),
            ("cigdem", "ÇİĞDEM"),
        ] {
            assert!(
                match_score(query, title) > 0.0,
                "{query:?} did not find {title:?}"
            );
        }
    }

    #[test]
    fn typing_the_accents_still_works() {
        assert!(match_score("Oğuzhan", "004 - Oğuzhan Koç - Ayy") > 0.0);
        assert!(match_score("dünya", "002 - KADR - Dünya Boştur Lo") > 0.0);
    }

    #[test]
    fn folding_does_not_merge_unrelated_words() {
        assert_eq!(match_score("zzz", "Dünya Boştur"), 0.0);
    }
}
