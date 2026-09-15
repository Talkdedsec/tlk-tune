use std::time::Instant;

use crate::app::{App, ListSource, Mode, ViewMode, LIST_ROWS};
use crate::config::{self, Config};
use crate::source::lyrics::LyricLine;
use crate::text;
use crate::ui::chrome::{Chrome, RESET};
use crate::visual::spectrum::bar_glyph;
use crate::visual::waveform;

const UNDERLINE: &str = "\x1b[4m";

pub fn player_height(app: &App) -> i32 {
    let metadata = app.disk.height() as i32 + 2;
    let progress = 5;
    let search = 3;
    let list = LIST_ROWS as i32 + 2;
    metadata + progress + search + list
}

pub fn metadata(app: &mut App, total_width: usize) -> Vec<String> {
    let cfg = app.cfg.clone();
    let chrome = Chrome::new(&cfg);
    let inner = total_width.saturating_sub(4);
    let disk_w = if cfg.show_disk { app.disk.width() } else { 0 };
    let panel_h = app.disk.height();

    let separator = format!("  {}  ", cfg.meta_separator);
    let fixed = if cfg.show_disk {
        text::width(&separator)
    } else {
        2
    };
    let avail = inner.saturating_sub(disk_w + fixed).max(10);
    let (meta_w, lyrics_w) = if cfg.show_lyrics {
        let m = 42.min(avail.saturating_sub(10).max(10)).min(avail);
        (m, avail - m)
    } else {
        (avail, 0)
    };

    let mut disk_frame: Vec<String> = Vec::new();
    if cfg.show_disk {
        // Cover art takes the record's place when the file has one; the
        // spinning vinyl is what a track without artwork falls back to.
        match app.artwork.as_ref().filter(|rows| rows.len() == panel_h) {
            Some(art) => disk_frame = art.clone(),
            None => {
                disk_frame = app.disk.frame(app.angle);
                while disk_frame.len() < panel_h {
                    disk_frame.push(" ".repeat(disk_w));
                }
                for (i, row) in disk_frame.iter_mut().enumerate() {
                    let t = if panel_h > 1 {
                        i as f32 / (panel_h - 1) as f32
                    } else {
                        0.0
                    };
                    *row = format!(
                        "{}{}{}",
                        config::ramp(&cfg.disk, &cfg.disk_end, t),
                        row,
                        RESET
                    );
                }
            }
        }
    }

    let mut rows: Vec<String> = vec![String::new(); panel_h];
    let mut bars: Vec<i32> = Vec::new();

    if app.has_track {
        let key_colour = if cfg.meta_key.is_empty() {
            config::fg(&cfg.list_fg)
        } else {
            config::fg(&cfg.meta_key)
        };
        let val_colour = if cfg.meta_val.is_empty() {
            config::fg(&cfg.list_fg)
        } else {
            config::fg(&cfg.meta_val)
        };

        let fields = [
            (app.lang.meta_name, app.meta.name.clone()),
            (app.lang.meta_artist, app.meta.artist.clone()),
            (app.lang.meta_year, app.meta.year.clone()),
            (app.lang.meta_sampling, app.meta.sampling.clone()),
            (app.lang.meta_type, app.meta.kind.clone()),
            (app.lang.meta_format, app.meta.format.clone()),
            (app.lang.meta_size, app.meta.size.clone()),
            (app.lang.meta_location, app.meta.location.clone()),
        ];
        for (i, (label, value)) in fields.iter().enumerate() {
            rows[i + 1] = field_row(&cfg, label, value, meta_w, &key_colour, &val_colour);
        }
        if !app.meta.extra_label.is_empty() && panel_h > 9 {
            rows[9] = field_row(
                &cfg,
                &app.meta.extra_label,
                &app.meta.extra_value,
                meta_w,
                &key_colour,
                &val_colour,
            );
        }

        if cfg.show_visualizer {
            let viz_w = meta_w.min(48);
            bars = app.spectrum.bars(viz_w, app.dt);
            let mut top = String::new();
            let mut bottom = String::new();
            for (i, level) in bars.iter().enumerate() {
                let t = if bars.len() > 1 {
                    i as f32 / (bars.len() - 1) as f32
                } else {
                    0.0
                };
                let colour = if cfg.viz_centre.is_empty() {
                    config::ramp(&cfg.viz_left, &cfg.viz_right, t)
                } else {
                    config::ramp3(&cfg.viz_left, &cfg.viz_centre, &cfg.viz_right, t)
                };
                bottom.push_str(&colour);
                bottom.push(bar_glyph((*level).min(4)));
                top.push_str(&colour);
                top.push(bar_glyph((*level - 4).max(0)));
            }
            top.push_str(RESET);
            bottom.push_str(RESET);
            if viz_w < meta_w {
                let tail = " ".repeat(meta_w - viz_w);
                top.push_str(&tail);
                bottom.push_str(&tail);
            }
            rows[panel_h - 2] = top;
            rows[panel_h - 1] = bottom;
        } else {
            bars = app.spectrum.bars(48, app.dt);
        }
    } else {
        rows[0] = text::pad_right(app.lang.no_track, meta_w);
    }

    for row in rows.iter_mut() {
        if row.is_empty() {
            *row = " ".repeat(meta_w);
        }
    }

    let mut lyric_rows: Vec<String> = vec![" ".repeat(lyrics_w); panel_h];
    if cfg.show_lyrics && lyrics_w > 0 {
        // Lyrics run on their own clock so a track's shift can correct
        // timings published for a different master.
        let lyric_time = app.lyric_clock();
        build_lyrics(
            app,
            &cfg,
            &bars,
            lyrics_w,
            panel_h,
            lyric_time,
            &mut lyric_rows,
        );
    }

    let border = config::fg(&cfg.border);
    let border_bottom = config::fg(&cfg.border_bottom);
    let bar = chrome.bar(&border);
    let separator_ansi = format!("{}{}{}", border, separator, RESET);

    let mut out = Vec::with_capacity(panel_h + 2);
    out.push(chrome.top("", total_width, &border));
    for row in 0..panel_h {
        let mut content = String::new();
        if cfg.show_disk {
            content.push_str(&disk_frame[row]);
            content.push_str(&separator_ansi);
        } else {
            content.push_str("  ");
        }
        content.push_str(&rows[row]);
        if cfg.show_lyrics {
            content.push_str(&lyric_rows[row]);
        }
        out.push(format!("{} {} {}", bar, content, bar));
    }
    out.push(chrome.bottom(total_width, "", &border_bottom));
    out
}

