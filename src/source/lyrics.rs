use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Default)]
pub struct LyricLine {
    pub start: f64,
    pub text: String,
    pub words: Vec<(f64, String)>,
}

#[derive(Clone, Default)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    pub message: String,
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tlk-tune")
        .join("lyrics")
}

fn cache_key(title: &str, artist: &str) -> String {
    // FNV-1a keeps the filename short and stable without pulling in a hasher.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in format!("{}|{}", artist, title).to_lowercase().bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("{:016x}.lrc", hash)
}

fn parse_stamp(s: &str) -> Option<f64> {
    let (minutes, rest) = s.split_once(':')?;
    let minutes: f64 = minutes.trim().parse().ok()?;
    let seconds: f64 = rest.trim().parse().ok()?;
    Some(minutes * 60.0 + seconds)
}

/// Handles both plain LRC line tags and enhanced per-word tags.
pub fn parse_lrc(text: &str) -> Vec<LyricLine> {
    let mut lines: Vec<LyricLine> = Vec::new();

    for raw in text.lines() {
        let mut rest = raw.trim();
        let mut stamps: Vec<f64> = Vec::new();
        while rest.starts_with('[') {
            let Some(close) = rest.find(']') else { break };
            let inside = &rest[1..close];
            match parse_stamp(inside) {
                Some(t) => stamps.push(t),
                None => {
                    // Metadata tag such as [ar:...]; skip it and stop scanning.
                    if stamps.is_empty() {
                        rest = "";
                    }
                    break;
                }
            }
            rest = rest[close + 1..].trim_start();
        }
        if stamps.is_empty() || rest.is_empty() {
            continue;
        }

        let (plain, words) = split_word_tags(rest);
        if plain.trim().is_empty() {
            continue;
        }
        for start in stamps {
            let mut words = words.clone();
            for w in words.iter_mut() {
                if w.0 < 0.0 {
                    w.0 = start;
                }
            }
            lines.push(LyricLine {
                start,
                text: plain.clone(),
                words,
            });
        }
    }

    lines.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap_or(std::cmp::Ordering::Equal));
    lines
}

