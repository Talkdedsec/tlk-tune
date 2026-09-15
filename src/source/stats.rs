use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// What the listener did, as opposed to what the files say. Kept next to the
/// config so a library rescan never loses it.
#[derive(Serialize, Deserialize, Default)]
pub struct Stats {
    pub plays: HashMap<String, u32>,
    pub liked: HashSet<String>,
    pub last_played: HashMap<String, u64>,
    /// Measured integrated loudness per file, so a track is only analysed the
    /// first time it is played.
    #[serde(default)]
    pub loudness: HashMap<String, f64>,
    #[serde(default)]
    dirty: bool,
}

fn path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tlk-tune")
        .join("stats.json")
}

fn key(track: &Path) -> String {
    track.to_string_lossy().to_lowercase()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Stats {
    pub fn load() -> Stats {
        std::fs::read_to_string(path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if !self.dirty {
            return;
        }
        let file = path();
        if let Some(parent) = file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string(self) {
            let _ = std::fs::write(file, text);
        }
    }

    pub fn is_liked(&self, track: &Path) -> bool {
        self.liked.contains(&key(track))
    }

    pub fn toggle_like(&mut self, track: &Path) -> bool {
        let k = key(track);
        self.dirty = true;
        if self.liked.remove(&k) {
            false
        } else {
            self.liked.insert(k);
            true
        }
    }

    pub fn plays(&self, track: &Path) -> u32 {
        self.plays.get(&key(track)).copied().unwrap_or(0)
    }

    pub fn last_played(&self, track: &Path) -> u64 {
        self.last_played.get(&key(track)).copied().unwrap_or(0)
    }

    /// Counted once a track has actually been listened to for a while, so
    /// skipping through the library does not rewrite the play counts.
    pub fn record_play(&mut self, track: &Path) {
        let k = key(track);
        *self.plays.entry(k.clone()).or_insert(0) += 1;
        self.last_played.insert(k, now());
        self.dirty = true;
    }

    pub fn merge_like(&mut self, track: &Path) {
        self.liked.insert(key(track));
        self.dirty = true;
    }

    pub fn merge_plays(&mut self, track: &Path, count: u32, at: u64) {
        let k = key(track);
        let slot = self.plays.entry(k.clone()).or_insert(0);
        *slot = (*slot).max(count);
        let seen = self.last_played.entry(k).or_insert(0);
        *seen = (*seen).max(at);
        self.dirty = true;
    }

    pub fn loudness(&self, track: &Path) -> Option<f64> {
        self.loudness.get(&key(track)).copied()
    }

    pub fn set_loudness(&mut self, track: &Path, lufs: f64) {
        self.loudness.insert(key(track), lufs);
        self.dirty = true;
    }

    pub fn liked_count(&self) -> usize {
        self.liked.len()
    }
}

/// Pulls likes and play counts out of a tlk-player install. Its library maps
/// its own track ids to real paths, so both files are needed.
pub fn import_from_tlk_player(stats: &mut Stats) -> Option<(usize, usize)> {
    let dir = dirs::config_dir()?.join("com.talkdedsec.tlkplayer");
    let library: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("library.json")).ok()?).ok()?;
    let playlists: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("playlists.json")).ok()?).ok()?;

    let mut by_id: HashMap<&str, &str> = HashMap::new();
    for entry in library.as_array()? {
        if let (Some(id), Some(p)) = (
            entry.get("id").and_then(|v| v.as_str()),
            entry.get("path").and_then(|v| v.as_str()),
        ) {
            by_id.insert(id, p);
        }
    }

    let mut likes = 0usize;
    if let Some(list) = playlists.get("likes").and_then(|v| v.as_array()) {
        for like in list {
            if like
                .get("removed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                continue;
            }
            let Some(id) = like.get("id").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(p) = by_id.get(id) else { continue };
            stats.merge_like(Path::new(p));
            likes += 1;
        }
    }

    let mut counted: HashMap<&str, (u32, u64)> = HashMap::new();
    if let Some(list) = playlists.get("plays").and_then(|v| v.as_array()) {
        for play in list {
            let Some(id) = play.get("track").and_then(|v| v.as_str()) else {
                continue;
            };
            let at = play.get("at").and_then(|v| v.as_u64()).unwrap_or(0);
            let slot = counted.entry(id).or_insert((0, 0));
            slot.0 += 1;
            slot.1 = slot.1.max(at);
        }
    }
    let mut plays = 0usize;
    for (id, (count, at)) in counted {
        let Some(p) = by_id.get(id) else { continue };
        stats.merge_plays(Path::new(p), count, at);
        plays += 1;
    }

    Some((likes, plays))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn likes_toggle_and_are_case_insensitive() {
        let mut stats = Stats::default();
        let track = Path::new("E:/Muzik/Song.mp3");
        assert!(stats.toggle_like(track));
        assert!(stats.is_liked(Path::new("e:/muzik/song.mp3")));
        assert!(!stats.toggle_like(track));
        assert!(!stats.is_liked(track));
    }

    #[test]
    fn plays_accumulate() {
        let mut stats = Stats::default();
        let track = Path::new("E:/Muzik/Song.mp3");
        stats.record_play(track);
        stats.record_play(track);
        assert_eq!(stats.plays(track), 2);
        assert!(stats.last_played(track) > 0);
    }

    #[test]
    fn merging_keeps_the_higher_count() {
        let mut stats = Stats::default();
        let track = Path::new("E:/Muzik/Song.mp3");
        stats.record_play(track);
        stats.merge_plays(track, 9, 100);
        assert_eq!(stats.plays(track), 9);
        stats.merge_plays(track, 3, 50);
        assert_eq!(stats.plays(track), 9);
    }
}