fn field_row(
    cfg: &Config,
    label: &str,
    value: &str,
    meta_w: usize,
    key_colour: &str,
    val_colour: &str,
) -> String {
    let label = config::map_font(label, &cfg.font);
    let value = config::map_font(value, &cfg.font);
    let room = meta_w.saturating_sub(12);
    let padded_label = text::pad_right(&label, 10);
    let shown = text::truncate(&value, room);
    let plain_width = text::width(&padded_label) + 2 + text::width(&shown);
    format!(
        "{}{}{}: {}{}{}{}",
        key_colour,
        padded_label,
        RESET,
        val_colour,
        shown,
        RESET,
        " ".repeat(meta_w.saturating_sub(plain_width))
    )
}

fn build_lyrics(
    app: &mut App,
    cfg: &Config,
    bars: &[i32],
    lyrics_w: usize,
    panel_h: usize,
    elapsed: f64,
    out: &mut [String],
) {
    let has_lines = !app.lyrics.lines.is_empty();

    if !has_lines {
        let status = if app.has_track {
            app.lyrics_status.clone()
        } else {
            String::new()
        };
        // A caption only holds the panel for ten seconds; after that the
        // sphere gets the space instead of a permanently stuck line.
        if status != app.lyrics.message {
            app.lyrics.message = status.clone();
            app.lyrics_status_at = Instant::now();
        }
        let show_caption =
            !status.is_empty() && app.lyrics_status_at.elapsed().as_secs_f64() < 10.0;

        if app.has_track && cfg.show_lyric_ball && lyrics_w >= 6 && panel_h >= 3 {
            let rows_h = if show_caption { panel_h - 1 } else { panel_h };
            let sphere = app.sphere.render(lyrics_w, rows_h, bars, app.dt);
            let colour = config::ramp(&cfg.viz_left, &cfg.viz_right, 0.5);
            for (i, line) in sphere.iter().enumerate().take(panel_h) {
                out[i] = format!("{}{}{}", colour, text::pad_right(line, lyrics_w), RESET);
            }
            if show_caption {
                out[panel_h - 1] = centred(&status, lyrics_w);
            }
        } else if show_caption {
            out[0] = centred(&status, lyrics_w);
        }
        return;
    }

    let mut active = 0usize;
    for (i, line) in app.lyrics.lines.iter().enumerate() {
        if line.start <= elapsed {
            active = i;
        } else {
            break;
        }
    }

    match cfg.lyric_animation {
        4 => {
            let line = &app.lyrics.lines[active];
            let mut word = line.text.clone();
            if let Some(first) = line.words.first() {
                word = first.1.clone();
                for (t, w) in &line.words {
                    if *t <= elapsed {
                        word = w.clone();
                    }
                }
            }
            let word = config::map_font(&word, &cfg.font);
            let body = centred(&word, lyrics_w);
            out[panel_h / 2] = format!("{}{}{}", config::fg(&cfg.lyric_word_fg), body, RESET);
        }
        3 => {
            let wrapped = wrap_line(&app.lyrics.lines[active], elapsed, lyrics_w, true, cfg);
            let start = (panel_h.saturating_sub(wrapped.len())) / 2;
            for (i, row) in wrapped.into_iter().enumerate() {
                if start + i < panel_h {
                    out[start + i] = row;
                }
            }
        }
        _ => {
            let context = 6usize;
            let lo = active.saturating_sub(context);
            let hi = (active + context).min(app.lyrics.lines.len() - 1);

            let mut flat: Vec<String> = Vec::new();
            let mut active_start = 0usize;
            let mut active_count = 1usize;
            for i in lo..=hi {
                let wrapped = wrap_line(&app.lyrics.lines[i], elapsed, lyrics_w, i == active, cfg);
                if i == active {
                    active_start = flat.len();
                    active_count = wrapped.len().max(1);
                }
                flat.extend(wrapped);
            }

            let middle = active_start as i32 + active_count as i32 / 2;
            let first = middle - panel_h as i32 / 2;
            for (row, slot) in out.iter_mut().enumerate().take(panel_h) {
                let idx = first + row as i32;
                if idx >= 0 && (idx as usize) < flat.len() {
                    *slot = flat[idx as usize].clone();
                }
            }
        }
    }
}

