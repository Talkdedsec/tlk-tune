use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use crate::lang::Language;

#[derive(Clone, Copy)]
pub struct Rgb {
    pub r: i32,
    pub g: i32,
    pub b: i32,
}

pub fn palette_rgb(index: i32) -> Rgb {
    const BASIC16: [(i32, i32, i32); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    if (0..16).contains(&index) {
        let (r, g, b) = BASIC16[index as usize];
        return Rgb { r, g, b };
    }
    if (16..=231).contains(&index) {
        let c = index - 16;
        let step = |v: i32| if v == 0 { 0 } else { 55 + 40 * v };
        return Rgb {
            r: step(c / 36),
            g: step((c / 6) % 6),
            b: step(c % 6),
        };
    }
    if (232..=255).contains(&index) {
        let g = 8 + (index - 232) * 10;
        return Rgb { r: g, g, b: g };
    }
    Rgb {
        r: 255,
        g: 255,
        b: 255,
    }
}

pub fn to_rgb(value: &str) -> Option<Rgb> {
    let idx: i32 = value.trim().parse().ok()?;
    if idx <= 0 || idx > 255 {
        return None;
    }
    Some(palette_rgb(idx))
}

/// Foreground SGR sequence. "0" and empty both mean "no colour".
pub fn fg(value: &str) -> String {
    let Ok(v) = value.trim().parse::<i32>() else {
        return String::new();
    };
    if v <= 0 {
        return String::new();
    }
    if (30..=47).contains(&v) || (90..=107).contains(&v) {
        return format!("\x1b[{}m", v);
    }
    format!("\x1b[38;5;{}m", v)
}

pub fn bg(value: &str) -> String {
    let Ok(v) = value.trim().parse::<i32>() else {
        return String::new();
    };
    if v <= 0 {
        return String::new();
    }
    if (30..=47).contains(&v) || (90..=107).contains(&v) {
        return format!("\x1b[{}m", v);
    }
    format!("\x1b[48;5;{}m", v)
}

pub fn ramp(start: &str, end: &str, t: f32) -> String {
    if end.trim().is_empty() {
        return fg(start);
    }
    let (Some(a), Some(b)) = (to_rgb(start), to_rgb(end)) else {
        return fg(start);
    };
    let t = t.clamp(0.0, 1.0);
    let mix = |x: i32, y: i32| (x as f32 + (y - x) as f32 * t) as i32;
    format!(
        "\x1b[38;2;{};{};{}m",
        mix(a.r, b.r),
        mix(a.g, b.g),
        mix(a.b, b.b)
    )
}

pub fn ramp3(left: &str, centre: &str, right: &str, t: f32) -> String {
    if centre.trim().is_empty() {
        return ramp(left, right, t);
    }
    let t = t.clamp(0.0, 1.0);
    if t <= 0.5 {
        ramp(left, centre, t * 2.0)
    } else {
        ramp(centre, right, (t - 0.5) * 2.0)
    }
}

pub fn ramp_swatch(start: &str, end: &str, width: i32) -> String {
    let mut out = String::new();
    for i in 0..width {
        let t = if width > 1 {
            i as f32 / (width - 1) as f32
        } else {
            0.0
        };
        out.push_str(&ramp(start, end, t));
        out.push('\u{2588}');
    }
    out.push_str("\x1b[0m");
    out
}

pub type FontMap = BTreeMap<char, (String, String)>;

pub fn map_font(text: &str, map: &FontMap) -> String {
    if map.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if !c.is_ascii() {
            out.push(c);
            continue;
        }
        match map.get(&c.to_ascii_uppercase()) {
            Some((upper, lower)) => {
                out.push_str(if c.is_ascii_uppercase() { upper } else { lower })
            }
            None => out.push(c),
        }
    }
    out
}

pub const PLAY_MODES: [&str; 4] = ["list", "loop", "shuffle", "stop"];
pub const LYRIC_ALIGNMENTS: [&str; 3] = ["center", "left", "right"];
pub const LYRIC_ANIMATIONS: [&str; 5] = [
    "full",
    "word by word",
    "letter by letter",
    "active line only",
    "active word only",
];
pub const LANGUAGES: [&str; 2] = ["en", "tr"];

