use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::audio::buffer::PcmStream;
use crate::audio::decoder::{self, TrackInfo};
use crate::audio::player::Player;
use crate::config::{self, Config};
use crate::lang::{Language, Strings};
use crate::source::local::{self, LocalTrack, SortMode};
use crate::source::lyrics::{self, Lyrics};
use crate::source::online::{self, OnlineResult};
use crate::terminal::{binding, Console, Key};
use crate::ui;
use crate::visual::disk::Disk;
use crate::visual::spectrum::Spectrum;
use crate::visual::sphere::Sphere;
use crate::visual::waveform;

pub const LIST_ROWS: usize = 8;
const ANGULAR_VELOCITY: f64 = (2.0 * std::f64::consts::PI / 48.0) / 0.035;
const FRAME: Duration = Duration::from_millis(40);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Browse,
    Search,
    Settings,
    ColorEdit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ListSource {
    Local,
    Online,
}

#[derive(Clone)]
pub struct QueueItem {
    pub title: String,
    pub path: Option<PathBuf>,
    pub id: Option<String>,
}

#[derive(Clone, Default)]
pub struct MetaView {
    pub name: String,
    pub artist: String,
    pub year: String,
    pub sampling: String,
    pub kind: String,
    pub format: String,
    pub size: String,
    pub location: String,
    pub extra_label: String,
    pub extra_value: String,
}

#[derive(Clone, Default)]
pub struct RowMeta {
    pub duration: f64,
    pub artist: String,
}

pub enum Message {
    Library(Vec<LocalTrack>),
    RowMeta(PathBuf, RowMeta),
    SearchResults(Vec<OnlineResult>),
    Resolved {
        path: PathBuf,
        title: String,
        artist: String,
    },
    ResolveFailed,
    Lyrics(Lyrics),
    Waveform(Vec<f32>),
    Status(String),
}

pub struct App {
    pub cfg: Config,
    pub lang: &'static Strings,
    pub player: Player,
    pub spectrum: Spectrum,
    pub disk: Disk,
    pub sphere: Sphere,
    pub angle: f64,
    pub dt: f64,

    pub tracks: Vec<LocalTrack>,
    pub view: Vec<LocalTrack>,
    pub online: Vec<OnlineResult>,
    pub source: ListSource,
    pub selected: usize,
    pub scroll: usize,

    pub queue: Vec<QueueItem>,
    pub queue_selected: usize,
    pub queue_scroll: usize,
    pub queue_focus: bool,

    pub mode: Mode,
    pub search_buffer: String,
    pub last_local_query: String,
    pub last_online_query: String,

    pub meta: MetaView,
    pub total_sec: f64,
    pub has_track: bool,
    pub current_path: Option<PathBuf>,

    pub waveform: Vec<f32>,
    pub waveform_ready: bool,
    pub waveform_at: Instant,

    pub lyrics: Lyrics,
    pub lyrics_ready: bool,
    pub lyrics_status: String,
    pub lyrics_status_at: Instant,

    pub status: String,
    pub row_meta: HashMap<PathBuf, RowMeta>,
    pub sort_mode: SortMode,
    pub folder_filter: Option<String>,

    pub settings_tab: i32,
    pub settings_row: i32,
    pub settings_col: i32,
    pub edit_buffer: String,

    pub loading: bool,
    pub load_started: Instant,
    pub online_enabled: bool,

    pub quit: bool,
    pub pcm: Option<Arc<PcmStream>>,

    tx: Sender<Message>,
    rx: Receiver<Message>,
    seed: u64,
    last_width: i32,
    last_mode: u8,
    force_redraw: bool,
    meta_probed: bool,
}