fn centred(value: &str, width: usize) -> String {
    let pad = width.saturating_sub(text::width(value)) / 2;
    text::pad_right(&format!("{}{}", " ".repeat(pad), value), width)
}

struct Word {
    text: String,
    at: f64,
    timed: bool,
}

/// Word-wraps one lyric line and paints the karaoke state onto it.
fn wrap_line(
    line: &LyricLine,
    elapsed: f64,
    width: usize,
    is_active: bool,
    cfg: &Config,
) -> Vec<String> {
    let words: Vec<Word> = if line.words.is_empty() {
        line.text
            .split_whitespace()
            .map(|w| Word {
                text: w.to_string(),
                at: line.start,
                timed: false,
            })
            .collect()
    } else {
        line.words
            .iter()
            .map(|(t, w)| Word {
                text: w.clone(),
                at: *t,
                timed: true,
            })
            .collect()
    };
    if words.is_empty() || width == 0 {
        return vec![" ".repeat(width)];
    }

    let mut rows: Vec<Vec<&Word>> = Vec::new();
    let mut current: Vec<&Word> = Vec::new();
    let mut used = 0usize;
    for w in &words {
        let w_len = text::width(&w.text);
        let room = if current.is_empty() {
            width
        } else {
            width.saturating_sub(used + 1)
        };
        if w_len <= room {
            used += if current.is_empty() { w_len } else { w_len + 1 };
            current.push(w);
        } else {
            if !current.is_empty() {
                rows.push(std::mem::take(&mut current));
            }
            used = w_len.min(width);
            current.push(w);
        }
    }
    if !current.is_empty() {
        rows.push(current);
    }

    let word_ansi = format!(
        "{}{}",
        config::fg(&cfg.lyric_word_fg),
        config::bg(&cfg.lyric_word_bg)
    );
    let line_ansi = format!(
        "{}{}",
        config::fg(&cfg.lyric_line_fg),
        config::bg(&cfg.lyric_line_bg)
    );
    let idle_ansi = format!("{}{}", config::fg(&cfg.lyric_fg), config::bg(&cfg.lyric_bg));
    let progressive = is_active && (cfg.lyric_animation == 1 || cfg.lyric_animation == 2);

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let mapped: Vec<String> = row
            .iter()
            .map(|w| config::map_font(&w.text, &cfg.font))
            .collect();
        let plain: usize =
            mapped.iter().map(|w| text::width(w)).sum::<usize>() + mapped.len().saturating_sub(1);
        let slack = width.saturating_sub(plain);
        let (left, right) = match cfg.lyric_alignment {
            1 => (0, slack),
            2 => (slack, 0),
            _ => (slack / 2, slack - slack / 2),
        };

        let mut singing: Option<usize> = None;
        for (i, w) in row.iter().enumerate() {
            if is_active && w.timed && w.at <= elapsed {
                singing = Some(i);
            }
        }

        let mut s = " ".repeat(left);
        for (i, w) in row.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            let sung = is_active && w.timed && w.at <= elapsed;
            let untimed_active = is_active && !w.timed;
            let current = sung && singing == Some(i);

            if current && cfg.lyric_animation == 2 {
                let full = &mapped[i];
                let chars = text::width(full);
                let span = word_span(&row, i, elapsed);
                let frac = ((elapsed - w.at) / span).clamp(0.0, 1.0);
                let shown = text::take_cols(full, (frac * chars as f64) as usize);
                let hidden = chars.saturating_sub(text::width(&shown));
                s.push_str(&word_ansi);
                s.push_str(UNDERLINE);
                s.push_str(&shown);
                s.push_str(RESET);
                s.push_str(&" ".repeat(hidden));
            } else if current {
                s.push_str(&word_ansi);
                s.push_str(UNDERLINE);
                s.push_str(&mapped[i]);
                s.push_str(RESET);
            } else if sung {
                s.push_str(&word_ansi);
                s.push_str(&mapped[i]);
                s.push_str(RESET);
            } else if progressive && w.timed {
                s.push_str(&" ".repeat(text::width(&mapped[i])));
            } else if untimed_active {
                s.push_str(&line_ansi);
                s.push_str(&mapped[i]);
                s.push_str(RESET);
            } else {
                s.push_str(&idle_ansi);
                s.push_str(&mapped[i]);
                s.push_str(RESET);
            }
        }
        s.push_str(&" ".repeat(right));
        out.push(s);
    }
    out
}