#[derive(Clone, PartialEq)]
pub struct Config {
    pub language: Language,

    pub show_disk: bool,
    pub show_album_art: bool,
    pub show_buttons: bool,
    pub show_queue: bool,
    pub show_waveform: bool,
    pub show_lyrics: bool,
    pub show_lyric_ball: bool,
    pub show_visualizer: bool,

    pub lyric_alignment: i32,
    pub lyric_animation: i32,

    pub viz_fluidity: i32,
    pub viz_decay: i32,
    pub viz_viscosity: i32,
    pub waveform_smooth: bool,
    pub disk_speed: f64,
    pub play_mode: i32,

    pub corner_tl: String,
    pub corner_tr: String,
    pub corner_bl: String,
    pub corner_br: String,
    pub edge_v: String,
    pub edge_h: String,
    pub meta_separator: String,
    pub list_separator: String,

    pub font: FontMap,
    pub about: Vec<String>,

    pub border: String,
    pub border_bottom: String,
    pub disk: String,
    pub disk_end: String,
    pub meta_key: String,
    pub meta_val: String,
    pub viz_left: String,
    pub viz_centre: String,
    pub viz_right: String,
    pub progress_played: String,
    pub progress_pending: String,
    pub progress_time: String,

    pub list_fg: String,
    pub list_bg: String,
    pub list_playing_fg: String,
    pub list_playing_bg: String,
    pub list_cursor_fg: String,
    pub list_cursor_bg: String,

    pub queue_fg: String,
    pub queue_bg: String,
    pub queue_playing_fg: String,
    pub queue_playing_bg: String,
    pub queue_cursor_fg: String,
    pub queue_cursor_bg: String,

    pub lyric_fg: String,
    pub lyric_bg: String,
    pub lyric_line_fg: String,
    pub lyric_line_bg: String,
    pub lyric_word_fg: String,
    pub lyric_word_bg: String,

    pub button: String,

    pub crossfade_ms: u32,
    pub normalize: bool,
    pub normalize_target: f64,
    pub eq: [f32; 10],
    pub list_liked_fg: String,

    pub music_paths: Vec<String>,
    pub output_device: Option<String>,
    pub keys: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut c = Config {
            language: Language::En,

            show_disk: true,
            show_album_art: true,
            show_buttons: true,
            show_queue: true,
            show_waveform: true,
            show_lyrics: true,
            show_lyric_ball: false,
            show_visualizer: true,

            lyric_alignment: 0,
            lyric_animation: 1,

            viz_fluidity: 10,
            viz_decay: 10,
            viz_viscosity: 1,
            waveform_smooth: false,
            disk_speed: 0.25,
            play_mode: 0,

            corner_tl: "\u{256d}".into(),
            corner_tr: "\u{256e}".into(),
            corner_bl: "\u{2570}".into(),
            corner_br: "\u{256f}".into(),
            edge_v: "\u{2502}".into(),
            edge_h: "\u{2500}".into(),
            meta_separator: ":  :".into(),
            list_separator: "|".into(),

            font: FontMap::new(),
            about: Vec::new(),

            border: "250".into(),
            border_bottom: "250".into(),
            disk: "240".into(),
            disk_end: "255".into(),
            meta_key: String::new(),
            meta_val: String::new(),
            viz_left: "250".into(),
            viz_centre: String::new(),
            viz_right: "250".into(),
            progress_played: "250".into(),
            progress_pending: "245".into(),
            progress_time: String::new(),

            list_fg: "255".into(),
            list_bg: String::new(),
            list_playing_fg: "245".into(),
            list_playing_bg: String::new(),
            list_cursor_fg: String::new(),
            list_cursor_bg: "240".into(),

            queue_fg: "255".into(),
            queue_bg: String::new(),
            queue_playing_fg: "245".into(),
            queue_playing_bg: String::new(),
            queue_cursor_fg: String::new(),
            queue_cursor_bg: "240".into(),

            lyric_fg: "245".into(),
            lyric_bg: String::new(),
            lyric_line_fg: "251".into(),
            lyric_line_bg: String::new(),
            lyric_word_fg: "255".into(),
            lyric_word_bg: "0".into(),

            button: String::new(),

            crossfade_ms: 0,
            normalize: true,
            normalize_target: -18.0,
            eq: [0.0; 10],
            list_liked_fg: "214".into(),

            music_paths: Vec::new(),
            output_device: None,
            keys: BTreeMap::new(),
        };
        c.default_keys();
        c.default_font();
        c.about = default_about();
        c
    }
}

