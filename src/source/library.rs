use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// What one probe of a file produced. Stored on disk so a library of a few
/// thousand tracks does not have to be re-read from scratch on every start.
#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Entry {
    pub mtime: u64,
    pub size: u64,
    pub duration: f64,
    pub artist: String,
    pub title: String,
    pub year: String,
    pub sample_rate: u32,
    pub channels: usize,
    pub bits: String,
}

#[derive(Serialize, Deserialize, Default)]
struct Disk {
    version: u32,
    entries: HashMap<String, Entry>,
}

const VERSION: u32 = 1;
const MAX_ENTRIES: usize = 50_000;

pub struct Cache {
    entries: HashMap<String, Entry>,
    dirty: bool,
}

pub fn cache_path() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tlk-tune")
        .join("library.json")
}

pub fn stamp(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Cache {
    pub fn load() -> Cache {
        let entries = std::fs::read_to_string(cache_path())
            .ok()
            .and_then(|text| serde_json::from_str::<Disk>(&text).ok())
            .filter(|d| d.version == VERSION)
            .map(|d| d.entries)
            .unwrap_or_default();
        Cache {
            entries,
            dirty: false,
        }
    }

    fn key(path: &Path) -> String {
        path.to_string_lossy().to_string()
    }

    /// Returns the stored probe only when the file still looks the same.
    /// A re-encode or a tag edit changes size or mtime and invalidates it.
    pub fn get(&self, path: &Path, mtime: u64, size: u64) -> Option<&Entry> {
        let entry = self.entries.get(&Self::key(path))?;
        if entry.mtime == mtime && entry.size == size {
            Some(entry)
        } else {
            None
        }
    }

    pub fn put(&mut self, path: &Path, entry: Entry) {
        if self.entries.len() >= MAX_ENTRIES {
            return;
        }
        self.entries.insert(Self::key(path), entry);
        self.dirty = true;
    }

    /// Drops rows for files that no longer exist, so a cache does not grow
    /// forever as a library is reorganised.
    pub fn prune(&mut self, present: &[PathBuf]) {
        let keep: std::collections::HashSet<String> =
            present.iter().map(|p| Self::key(p)).collect();
        let before = self.entries.len();
        self.entries.retain(|k, _| keep.contains(k));
        if self.entries.len() != before {
            self.dirty = true;
        }
    }

    pub fn save(&self) {
        if !self.dirty {
            return;
        }
        let path = cache_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let disk = Disk {
            version: VERSION,
            entries: self.entries.clone(),
        };
        if let Ok(text) = serde_json::to_string(&disk) {
            let _ = std::fs::write(path, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_an_entry_whose_file_changed() {
        let mut cache = Cache {
            entries: HashMap::new(),
            dirty: false,
        };
        let path = Path::new("C:/x/y.mp3");
        cache.put(
            path,
            Entry {
                mtime: 100,
                size: 200,
                duration: 3.0,
                ..Default::default()
            },
        );
        assert!(cache.get(path, 100, 200).is_some());
        assert!(cache.get(path, 101, 200).is_none());
        assert!(cache.get(path, 100, 201).is_none());
    }

    #[test]
    fn prune_drops_missing_files() {
        let mut cache = Cache {
            entries: HashMap::new(),
            dirty: false,
        };
        let kept = PathBuf::from("C:/x/keep.mp3");
        let gone = PathBuf::from("C:/x/gone.mp3");
        cache.put(&kept, Entry::default());
        cache.put(&gone, Entry::default());
        cache.prune(std::slice::from_ref(&kept));
        assert!(cache.get(&kept, 0, 0).is_some());
        assert!(cache.get(&gone, 0, 0).is_none());
    }
}