/// How long the current word should take to reveal. Falls back to the
/// previous word's pace so the last word of a line does not pop in whole.
fn word_span(row: &[&Word], index: usize, _elapsed: f64) -> f64 {
    if index + 1 < row.len() && row[index + 1].timed && row[index + 1].at > row[index].at {
        return row[index + 1].at - row[index].at;
    }
    if index > 0 && row[index - 1].timed && row[index].at > row[index - 1].at {
        return row[index].at - row[index - 1].at;
    }
    0.4
}

pub fn progress(app: &mut App, total_width: usize) -> Vec<String> {
    let cfg = app.cfg.clone();
    let chrome = Chrome::new(&cfg);

    let button_inner = 9usize;
    let button_w = button_inner + 4;
    let side_w = if cfg.show_buttons { button_w * 3 } else { 38 };
    let main_w = total_width.saturating_sub(side_w).max(24);
    let wave_w = main_w.saturating_sub(4);

    let elapsed = if app.has_track {
        app.player.elapsed()
    } else {
        0.0
    };
    let played_cols = if app.total_sec > 0.0 {
        ((elapsed / app.total_sec) * wave_w as f64) as usize
    } else {
        0
    }
    .min(wave_w);

    let reveal_cols = if app.waveform_ready {
        let t = app.waveform_at.elapsed().as_secs_f64();
        if t < 0.7 {
            ((t / 0.7) * wave_w as f64) as usize
        } else {
            wave_w
        }
    } else {
        0
    };

    let played = config::fg(&cfg.progress_played);
    let pending = config::fg(&cfg.progress_pending);

    let levels = if app.waveform.is_empty() {
        vec![0; wave_w]
    } else {
        waveform::fit(&app.waveform, wave_w)
    };

    let (mut top, mut mid, mut bot) = (String::new(), String::new(), String::new());
    for i in 0..wave_w {
        let level = if i < reveal_cols {
            *levels.get(i).unwrap_or(&0)
        } else {
            0
        };
        let column = waveform::column(level);
        if i == 0 {
            let c = if i < played_cols { &played } else { &pending };
            top.push_str(c);
            mid.push_str(c);
            bot.push_str(c);
        } else if i == played_cols {
            top.push_str(&pending);
            mid.push_str(&pending);
            bot.push_str(&pending);
        }
        top.push(column.top);
        mid.push(column.mid);
        bot.push(column.bot);
    }
    top.push_str(RESET);
    mid.push_str(RESET);
    bot.push_str(RESET);

    let blank = " ".repeat(wave_w);
    let plain_bar = if cfg.show_waveform {
        String::new()
    } else {
        let inner = wave_w.saturating_sub(2);
        let filled = if app.total_sec > 0.0 {
            ((elapsed / app.total_sec) * inner as f64) as usize
        } else {
            0
        }
        .min(inner);
        format!(
            "[{}{}{}{}{}{}]",
            played,
            "#".repeat(filled),
            RESET,
            pending,
            "-".repeat(inner - filled),
            RESET
        )
    };

    let border = config::fg(&cfg.border);
    let border_bottom = config::fg(&cfg.border_bottom);
    let button_colour = config::fg(&cfg.button);
    let bar = chrome.bar(&border);

    let button = |label: &str| -> String {
        let body = format!(
            "{}{}{}",
            button_colour,
            text::center(label, button_inner),
            RESET
        );
        format!("{} {} {}", chrome.bar(&border), body, chrome.bar(&border))
    };

    let play_label = if app.has_track && app.player.is_paused() {
        app.lang.play
    } else if app.has_track {
        app.lang.pause
    } else {
        app.lang.play
    };

    let mut out = Vec::with_capacity(5);
    if cfg.show_buttons {
        out.push(format!(
            "{}{}{}{}",
            chrome.top(app.lang.progress_bar, main_w, &border),
            chrome.top("", button_w, &border),
            chrome.top("", button_w, &border),
            chrome.top("", button_w, &border)
        ));
    } else {
        out.push(chrome.top(app.lang.progress_bar, main_w, &border));
    }

    let row1 = if cfg.show_waveform { &top } else { &blank };
    let row2 = if cfg.show_waveform { &mid } else { &plain_bar };
    let row3 = if cfg.show_waveform { &bot } else { &blank };

    if cfg.show_buttons {
        out.push(format!(
            "{} {} {}{}{}{}",
            bar,
            row1,
            bar,
            button("<<<"),
            button(play_label),
            button(">>>")
        ));
        out.push(format!(
            "{} {} {}{}{}{}",
            bar,
            row2,
            bar,
            chrome.bottom(button_w, "", &border_bottom),
            chrome.bottom(button_w, "", &border_bottom),
            chrome.bottom(button_w, "", &border_bottom)
        ));
    } else {
        out.push(format!("{} {} {}", bar, row1, bar));
        out.push(format!("{} {} {}", bar, row2, bar));
    }

    let volume = app.player.volume();
    let hashes = (volume * 20 / 100) as usize;
    let volume_body = format!("{}{}", "#".repeat(hashes), "-".repeat(20 - hashes));
    let visible = text::width(&format!(
        "{}:[{}] {}%",
        app.lang.volume_bar, volume_body, volume
    ));
    let side = total_width.saturating_sub(main_w);
    let mut volume_tail = format!(
        "{}:[{}{}{}] {}%",
        app.lang.volume_bar, button_colour, volume_body, RESET, volume
    );
    if side > visible {
        volume_tail = format!("{}{}", " ".repeat(side - visible), volume_tail);
    }

    let stamp = format!(
        "[ {} ]{}[ {} ]",
        text::mmss(elapsed),
        cfg.edge_h,
        text::mmss(app.total_sec)
    );
    let stamp_colour = if cfg.progress_time.is_empty() {
        config::fg(&cfg.border)
    } else {
        config::fg(&cfg.progress_time)
    };
    let prefix_width = text::width(&format!("{}{} {} ", cfg.corner_bl, cfg.edge_h, stamp));
    let dashes = main_w.saturating_sub(prefix_width + 1);
    let bottom_line = format!(
        "{}{}{} {}{}{} {}{}{}",
        border_bottom,
        cfg.corner_bl,
        cfg.edge_h,
        stamp_colour,
        stamp,
        border_bottom,
        cfg.edge_h.repeat(dashes),
        cfg.corner_br,
        RESET
    );

    out.push(format!("{} {} {}{}", bar, row3, bar, volume_tail));
    out.push(bottom_line);
    out
}

