use crate::app::{App, Mode};
use crate::config::{self, Config, LANGUAGES, LYRIC_ALIGNMENTS, LYRIC_ANIMATIONS, PLAY_MODES};
use crate::lang::Language;
use crate::terminal::Key;
use crate::text;

const RESET: &str = "\x1b[0m";
const INVERT: &str = "\x1b[7m";
const EDITING: &str = "\x1b[41;37m";
const COLOUR_ROWS: usize = 14;
const HOTKEY_ROWS: usize = 11;

const HOTKEY_ACTIONS: [&str; HOTKEY_ROWS] = [
    "HKeySetting",
    "HKeyNavigateUp",
    "HKeyNavigateDown",
    "HKeyTogglePlayPause",
    "HKeyPlayNextSong",
    "HKeyPlayPreviousSong",
    "HKeyToggleRepeat",
    "HKeyToggleShuffle",
    "HKeySearch",
    "HKeySearchOnline",
    "HKeyQuit",
];

fn colour_field(cfg: &mut Config, row: usize, col: usize) -> Option<&mut String> {
    let field = match (row, col) {
        (0, 0) => &mut cfg.border,
        (0, 1) => &mut cfg.border_bottom,
        (1, 0) => &mut cfg.disk,
        (1, 1) => &mut cfg.disk_end,
        (2, 0) => &mut cfg.meta_key,
        (2, 1) => &mut cfg.meta_val,
        (3, 0) => &mut cfg.viz_left,
        (3, 1) => &mut cfg.viz_right,
        (4, 0) => &mut cfg.progress_played,
        (4, 1) => &mut cfg.progress_pending,
        (5, 0) => &mut cfg.list_fg,
        (5, 1) => &mut cfg.list_bg,
        (6, 0) => &mut cfg.list_playing_fg,
        (6, 1) => &mut cfg.list_playing_bg,
        (7, 0) => &mut cfg.list_cursor_fg,
        (7, 1) => &mut cfg.list_cursor_bg,
        (8, 0) => &mut cfg.queue_fg,
        (8, 1) => &mut cfg.queue_bg,
        (9, 0) => &mut cfg.queue_playing_fg,
        (9, 1) => &mut cfg.queue_playing_bg,
        (10, 0) => &mut cfg.queue_cursor_fg,
        (10, 1) => &mut cfg.queue_cursor_bg,
        (11, 0) => &mut cfg.lyric_fg,
        (11, 1) => &mut cfg.lyric_bg,
        (12, 0) => &mut cfg.lyric_line_fg,
        (12, 1) => &mut cfg.lyric_line_bg,
        (13, 0) => &mut cfg.lyric_word_fg,
        (13, 1) => &mut cfg.lyric_word_bg,
        _ => return None,
    };
    Some(field)
}

fn colour_value(cfg: &Config, row: usize, col: usize) -> String {
    let mut clone = cfg.clone();
    colour_field(&mut clone, row, col)
        .map(|v| v.clone())
        .unwrap_or_default()
}

fn toggle_value(cfg: &Config, row: usize) -> bool {
    match row {
        0 => cfg.show_disk,
        1 => cfg.show_buttons,
        2 => cfg.show_queue,
        3 => cfg.show_waveform,
        4 => cfg.show_lyrics,
        5 => cfg.show_lyric_ball,
        _ => cfg.show_visualizer,
    }
}

fn set_toggle(cfg: &mut Config, row: usize, value: bool) {
    match row {
        0 => cfg.show_disk = value,
        1 => cfg.show_buttons = value,
        2 => cfg.show_queue = value,
        3 => cfg.show_waveform = value,
        4 => cfg.show_lyrics = value,
        5 => cfg.show_lyric_ball = value,
        _ => cfg.show_visualizer = value,
    }
}

fn animation_value(cfg: &Config, row: usize) -> String {
    match row {
        0 => cfg.viz_fluidity.to_string(),
        1 => if cfg.waveform_smooth { "smooth" } else { "raw" }.to_string(),
        2 => format!("{:.2}", cfg.disk_speed),
        3 => PLAY_MODES[cfg.play_mode.clamp(0, 3) as usize].to_string(),
        4 => cfg.viz_decay.to_string(),
        5 => cfg.viz_viscosity.to_string(),
        6 => LYRIC_ALIGNMENTS[cfg.lyric_alignment.clamp(0, 2) as usize].to_string(),
        7 => LYRIC_ANIMATIONS[cfg.lyric_animation.clamp(0, 4) as usize].to_string(),
        _ => cfg.language.code().to_string(),
    }
}