impl Config {
    fn default_keys(&mut self) {
        const BINDINGS: [(&str, &str); 22] = [
            ("HKeySetting", "s"),
            ("HKeyNavigateUp", "ARROW_KEY_UP"),
            ("HKeyNavigateDown", "ARROW_KEY_DOWN"),
            ("HKeyPlay", "ENTER"),
            ("HKeySearch", "/"),
            ("HKeySearchOnline", "/s:"),
            ("HKeyPlayNextSong", "n"),
            ("HKeyPlayPreviousSong", "b"),
            ("HKeySeekForward", "ARROW_KEY_RIGHT"),
            ("HKeySeekBackward", "ARROW_KEY_LEFT"),
            ("HKeyIncreaseVolume", "1"),
            ("HKeyDecreaseVolume", "2"),
            ("HKeyAddHoveringSongToQueue", "a"),
            ("HKeyRemoveHoveringSongFromQueue", "d"),
            ("HKeySwitchBetweenCards", "TAB"),
            ("HKeyToggleRepeat", "r"),
            ("HKeyTogglePlayPause", "p"),
            ("HKeyToggleShuffle", "m"),
            ("HKeyFilterForFolder", "f"),
            ("HKeyClearFilter", "c"),
            ("HKeyQuit", "q"),
            ("HKeyDownloadStream", "y"),
        ];
        for (action, key) in BINDINGS {
            self.keys
                .entry(action.to_string())
                .or_insert_with(|| key.to_string());
        }
    }

    fn default_font(&mut self) {
        for c in 'A'..='Z' {
            self.font
                .insert(c, (c.to_string(), c.to_ascii_lowercase().to_string()));
        }
    }

    pub fn key(&self, action: &str) -> &str {
        self.keys.get(action).map(|s| s.as_str()).unwrap_or("")
    }
}

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tlk-tune")
        .join("config.txt")
}

fn unquote(value: &str) -> String {
    let t = value.trim();
    let t = t.strip_suffix(';').unwrap_or(t).trim();
    let b = t.as_bytes();
    if b.len() >= 2
        && ((b[0] == b'"' && b[b.len() - 1] == b'"') || (b[0] == b'\'' && b[b.len() - 1] == b'\''))
    {
        return t[1..t.len() - 1].to_string();
    }
    t.to_string()
}

fn as_bool(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes" | "on"
    )
}

fn as_colour(value: &str) -> String {
    let t = unquote(value);
    match t.split_once(',') {
        Some((idx, _)) if idx.trim().parse::<i32>().is_ok() => idx.trim().to_string(),
        _ => t,
    }
}

fn parse_font_block(body: &str, map: &mut FontMap) {
    let mut rest = body;
    while let Some(eq) = rest.find('=') {
        let letter = rest[..eq].trim().chars().last();
        let Some(open_rel) = rest[eq..].find('{') else {
            break;
        };
        let open = eq + open_rel;
        let Some(close_rel) = rest[open..].find('}') else {
            break;
        };
        let close = open + close_rel;
        if let (Some(c), Some((upper, lower))) = (letter, rest[open + 1..close].split_once(',')) {
            if c.is_ascii_alphabetic() {
                map.insert(
                    c.to_ascii_uppercase(),
                    (upper.trim().to_string(), lower.trim().to_string()),
                );
            }
        }
        rest = &rest[close + 1..];
    }
}

pub fn load() -> Config {
    match fs::read_to_string(config_path()) {
        Ok(text) => parse(&text),
        Err(_) => Config::default(),
    }
}