pub fn search_bar(app: &App, total_width: usize) -> Vec<String> {
    let cfg = &app.cfg;
    let chrome = Chrome::new(cfg);
    let label = if app.source == ListSource::Online {
        app.lang.search_online
    } else {
        app.lang.search_local
    };

    let content = match app.mode {
        Mode::Search => format!("/{}\u{2588}", app.search_buffer),
        _ if app.source == ListSource::Online => format!("/s:{}", app.last_online_query),
        _ => format!("/l:{}", app.last_local_query),
    };

    let border = config::fg(&cfg.border);
    let border_bottom = config::fg(&cfg.border_bottom);
    let width = total_width.saturating_sub(5);
    vec![
        format!(
            "{}{}\u{256d}\u{2500}\u{2500}\u{2500}\u{256e}{}",
            chrome.top(label, width, &border),
            border,
            RESET
        ),
        format!(
            "{}{}{} \u{2726} {}{}",
            chrome.line(&content, width, &border),
            border,
            cfg.edge_v,
            cfg.edge_v,
            RESET
        ),
        format!(
            "{}{}\u{2570}\u{2500}\u{2500}\u{2500}\u{256f}{}",
            chrome.bottom(width, "", &border_bottom),
            border_bottom,
            RESET
        ),
    ]
}