impl App {
    pub fn new() -> App {
        let cfg = config::load();
        let lang = cfg.language.strings();
        let (tx, rx) = channel();
        let mut player = Player::new();
        player.set_volume(70);
        let mut spectrum = Spectrum::new();
        spectrum.set_fluidity(cfg.viz_fluidity);
        spectrum.set_decay(cfg.viz_decay);
        spectrum.set_viscosity(cfg.viz_viscosity);

        App {
            cfg,
            lang,
            player,
            spectrum,
            disk: Disk,
            sphere: Sphere::new(),
            angle: 0.0,
            dt: 0.04,

            tracks: Vec::new(),
            view: Vec::new(),
            online: Vec::new(),
            source: ListSource::Local,
            selected: 0,
            scroll: 0,

            queue: Vec::new(),
            queue_selected: 0,
            queue_scroll: 0,
            queue_focus: false,

            mode: Mode::Browse,
            search_buffer: String::new(),
            last_local_query: String::new(),
            last_online_query: String::new(),

            meta: MetaView::default(),
            total_sec: 0.0,
            has_track: false,
            current_path: None,

            waveform: Vec::new(),
            waveform_ready: false,
            waveform_at: Instant::now(),

            lyrics: Lyrics::default(),
            lyrics_ready: false,
            lyrics_status: String::new(),
            lyrics_status_at: Instant::now(),

            status: String::new(),
            row_meta: HashMap::new(),
            sort_mode: SortMode::Name,
            folder_filter: None,

            settings_tab: 0,
            settings_row: 0,
            settings_col: 0,
            edit_buffer: String::new(),

            loading: false,
            load_started: Instant::now(),
            online_enabled: false,

            quit: false,
            pcm: None,

            tx,
            rx,
            seed: 0x2545_F491_4F6C_DD1D,
            last_width: 0,
            last_mode: 255,
            force_redraw: true,
            meta_probed: false,
        }
    }

    fn roots(&self) -> Vec<PathBuf> {
        if self.cfg.music_paths.is_empty() {
            local::default_paths()
        } else {
            self.cfg.music_paths.iter().map(PathBuf::from).collect()
        }
    }

    fn start_library_scan(&mut self) {
        let roots = self.roots();
        let tx = self.tx.clone();
        self.status = self.lang.scanning.to_string();
        std::thread::spawn(move || {
            let found = local::scan(&roots);
            let _ = tx.send(Message::Library(found));
        });
    }

