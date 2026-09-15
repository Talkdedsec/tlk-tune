use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// What the player was doing when it last closed. Kept apart from the config
/// so a user editing their settings by hand never trips over it.
#[derive(Serialize, Deserialize, Default)]
pub struct Session {
    pub track: Option<String>,
    pub position: f64,
    pub volume: i32,
    pub paused: bool,
    pub sort: u8,
    pub queue: Vec<QueuedTrack>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct QueuedTrack {
    pub title: String,
    pub path: Option<String>,
    pub id: Option<String>,
}

fn path() -> PathBuf {
    crate::config::profile_dir().join("session.json")
}

pub fn load() -> Session {
    std::fs::read_to_string(path())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(session: &Session) {
    let file = path();
    if let Some(parent) = file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(text) = serde_json::to_string_pretty(session) {
        let _ = std::fs::write(file, text);
    }
}