fn split_word_tags(input: &str) -> (String, Vec<(f64, String)>) {
    if !input.contains('<') {
        return (input.trim().to_string(), Vec::new());
    }
    let mut plain = String::new();
    let mut words: Vec<(f64, String)> = Vec::new();
    let mut pending: Option<f64> = None;
    let mut buffer = String::new();
    let mut rest = input;

    let flush = |buffer: &mut String,
                 pending: &mut Option<f64>,
                 words: &mut Vec<(f64, String)>| {
        let chunk = buffer.trim();
        if !chunk.is_empty() {
            let t = pending.unwrap_or(-1.0);
            for w in chunk.split_whitespace() {
                words.push((t, w.to_string()));
            }
        }
        buffer.clear();
    };

    while let Some(open) = rest.find('<') {
        buffer.push_str(&rest[..open]);
        plain.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else {
            buffer.push_str(&rest[open..]);
            plain.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let stamp = parse_stamp(&after[..close]);
        flush(&mut buffer, &mut pending, &mut words);
        pending = stamp;
        rest = &after[close + 1..];
    }
    buffer.push_str(rest);
    plain.push_str(rest);
    flush(&mut buffer, &mut pending, &mut words);

    (plain.split_whitespace().collect::<Vec<_>>().join(" "), words)
}

/// LRCLIB only ships line-level timing. Spreading each line's duration over
/// its words by character count is what keeps the karaoke highlight moving
/// instead of snapping one whole line at a time.
pub fn interpolate_words(lines: &mut [LyricLine], total: f64) {
    for i in 0..lines.len() {
        if !lines[i].words.is_empty() {
            continue;
        }
        let start = lines[i].start;
        let end = if i + 1 < lines.len() {
            lines[i + 1].start
        } else if total > start {
            total
        } else {
            start + 4.0
        };
        let span = (end - start).max(0.1);
        let words: Vec<String> = lines[i]
            .text
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        let total_chars: usize = words.iter().map(|w| w.chars().count() + 1).sum();
        if total_chars == 0 {
            continue;
        }
        let mut cursor = 0usize;
        lines[i].words = words
            .into_iter()
            .map(|w| {
                let t = start + span * (cursor as f64 / total_chars as f64);
                cursor += w.chars().count() + 1;
                (t, w)
            })
            .collect();
    }
}

fn read_sidecar(track: &Path) -> Option<String> {
    let lrc = track.with_extension("lrc");
    std::fs::read_to_string(lrc).ok()
}

fn clean_query(value: &str) -> String {
    let mut s = value.trim().to_string();
    for marker in [" - Official", " (Official", " [Official", " | ", " ft. ", " feat. "] {
        if let Some(pos) = s.find(marker) {
            s.truncate(pos);
        }
    }
    s.trim().to_string()
}

/// Sidecar .lrc first, then the on-disk cache, then LRCLIB.
pub fn fetch(track: Option<&Path>, title: &str, artist: &str, duration: f64) -> Lyrics {
    if let Some(path) = track {
        if let Some(text) = read_sidecar(path) {
            let mut lines = parse_lrc(&text);
            interpolate_words(&mut lines, duration);
            if !lines.is_empty() {
                return Lyrics {
                    lines,
                    message: String::new(),
                };
            }
        }
    }

    let key = cache_key(title, artist);
    let cached = cache_dir().join(&key);
    if let Ok(text) = std::fs::read_to_string(&cached) {
        let mut lines = parse_lrc(&text);
        interpolate_words(&mut lines, duration);
        return Lyrics {
            lines,
            message: String::new(),
        };
    }

    let Some(text) = request(&clean_query(title), &clean_query(artist), duration) else {
        return Lyrics {
            lines: Vec::new(),
            message: String::new(),
        };
    };

    let _ = std::fs::create_dir_all(cache_dir());
    let _ = std::fs::write(&cached, &text);
    let mut lines = parse_lrc(&text);
    interpolate_words(&mut lines, duration);
    Lyrics {
        lines,
        message: String::new(),
    }
}

fn request(title: &str, artist: &str, duration: f64) -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .user_agent("tlk-tune/0.1.0 (https://github.com/Talkdedsec/tlk-tune)")
        .build();

    let mut call = agent
        .get("https://lrclib.net/api/get")
        .query("track_name", title)
        .query("artist_name", artist);
    if duration > 0.0 {
        call = call.query("duration", &format!("{}", duration.round() as i64));
    }

    let body: serde_json::Value = match call.call() {
        Ok(r) => r.into_json().ok()?,
        Err(_) => {
            let search: serde_json::Value = agent
                .get("https://lrclib.net/api/search")
                .query("q", &format!("{} {}", artist, title))
                .call()
                .ok()?
                .into_json()
                .ok()?;
            search.as_array()?.first()?.clone()
        }
    };

    body.get("syncedLyrics")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_plain_line_tags() {
        let lines = parse_lrc("[ar:Someone]\n[00:12.50]first line\n[00:15.00]second line\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start, 12.5);
        assert_eq!(lines[1].text, "second line");
    }

    #[test]
    fn reads_per_word_tags() {
        let lines = parse_lrc("[00:10.00]<00:10.00>hold <00:10.60>on\n");
        assert_eq!(lines[0].words.len(), 2);
        assert_eq!(lines[0].words[1].0, 10.6);
        assert_eq!(lines[0].text, "hold on");
    }

    #[test]
    fn repeats_a_line_for_each_stamp() {
        let lines = parse_lrc("[00:01.00][00:31.00]chorus\n");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].start, 31.0);
    }

    #[test]
    fn spreads_word_times_across_a_line() {
        let mut lines = parse_lrc("[00:00.00]one two three\n[00:03.00]next\n");
        interpolate_words(&mut lines, 6.0);
        let words = &lines[0].words;
        assert_eq!(words.len(), 3);
        assert!(words[0].0 < words[1].0 && words[1].0 < words[2].0);
        assert!(words[2].0 < 3.0);
    }
}