fn row_value(app: &App, tab: i32, row: usize, col: usize) -> String {
    match tab {
        0 => colour_value(&app.cfg, row, col),
        1 => if toggle_value(&app.cfg, row) { "true" } else { "false" }.to_string(),
        2 => animation_value(&app.cfg, row),
        3 => HOTKEY_ACTIONS
            .get(row)
            .map(|a| app.cfg.key(a).to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn max_row(app: &App) -> usize {
    match app.settings_tab {
        0 => COLOUR_ROWS,
        1 => 7,
        2 => 9,
        3 => HOTKEY_ROWS + app.cfg.font.len(),
        _ => app.cfg.about.len().max(1),
    }
}

fn cycle(app: &mut App, direction: i32) {
    let row = app.settings_row as usize;
    match app.settings_tab {
        1 => {
            let value = !toggle_value(&app.cfg, row);
            set_toggle(&mut app.cfg, row, value);
        }
        2 => match row {
            0 => {
                app.cfg.viz_fluidity = (app.cfg.viz_fluidity + direction).clamp(1, 10);
                app.spectrum.set_fluidity(app.cfg.viz_fluidity);
            }
            1 => app.cfg.waveform_smooth = !app.cfg.waveform_smooth,
            2 => {
                app.cfg.disk_speed =
                    ((app.cfg.disk_speed * 100.0).round() as i32 + direction * 5) as f64 / 100.0;
                app.cfg.disk_speed = app.cfg.disk_speed.clamp(0.01, 1.0);
            }
            3 => {
                app.cfg.play_mode =
                    (app.cfg.play_mode + direction).rem_euclid(PLAY_MODES.len() as i32)
            }
            4 => {
                app.cfg.viz_decay = (app.cfg.viz_decay + direction).clamp(1, 10);
                app.spectrum.set_decay(app.cfg.viz_decay);
            }
            5 => {
                app.cfg.viz_viscosity = (app.cfg.viz_viscosity + direction).clamp(0, 10);
                app.spectrum.set_viscosity(app.cfg.viz_viscosity);
            }
            6 => {
                app.cfg.lyric_alignment =
                    (app.cfg.lyric_alignment + direction).rem_euclid(LYRIC_ALIGNMENTS.len() as i32)
            }
            7 => {
                app.cfg.lyric_animation =
                    (app.cfg.lyric_animation + direction).rem_euclid(LYRIC_ANIMATIONS.len() as i32)
            }
            _ => {
                let current = LANGUAGES
                    .iter()
                    .position(|l| *l == app.cfg.language.code())
                    .unwrap_or(0) as i32;
                let next = (current + direction).rem_euclid(LANGUAGES.len() as i32) as usize;
                let language = Language::from_config(LANGUAGES[next]);
                app.apply_language(language);
            }
        },
        _ => {}
    }
}

fn commit_edit(app: &mut App) {
    let row = app.settings_row as usize;
    let col = app.settings_col as usize;
    let value = app.edit_buffer.trim().to_string();
    match app.settings_tab {
        0 => {
            if let Some(field) = colour_field(&mut app.cfg, row, col) {
                *field = value;
            }
        }
        3 => {
            if let Some(action) = HOTKEY_ACTIONS.get(row) {
                if !value.is_empty() {
                    app.cfg.keys.insert(action.to_string(), value);
                }
            }
        }
        _ => {}
    }
    app.mode = Mode::Settings;
    app.edit_buffer.clear();
}

pub fn handle_key(app: &mut App, key: Key) {
    if app.mode == Mode::ColorEdit {
        match key {
            Key::Enter => commit_edit(app),
            Key::Esc => {
                app.mode = Mode::Settings;
                app.edit_buffer.clear();
            }
            Key::Backspace => {
                app.edit_buffer.pop();
            }
            Key::Char(c) if !c.is_control() && app.edit_buffer.chars().count() < 7 => {
                app.edit_buffer.push(c)
            }
            _ => {}
        }
        return;
    }

    match key {
        Key::Tab => {
            app.settings_tab = (app.settings_tab + 1) % 5;
            app.settings_row = 0;
            app.settings_col = 0;
        }
        Key::Up => app.settings_row = (app.settings_row - 1).max(0),
        Key::Down => {
            let limit = max_row(app) as i32 - 1;
            app.settings_row = (app.settings_row + 1).min(limit.max(0));
        }
        Key::Left => {
            if app.settings_tab == 0 {
                app.settings_col = (app.settings_col - 1).max(0);
            } else {
                cycle(app, -1);
            }
        }
        Key::Right => {
            if app.settings_tab == 0 {
                app.settings_col = (app.settings_col + 1).min(1);
            } else {
                cycle(app, 1);
            }
        }
        Key::Enter => {
            if app.settings_tab == 0 || (app.settings_tab == 3 && app.settings_row < HOTKEY_ROWS as i32)
            {
                app.edit_buffer = row_value(
                    app,
                    app.settings_tab,
                    app.settings_row as usize,
                    app.settings_col as usize,
                );
                app.mode = Mode::ColorEdit;
            }
        }
        Key::Char('s') | Key::Char('S') => {
            let saved = config::save(&app.cfg).is_ok();
            app.status = if saved {
                app.lang.settings_saved.to_string()
            } else {
                String::new()
            };
        }
        Key::Char('q') | Key::Char('Q') | Key::Esc => {
            app.mode = Mode::Browse;
            app.status.clear();
        }
        _ => {}
    }
}

pub fn build(app: &App, width: usize, player_height: i32) -> String {
    let cfg = &app.cfg;
    let s = app.lang;
    let mut frame = String::from("\x1b[2J\x1b[H\x1b[?25l");

    let max_y = (player_height - 2).max(10);
    let border = |y: i32| -> String {
        let t = if max_y > 1 {
            (y - 1) as f32 / (max_y - 1) as f32
        } else {
            0.0
        };
        config::ramp(&cfg.border, &cfg.border_bottom, t)
    };
    let at = |frame: &mut String, y: i32, x: i32, body: &str| {
        frame.push_str(&format!("\x1b[{};{}H{}", y, x, body));
    };
    let pad = |value: &str, w: usize, left: bool| -> String {
        if text::width(value) >= w {
            value.to_string()
        } else if left {
            text::pad_right(value, w)
        } else {
            text::pad_left(value, w)
        }
    };

    let tabs = [
        s.tab_colors,
        s.tab_onoff,
        s.tab_animation,
        s.tab_reference,
        s.tab_about,
    ];

    let mut top_tabs: Vec<usize> = Vec::new();
    let mut bottom_tabs: Vec<usize> = Vec::new();
    let mut track = 22usize;
    for (i, name) in tabs.iter().enumerate() {
        let cost = name.chars().count() + 10;
        if track + cost < width.saturating_sub(2) {
            top_tabs.push(i);
            track += cost;
        } else {
            bottom_tabs.push(i);
        }
    }

    let mut line1 = String::new();
    let mut line2 = String::new();
    let mut used = 0usize;
    let title_cell = format!(
        "\u{250c}\u{2500} {} {}\u{2510}",
        s.settings_title,
        "\u{2500}".repeat(17usize.saturating_sub(s.settings_title.chars().count()))
    );
    line1.push_str(&title_cell);
    line2.push_str("\u{2502}                    \u{2514}");
    used += 22;

    for (n, i) in top_tabs.iter().enumerate() {
        let label = if *i as i32 == app.settings_tab {
            format!("[{}]", tabs[*i])
        } else {
            tabs[*i].to_string()
        };
        let len = label.chars().count();
        line1.push_str(&format!("  {}  \u{250c}", label));
        line2.push_str(&format!("{}\u{2518}", "\u{2500}".repeat(4 + len)));
        used += 5 + len;
        if n == top_tabs.len() - 1 {
            line1.push('\u{2500}');
            line2.push(' ');
            used += 1;
        } else {
            line1.push_str("\u{2500}\u{2500}\u{2510}");
            line2.push_str("  \u{2514}");
            used += 3;
        }
    }
    if used < width - 1 {
        let fill = width - 1 - used;
        line1.push_str(&"\u{2500}".repeat(fill));
        line2.push_str(&" ".repeat(fill));
    }
    line1.push('\u{2510}');
    line2.push('\u{2502}');

    at(&mut frame, 1, 1, &format!("{}{}{}", border(1), line1, RESET));
    at(&mut frame, 2, 1, &format!("{}{}{}", border(2), line2, RESET));

    let mut y = 3i32;
    match app.settings_tab {
        0 => {
            let groups = [
                s.grp_border,
                s.grp_disk,
                s.grp_metadata,
                s.grp_viz,
                s.grp_progress,
                s.grp_list,
                "",
                "",
                s.grp_queue,
                "",
                "",
                s.grp_lyrics,
                "",
                "",
            ];
            let left_labels = [
                s.col_top,
                s.col_top,
                s.col_key,
                s.col_left,
                s.col_played,
                s.col_inactive_fg,
                s.col_playing_fg,
                s.col_cursor_fg,
                s.col_inactive_fg,
                s.col_playing_fg,
                s.col_cursor_fg,
                s.col_inactive_fg,
                s.col_active_line_fg,
                s.col_active_word_fg,
            ];
            let right_labels = [
                s.col_bottom,
                s.col_bottom,
                s.col_val,
                s.col_right,
                s.col_pending,
                s.col_bg,
                s.col_bg,
                s.col_bg,
                s.col_bg,
                s.col_bg,
                s.col_bg,
                s.col_bg,
                s.col_bg,
                s.col_bg,
            ];

            for i in 0..COLOUR_ROWS {
                if i == 5 {
                    at(
                        &mut frame,
                        y,
                        1,
                        &format!(
                            "{}\u{251c}{}\u{2524}{}",
                            border(y),
                            "\u{2500}".repeat(width - 2),
                            RESET
                        ),
                    );
                    y += 1;
                }
                at(&mut frame, y, 1, &format!("{}\u{2502}{}", border(y), RESET));
                at(
                    &mut frame,
                    y,
                    width as i32,
                    &format!("{}\u{2502}{}", border(y), RESET),
                );

                at(&mut frame, y, 3, &pad(groups[i], 15, true));
                at(&mut frame, y, 18, ":");
                at(&mut frame, y, 20, &pad(left_labels[i], 14, false));
                at(&mut frame, y, 35, ":");
                at(&mut frame, y, 38, &value_cell(app, i, 0, &pad));
                at(&mut frame, y, 48, &pad(right_labels[i], 7, false));
                at(&mut frame, y, 56, ":");
                at(&mut frame, y, 59, &value_cell(app, i, 1, &pad));

                let a = colour_value(cfg, i, 0);
                let b = colour_value(cfg, i, 1);
                let preview = match i {
                    0 | 1 | 3 => config::ramp_swatch(&a, &b, 23),
                    2 => format!(
                        "{}{}{}{}{}{}",
                        config::fg(&a),
                        s.preview_name,
                        RESET,
                        config::fg(&b),
                        s.preview_song,
                        RESET
                    ),
                    4 => format!(
                        "{}[###########{}{}----------]{}",
                        config::fg(&a),
                        RESET,
                        config::fg(&b),
                        RESET
                    ),
                    13 => format!(
                        "{}{}{}{}{}{}{}{}{}{}{}{}",
                        config::bg(&cfg.lyric_line_bg),
                        config::fg(&cfg.lyric_line_fg),
                        s.preview_lyric_a,
                        RESET,
                        config::bg(&b),
                        config::fg(&a),
                        s.preview_lyric_b,
                        RESET,
                        config::bg(&cfg.lyric_bg),
                        config::fg(&cfg.lyric_fg),
                        s.preview_lyric_c,
                        RESET
                    ),
                    _ => format!(
                        "{}{}{}{}",
                        config::bg(&b),
                        config::fg(&a),
                        s.preview_sample,
                        RESET
                    ),
                };
                at(&mut frame, y, 72, &preview);
                y += 1;
            }
        }
        1 | 2 => {
            let labels: &[&str] = if app.settings_tab == 1 {
                &s.onoff_rows
            } else {
                &s.anim_rows
            };
            for (i, label) in labels.iter().enumerate() {
                at(&mut frame, y, 1, &format!("{}\u{2502}{}", border(y), RESET));
                at(
                    &mut frame,
                    y,
                    width as i32,
                    &format!("{}\u{2502}{}", border(y), RESET),
                );
                at(&mut frame, y, 6, &pad(label, 25, true));
                at(&mut frame, y, 32, ":");

                let selected = i as i32 == app.settings_row;
                let value = pad(&row_value(app, app.settings_tab, i, 0), 20, true);
                at(
                    &mut frame,
                    y,
                    35,
                    &format!("{}{}{}", if selected { INVERT } else { "" }, value, RESET),
                );
                if selected {
                    at(&mut frame, y, 57, "\x1b[90m< \u{2194} >\x1b[0m");
                }
                y += 1;
            }
        }
        3 => {
            let letters: Vec<char> = cfg.font.keys().copied().collect();
            let total = HOTKEY_ROWS + 1 + letters.len();
            let visible = (max_y - 3).max(1) as usize;
            let cursor = if (app.settings_row as usize) < HOTKEY_ROWS {
                app.settings_row as usize
            } else {
                app.settings_row as usize + 1
            };
            let scroll = cursor
                .saturating_sub(visible / 2)
                .min(total.saturating_sub(visible));

            for r in 0..visible {
                let index = scroll + r;
                if index >= total {
                    break;
                }
                at(&mut frame, y, 1, &format!("{}\u{2502}{}", border(y), RESET));
                at(
                    &mut frame,
                    y,
                    width as i32,
                    &format!("{}\u{2502}{}", border(y), RESET),
                );
                if index == HOTKEY_ROWS {
                    y += 1;
                    continue;
                }
                if index < HOTKEY_ROWS {
                    at(&mut frame, y, 6, &pad(s.ref_rows[index], 25, true));
                    at(&mut frame, y, 32, ":");
                    let selected = index as i32 == app.settings_row && app.mode != Mode::ColorEdit;
                    let editing = index as i32 == app.settings_row && app.mode == Mode::ColorEdit;
                    let raw = if editing {
                        app.edit_buffer.clone()
                    } else {
                        row_value(app, 3, index, 0)
                    };
                    let value = pad(&raw, 20, true);
                    at(
                        &mut frame,
                        y,
                        35,
                        &format!(
                            "{}{}{}{}",
                            if selected { INVERT } else { "" },
                            if editing { EDITING } else { "" },
                            value,
                            RESET
                        ),
                    );
                } else {
                    let li = index - HOTKEY_ROWS - 1;
                    let c = letters[li];
                    let (upper, lower) = &cfg.font[&c];
                    let selected = (HOTKEY_ROWS + li) as i32 == app.settings_row;
                    let body = format!("{} = {}, {}", c, upper, lower);
                    at(
                        &mut frame,
                        y,
                        6,
                        &format!("{}{}{}", if selected { INVERT } else { "" }, body, RESET),
                    );
                }
                y += 1;
            }
        }
        _ => {
            let visible = (max_y - 3).max(1) as usize;
            let total = cfg.about.len();
            let scroll = (app.settings_row as usize).min(total.saturating_sub(visible));
            for r in 0..visible {
                at(&mut frame, y, 1, &format!("{}\u{2502}{}", border(y), RESET));
                at(
                    &mut frame,
                    y,
                    width as i32,
                    &format!("{}\u{2502}{}", border(y), RESET),
                );
                if let Some(line) = cfg.about.get(scroll + r) {
                    at(&mut frame, y, 6, line);
                }
                y += 1;
            }
        }
    }

    while y < max_y {
        at(&mut frame, y, 1, &format!("{}\u{2502}{}", border(y), RESET));
        at(
            &mut frame,
            y,
            width as i32,
            &format!("{}\u{2502}{}", border(y), RESET),
        );
        y += 1;
    }

    if bottom_tabs.is_empty() {
        at(
            &mut frame,
            y,
            1,
            &format!(
                "{}\u{2514}{}\u{2518}{}",
                border(y),
                "\u{2500}".repeat(width - 2),
                RESET
            ),
        );
    } else {
        let mut line = String::from("\u{2514}\u{2500}");
        for i in &bottom_tabs {
            if *i as i32 == app.settings_tab {
                line.push_str(&format!(" [{}] \u{2500}", tabs[*i]));
            } else {
                line.push_str(&format!("  {}  \u{2500}", tabs[*i]));
            }
        }
        let rest = width.saturating_sub(line.chars().count()).max(1);
        at(
            &mut frame,
            y,
            1,
            &format!(
                "{}{}{}\u{2518}{}",
                border(y),
                line,
                "\u{2500}".repeat(rest - 1),
                RESET
            ),
        );
    }
    y += 1;

    at(&mut frame, y, 1, &format!("\x1b[90m{}\x1b[0m", s.settings_hint));
    y += 1;
    if !app.status.is_empty() {
        at(&mut frame, y, 1, &format!("\x1b[32m{}\x1b[0m", app.status));
    }

    if app.mode == Mode::ColorEdit {
        let (cy, cx) = if app.settings_tab == 0 {
            let row = 3 + app.settings_row + i32::from(app.settings_row >= 5);
            (row, if app.settings_col == 0 { 38 } else { 59 })
        } else {
            (3 + app.settings_row, 35)
        };
        frame.push_str(&format!(
            "\x1b[{};{}H\x1b[?25h",
            cy,
            cx + app.edit_buffer.chars().count() as i32
        ));
    }

    frame
}

fn value_cell(
    app: &App,
    row: usize,
    col: usize,
    pad: &dyn Fn(&str, usize, bool) -> String,
) -> String {
    let selected =
        row as i32 == app.settings_row && col as i32 == app.settings_col && app.mode != Mode::ColorEdit;
    let editing =
        row as i32 == app.settings_row && col as i32 == app.settings_col && app.mode == Mode::ColorEdit;
    let raw = if editing {
        app.edit_buffer.clone()
    } else {
        colour_value(&app.cfg, row, col)
    };
    format!(
        "{}{}{}{}",
        if selected { INVERT } else { "" },
        if editing { EDITING } else { "" },
        pad(&raw, 7, true),
        RESET
    )
}