    fn start_row_meta(&mut self) {
        if self.meta_probed {
            return;
        }
        self.meta_probed = true;
        let paths: Vec<PathBuf> = self.tracks.iter().map(|t| t.path.clone()).collect();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            for p in paths {
                if let Some(info) = decoder::probe(&p) {
                    let meta = RowMeta {
                        duration: info.duration,
                        artist: info.artist,
                    };
                    if tx.send(Message::RowMeta(p, meta)).is_err() {
                        return;
                    }
                }
            }
        });
    }

    pub fn refresh_view(&mut self) {
        let query = self.last_local_query.clone();
        let folder = self.folder_filter.clone();
        let meta = &self.row_meta;

        let mut rows: Vec<LocalTrack> = self
            .tracks
            .iter()
            .filter(|t| match &folder {
                Some(f) => &t.folder == f,
                None => true,
            })
            .filter(|t| {
                if query.is_empty() {
                    return true;
                }
                let artist = meta
                    .get(&t.path)
                    .map(|m| m.artist.clone())
                    .unwrap_or_default();
                local::match_score(&query, &t.title) > 0.0
                    || local::match_score(&query, &artist) > 0.0
                    || local::match_score(&query, &t.folder) > 0.0
            })
            .cloned()
            .collect();

        if query.is_empty() {
            let lookup = self.row_meta.clone();
            local::sort(&mut rows, self.sort_mode, move |t| {
                lookup
                    .get(&t.path)
                    .map(|m| m.artist.clone())
                    .unwrap_or_else(|| t.folder.clone())
            });
        } else {
            rows.sort_by(|a, b| {
                local::match_score(&query, &b.title)
                    .partial_cmp(&local::match_score(&query, &a.title))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        self.view = rows;
        if self.selected >= self.view.len() {
            self.selected = self.view.len().saturating_sub(1);
        }
        self.clamp_scroll();
    }

    fn clamp_scroll(&mut self) {
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        if self.selected >= self.scroll + LIST_ROWS {
            self.scroll = self.selected + 1 - LIST_ROWS;
        }
    }

    fn clamp_queue_scroll(&mut self) {
        if self.queue_selected < self.queue_scroll {
            self.queue_scroll = self.queue_selected;
        }
        if self.queue_selected >= self.queue_scroll + LIST_ROWS {
            self.queue_scroll = self.queue_selected + 1 - LIST_ROWS;
        }
    }

    fn next_random(&mut self) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }

    /// Fills the metadata panel from the file header. Returns what the decoder
    /// found so the caller can size the playback buffer.
    fn describe(&mut self, path: &Path) -> TrackInfo {
        let info = decoder::probe(path).unwrap_or(TrackInfo {
            duration: 0.0,
            sample_rate: 44100,
            channels: 2,
            ..Default::default()
        });

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let title = if info.title.is_empty() {
            stem.clone()
        } else {
            info.title.clone()
        };
        let artist = if info.artist.is_empty() {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string()
        } else {
            info.artist.clone()
        };

        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        self.meta = MetaView {
            name: title.clone(),
            artist: artist.clone(),
            year: info.year.clone(),
            sampling: format!("{} Hz", info.sample_rate),
            kind: if info.channels >= 2 { "stereo" } else { "mono" }.to_string(),
            format: path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_uppercase(),
            size: local::human_size(size),
            location: path
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            extra_label: if info.bits.is_empty() {
                String::new()
            } else {
                self.lang.meta_channels.to_string()
            },
            extra_value: info.bits.clone(),
        };

        self.total_sec = info.duration;
        self.current_path = Some(path.to_path_buf());
        self.has_track = true;
        self.waveform.clear();
        self.waveform_ready = false;
        self.lyrics = Lyrics::default();
        self.lyrics_ready = false;
        self.lyrics_status = self.lang.fetching_lyrics.to_string();
        self.lyrics_status_at = Instant::now();
        self.spectrum.reset();
        info
    }

    pub fn start_local(&mut self, path: PathBuf) {
        let info = self.describe(&path);
        let title = self.meta.name.clone();
        let artist = self.meta.artist.clone();

        let out = self.player.output();
        let seconds = if info.duration > 0.0 {
            info.duration
        } else {
            600.0
        };
        let pcm = Arc::new(PcmStream::with_seconds(
            seconds,
            out.sample_rate,
            out.channels,
        ));

        let decode_path = path.clone();
        let decode_sink = Arc::clone(&pcm);
        std::thread::spawn(move || decoder::decode_into(&decode_path, decode_sink));

        let wave_sink = Arc::clone(&pcm);
        let wave_tx = self.tx.clone();
        let smooth = self.cfg.waveform_smooth;
        std::thread::spawn(move || {
            while !wave_sink.is_done() {
                std::thread::sleep(Duration::from_millis(120));
            }
            if wave_sink.has_failed() {
                return;
            }
            let interleaved = wave_sink.snapshot();
            let channels = wave_sink.channels.max(1);
            let mono: Vec<f32> = interleaved
                .chunks(channels)
                .map(|f| f.iter().sum::<f32>() / channels as f32)
                .collect();
            let model = waveform::envelope(&mono, waveform::RESOLUTION, smooth);
            let _ = wave_tx.send(Message::Waveform(model));
        });

        let sink = self.spectrum.sink();
        self.player.play(Arc::clone(&pcm), 0.0, Some(sink));
        self.pcm = Some(pcm);

        let lyrics_tx = self.tx.clone();
        let duration = info.duration;
        std::thread::spawn(move || {
            let result = lyrics::fetch(Some(&path), &title, &artist, duration);
            let _ = lyrics_tx.send(Message::Lyrics(result));
        });
    }

    fn start_online(&mut self, result: OnlineResult) {
        self.loading = true;
        self.load_started = Instant::now();
        self.status = self.lang.resolving.to_string();
        let tx = self.tx.clone();
        std::thread::spawn(move || match online::resolve(&result.id) {
            Some(path) => {
                let _ = tx.send(Message::Resolved {
                    path,
                    title: result.title,
                    artist: result.uploader,
                });
            }
            None => {
                let _ = tx.send(Message::ResolveFailed);
            }
        });
    }

    fn play_selected(&mut self) {
        match self.source {
            ListSource::Local => {
                if let Some(track) = self.view.get(self.selected).cloned() {
                    self.start_local(track.path);
                }
            }
            ListSource::Online => {
                if let Some(result) = self.online.get(self.selected).cloned() {
                    self.start_online(result);
                }
            }
        }
    }

    fn play_relative(&mut self, delta: i32) {
        if self.view.is_empty() {
            return;
        }
        let len = self.view.len() as i32;
        let next = (self.selected as i32 + delta).rem_euclid(len);
        self.selected = next as usize;
        self.source = ListSource::Local;
        self.clamp_scroll();
        if let Some(track) = self.view.get(self.selected).cloned() {
            self.start_local(track.path);
        }
    }

    fn play_random(&mut self) {
        if self.view.is_empty() {
            return;
        }
        let pick = (self.next_random() % self.view.len() as u64) as usize;
        self.selected = pick;
        self.clamp_scroll();
        if let Some(track) = self.view.get(pick).cloned() {
            self.start_local(track.path);
        }
    }

    fn advance(&mut self) {
        self.player.clear_finished();
        if !self.queue.is_empty() {
            let item = self.queue.remove(0);
            self.clamp_queue();
            match (item.path, item.id) {
                (Some(p), _) => {
                    self.start_local(p);
                    return;
                }
                (None, Some(id)) => {
                    self.start_online(OnlineResult {
                        id,
                        title: item.title,
                        ..Default::default()
                    });
                    return;
                }
                _ => {}
            }
        }
        match self.cfg.play_mode {
            1 => {
                if let Some(p) = self.current_path.clone() {
                    self.start_local(p);
                }
            }
            2 => self.play_random(),
            3 => {
                self.player.stop();
                self.has_track = false;
            }
            _ => self.play_relative(1),
        }
    }

    fn clamp_queue(&mut self) {
        if self.queue_selected >= self.queue.len() {
            self.queue_selected = self.queue.len().saturating_sub(1);
        }
        self.clamp_queue_scroll();
    }

    fn queue_add_selected(&mut self) {
        let item = match self.source {
            ListSource::Local => self.view.get(self.selected).map(|t| QueueItem {
                title: t.title.clone(),
                path: Some(t.path.clone()),
                id: None,
            }),
            ListSource::Online => self.online.get(self.selected).map(|r| QueueItem {
                title: r.title.clone(),
                path: None,
                id: Some(r.id.clone()),
            }),
        };
        if let Some(item) = item {
            self.status = format!("{}: {}", self.lang.queued, item.title);
            self.queue.push(item);
        }
    }

    fn submit_search(&mut self) {
        let raw = self.search_buffer.clone();
        self.mode = Mode::Browse;
        self.search_buffer.clear();

        if let Some(rest) = raw.strip_prefix("s:") {
            let query = rest.trim().to_string();
            if query.is_empty() {
                return;
            }
            if !self.online_enabled {
                self.status = self.lang.yt_dlp_missing.to_string();
                return;
            }
            self.last_online_query = query.clone();
            self.source = ListSource::Online;
            self.selected = 0;
            self.scroll = 0;
            self.status = self.lang.searching.to_string();
            let tx = self.tx.clone();
            std::thread::spawn(move || {
                let found = online::search(&query, 20);
                let _ = tx.send(Message::SearchResults(found));
            });
            return;
        }

        self.last_local_query = raw.trim().to_string();
        self.source = ListSource::Local;
        self.selected = 0;
        self.scroll = 0;
        self.refresh_view();
    }

    fn download_current(&mut self) {
        if self.source != ListSource::Online {
            return;
        }
        let Some(result) = self.online.get(self.selected).cloned() else {
            return;
        };
        let target = self
            .roots()
            .into_iter()
            .next()
            .unwrap_or_else(|| PathBuf::from("."));
        self.status = self.lang.download_started.to_string();
        let tx = self.tx.clone();
        let done = self.lang.download_done;
        std::thread::spawn(move || {
            let text = match online::download(&result.id, &target) {
                Some(path) => format!("{} {}", done, path.display()),
                None => String::new(),
            };
            let _ = tx.send(Message::Status(text));
        });
    }

    fn handle_browse_key(&mut self, key: Key) {
        let cfg_key = |action: &str| binding(self.cfg.key(action));

        if key == cfg_key("HKeyQuit") {
            self.quit = true;
            return;
        }
        if key == cfg_key("HKeySetting") {
            self.mode = Mode::Settings;
            self.settings_tab = 0;
            self.settings_row = 0;
            self.settings_col = 0;
            self.force_redraw = true;
            return;
        }
        if key == Key::Char('/') {
            self.mode = Mode::Search;
            self.search_buffer.clear();
            return;
        }
        if key == cfg_key("HKeySwitchBetweenCards") {
            if self.cfg.show_queue {
                self.queue_focus = !self.queue_focus;
            }
            return;
        }
        if key == cfg_key("HKeyNavigateUp") {
            if self.queue_focus {
                self.queue_selected = self.queue_selected.saturating_sub(1);
                self.clamp_queue_scroll();
            } else {
                self.selected = self.selected.saturating_sub(1);
                self.clamp_scroll();
            }
            return;
        }
        if key == cfg_key("HKeyNavigateDown") {
            if self.queue_focus {
                if self.queue_selected + 1 < self.queue.len() {
                    self.queue_selected += 1;
                }
                self.clamp_queue_scroll();
            } else {
                let len = self.list_len();
                if self.selected + 1 < len {
                    self.selected += 1;
                }
                self.clamp_scroll();
            }
            return;
        }
        if key == cfg_key("HKeyPlay") {
            if self.queue_focus {
                if self.queue_selected < self.queue.len() {
                    let item = self.queue.remove(self.queue_selected);
                    self.clamp_queue();
                    match (item.path, item.id) {
                        (Some(p), _) => self.start_local(p),
                        (None, Some(id)) => self.start_online(OnlineResult {
                            id,
                            title: item.title,
                            ..Default::default()
                        }),
                        _ => {}
                    }
                }
            } else {
                self.play_selected();
            }
            return;
        }
        if key == cfg_key("HKeyTogglePlayPause") {
            self.player.toggle_pause();
            return;
        }
        if key == cfg_key("HKeyPlayNextSong") {
            self.play_relative(1);
            return;
        }
        if key == cfg_key("HKeyPlayPreviousSong") {
            self.play_relative(-1);
            return;
        }
        if key == cfg_key("HKeySeekForward") {
            self.player.seek(5.0);
            return;
        }
        if key == cfg_key("HKeySeekBackward") {
            self.player.seek(-5.0);
            return;
        }
        if key == cfg_key("HKeyIncreaseVolume") {
            let v = self.player.volume() + 5;
            self.player.set_volume(v);
            return;
        }
        if key == cfg_key("HKeyDecreaseVolume") {
            let v = self.player.volume() - 5;
            self.player.set_volume(v);
            return;
        }
        if key == cfg_key("HKeyToggleRepeat") {
            self.cfg.play_mode = if self.cfg.play_mode == 1 { 0 } else { 1 };
            return;
        }
        if key == cfg_key("HKeyToggleShuffle") {
            self.cfg.play_mode = if self.cfg.play_mode == 2 { 0 } else { 2 };
            return;
        }
        if key == cfg_key("HKeyAddHoveringSongToQueue") {
            self.queue_add_selected();
            return;
        }
        if key == cfg_key("HKeyRemoveHoveringSongFromQueue") {
            if self.queue_focus && self.queue_selected < self.queue.len() {
                self.queue.remove(self.queue_selected);
                self.clamp_queue();
            } else if !self.queue.is_empty() {
                self.queue.pop();
            }
            return;
        }
        if key == cfg_key("HKeyFilterForFolder") {
            if self.source == ListSource::Local {
                self.folder_filter = self.view.get(self.selected).map(|t| t.folder.clone());
                self.selected = 0;
                self.refresh_view();
            }
            return;
        }
        if key == cfg_key("HKeyClearFilter") {
            self.folder_filter = None;
            self.last_local_query.clear();
            self.source = ListSource::Local;
            self.selected = 0;
            self.refresh_view();
            return;
        }
        if key == cfg_key("HKeyDownloadStream") {
            self.download_current();
            return;
        }
        if key == Key::Char('o') {
            self.sort_mode = self.sort_mode.next();
            self.refresh_view();
        }
    }

    fn handle_search_key(&mut self, key: Key) {
        match key {
            Key::Esc => {
                self.mode = Mode::Browse;
                self.search_buffer.clear();
            }
            Key::Enter => self.submit_search(),
            Key::Backspace => {
                self.search_buffer.pop();
                self.live_preview();
            }
            Key::Char(c) if !c.is_control() => {
                self.search_buffer.push(c);
                self.live_preview();
            }
            _ => {}
        }
    }

    fn live_preview(&mut self) {
        if self.search_buffer.starts_with("s:") {
            return;
        }
        self.last_local_query = self.search_buffer.clone();
        self.source = ListSource::Local;
        self.selected = 0;
        self.scroll = 0;
        self.refresh_view();
    }

    pub fn list_len(&self) -> usize {
        match self.source {
            ListSource::Local => self.view.len(),
            ListSource::Online => self.online.len(),
        }
    }

    pub fn sort_name(&self) -> &'static str {
        match self.sort_mode {
            SortMode::Name => self.lang.sort_name,
            SortMode::Artist => self.lang.sort_artist,
            SortMode::Folder => self.lang.sort_folder,
            SortMode::Recent => self.lang.sort_recent,
        }
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Message::Library(found) => {
                    self.tracks = found;
                    self.status = if self.tracks.is_empty() {
                        self.lang.no_music_path.to_string()
                    } else {
                        String::new()
                    };
                    self.refresh_view();
                    self.start_row_meta();
                }
                Message::RowMeta(path, meta) => {
                    self.row_meta.insert(path, meta);
                }
                Message::SearchResults(found) => {
                    self.online = found;
                    self.selected = 0;
                    self.scroll = 0;
                    self.status = if self.online.is_empty() {
                        self.lang.no_results.to_string()
                    } else {
                        String::new()
                    };
                }
                Message::Resolved {
                    path,
                    title,
                    artist,
                } => {
                    self.loading = false;
                    self.status.clear();
                    self.start_local(path);
                    if !title.is_empty() {
                        self.meta.name = title;
                    }
                    if !artist.is_empty() {
                        self.meta.artist = artist;
                    }
                }
                Message::ResolveFailed => {
                    self.loading = false;
                    self.status = self.lang.no_results.to_string();
                }
                Message::Lyrics(result) => {
                    self.lyrics_ready = true;
                    if result.lines.is_empty() {
                        self.lyrics_status = self.lang.no_lyrics.to_string();
                    } else {
                        self.lyrics_status.clear();
                    }
                    self.lyrics = result;
                }
                Message::Waveform(model) => {
                    self.waveform = model;
                    self.waveform_ready = true;
                    self.waveform_at = Instant::now();
                }
                Message::Status(text) => self.status = text,
            }
        }
    }

    pub fn apply_language(&mut self, language: Language) {
        self.cfg.language = language;
        self.lang = language.strings();
        self.force_redraw = true;
    }

    /// Renders a single frame with the library loaded but nothing playing.
    /// Useful for checking a colour scheme without launching the player.
    pub fn preview(&mut self, width: usize) -> String {
        self.tracks = local::scan(&self.roots());
        self.refresh_view();
        for track in self.view.iter().take(LIST_ROWS) {
            if let Some(info) = decoder::probe(&track.path) {
                self.row_meta.insert(
                    track.path.clone(),
                    RowMeta {
                        duration: info.duration,
                        artist: info.artist,
                    },
                );
            }
        }
        if let Some(track) = self.view.first().cloned() {
            self.describe(&track.path);
        }
        self.render_frame(width)
    }

    pub fn render_frame(&mut self, width: usize) -> String {
        let mode_id = match self.mode {
            Mode::Browse => 0,
            Mode::Search => 1,
            Mode::Settings => 2,
            Mode::ColorEdit => 3,
        };
        let hard = width as i32 != self.last_width || mode_id != self.last_mode || self.force_redraw;
        self.force_redraw = false;
        self.last_width = width as i32;
        self.last_mode = mode_id;

        if matches!(self.mode, Mode::Settings | Mode::ColorEdit) {
            let player_height = ui::panels::player_height(self);
            return ui::settings_screen::build(self, width.max(80), player_height);
        }

        let mut frame = String::new();
        frame.push_str(if hard { "\x1b[2J\x1b[H" } else { "\x1b[H" });

        for line in ui::panels::metadata(self, width) {
            frame.push_str(&line);
            frame.push('\n');
        }
        for line in ui::panels::progress(self, width) {
            frame.push_str(&line);
            frame.push('\n');
        }
        for line in ui::panels::search_bar(self, width) {
            frame.push_str(&line);
            frame.push('\n');
        }

        if self.cfg.show_queue {
            let list_w = width / 2;
            let queue_w = width - list_w;
            let left = ui::panels::list(self, list_w, LIST_ROWS);
            let right = ui::panels::queue(self, queue_w, LIST_ROWS);
            let rows = left.len().max(right.len());
            for i in 0..rows {
                let l = left.get(i).cloned().unwrap_or_else(|| " ".repeat(list_w));
                let r = right.get(i).cloned().unwrap_or_else(|| " ".repeat(queue_w));
                frame.push_str(&l);
                frame.push_str(&r);
                frame.push('\n');
            }
        } else {
            for line in ui::panels::list(self, width, LIST_ROWS) {
                frame.push_str(&line);
                frame.push('\n');
            }
        }

        frame.push('\n');
        if self.loading {
            let secs = self.load_started.elapsed().as_secs();
            frame.push_str(&format!("  {} ({}s)\n", self.lang.resolving, secs));
        } else if !self.status.is_empty() {
            frame.push_str(&format!("  {}\n", self.status));
        }
        frame.push_str("\x1b[0J");
        frame
    }

    pub fn run(&mut self) {
        let mut console = match Console::open() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("console: {e}");
                return;
            }
        };

        self.online_enabled = online::available();
        self.start_library_scan();
        let mut last_frame = Instant::now();

        while !self.quit {
            loop {
                let key = console.read_key();
                if key == Key::None {
                    break;
                }
                if key == Key::Char('\u{3}') {
                    self.quit = true;
                    break;
                }
                match self.mode {
                    Mode::Browse => self.handle_browse_key(key),
                    Mode::Search => self.handle_search_key(key),
                    Mode::Settings | Mode::ColorEdit => ui::settings_screen::handle_key(self, key),
                }
            }

            self.drain_messages();

            let now = Instant::now();
            self.dt = now.duration_since(last_frame).as_secs_f64();
            last_frame = now;

            if self.has_track && self.player.finished() {
                self.advance();
            }
            if self.has_track && !self.player.is_paused() {
                self.angle = (self.angle
                    + ANGULAR_VELOCITY * self.cfg.disk_speed * self.dt)
                    % (2.0 * std::f64::consts::PI);
            }

            let width = console.cols().clamp(40, 200) as usize;
            let frame = self.render_frame(width);
            console.write(&frame);
            std::thread::sleep(FRAME);
        }

        self.player.stop();
        console.close();
        let _ = config::save(&self.cfg);
    }
}