pub fn list(app: &App, total_width: usize, height: usize) -> Vec<String> {
    let cfg = &app.cfg;
    let chrome = Chrome::new(cfg);
    let online = app.source == ListSource::Online;
    // The default view and playback mode keep the reference label exactly;
    // anything else names itself so the state is never invisible.
    let label = if online {
        app.lang.online_results.to_string()
    } else {
        let mut notes: Vec<&str> = Vec::new();
        if app.view_mode != ViewMode::All {
            notes.push(app.view_name());
        }
        match cfg.play_mode {
            1 => notes.push(app.lang.mode_loop),
            2 => notes.push(app.lang.mode_shuffle),
            3 => notes.push(app.lang.mode_stop),
            _ => {}
        }
        let mut label = String::from(app.lang.local_files);
        label.push_str(" (");
        for note in notes {
            label.push_str(note);
            label.push_str(", ");
        }
        label.push_str(&format!("{}: {})", app.lang.sort_prefix, app.sort_name()));
        label
    };
    let total = app.list_len();
    let inner = total_width.saturating_sub(4);
    let border = config::fg(&cfg.border);
    let border_bottom = config::fg(&cfg.border_bottom);
    let bar = chrome.bar(&border);

    let mut out = Vec::with_capacity(height + 2);
    out.push(chrome.top(&label, total_width, &border));

    const INDEX_W: usize = 3;
    for row in 0..height {
        let idx = app.scroll + row;
        let mut content = String::new();
        if idx < total {
            if online {
                let r = &app.online[idx];
                let uploader_w = 18usize;
                let title_w = inner.saturating_sub(INDEX_W + 2 + 2 + uploader_w).max(5);
                content = format!(
                    "{}{} {}{} {}",
                    text::pad_right(
                        &config::map_font(&(idx + 1).to_string(), &cfg.font),
                        INDEX_W
                    ),
                    cfg.list_separator,
                    text::pad_right(
                        &text::truncate(&config::map_font(&r.title, &cfg.font), title_w),
                        title_w
                    ),
                    cfg.list_separator,
                    text::pad_right(
                        &text::truncate(&config::map_font(&r.uploader, &cfg.font), uploader_w),
                        uploader_w
                    )
                );
            } else {
                let t = &app.view[idx];
                let artist_w = 16usize;
                let duration_w = 5usize;
                let title_w = inner
                    .saturating_sub(INDEX_W + 2 + 2 + artist_w + 2 + duration_w)
                    .max(5);
                let duration = app
                    .row_meta
                    .get(&t.path)
                    .map(|m| m.duration)
                    .unwrap_or(-1.0);
                let artist = app.display_artist(t);
                content = format!(
                    "{}{} {}{} {}{} {}",
                    text::pad_right(
                        &config::map_font(&(idx + 1).to_string(), &cfg.font),
                        INDEX_W
                    ),
                    cfg.list_separator,
                    text::pad_right(
                        &text::truncate(
                            &config::map_font(&app.display_title(t), &cfg.font),
                            title_w
                        ),
                        title_w
                    ),
                    cfg.list_separator,
                    text::pad_right(
                        &text::truncate(&config::map_font(&artist, &cfg.font), artist_w),
                        artist_w
                    ),
                    cfg.list_separator,
                    config::map_font(&text::mmss(duration), &cfg.font)
                );
            }
        }

        let selected = idx == app.selected && idx < total && !app.queue_focus;
        let playing = app.has_track
            && !online
            && idx < total
            && Some(&app.view[idx].path) == app.current_path.as_ref();

        let liked = !online && idx < total && app.stats.is_liked(&app.view[idx].path);

        let body = text::pad_right(&text::truncate(&content, inner), inner);
        let paint = if selected {
            format!(
                "{}{}",
                config::fg(&cfg.list_cursor_fg),
                config::bg(&cfg.list_cursor_bg)
            )
        } else if playing {
            format!(
                "{}{}",
                config::fg(&cfg.list_playing_fg),
                config::bg(&cfg.list_playing_bg)
            )
        } else if liked {
            format!(
                "{}{}",
                config::fg(&cfg.list_liked_fg),
                config::bg(&cfg.list_bg)
            )
        } else {
            format!("{}{}", config::fg(&cfg.list_fg), config::bg(&cfg.list_bg))
        };
        out.push(format!("{} {}{}{} {}", bar, paint, body, RESET, bar));
    }

    let remaining = total as i64 - (app.scroll + height) as i64;
    let footer = if remaining > 0 {
        format!("( {} {} )", remaining, app.lang.more_suffix)
    } else {
        String::new()
    };
    out.push(chrome.bottom(total_width, &footer, &border_bottom));
    out
}

