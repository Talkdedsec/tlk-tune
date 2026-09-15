use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::audio::buffer::PcmStream;
use crate::audio::decoder::{self, Source, TrackInfo};
use crate::audio::loudness;
use crate::audio::player::Player;
use crate::config::{self, Config};
use crate::lang::{Language, Strings};
use crate::mediakeys::{self, MediaKey};
use crate::session::{self, QueuedTrack, Session};
use crate::source::library::{self, Cache};
use crate::source::local::{self, LocalTrack, SortMode};
use crate::source::lyrics::{self, Lyrics};
use crate::source::online::{self, OnlineResult};
use crate::source::paths;
use crate::source::stats::{self, Stats};
use crate::terminal::{binding, Console, Input, Key};
use crate::ui::layout::{self, Target};
use crate::ui::{self, settings_screen};
use crate::visual::disk::Disk;
use crate::visual::spectrum::Spectrum;
use crate::visual::sphere::Sphere;
use crate::visual::{artwork, waveform};

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

/// Which slice of the library the list shows. A flat folder of several
/// hundred files is unusable without these.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    All,
    Liked,
    Played,
    Recent,
}

impl ViewMode {
    pub fn next(self) -> ViewMode {
        match self {
            ViewMode::All => ViewMode::Liked,
            ViewMode::Liked => ViewMode::Played,
            ViewMode::Played => ViewMode::Recent,
            ViewMode::Recent => ViewMode::All,
        }
    }
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
    pub title: String,
}

pub enum Message {
    Library(Vec<LocalTrack>),
    RowMeta(PathBuf, RowMeta),
    ProbeDone,
    SearchResults(Vec<OnlineResult>),
    Resolved {
        source: Source,
        title: String,
        artist: String,
    },
    ResolveFailed,
    Lyrics(Lyrics),
    Waveform(Vec<f32>),
    Loudness(PathBuf, f64),
    Artwork(Vec<String>),
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

    pub artwork: Option<Vec<String>>,
    pub waveform: Vec<f32>,
    pub waveform_ready: bool,
    pub waveform_at: Instant,

    pub lyrics: Lyrics,
    pub lyrics_status: String,
    pub lyrics_status_at: Instant,

    pub status: String,
    pub row_meta: HashMap<PathBuf, RowMeta>,
    pub sort_mode: SortMode,
    pub view_mode: ViewMode,
    pub folder_filter: Option<String>,
    pub artist_filter: Option<String>,
    pub stats: Stats,

    pub settings_tab: i32,
    pub settings_row: i32,
    pub settings_col: i32,
    pub edit_buffer: String,

    pub loading: bool,
    pub load_started: Instant,
    pub online_enabled: bool,

    pub quit: bool,
    pub last_width: i32,
    play_counted: bool,

    tx: Sender<Message>,
    rx: Receiver<Message>,
    media_rx: Receiver<MediaKey>,
    seed: u64,
    last_mode: u8,
    force_redraw: bool,
    probe_started: bool,
}