pub fn parse(text: &str) -> Config {
    let mut c = Config::default();

    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].trim();
        i += 1;
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("font_en") && line.contains('{') {
            let mut raw = line.to_string();
            let mut depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;
            while depth > 0 && i < lines.len() {
                let s = lines[i];
                i += 1;
                depth += s.matches('{').count() as i32;
                depth -= s.matches('}').count() as i32;
                raw.push_str(s);
            }
            c.font.clear();
            if let Some(open) = raw.find('{') {
                parse_font_block(&raw[open + 1..], &mut c.font);
            }
            if c.font.is_empty() {
                c.default_font();
            }
            continue;
        }

        if line.starts_with("ClassTextAboutApp") && line.contains('{') {
            c.about.clear();
            while i < lines.len() {
                let s = lines[i];
                i += 1;
                if s.trim_start().starts_with('}') {
                    break;
                }
                c.about.push(s.to_string());
            }
            if c.about.is_empty() {
                c.about = default_about();
            }
            continue;
        }

        let (key, value) = match line.split_once("==") {
            Some((k, v)) => (k.trim(), v.trim()),
            None => match line.split_once('=') {
                Some((k, v)) => (k.trim(), v.trim()),
                None => continue,
            },
        };

        match key {
            "Language" => c.language = Language::from_config(&unquote(value)),

            "ColorBorderTop" => c.border = as_colour(value),
            "ColorBorderBottom" => c.border_bottom = as_colour(value),
            "ColorDiskTop" => c.disk = as_colour(value),
            "ColorDiskBottom" => c.disk_end = as_colour(value),
            "ColorMetadataKey" => c.meta_key = as_colour(value),
            "ColorMetadataVal" => c.meta_val = as_colour(value),
            "ColorVizLeft" => c.viz_left = as_colour(value),
            "ColorVizCenter" => c.viz_centre = as_colour(value),
            "ColorVizRight" => c.viz_right = as_colour(value),
            "ColorProgressBarPlayed" => c.progress_played = as_colour(value),
            "ColorProgressBarPending" => c.progress_pending = as_colour(value),
            "ColorProgressBarTimestamp" => c.progress_time = as_colour(value),
            "ColorListInactiveFg" => c.list_fg = as_colour(value),
            "ColorListInactiveBg" => c.list_bg = as_colour(value),
            "ColorListPlayingFg" => c.list_playing_fg = as_colour(value),
            "ColorListPlayingBg" => c.list_playing_bg = as_colour(value),
            "ColorListCursorFg" => c.list_cursor_fg = as_colour(value),
            "ColorListCursorBg" => c.list_cursor_bg = as_colour(value),
            "ColorQueueInactiveFg" => c.queue_fg = as_colour(value),
            "ColorQueueInactiveBg" => c.queue_bg = as_colour(value),
            "ColorQueuePlayingFg" => c.queue_playing_fg = as_colour(value),
            "ColorQueuePlayingBg" => c.queue_playing_bg = as_colour(value),
            "ColorQueueCursorFg" => c.queue_cursor_fg = as_colour(value),
            "ColorQueueCursorBg" => c.queue_cursor_bg = as_colour(value),
            "ColorLyricsInactiveFg" => c.lyric_fg = as_colour(value),
            "ColorLyricsInactiveBg" => c.lyric_bg = as_colour(value),
            "ColorLyricsActiveLineFg" => c.lyric_line_fg = as_colour(value),
            "ColorLyricsActiveLineBg" => c.lyric_line_bg = as_colour(value),
            "ColorLyricsActiveWordFg" => c.lyric_word_fg = as_colour(value),
            "ColorLyricsActiveWordBg" => c.lyric_word_bg = as_colour(value),
            "ColorButton" => c.button = as_colour(value),
            "ColorListLikedFg" => c.list_liked_fg = as_colour(value),

            "CrossfadeMs" => c.crossfade_ms = value.trim().parse().unwrap_or(0).min(12_000),
            "Normalize" => c.normalize = as_bool(value),
            "NormalizeTarget" => {
                c.normalize_target = value
                    .trim()
                    .parse::<f64>()
                    .unwrap_or(-18.0)
                    .clamp(-30.0, -6.0)
            }
            "Equalizer" => {
                for (i, part) in unquote(value).split(',').take(10).enumerate() {
                    c.eq[i] = part.trim().parse::<f32>().unwrap_or(0.0).clamp(-12.0, 12.0);
                }
            }

            "ElimentDisk" => c.show_disk = as_bool(value),
            "ElimentAlbumArt" => c.show_album_art = as_bool(value),
            "ElimentDummyButtons" => c.show_buttons = as_bool(value),
            "ElimentQueue" => c.show_queue = as_bool(value),
            "ElimentWaveForm" => c.show_waveform = as_bool(value),
            "ElimentLyrics" => c.show_lyrics = as_bool(value),
            "LyricsPlaceholderBall" => c.show_lyric_ball = as_bool(value),
            "Visualizer" => c.show_visualizer = as_bool(value),

            "VisualizerFluidity" => {
                c.viz_fluidity = value.trim().parse().unwrap_or(10).clamp(1, 10)
            }
            "VisualizerDegradationSpeed" => {
                c.viz_decay = value.trim().parse().unwrap_or(10).clamp(1, 10)
            }
            "VisualizerViscosity" => {
                c.viz_viscosity = value.trim().parse().unwrap_or(1).clamp(0, 10)
            }
            "WaveformStyle" => c.waveform_smooth = unquote(value) == "smooth",
            "DiskRotationSpeed" => {
                c.disk_speed = value.trim().parse::<f64>().unwrap_or(0.25).clamp(0.01, 1.0)
            }
            "PlaybackMode" => {
                if let Some(i) = PLAY_MODES.iter().position(|m| *m == unquote(value)) {
                    c.play_mode = i as i32;
                }
            }
            "LyricsAlignment" => {
                if let Some(i) = LYRIC_ALIGNMENTS.iter().position(|m| *m == unquote(value)) {
                    c.lyric_alignment = i as i32;
                }
            }
            "LyricsAnimation" => {
                if let Some(i) = LYRIC_ANIMATIONS.iter().position(|m| *m == unquote(value)) {
                    c.lyric_animation = i as i32;
                }
            }

            "UpperLeftCorner" => c.corner_tl = unquote(value),
            "UpperRightCorner" => c.corner_tr = unquote(value),
            "BottomLeftCorner" => c.corner_bl = unquote(value),
            "LowerRightCorner" => c.corner_br = unquote(value),
            "Vertical" => c.edge_v = unquote(value),
            "Horizontal" => c.edge_h = unquote(value),
            "Seprator" | "Separator" => c.meta_separator = unquote(value),
            "ListSeparator" => c.list_separator = unquote(value),

            "OutputDevice" => {
                let name = unquote(value);
                c.output_device = if name.is_empty() { None } else { Some(name) };
            }
            "LocalMusicPath" => {
                let p = unquote(value);
                if !p.is_empty() {
                    c.music_paths.push(p);
                }
            }
            _ if key.starts_with("HKey") => {
                c.keys.insert(key.to_string(), unquote(value));
            }
            _ => {}
        }
    }
    c
}