pub fn queue(app: &App, total_width: usize, height: usize) -> Vec<String> {
    let cfg = &app.cfg;
    let chrome = Chrome::new(cfg);
    let inner = total_width.saturating_sub(4);
    let border = config::fg(&cfg.border);
    let border_bottom = config::fg(&cfg.border_bottom);
    let bar = chrome.bar(&border);

    let title = if app.queue_focus {
        app.lang.queue_focused
    } else {
        app.lang.queue
    };
    let mut out = Vec::with_capacity(height + 2);
    out.push(chrome.top(title, total_width, &border));

    if app.queue.is_empty() {
        let middle = height / 2;
        for row in 0..height {
            let content = if row == middle {
                let label = config::map_font(app.lang.queue_empty, &cfg.font);
                let left = inner.saturating_sub(text::width(&label)) / 2;
                format!("{}{}", " ".repeat(left), label)
            } else {
                String::new()
            };
            let body = text::pad_right(&text::truncate(&content, inner), inner);
            out.push(format!(
                "{} {}{}{} {}",
                bar,
                config::fg(&cfg.queue_fg),
                body,
                RESET,
                bar
            ));
        }
    } else {
        for row in 0..height {
            let idx = app.queue_scroll + row;
            let mut content = String::new();
            let mut playing = false;
            let mut hovering = false;
            if idx < app.queue.len() {
                let item = &app.queue[idx];
                content = format!(
                    "{}{} {}",
                    text::pad_right(&config::map_font(&(idx + 1).to_string(), &cfg.font), 3),
                    cfg.list_separator,
                    config::map_font(&item.title, &cfg.font)
                );
                playing = app.has_track && item.path.is_some() && item.path == app.current_path;
                hovering = app.queue_focus && idx == app.queue_selected;
            }
            let body = text::pad_right(&text::truncate(&content, inner), inner);
            let paint = if hovering {
                format!(
                    "{}{}",
                    config::fg(&cfg.queue_cursor_fg),
                    config::bg(&cfg.queue_cursor_bg)
                )
            } else if playing {
                format!(
                    "{}{}",
                    config::fg(&cfg.queue_playing_fg),
                    config::bg(&cfg.queue_playing_bg)
                )
            } else {
                format!("{}{}", config::fg(&cfg.queue_fg), config::bg(&cfg.queue_bg))
            };
            out.push(format!("{} {}{}{} {}", bar, paint, body, RESET, bar));
        }
    }

    out.push(chrome.bottom(total_width, "", &border_bottom));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;

    /// Starts from a known config rather than whatever the machine has.
    fn fresh() -> App {
        let mut app = App::new();
        app.cfg = crate::config::Config::default();
        app.apply_language(crate::lang::Language::En);
        app
    }

    fn header(app: &App) -> String {
        let line = list(app, 100, 3).remove(0);
        let plain: String = line.chars().filter(|c| *c != '\u{1b}').collect();
        plain
    }

    #[test]
    fn the_default_view_keeps_the_reference_label() {
        let app = fresh();
        let text = header(&app);
        assert!(text.contains("LOCAL AUDIO FILES (sort: name)"), "{text}");
    }

    #[test]
    fn a_non_default_state_names_itself() {
        let mut app = fresh();
        app.cfg.play_mode = 2;
        assert!(header(&app).contains(app.lang.mode_shuffle));

        app.view_mode = crate::app::ViewMode::Liked;
        let text = header(&app);
        assert!(text.contains(app.lang.view_liked), "{text}");
        assert!(text.contains(app.lang.mode_shuffle), "{text}");
        assert!(text.contains("sort: name"), "{text}");
    }
}