impl App {
    pub fn new() -> App {
        let cfg = config::load();
        let restored = session::load();
        let lang = cfg.language.strings();
        let (tx, rx) = channel();
        let (media_tx, media_rx) = channel();
        mediakeys::listen(media_tx);

        let mut stats = Stats::load();
        let imported = if stats.liked_count() == 0 && stats.plays.is_empty() {
            stats::import_from_tlk_player(&mut stats)
        } else {
            None
        };

        let mut player = Player::new(cfg.output_device.clone());
        player.set_crossfade_ms(cfg.crossfade_ms);
        player.set_eq(cfg.eq);
        player.set_volume(if restored.volume > 0 {
            restored.volume
        } else {
            70
        });
        let mut spectrum = Spectrum::new();
        spectrum.set_fluidity(cfg.viz_fluidity);
        spectrum.set_decay(cfg.viz_decay);
        spectrum.set_viscosity(cfg.viz_viscosity);

        let sort_mode = match restored.sort {
            1 => SortMode::Artist,
            2 => SortMode::Folder,
            3 => SortMode::Recent,
            _ => SortMode::Name,
        };
        let queue = restored
            .queue
            .iter()
            .map(|q| QueueItem {
                title: q.title.clone(),
                path: q.path.as_ref().map(PathBuf::from),
                id: q.id.clone(),
            })
            .collect();

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

            queue,
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

            artwork: None,
            waveform: Vec::new(),
            waveform_ready: false,
            waveform_at: Instant::now(),

            lyrics: Lyrics::default(),
            lyrics_status: String::new(),
            lyrics_status_at: Instant::now(),

            status: match imported {
                Some((likes, plays)) if likes + plays > 0 => {
                    format!("imported {likes} likes and {plays} tracks from tlk-player")
                }
                _ => String::new(),
            },
            row_meta: HashMap::new(),
            sort_mode,
            view_mode: ViewMode::All,
            folder_filter: None,
            artist_filter: None,
            stats,

            settings_tab: 0,
            settings_row: 0,
            settings_col: 0,
            edit_buffer: String::new(),

            loading: false,
            load_started: Instant::now(),
            online_enabled: false,

            quit: false,
            last_width: 155,
            play_counted: false,

            tx,
            rx,
            media_rx,
            seed: 0x2545_F491_4F6C_DD1D,
            last_mode: 255,
            force_redraw: true,
            probe_started: false,
        }
    }

    pub fn roots(&self) -> Vec<String> {
        if self.cfg.music_paths.is_empty() {
            local::default_paths()
        } else {
            self.cfg.music_paths.clone()
        }
    }

    pub fn rescan(&mut self) {
        self.probe_started = false;
        self.start_library_scan();
    }

    fn start_library_scan(&mut self) {
        let roots = self.roots();
        let tx = self.tx.clone();
        self.status = self.lang.scanning.to_string();
        std::thread::spawn(move || {
            let _ = tx.send(Message::Library(local::scan(&roots)));
        });
    }

    /// Reads tags for the whole library in the background, using the on-disk
    /// cache so only new or edited files are opened again.
    fn start_probe(&mut self) {
        if self.probe_started {
            return;
        }
        self.probe_started = true;
        let files: Vec<(PathBuf, u64, u64)> = self
            .tracks
            .iter()
            .map(|t| (t.path.clone(), library::stamp(t.modified), t.size))
            .collect();
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            let mut cache = Cache::load();
            let present: Vec<PathBuf> = files.iter().map(|(p, _, _)| p.clone()).collect();
            cache.prune(&present);

            for (path, mtime, size) in &files {
                if let Some(entry) = cache.get(path, *mtime, *size) {
                    let meta = RowMeta {
                        duration: entry.duration,
                        artist: entry.artist.clone(),
                        title: entry.title.clone(),
                    };
                    if tx.send(Message::RowMeta(path.clone(), meta)).is_err() {
                        return;
                    }
                    continue;
                }
                let Some(info) = decoder::probe(path) else {
                    continue;
                };
                cache.put(
                    path,
                    library::Entry {
                        mtime: *mtime,
                        size: *size,
                        duration: info.duration,
                        artist: info.artist.clone(),
                        title: info.title.clone(),
                        year: info.year.clone(),
                        sample_rate: info.sample_rate,
                        channels: info.channels,
                        bits: info.bits.clone(),
                    },
                );
                let meta = RowMeta {
                    duration: info.duration,
                    artist: info.artist,
                    title: info.title,
                };
                if tx.send(Message::RowMeta(path.clone(), meta)).is_err() {
                    return;
                }
            }
            cache.save();
            let _ = tx.send(Message::ProbeDone);
        });
    }

    /// The tag title when one was read, otherwise the filename. Libraries
    /// named "001 - Artist - Title.mp3" read far better this way.
    pub fn display_title(&self, track: &LocalTrack) -> String {
        match self.row_meta.get(&track.path) {
            Some(meta) if !meta.title.trim().is_empty() => meta.title.clone(),
            _ => track.title.clone(),
        }
    }

    pub fn display_artist(&self, track: &LocalTrack) -> String {
        match self.row_meta.get(&track.path) {
            Some(meta) if !meta.artist.trim().is_empty() => meta.artist.clone(),
            _ => track.folder.clone(),
        }
    }

    pub fn refresh_view(&mut self) {
        let query = self.last_local_query.clone();
        let folder = self.folder_filter.clone();
        let artist_filter = self.artist_filter.clone();
        let mode = self.view_mode;
        let meta = &self.row_meta;
        let stats = &self.stats;

        let mut rows: Vec<LocalTrack> = self
            .tracks
            .iter()
            .filter(|t| match mode {
                ViewMode::All => true,
                ViewMode::Liked => stats.is_liked(&t.path),
                ViewMode::Played => stats.plays(&t.path) > 0,
                ViewMode::Recent => stats.last_played(&t.path) > 0,
            })
            .filter(|t| match &folder {
                Some(f) => &t.folder == f,
                None => true,
            })
            .filter(|t| match &artist_filter {
                Some(want) => meta
                    .get(&t.path)
                    .map(|m| m.artist.eq_ignore_ascii_case(want))
                    .unwrap_or(false),
                None => true,
            })
            .filter(|t| {
                if query.is_empty() {
                    return true;
                }
                let (artist, title) = match meta.get(&t.path) {
                    Some(m) => (m.artist.clone(), m.title.clone()),
                    None => (String::new(), String::new()),
                };
                local::match_score(&query, &t.title) > 0.0
                    || local::match_score(&query, &title) > 0.0
                    || local::match_score(&query, &artist) > 0.0
                    || local::match_score(&query, &t.folder) > 0.0
            })
            .cloned()
            .collect();

        if query.is_empty() {
            match mode {
                ViewMode::Played => {
                    rows.sort_by_key(|t| std::cmp::Reverse(self.stats.plays(&t.path)))
                }
                ViewMode::Recent => {
                    rows.sort_by_key(|t| std::cmp::Reverse(self.stats.last_played(&t.path)))
                }
                _ => {
                    let lookup = self.row_meta.clone();
                    local::sort(&mut rows, self.sort_mode, move |t| {
                        lookup
                            .get(&t.path)
                            .map(|m| m.artist.clone())
                            .filter(|a| !a.is_empty())
                            .unwrap_or_else(|| t.folder.clone())
                    });
                }
            }
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
        let max_scroll = self.list_len().saturating_sub(LIST_ROWS);
        if self.selected < self.scroll {
            self.scroll = self.selected;
        }
        if self.selected >= self.scroll + LIST_ROWS {
            self.scroll = self.selected + 1 - LIST_ROWS;
        }
        self.scroll = self.scroll.min(max_scroll);
    }

    fn clamp_queue_scroll(&mut self) {
        let max_scroll = self.queue.len().saturating_sub(LIST_ROWS);
        if self.queue_selected < self.queue_scroll {
            self.queue_scroll = self.queue_selected;
        }
        if self.queue_selected >= self.queue_scroll + LIST_ROWS {
            self.queue_scroll = self.queue_selected + 1 - LIST_ROWS;
        }
        self.queue_scroll = self.queue_scroll.min(max_scroll);
    }

    fn next_random(&mut self) -> u64 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        self.seed
    }

    /// Fills the metadata panel from the stream header and returns what the
    /// decoder found, so the caller can size the playback buffer.
    fn describe(&mut self, source: &Source, fallback_title: &str) -> TrackInfo {
        let info = decoder::probe_source(source).unwrap_or(TrackInfo {
            sample_rate: 44100,
            channels: 2,
            ..Default::default()
        });

        let (title, artist, format, size, location) = match source {
            Source::File(path) => {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let artist = if info.artist.is_empty() {
                    path.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string()
                } else {
                    info.artist.clone()
                };
                (
                    if info.title.is_empty() {
                        stem
                    } else {
                        info.title.clone()
                    },
                    artist,
                    path.extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_uppercase(),
                    local::human_size(std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)),
                    path.parent()
                        .map(|p| p.display().to_string())
                        .unwrap_or_default(),
                )
            }
            Source::Remote { hint, .. } => (
                if info.title.is_empty() {
                    fallback_title.to_string()
                } else {
                    info.title.clone()
                },
                info.artist.clone(),
                hint.to_uppercase(),
                String::new(),
                "stream".to_string(),
            ),
        };

        self.meta = MetaView {
            name: title,
            artist,
            year: info.year.clone(),
            sampling: format!("{} Hz", info.sample_rate),
            kind: if info.channels >= 2 { "stereo" } else { "mono" }.to_string(),
            format,
            size,
            location,
            extra_label: if info.bits.is_empty() {
                String::new()
            } else {
                self.lang.meta_channels.to_string()
            },
            extra_value: info.bits.clone(),
        };

        self.total_sec = info.duration;
        self.current_path = match source {
            Source::File(path) => Some(path.clone()),
            Source::Remote { .. } => None,
        };
        self.has_track = true;
        self.artwork = None;
        self.waveform.clear();
        self.waveform_ready = false;
        self.lyrics = Lyrics::default();
        self.lyrics_status = self.lang.fetching_lyrics.to_string();
        self.lyrics_status_at = Instant::now();
        self.play_counted = false;
        self.spectrum.reset();
        self.apply_track_gain();
        info
    }

    /// Replay gain for the track that is starting. A file measured on an
    /// earlier run is corrected from the first sample; a new one gets its
    /// correction once the analysis thread reports back.
    fn apply_track_gain(&self) {
        let measured = self
            .current_path
            .as_ref()
            .filter(|_| self.cfg.normalize)
            .and_then(|p| self.stats.loudness(p));
        let db = match measured {
            Some(lufs) => loudness::gain_db(lufs, self.cfg.normalize_target),
            None => 0.0,
        };
        self.player.set_track_gain_db(db);
    }

    pub fn start(&mut self, source: Source, fallback_title: &str) {
        let info = self.describe(&source, fallback_title);
        let title = self.meta.name.clone();
        let artist = self.meta.artist.clone();

        let (rate, channels) = {
            let out = self.player.output();
            (out.sample_rate, out.channels)
        };
        let seconds = if info.duration > 0.0 {
            info.duration
        } else {
            600.0
        };
        let pcm = Arc::new(PcmStream::with_seconds(seconds, rate, channels));

        let decode_source = source.clone();
        let decode_sink = Arc::clone(&pcm);
        std::thread::spawn(move || decoder::decode_into(decode_source, decode_sink));

        let wave_sink = Arc::clone(&pcm);
        let wave_tx = self.tx.clone();
        let smooth = self.cfg.waveform_smooth;
        let measure = self.cfg.normalize
            && self
                .current_path
                .as_ref()
                .map(|p| self.stats.loudness(p).is_none())
                .unwrap_or(false);
        let measured_path = self.current_path.clone();
        std::thread::spawn(move || {
            while !wave_sink.is_done() {
                std::thread::sleep(Duration::from_millis(120));
            }
            if wave_sink.has_failed() {
                return;
            }
            let mut mono = Vec::with_capacity(wave_sink.available_frames());
            wave_sink.for_each_mono(|v| mono.push(v));
            let model = waveform::envelope(&mono, waveform::RESOLUTION, smooth);
            let _ = wave_tx.send(Message::Waveform(model));

            if measure {
                if let (Some(path), Some(lufs)) = (measured_path, loudness::integrated(&wave_sink))
                {
                    let _ = wave_tx.send(Message::Loudness(path, lufs));
                }
            }
        });

        if self.cfg.show_album_art {
            let art_source = source.clone();
            let art_tx = self.tx.clone();
            let cols = self.disk.width();
            let rows = self.disk.height();
            std::thread::spawn(move || {
                let bytes = decoder::artwork(&art_source)
                    .or_else(|| artwork::beside_the_track(&art_source));
                let Some(bytes) = bytes else { return };
                if let Some(cells) = artwork::render(&bytes, cols, rows) {
                    let _ = art_tx.send(Message::Artwork(cells));
                }
            });
        }

        let sink = self.spectrum.sink();
        self.player.play(Arc::clone(&pcm), 0.0, Some(sink));

        let track_path = match &source {
            Source::File(path) => Some(path.clone()),
            Source::Remote { .. } => None,
        };
        let lyrics_tx = self.tx.clone();
        let duration = info.duration;
        std::thread::spawn(move || {
            let result = lyrics::fetch(track_path.as_deref(), &title, &artist, duration);
            let _ = lyrics_tx.send(Message::Lyrics(result));
        });
    }

    pub fn start_local(&mut self, path: PathBuf) {
        self.start(Source::File(path), "");
    }

    fn start_online(&mut self, result: OnlineResult) {
        self.loading = true;
        self.load_started = Instant::now();
        self.status = self.lang.resolving.to_string();
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            // Direct streaming first; a full download is the fallback for
            // hosts or formats that refuse range requests.
            if let Some((url, hint)) = online::stream_url(&result.id) {
                let source = Source::Remote { url, hint };
                if decoder::probe_source(&source).is_some() {
                    let _ = tx.send(Message::Resolved {
                        source,
                        title: result.title,
                        artist: result.uploader,
                    });
                    return;
                }
            }
            match online::resolve(&result.id) {
                Some(path) => {
                    let _ = tx.send(Message::Resolved {
                        source: Source::File(path),
                        title: result.title,
                        artist: result.uploader,
                    });
                }
                None => {
                    let _ = tx.send(Message::ResolveFailed);
                }
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
        self.selected = (self.selected as i32 + delta).rem_euclid(len) as usize;
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

    fn play_queue_entry(&mut self, item: QueueItem) {
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

    fn advance(&mut self) {
        self.player.clear_finished();
        if !self.queue.is_empty() {
            let item = self.queue.remove(0);
            self.clamp_queue();
            self.play_queue_entry(item);
            return;
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
                let _ = tx.send(Message::SearchResults(online::search(&query, 20)));
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
            .first()
            .map(|r| paths::expand(r))
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

    pub fn apply_language(&mut self, language: Language) {
        self.cfg.language = language;
        self.lang = language.strings();
        self.force_redraw = true;
    }

    // -- input ---------------------------------------------------------

    fn handle_browse_key(&mut self, key: Key) {
        let bound = |action: &str| binding(self.cfg.key(action));

        if key == bound("HKeyQuit") {
            self.quit = true;
        } else if key == bound("HKeySetting") {
            self.open_settings();
        } else if key == Key::Char('/') {
            self.mode = Mode::Search;
            self.search_buffer.clear();
        } else if key == bound("HKeySwitchBetweenCards") {
            if self.cfg.show_queue {
                self.queue_focus = !self.queue_focus;
            }
        } else if key == bound("HKeyNavigateUp") {
            self.move_cursor(-1);
        } else if key == bound("HKeyNavigateDown") {
            self.move_cursor(1);
        } else if key == bound("HKeyPlay") {
            self.activate_selection();
        } else if key == bound("HKeyTogglePlayPause") {
            self.player.toggle_pause();
        } else if key == bound("HKeyPlayNextSong") {
            self.play_relative(1);
        } else if key == bound("HKeyPlayPreviousSong") {
            self.play_relative(-1);
        } else if key == bound("HKeySeekForward") {
            self.player.seek(5.0);
        } else if key == bound("HKeySeekBackward") {
            self.player.seek(-5.0);
        } else if key == bound("HKeyIncreaseVolume") {
            let v = self.player.volume() + 5;
            self.player.set_volume(v);
        } else if key == bound("HKeyDecreaseVolume") {
            let v = self.player.volume() - 5;
            self.player.set_volume(v);
        } else if key == bound("HKeyToggleRepeat") {
            self.cfg.play_mode = if self.cfg.play_mode == 1 { 0 } else { 1 };
        } else if key == bound("HKeyToggleShuffle") {
            self.cfg.play_mode = if self.cfg.play_mode == 2 { 0 } else { 2 };
        } else if key == bound("HKeyAddHoveringSongToQueue") {
            self.queue_add_selected();
        } else if key == bound("HKeyRemoveHoveringSongFromQueue") {
            self.remove_from_queue();
        } else if key == bound("HKeyFilterForFolder") {
            self.filter_to_folder();
        } else if key == Key::ShiftUp {
            self.move_queue_entry(-1);
        } else if key == Key::ShiftDown {
            self.move_queue_entry(1);
        } else if key == Key::Char('g') {
            self.filter_to_artist();
        } else if key == bound("HKeyClearFilter") {
            self.clear_filter();
        } else if key == bound("HKeyDownloadStream") {
            self.download_current();
        } else if key == Key::Char('o') {
            self.sort_mode = self.sort_mode.next();
            self.refresh_view();
        } else if key == Key::Char('l') {
            self.toggle_like();
        } else if key == Key::Char('v') {
            self.view_mode = self.view_mode.next();
            self.selected = 0;
            self.scroll = 0;
            self.refresh_view();
        }
    }

    fn toggle_like(&mut self) {
        let target = if self.queue_focus {
            None
        } else {
            self.view.get(self.selected).map(|t| t.path.clone())
        }
        .or_else(|| self.current_path.clone());
        let Some(path) = target else { return };

        let liked = self.stats.toggle_like(&path);
        self.status = format!(
            "{} {}",
            if liked {
                self.lang.liked
            } else {
                self.lang.unliked
            },
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
        );
        if self.view_mode == ViewMode::Liked {
            self.refresh_view();
        }
    }

    /// Counted once a track has been listened to rather than skipped past.
    fn count_play(&mut self) {
        if self.play_counted || !self.has_track {
            return;
        }
        let threshold = if self.total_sec > 0.0 {
            (self.total_sec * 0.25).min(20.0)
        } else {
            20.0
        };
        if self.player.elapsed() < threshold {
            return;
        }
        if let Some(path) = self.current_path.clone() {
            self.stats.record_play(&path);
            self.play_counted = true;
        }
    }

    pub fn view_name(&self) -> &'static str {
        match self.view_mode {
            ViewMode::All => self.lang.view_all,
            ViewMode::Liked => self.lang.view_liked,
            ViewMode::Played => self.lang.view_played,
            ViewMode::Recent => self.lang.view_recent,
        }
    }

    fn open_settings(&mut self) {
        self.mode = Mode::Settings;
        self.settings_tab = 0;
        self.settings_row = 0;
        self.settings_col = 0;
        self.force_redraw = true;
    }

    fn move_cursor(&mut self, delta: i32) {
        if self.queue_focus {
            let last = self.queue.len().saturating_sub(1) as i32;
            self.queue_selected =
                (self.queue_selected as i32 + delta).clamp(0, last.max(0)) as usize;
            self.clamp_queue_scroll();
        } else {
            let last = self.list_len().saturating_sub(1) as i32;
            self.selected = (self.selected as i32 + delta).clamp(0, last.max(0)) as usize;
            self.clamp_scroll();
        }
    }

    fn activate_selection(&mut self) {
        if self.queue_focus {
            if self.queue_selected < self.queue.len() {
                let item = self.queue.remove(self.queue_selected);
                self.clamp_queue();
                self.play_queue_entry(item);
            }
        } else {
            self.play_selected();
        }
    }

    /// Reorders the queue around the highlighted entry.
    fn move_queue_entry(&mut self, delta: i32) {
        if !self.queue_focus || self.queue.is_empty() {
            return;
        }
        let from = self.queue_selected;
        let to = (from as i32 + delta).clamp(0, self.queue.len() as i32 - 1) as usize;
        if from == to {
            return;
        }
        let item = self.queue.remove(from);
        self.queue.insert(to, item);
        self.queue_selected = to;
        self.clamp_queue_scroll();
    }

    fn remove_from_queue(&mut self) {
        if self.queue_focus && self.queue_selected < self.queue.len() {
            self.queue.remove(self.queue_selected);
            self.clamp_queue();
        } else if !self.queue.is_empty() {
            self.queue.pop();
        }
    }

    fn filter_to_folder(&mut self) {
        if self.source == ListSource::Local {
            self.folder_filter = self.view.get(self.selected).map(|t| t.folder.clone());
            self.selected = 0;
            self.refresh_view();
        }
    }

    /// Everything by whoever made the highlighted track. A library that lives
    /// in one flat folder has no other way to group itself.
    fn filter_to_artist(&mut self) {
        if self.source != ListSource::Local {
            return;
        }
        let Some(track) = self.view.get(self.selected).cloned() else {
            return;
        };
        let artist = self.display_artist(&track);
        if artist.trim().is_empty() {
            return;
        }
        self.status = artist.clone();
        self.artist_filter = Some(artist);
        self.folder_filter = None;
        self.selected = 0;
        self.refresh_view();
    }

    fn clear_filter(&mut self) {
        self.folder_filter = None;
        self.artist_filter = None;
        self.last_local_query.clear();
        self.source = ListSource::Local;
        self.selected = 0;
        self.refresh_view();
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

    fn handle_media_key(&mut self, key: MediaKey) {
        match key {
            MediaKey::PlayPause => self.player.toggle_pause(),
            MediaKey::Next => self.play_relative(1),
            MediaKey::Previous => self.play_relative(-1),
            MediaKey::Stop => {
                self.player.stop();
                self.has_track = false;
            }
        }
    }

    fn handle_pointer(&mut self, input: Input) {
        if matches!(self.mode, Mode::Settings | Mode::ColorEdit) {
            settings_screen::handle_pointer(self, input);
            return;
        }
        let layout = layout::layout_for(self, self.last_width.max(40) as usize);
        match input {
            Input::Click { col, row } | Input::Drag { col, row } => {
                let dragging = matches!(input, Input::Drag { .. });
                match layout::hit(&layout, col, row) {
                    Target::PlayPause if !dragging => self.player.toggle_pause(),
                    Target::Previous if !dragging => self.play_relative(-1),
                    Target::Next if !dragging => self.play_relative(1),
                    Target::Seek(per_mille) => {
                        if self.total_sec > 0.0 {
                            self.player
                                .seek_to(self.total_sec * per_mille as f64 / 1000.0);
                        }
                    }
                    Target::Volume(percent) => self.player.set_volume(percent as i32),
                    Target::Search if !dragging => {
                        self.mode = Mode::Search;
                        self.search_buffer.clear();
                    }
                    Target::Settings if !dragging => self.open_settings(),
                    Target::ListRow(index) if !dragging => self.click_list_row(index),
                    Target::QueueRow(index) if !dragging => self.click_queue_row(index),
                    _ => {}
                }
            }
            Input::RightClick { col, row } => match layout::hit(&layout, col, row) {
                Target::ListRow(index) => {
                    let target = self.scroll + index;
                    if target < self.list_len() {
                        self.selected = target;
                        self.queue_add_selected();
                    }
                }
                Target::QueueRow(index) => {
                    let target = self.queue_scroll + index;
                    if target < self.queue.len() {
                        self.queue.remove(target);
                        self.clamp_queue();
                    }
                }
                _ => {}
            },
            Input::Scroll { col, row, up } => {
                let step = if up { -3 } else { 3 };
                let over_queue = self.cfg.show_queue
                    && col >= layout.queue_x.0
                    && col <= layout.queue_x.1
                    && row >= layout.rows_y.0
                    && row <= layout.rows_y.1;
                if over_queue {
                    let max = self.queue.len().saturating_sub(LIST_ROWS) as i32;
                    self.queue_scroll =
                        (self.queue_scroll as i32 + step).clamp(0, max.max(0)) as usize;
                } else {
                    let max = self.list_len().saturating_sub(LIST_ROWS) as i32;
                    self.scroll = (self.scroll as i32 + step).clamp(0, max.max(0)) as usize;
                }
            }
            _ => {}
        }
    }

    /// First click moves the cursor, a second click on the same row plays it.
    fn click_list_row(&mut self, index: usize) {
        let target = self.scroll + index;
        if target >= self.list_len() {
            return;
        }
        self.queue_focus = false;
        if self.selected == target {
            self.play_selected();
        } else {
            self.selected = target;
        }
    }

    fn click_queue_row(&mut self, index: usize) {
        let target = self.queue_scroll + index;
        if target >= self.queue.len() {
            return;
        }
        if self.queue_focus && self.queue_selected == target {
            let item = self.queue.remove(target);
            self.clamp_queue();
            self.play_queue_entry(item);
        } else {
            self.queue_focus = true;
            self.queue_selected = target;
        }
    }

    // -- background results --------------------------------------------

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
                    self.start_probe();
                }
                Message::RowMeta(path, meta) => {
                    self.row_meta.insert(path, meta);
                }
                Message::ProbeDone => {
                    if self.last_local_query.is_empty() {
                        self.refresh_view();
                    }
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
                    source,
                    title,
                    artist,
                } => {
                    self.loading = false;
                    self.status.clear();
                    self.start(source, &title);
                    if !title.is_empty() {
                        self.meta.name = title;
                    }
                    if !artist.is_empty() && self.meta.artist.is_empty() {
                        self.meta.artist = artist;
                    }
                }
                Message::ResolveFailed => {
                    self.loading = false;
                    self.status = self.lang.no_results.to_string();
                }
                Message::Lyrics(result) => {
                    self.lyrics_status = if result.lines.is_empty() {
                        self.lang.no_lyrics.to_string()
                    } else {
                        String::new()
                    };
                    self.lyrics = result;
                }
                Message::Artwork(cells) => self.artwork = Some(cells),
                Message::Loudness(path, lufs) => {
                    self.stats.set_loudness(&path, lufs);
                    if self.current_path.as_ref() == Some(&path) {
                        self.apply_track_gain();
                    }
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

    // -- rendering -----------------------------------------------------

    /// Renders a single frame with the library loaded but nothing playing.
    /// Useful for checking a colour scheme without launching the player.
    pub fn preview(&mut self, width: usize, query: &str) -> String {
        self.last_width = width as i32;
        self.tracks = local::scan(&self.roots());
        self.last_local_query = query.to_string();
        self.refresh_view();
        for track in self.view.iter().take(LIST_ROWS) {
            if let Some(info) = decoder::probe(&track.path) {
                self.row_meta.insert(
                    track.path.clone(),
                    RowMeta {
                        duration: info.duration,
                        artist: info.artist,
                        title: info.title,
                    },
                );
            }
        }
        if let Some(track) = self.view.first().cloned() {
            let source = Source::File(track.path);
            self.describe(&source, "");
            if self.cfg.show_album_art {
                self.artwork = decoder::artwork(&source).and_then(|bytes| {
                    artwork::render(&bytes, self.disk.width(), self.disk.height())
                });
            }
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
        let hard =
            width as i32 != self.last_width || mode_id != self.last_mode || self.force_redraw;
        self.force_redraw = false;
        self.last_width = width as i32;
        self.last_mode = mode_id;

        if matches!(self.mode, Mode::Settings | Mode::ColorEdit) {
            let player_height = ui::panels::player_height(self);
            return settings_screen::build(self, width.max(80), player_height);
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
            for i in 0..left.len().max(right.len()) {
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

    // -- lifecycle -----------------------------------------------------

    fn restore(&mut self, saved: &Session) {
        let Some(track) = saved.track.as_ref().map(PathBuf::from) else {
            return;
        };
        if !track.exists() {
            return;
        }
        self.start_local(track);
        if saved.position > 1.0 {
            self.player.seek_to(saved.position);
        }
        self.player.set_paused(true);
    }

    fn persist(&self) {
        session::save(&Session {
            track: self
                .current_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            position: self.player.elapsed(),
            volume: self.player.volume(),
            paused: self.player.is_paused(),
            sort: match self.sort_mode {
                SortMode::Name => 0,
                SortMode::Artist => 1,
                SortMode::Folder => 2,
                SortMode::Recent => 3,
            },
            queue: self
                .queue
                .iter()
                .map(|q| QueuedTrack {
                    title: q.title.clone(),
                    path: q.path.as_ref().map(|p| p.to_string_lossy().to_string()),
                    id: q.id.clone(),
                })
                .collect(),
        });
    }

    /// Rows shown on the settings screen's PATHS tab.
    pub fn music_path_rows(&self) -> Vec<String> {
        self.roots()
    }

    pub fn path_exists(&self, index: usize) -> bool {
        self.music_path_rows()
            .get(index)
            .map(|p| Path::new(&paths::expand(p)).exists())
            .unwrap_or(false)
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
        let saved = session::load();
        self.restore(&saved);

        let mut last_frame = Instant::now();
        while !self.quit {
            loop {
                let input = console.read();
                match input {
                    Input::None => break,
                    Input::Key(Key::Char('\u{3}')) => {
                        self.quit = true;
                        break;
                    }
                    Input::Key(key) => match self.mode {
                        Mode::Browse => self.handle_browse_key(key),
                        Mode::Search => self.handle_search_key(key),
                        Mode::Settings | Mode::ColorEdit => settings_screen::handle_key(self, key),
                    },
                    Input::Resize => self.force_redraw = true,
                    other => self.handle_pointer(other),
                }
            }

            while let Ok(key) = self.media_rx.try_recv() {
                self.handle_media_key(key);
            }
            self.drain_messages();

            if self.player.device_lost() {
                self.player.reopen();
            }

            let now = Instant::now();
            self.dt = now.duration_since(last_frame).as_secs_f64();
            last_frame = now;

            self.count_play();
            if self.has_track && self.player.finished() {
                self.advance();
            }
            if self.has_track && !self.player.is_paused() {
                self.angle = (self.angle + ANGULAR_VELOCITY * self.cfg.disk_speed * self.dt)
                    % (2.0 * std::f64::consts::PI);
            }

            let width = console.cols().clamp(40, 200) as usize;
            let frame = self.render_frame(width);
            console.write(&frame);
            std::thread::sleep(FRAME);
        }

        self.persist();
        self.stats.save();
        self.player.close();
        console.close();
        let _ = config::save(&self.cfg);
    }
}