const ABOUT_TEXT: &str = "

Developer : Talkdedsec
Github    : Talkdedsec
Version   : 0.1.0
License   : MIT


tlk-tune
A terminal music player for people who prefer
control, simplicity, and a keyboard.

No unnecessary interface layers, no mouse,
no background service. One executable.

Everything is keyboard driven.
Everything is configurable.
Everything stays in your terminal.

Features
- Local playback with native decoding
- Search and filtering
- Queue management
- Online search and streaming
- Lyrics with active word highlighting
- Shuffle and repeat
- Fully configurable colours
- Fully configurable hotkeys
- English and Turkish interface

Configuration
%APPDATA%\\tlk-tune\\config.txt

";

pub fn default_about() -> Vec<String> {
    ABOUT_TEXT.lines().map(|s| s.to_string()).collect()
}

pub fn save(c: &Config) -> std::io::Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, render(c, &path.display().to_string()))
}

/// Everything the parser understands has to come back out here, or quitting
/// would quietly throw the setting away.
pub fn render(c: &Config, location: &str) -> String {
    let tf = |v: bool| if v { "true" } else { "false" };
    let mut o = String::new();
    o.push_str("# tlk-tune configuration\n");
    o.push_str(&format!("# Location: {}\n\n", location));
    o.push_str(&format!("Language={}\n", c.language.code()));
    o.push_str(&format!(
        "OutputDevice={}\n\n",
        c.output_device.clone().unwrap_or_default()
    ));

    o.push_str("##-------------------------------------------\n");
    o.push_str("##              PANEL 1: COLORS\n");
    o.push_str("##-------------------------------------------\n\n");
    o.push_str("# Frame & Controls\n");
    for (k, v) in [
        ("ColorBorderTop", &c.border),
        ("ColorBorderBottom", &c.border_bottom),
        ("ColorDiskTop", &c.disk),
        ("ColorDiskBottom", &c.disk_end),
        ("ColorMetadataKey", &c.meta_key),
        ("ColorMetadataVal", &c.meta_val),
        ("ColorVizLeft", &c.viz_left),
        ("ColorVizCenter", &c.viz_centre),
        ("ColorVizRight", &c.viz_right),
        ("ColorProgressBarPlayed", &c.progress_played),
        ("ColorProgressBarPending", &c.progress_pending),
        ("ColorProgressBarTimestamp", &c.progress_time),
        ("ColorButton", &c.button),
    ] {
        o.push_str(&format!("{}={}\n", k, v));
    }
    o.push_str("\n# List\n");
    for (k, v) in [
        ("ColorListInactiveFg", &c.list_fg),
        ("ColorListInactiveBg", &c.list_bg),
        ("ColorListPlayingFg", &c.list_playing_fg),
        ("ColorListPlayingBg", &c.list_playing_bg),
        ("ColorListCursorFg", &c.list_cursor_fg),
        ("ColorListCursorBg", &c.list_cursor_bg),
        ("ColorListLikedFg", &c.list_liked_fg),
    ] {
        o.push_str(&format!("{}={}\n", k, v));
    }
    o.push_str("\n# Queue\n");
    for (k, v) in [
        ("ColorQueueInactiveFg", &c.queue_fg),
        ("ColorQueueInactiveBg", &c.queue_bg),
        ("ColorQueuePlayingFg", &c.queue_playing_fg),
        ("ColorQueuePlayingBg", &c.queue_playing_bg),
        ("ColorQueueCursorFg", &c.queue_cursor_fg),
        ("ColorQueueCursorBg", &c.queue_cursor_bg),
    ] {
        o.push_str(&format!("{}={}\n", k, v));
    }
    o.push_str("\n# Lyrics\n");
    for (k, v) in [
        ("ColorLyricsInactiveFg", &c.lyric_fg),
        ("ColorLyricsInactiveBg", &c.lyric_bg),
        ("ColorLyricsActiveLineFg", &c.lyric_line_fg),
        ("ColorLyricsActiveLineBg", &c.lyric_line_bg),
        ("ColorLyricsActiveWordFg", &c.lyric_word_fg),
        ("ColorLyricsActiveWordBg", &c.lyric_word_bg),
    ] {
        o.push_str(&format!("{}={}\n", k, v));
    }

    o.push_str("\n##-------------------------------------------\n");
    o.push_str("##              PANEL 2: ON/OFF\n");
    o.push_str("##-------------------------------------------\n\n");
    for (k, v) in [
        ("ElimentDisk", c.show_disk),
        ("ElimentAlbumArt", c.show_album_art),
        ("ElimentDummyButtons", c.show_buttons),
        ("ElimentQueue", c.show_queue),
        ("ElimentWaveForm", c.show_waveform),
        ("ElimentLyrics", c.show_lyrics),
        ("LyricsPlaceholderBall", c.show_lyric_ball),
        ("Visualizer", c.show_visualizer),
    ] {
        o.push_str(&format!("{}={}\n", k, tf(v)));
    }

    o.push_str("\n##-------------------------------------------\n");
    o.push_str("##             PANEL 3: ANIMATION\n");
    o.push_str("##-------------------------------------------\n\n");
    o.push_str(&format!(
        "VisualizerFluidity={}\n## 1 to 10\n",
        c.viz_fluidity
    ));
    o.push_str(&format!(
        "WaveformStyle={}\n## raw , smooth\n",
        if c.waveform_smooth { "smooth" } else { "raw" }
    ));
    o.push_str(&format!(
        "DiskRotationSpeed={:.2}\n## 0.01x to 1.00x\n",
        c.disk_speed
    ));
    o.push_str(&format!(
        "PlaybackMode={}\n## list , loop , shuffle , stop\n",
        PLAY_MODES[c.play_mode.clamp(0, 3) as usize]
    ));
    o.push_str(&format!(
        "VisualizerDegradationSpeed={}\n## 1 to 10\n",
        c.viz_decay
    ));
    o.push_str(&format!(
        "VisualizerViscosity={}\n## 1 to 10\n",
        c.viz_viscosity
    ));
    o.push_str(&format!(
        "LyricsAlignment={}\n## center , left , right\n",
        LYRIC_ALIGNMENTS[c.lyric_alignment.clamp(0, 2) as usize]
    ));
    o.push_str(&format!(
        "LyricsAnimation={}\n## full , word by word , letter by letter\n## active line only , active word only\n",
        LYRIC_ANIMATIONS[c.lyric_animation.clamp(0, 4) as usize]
    ));
    o.push_str(&format!(
        "CrossfadeMs={}\n## 0 to 12000, 0 is gapless\n",
        c.crossfade_ms
    ));
    o.push_str(&format!(
        "Normalize={}\nNormalizeTarget={:.1}\n## EBU R128, -30 to -6 LUFS\n",
        tf(c.normalize),
        c.normalize_target
    ));
    o.push_str(&format!(
        "Equalizer={}\n## 10 bands, 31Hz to 16kHz, -12 to +12 dB\n",
        c.eq.iter()
            .map(|g| format!("{:.1}", g))
            .collect::<Vec<_>>()
            .join(",")
    ));

    o.push_str("\n##-------------------------------------------\n");
    o.push_str("##             PANEL 4: REFERENCE\n");
    o.push_str("##-------------------------------------------\n\n");
    o.push_str("font_en={\n");
    for (c2, (upper, lower)) in &c.font {
        o.push_str(&format!("          {}={{{},{}}},\n", c2, upper, lower));
    }
    o.push_str("        };\n\n");
    o.push_str(&format!("UpperLeftCorner=\"{}\";\n", c.corner_tl));
    o.push_str(&format!("UpperRightCorner=\"{}\";\n", c.corner_tr));
    o.push_str(&format!("BottomLeftCorner=\"{}\";\n", c.corner_bl));
    o.push_str(&format!("LowerRightCorner=\"{}\";\n", c.corner_br));
    o.push_str(&format!("Vertical=\"{}\";\n", c.edge_v));
    o.push_str(&format!("Horizontal=\"{}\";\n", c.edge_h));
    o.push_str(&format!("Seprator=\"{}\"\n", c.meta_separator));
    o.push_str(&format!("ListSeparator=\"{}\"\n\n", c.list_separator));

    o.push_str("# Local Music Paths\n");
    for p in &c.music_paths {
        o.push_str(&format!("LocalMusicPath={}\n", p));
    }
    o.push_str("\n# Navigation\n");
    for (k, v) in &c.keys {
        o.push_str(&format!("{}=\"{}\"\n", k, v));
    }

    o.push_str("\n##-------------------------------------------\n");
    o.push_str("##             PANEL 5: ABOUT APP\n");
    o.push_str("##-------------------------------------------\n\n");
    o.push_str("ClassTextAboutApp= {\n");
    for line in &c.about {
        o.push_str(line);
        o.push('\n');
    }
    o.push_str("};\n");
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A config where nothing is left at its default, so a field that save
    /// forgets cannot hide behind matching the default on the way back.
    fn nothing_default() -> Config {
        let mut c = Config {
            language: Language::Tr,
            output_device: Some("Headphones (USB)".into()),
            ..Config::default()
        };

        c.show_disk = false;
        c.show_album_art = false;
        c.show_buttons = false;
        c.show_queue = false;
        c.show_waveform = false;
        c.show_lyrics = false;
        c.show_lyric_ball = true;
        c.show_visualizer = false;
        c.normalize = false;

        c.lyric_alignment = 2;
        c.lyric_animation = 4;
        c.viz_fluidity = 3;
        c.viz_decay = 7;
        c.viz_viscosity = 9;
        c.waveform_smooth = true;
        c.disk_speed = 0.35;
        c.play_mode = 2;
        c.crossfade_ms = 3250;
        c.normalize_target = -14.0;
        c.eq = [1.5, -2.0, 3.0, -4.5, 5.0, -6.0, 7.5, -8.0, 9.0, -10.5];

        c.corner_tl = "+".into();
        c.corner_tr = "+".into();
        c.corner_bl = "+".into();
        c.corner_br = "+".into();
        c.edge_v = "|".into();
        c.edge_h = "-".into();
        c.meta_separator = "::".into();
        c.list_separator = "/".into();

        for (n, field) in [
            &mut c.border,
            &mut c.border_bottom,
            &mut c.disk,
            &mut c.disk_end,
            &mut c.meta_key,
            &mut c.meta_val,
            &mut c.viz_left,
            &mut c.viz_centre,
            &mut c.viz_right,
            &mut c.progress_played,
            &mut c.progress_pending,
            &mut c.progress_time,
            &mut c.list_fg,
            &mut c.list_bg,
            &mut c.list_playing_fg,
            &mut c.list_playing_bg,
            &mut c.list_cursor_fg,
            &mut c.list_cursor_bg,
            &mut c.list_liked_fg,
            &mut c.queue_fg,
            &mut c.queue_bg,
            &mut c.queue_playing_fg,
            &mut c.queue_playing_bg,
            &mut c.queue_cursor_fg,
            &mut c.queue_cursor_bg,
            &mut c.lyric_fg,
            &mut c.lyric_bg,
            &mut c.lyric_line_fg,
            &mut c.lyric_line_bg,
            &mut c.lyric_word_fg,
            &mut c.lyric_word_bg,
            &mut c.button,
        ]
        .into_iter()
        .enumerate()
        {
            *field = (20 + n).to_string();
        }

        c.music_paths = vec!["E:/Muzik".into(), "D:/Lists/night.m3u".into()];
        c.keys
            .insert("HKeyTogglePlayPause".to_string(), "z".to_string());
        c.about = vec!["about line one".into(), "about line two".into()];
        c
    }

    #[test]
    fn saving_and_loading_keeps_every_setting() {
        let original = nothing_default();
        let round_tripped = parse(&render(&original, "test"));
        assert!(
            round_tripped == original,
            "a setting did not survive the round trip"
        );
    }

    #[test]
    fn an_empty_file_is_the_default() {
        assert!(parse("") == Config::default());
        assert!(parse("# only a comment\n\n") == Config::default());
    }

    #[test]
    fn unknown_keys_and_junk_are_ignored() {
        let c = parse("NoSuchSetting=7\nnot even a pair\nColorBorderTop=42\n");
        assert_eq!(c.border, "42");
        assert_eq!(c.border_bottom, Config::default().border_bottom);
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        let c = parse("CrossfadeMs=99999\nNormalizeTarget=-90\nEqualizer=99,-99,0,0,0,0,0,0,0,0\n");
        assert_eq!(c.crossfade_ms, 12_000);
        assert_eq!(c.normalize_target, -30.0);
        assert_eq!(c.eq[0], 12.0);
        assert_eq!(c.eq[1], -12.0);
    }

    #[test]
    fn a_colour_may_carry_a_brightness_it_does_not_use() {
        assert_eq!(parse("ColorBorderTop=39,100\n").border, "39");
    }
}
