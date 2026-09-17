use crate::app::{App, Mode};
use crate::audio::eq;
use crate::config::{self, Config, LANGUAGES, LYRIC_ALIGNMENTS, LYRIC_ANIMATIONS, PLAY_MODES};
use crate::lang::Language;
use crate::terminal::{Input, Key};
use crate::text;
use crate::visual::graphics;

const RESET: &str = "\x1b[0m";
const INVERT: &str = "\x1b[7m";
const EDITING: &str = "\x1b[41;37m";
const COLOUR_ROWS: usize = 14;
const HOTKEY_ROWS: usize = 11;
const ANIM_ROWS: usize = 13;
const ONOFF_ROWS: usize = 9;
const PATH_FIELD: usize = 60;
const EQ_ROWS: usize = eq::BANDS.len() + 1;
const SLIDER_X: usize = 34;
const SLIDER_W: usize = 25;

pub const TAB_COLORS: i32 = 0;
pub const TAB_ONOFF: i32 = 1;
pub const TAB_ANIMATION: i32 = 2;
pub const TAB_EQ: i32 = 3;
pub const TAB_PATHS: i32 = 4;
pub const TAB_REFERENCE: i32 = 5;
const TAB_COUNT: i32 = 7;

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
        1 => cfg.show_album_art,
        2 => cfg.show_buttons,
        3 => cfg.show_queue,
        4 => cfg.show_waveform,
        5 => cfg.show_lyrics,
        6 => cfg.show_lyric_ball,
        7 => cfg.show_visualizer,
        _ => cfg.normalize,
    }
}

fn set_toggle(cfg: &mut Config, row: usize, value: bool) {
    match row {
        0 => cfg.show_disk = value,
        1 => cfg.show_album_art = value,
        2 => cfg.show_buttons = value,
        3 => cfg.show_queue = value,
        4 => cfg.show_waveform = value,
        5 => cfg.show_lyrics = value,
        6 => cfg.show_lyric_ball = value,
        7 => cfg.show_visualizer = value,
        _ => cfg.normalize = value,
    }
}

fn animation_value(app: &App, row: usize) -> String {
    let cfg = &app.cfg;
    match row {
        0 => cfg.viz_fluidity.to_string(),
        1 => if cfg.waveform_smooth { "smooth" } else { "raw" }.to_string(),
        2 => format!("{:.2}", cfg.disk_speed),
        3 => PLAY_MODES[cfg.play_mode.clamp(0, 3) as usize].to_string(),
        4 => cfg.viz_decay.to_string(),
        5 => cfg.viz_viscosity.to_string(),
        6 => LYRIC_ALIGNMENTS[cfg.lyric_alignment.clamp(0, 2) as usize].to_string(),
        7 => LYRIC_ANIMATIONS[cfg.lyric_animation.clamp(0, 4) as usize].to_string(),
        8 => cfg.language.code().to_string(),
        9 => match &cfg.output_device {
            Some(name) => name.clone(),
            None if app.player.output().name.is_empty() => app.lang.device_default.to_string(),
            None => app.player.output().name.clone(),
        },
        10 => {
            if cfg.crossfade_ms == 0 {
                "gapless".to_string()
            } else {
                format!("{:.1} s", cfg.crossfade_ms as f32 / 1000.0)
            }
        }
        11 => format!("{:.0} LUFS", cfg.normalize_target),
        _ => {
            if cfg.art_mode.is_empty() {
                format!("auto ({})", app.art_protocol.name())
            } else {
                cfg.art_mode.clone()
            }
        }
    }
}

/// A centre-zero slider, so the whole curve is readable at a glance.
fn slider(gain: f32) -> String {
    let middle = SLIDER_W / 2;
    let span = (gain / eq::MAX_GAIN * middle as f32).round() as i32;
    let mut out = String::with_capacity(SLIDER_W * 3);
    for i in 0..SLIDER_W {
        let offset = i as i32 - middle as i32;
        let filled = if span >= 0 {
            offset > 0 && offset <= span
        } else {
            offset < 0 && offset >= span
        };
        if i == middle {
            out.push('\u{253c}');
        } else if filled {
            out.push('\u{2588}');
        } else {
            out.push('\u{2500}');
        }
    }
    out
}

fn eq_label(row: usize) -> String {
    let hz = eq::BANDS[row];
    if hz >= 1000.0 {
        format!("{:.0} kHz", hz / 1000.0)
    } else {
        format!("{:.0} Hz", hz)
    }
}

fn preset_name(app: &App) -> String {
    eq::PRESETS
        .iter()
        .find(|(_, gains)| {
            gains
                .iter()
                .zip(app.cfg.eq.iter())
                .all(|(a, b)| (a - b).abs() < 0.05)
        })
        .map(|(name, _)| name.to_string())
        .unwrap_or_else(|| "custom".to_string())
}

fn set_eq(app: &mut App, gains: [f32; 10]) {
    app.cfg.eq = gains;
    app.player.set_eq(gains);
}

fn cycle_preset(app: &mut App, direction: i32) {
    let current = eq::PRESETS
        .iter()
        .position(|(name, _)| *name == preset_name(app))
        .unwrap_or(0) as i32;
    let next = (current + direction).rem_euclid(eq::PRESETS.len() as i32) as usize;
    set_eq(app, eq::PRESETS[next].1);
}

fn nudge_band(app: &mut App, band: usize, delta: f32) {
    let mut gains = app.cfg.eq;
    gains[band] = (gains[band] + delta).clamp(-eq::MAX_GAIN, eq::MAX_GAIN);
    set_eq(app, gains);
}

fn row_value(app: &App, tab: i32, row: usize, col: usize) -> String {
    match tab {
        TAB_COLORS => colour_value(&app.cfg, row, col),
        TAB_ONOFF => if toggle_value(&app.cfg, row) {
            "true"
        } else {
            "false"
        }
        .to_string(),
        TAB_ANIMATION => animation_value(app, row),
        TAB_REFERENCE => HOTKEY_ACTIONS
            .get(row)
            .map(|a| app.cfg.key(a).to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

pub fn max_row(app: &App) -> usize {
    match app.settings_tab {
        TAB_COLORS => COLOUR_ROWS,
        TAB_ONOFF => ONOFF_ROWS,
        TAB_ANIMATION => ANIM_ROWS,
        TAB_EQ => EQ_ROWS,
        TAB_PATHS => app.music_path_rows().len() + 1,
        TAB_REFERENCE => HOTKEY_ROWS + app.cfg.font.len(),
        _ => app.cfg.about.len().max(1),
    }
}

fn cycle(app: &mut App, direction: i32) {
    let row = app.settings_row as usize;
    match app.settings_tab {
        TAB_ONOFF => {
            let value = !toggle_value(&app.cfg, row);
            set_toggle(&mut app.cfg, row, value);
        }
        TAB_ANIMATION => match row {
            0 => {
                app.cfg.viz_fluidity = (app.cfg.viz_fluidity + direction).clamp(1, 10);
                app.spectrum.set_fluidity(app.cfg.viz_fluidity);
            }
            1 => app.cfg.waveform_smooth = !app.cfg.waveform_smooth,
            2 => {
                let step = ((app.cfg.disk_speed * 100.0).round() as i32 + direction * 5) as f64;
                app.cfg.disk_speed = (step / 100.0).clamp(0.01, 1.0);
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
            8 => {
                let current = LANGUAGES
                    .iter()
                    .position(|l| *l == app.cfg.language.code())
                    .unwrap_or(0) as i32;
                let next = (current + direction).rem_euclid(LANGUAGES.len() as i32) as usize;
                app.apply_language(Language::from_config(LANGUAGES[next]));
            }
            9 => cycle_device(app, direction),
            10 => {
                let next = app.cfg.crossfade_ms as i32 + direction * 250;
                app.cfg.crossfade_ms = next.clamp(0, 12_000) as u32;
                app.player.set_crossfade_ms(app.cfg.crossfade_ms);
            }
            11 => {
                app.cfg.normalize_target =
                    (app.cfg.normalize_target + direction as f64).clamp(-30.0, -6.0);
            }
            _ => cycle_art_mode(app, direction),
        },
        TAB_EQ => {
            if row == 0 {
                cycle_preset(app, direction);
            } else {
                nudge_band(app, row - 1, direction as f32 * 0.5);
            }
        }
        _ => {}
    }
}

/// Walks the ways a cover can be drawn, with "auto" first: an empty setting
/// is the one that asks the terminal rather than being told.
fn cycle_art_mode(app: &mut App, direction: i32) {
    const MODES: [&str; 4] = ["", "blocks", "kitty", "sixel"];
    let current = MODES
        .iter()
        .position(|m| *m == app.cfg.art_mode)
        .unwrap_or(0) as i32;
    let next = (current + direction).rem_euclid(MODES.len() as i32) as usize;
    app.cfg.art_mode = MODES[next].to_string();
    app.art_protocol =
        graphics::Protocol::parse(&app.cfg.art_mode).unwrap_or_else(graphics::detect);
    // Whatever was on screen belongs to the old protocol.
    app.art_image = None;
    app.art_placed = false;
    app.force_redraw();
}

/// Walks the real output list, with "system default" as the first entry.
fn cycle_device(app: &mut App, direction: i32) {
    let mut names: Vec<Option<String>> = vec![None];
    names.extend(app.player.devices().into_iter().map(Some));
    let current = names
        .iter()
        .position(|n| n == &app.cfg.output_device)
        .unwrap_or(0) as i32;
    let next = (current + direction).rem_euclid(names.len() as i32) as usize;
    app.cfg.output_device = names[next].clone();
    let chosen = app.cfg.output_device.clone();
    app.player.use_device(chosen);
}

fn commit_edit(app: &mut App) {
    let row = app.settings_row as usize;
    let col = app.settings_col as usize;
    let value = app.edit_buffer.trim().to_string();
    match app.settings_tab {
        TAB_COLORS => {
            if let Some(field) = colour_field(&mut app.cfg, row, col) {
                *field = value;
            }
        }
        TAB_PATHS => {
            let mut rows = app.music_path_rows();
            if value.is_empty() {
                if row < rows.len() {
                    rows.remove(row);
                }
            } else if row < rows.len() {
                rows[row] = value;
            } else {
                rows.push(value);
            }
            app.cfg.music_paths = rows;
            app.rescan();
        }
        TAB_REFERENCE => {
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

fn begin_edit(app: &mut App) {
    let row = app.settings_row as usize;
    match app.settings_tab {
        TAB_COLORS => {
            app.edit_buffer = colour_value(&app.cfg, row, app.settings_col as usize);
            app.mode = Mode::ColorEdit;
        }
        TAB_REFERENCE => {
            if row < HOTKEY_ROWS {
                app.edit_buffer = row_value(app, TAB_REFERENCE, row, 0);
                app.mode = Mode::ColorEdit;
            }
        }
        TAB_PATHS => {
            app.edit_buffer = app.music_path_rows().get(row).cloned().unwrap_or_default();
            app.mode = Mode::ColorEdit;
        }
        _ => {}
    }
}

fn remove_path(app: &mut App) {
    if app.settings_tab != TAB_PATHS {
        return;
    }
    let row = app.settings_row as usize;
    let mut rows = app.music_path_rows();
    if row < rows.len() {
        rows.remove(row);
        app.cfg.music_paths = rows;
        app.settings_row = (app.settings_row - 1).max(0);
        app.rescan();
    }
}

fn field_limit(app: &App) -> usize {
    match app.settings_tab {
        TAB_PATHS => 200,
        TAB_REFERENCE => 20,
        _ => 7,
    }
}

pub fn handle_key(app: &mut App, key: Key) {
    if app.mode == Mode::ColorEdit {
        let limit = field_limit(app);
        match key {
            Key::Enter => commit_edit(app),
            Key::Esc => {
                app.mode = Mode::Settings;
                app.edit_buffer.clear();
            }
            Key::Backspace => {
                app.edit_buffer.pop();
            }
            Key::Char(c) if !c.is_control() && app.edit_buffer.chars().count() < limit => {
                app.edit_buffer.push(c)
            }
            _ => {}
        }
        return;
    }

    match key {
        Key::Tab => {
            app.settings_tab = (app.settings_tab + 1) % TAB_COUNT;
            app.settings_row = 0;
            app.settings_col = 0;
        }
        Key::Up => app.settings_row = (app.settings_row - 1).max(0),
        Key::Down => {
            let limit = max_row(app) as i32 - 1;
            app.settings_row = (app.settings_row + 1).min(limit.max(0));
        }
        Key::Left => {
            if app.settings_tab == TAB_COLORS {
                app.settings_col = (app.settings_col - 1).max(0);
            } else {
                cycle(app, -1);
            }
        }
        Key::Right => {
            if app.settings_tab == TAB_COLORS {
                app.settings_col = (app.settings_col + 1).min(1);
            } else {
                cycle(app, 1);
            }
        }
        Key::Enter => begin_edit(app),
        Key::Char('a') | Key::Char('A') if app.settings_tab == TAB_PATHS => {
            app.settings_row = max_row(app) as i32 - 1;
            app.edit_buffer.clear();
            app.mode = Mode::ColorEdit;
        }
        Key::Char('0') if app.settings_tab == TAB_EQ => set_eq(app, [0.0; 10]),
        Key::Char('d') | Key::Char('D') if app.settings_tab == TAB_PATHS => remove_path(app),
        Key::Char('r') | Key::Char('R') if app.settings_tab == TAB_PATHS => app.rescan(),
        Key::Char('s') | Key::Char('S') => {
            app.status = if config::save(&app.cfg).is_ok() {
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

// -- geometry shared by rendering and hit testing ----------------------

struct TabStrip {
    top: Vec<(usize, usize, usize)>,
    bottom: Vec<usize>,
}

fn tab_names(app: &App) -> [&'static str; TAB_COUNT as usize] {
    let s = app.lang;
    [
        s.tab_colors,
        s.tab_onoff,
        s.tab_animation,
        s.tab_eq,
        s.tab_paths,
        s.tab_reference,
        s.tab_about,
    ]
}

fn tab_strip(app: &App, width: usize) -> TabStrip {
    let names = tab_names(app);
    let mut top = Vec::new();
    let mut bottom = Vec::new();
    let mut track = 22usize;
    let mut x = 23usize;
    let mut last_on_top = names.len();
    for (i, name) in names.iter().enumerate() {
        let cost = name.chars().count() + 10;
        if track + cost < width.saturating_sub(2) {
            track += cost;
            last_on_top = i;
        } else {
            bottom.push(i);
        }
    }
    for (i, name) in names.iter().enumerate() {
        if bottom.contains(&i) {
            continue;
        }
        let label = name.chars().count() + if i as i32 == app.settings_tab { 2 } else { 0 };
        top.push((i, x + 2, x + 1 + label));
        x += 5 + label + if i == last_on_top { 1 } else { 3 };
    }
    TabStrip { top, bottom }
}

/// Content row under a screen row, mirroring how `build` lays each tab out.
fn row_at(app: &App, y: usize) -> Option<usize> {
    if y < 3 {
        return None;
    }
    let offset = y - 3;
    match app.settings_tab {
        TAB_COLORS => {
            // One divider line sits above the LIST group.
            if offset == 5 {
                return None;
            }
            let row = if offset > 5 { offset - 1 } else { offset };
            (row < COLOUR_ROWS).then_some(row)
        }
        TAB_ONOFF => (offset < ONOFF_ROWS).then_some(offset),
        TAB_ANIMATION => (offset < ANIM_ROWS).then_some(offset),
        TAB_EQ => (offset < EQ_ROWS).then_some(offset),
        TAB_PATHS => (offset < max_row(app)).then_some(offset),
        _ => None,
    }
}

pub fn handle_pointer(app: &mut App, input: Input) {
    let width = app.last_width.max(80) as usize;
    let strip = tab_strip(app, width);

    let (col, row, right) = match input {
        Input::Click { col, row } => (col, row, false),
        Input::RightClick { col, row } => (col, row, true),
        Input::Scroll { up, .. } => {
            let delta = if up { -1 } else { 1 };
            let limit = max_row(app) as i32 - 1;
            app.settings_row = (app.settings_row + delta).clamp(0, limit.max(0));
            return;
        }
        _ => return,
    };

    if app.mode == Mode::ColorEdit {
        commit_edit(app);
        return;
    }

    if row <= 2 {
        if let Some((index, _, _)) = strip
            .top
            .iter()
            .find(|(_, x0, x1)| col >= *x0 && col <= *x1)
        {
            app.settings_tab = *index as i32;
            app.settings_row = 0;
            app.settings_col = 0;
        }
        return;
    }

    let Some(target) = row_at(app, row) else {
        return;
    };
    let already = target as i32 == app.settings_row;
    app.settings_row = target as i32;

    match app.settings_tab {
        TAB_COLORS => {
            let col_index = i32::from(col >= 59);
            let same_cell = already && col_index == app.settings_col;
            app.settings_col = col_index;
            if same_cell {
                begin_edit(app);
            }
        }
        TAB_ONOFF | TAB_ANIMATION => cycle(app, if right { -1 } else { 1 }),
        TAB_EQ => {
            if target == 0 {
                cycle_preset(app, if right { -1 } else { 1 });
            } else if (SLIDER_X..SLIDER_X + SLIDER_W).contains(&col) {
                // Drop the band straight onto the dB the pointer is over.
                let middle = (SLIDER_W / 2) as f32;
                let offset = (col - SLIDER_X) as f32 - middle;
                let gain = (offset / middle * eq::MAX_GAIN).clamp(-eq::MAX_GAIN, eq::MAX_GAIN);
                let mut gains = app.cfg.eq;
                gains[target - 1] = (gain * 2.0).round() / 2.0;
                set_eq(app, gains);
            } else {
                nudge_band(app, target - 1, if right { -0.5 } else { 0.5 });
            }
        }
        TAB_PATHS => {
            if right {
                remove_path(app);
            } else if already || target >= app.music_path_rows().len() {
                begin_edit(app);
            }
        }
        TAB_REFERENCE if already => {
            begin_edit(app);
        }
        _ => {}
    }
}

// -- rendering ---------------------------------------------------------

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

    let tabs = tab_names(app);
    let strip = tab_strip(app, width);

    let mut line1 = format!(
        "\u{250c}\u{2500} {} {}\u{2510}",
        s.settings_title,
        "\u{2500}".repeat(17usize.saturating_sub(s.settings_title.chars().count()))
    );
    let mut line2 = String::from("\u{2502}                    \u{2514}");
    let mut used = 22usize;

    for (n, (index, _, _)) in strip.top.iter().enumerate() {
        let label = if *index as i32 == app.settings_tab {
            format!("[{}]", tabs[*index])
        } else {
            tabs[*index].to_string()
        };
        let len = label.chars().count();
        line1.push_str(&format!("  {}  \u{250c}", label));
        line2.push_str(&format!("{}\u{2518}", "\u{2500}".repeat(4 + len)));
        used += 5 + len;
        if n == strip.top.len() - 1 {
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

    at(
        &mut frame,
        1,
        1,
        &format!("{}{}{}", border(1), line1, RESET),
    );
    at(
        &mut frame,
        2,
        1,
        &format!("{}{}{}", border(2), line2, RESET),
    );

    let mut y = 3i32;
    macro_rules! edge {
        ($y:expr) => {
            at(
                &mut frame,
                $y,
                1,
                &format!("{}\u{2502}{}", border($y), RESET),
            );
            at(
                &mut frame,
                $y,
                width as i32,
                &format!("{}\u{2502}{}", border($y), RESET),
            );
        };
    }

    match app.settings_tab {
        TAB_COLORS => {
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
                edge!(y);
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
        TAB_ONOFF | TAB_ANIMATION => {
            let labels: &[&str] = if app.settings_tab == TAB_ONOFF {
                &s.onoff_rows
            } else {
                &s.anim_rows
            };
            for (i, label) in labels.iter().enumerate() {
                edge!(y);
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
        TAB_EQ => {
            edge!(y);
            let selected = app.settings_row == 0;
            at(&mut frame, y, 6, &pad(s.eq_preset, 25, true));
            at(&mut frame, y, 32, ":");
            at(
                &mut frame,
                y,
                35,
                &format!(
                    "{}{}{}",
                    if selected { INVERT } else { "" },
                    pad(&preset_name(app), 20, true),
                    RESET
                ),
            );
            if selected {
                at(&mut frame, y, 57, "\x1b[90m< \u{2194} >\x1b[0m");
            }
            y += 1;

            for band in 0..eq::BANDS.len() {
                edge!(y);
                let selected = band as i32 + 1 == app.settings_row;
                let gain = cfg.eq[band];
                at(
                    &mut frame,
                    y,
                    6,
                    &format!(
                        "{}{}{}",
                        if selected { INVERT } else { "" },
                        pad(&eq_label(band), 10, true),
                        RESET
                    ),
                );
                let tint = if gain > 0.05 {
                    config::fg("34")
                } else if gain < -0.05 {
                    config::fg("203")
                } else {
                    config::fg("240")
                };
                at(
                    &mut frame,
                    y,
                    SLIDER_X as i32,
                    &format!("{}{}{}", tint, slider(gain), RESET),
                );
                at(
                    &mut frame,
                    y,
                    (SLIDER_X + SLIDER_W + 2) as i32,
                    &pad(&format!("{:+.1} dB", gain), 9, false),
                );
                y += 1;
            }
        }
        TAB_PATHS => {
            let rows = app.music_path_rows();
            for i in 0..rows.len() + 1 {
                edge!(y);
                let selected = i as i32 == app.settings_row;
                let editing = selected && app.mode == Mode::ColorEdit;
                let body = if editing {
                    pad(&app.edit_buffer, PATH_FIELD, true)
                } else if i < rows.len() {
                    pad(&rows[i], PATH_FIELD, true)
                } else {
                    pad(s.paths_add, PATH_FIELD, true)
                };
                at(
                    &mut frame,
                    y,
                    6,
                    &format!(
                        "{}{}{}{}",
                        if selected && !editing { INVERT } else { "" },
                        if editing { EDITING } else { "" },
                        body,
                        RESET
                    ),
                );
                if i < rows.len() {
                    let mark = if app.path_exists(i) {
                        format!("{}ok{}", config::fg("34"), RESET)
                    } else {
                        format!("{}{}{}", config::fg("203"), s.paths_missing, RESET)
                    };
                    at(&mut frame, y, 8 + PATH_FIELD as i32, &mark);
                }
                y += 1;
            }
            if cfg.music_paths.is_empty() {
                edge!(y);
                at(
                    &mut frame,
                    y,
                    6,
                    &format!("\x1b[90m{}\x1b[0m", s.no_music_path),
                );
                y += 1;
            }
        }
        TAB_REFERENCE => {
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
                edge!(y);
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
                        row_value(app, TAB_REFERENCE, index, 0)
                    };
                    at(
                        &mut frame,
                        y,
                        35,
                        &format!(
                            "{}{}{}{}",
                            if selected { INVERT } else { "" },
                            if editing { EDITING } else { "" },
                            pad(&raw, 20, true),
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
                edge!(y);
                if let Some(line) = cfg.about.get(scroll + r) {
                    at(&mut frame, y, 6, line);
                }
                y += 1;
            }
        }
    }

    while y < max_y {
        edge!(y);
        y += 1;
    }

    if strip.bottom.is_empty() {
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
        for i in &strip.bottom {
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

    let hint = match app.settings_tab {
        TAB_PATHS => s.paths_hint,
        TAB_EQ => s.eq_hint,
        _ => s.settings_hint,
    };
    at(&mut frame, y, 1, &format!("\x1b[90m{}\x1b[0m", hint));
    y += 1;
    if !app.status.is_empty() {
        at(&mut frame, y, 1, &format!("\x1b[32m{}\x1b[0m", app.status));
    }

    if app.mode == Mode::ColorEdit {
        let (cy, cx) = match app.settings_tab {
            TAB_COLORS => (
                3 + app.settings_row + i32::from(app.settings_row >= 5),
                if app.settings_col == 0 { 38 } else { 59 },
            ),
            TAB_PATHS => (3 + app.settings_row, 6),
            _ => (3 + app.settings_row, 35),
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
    let here = row as i32 == app.settings_row && col as i32 == app.settings_col;
    let selected = here && app.mode != Mode::ColorEdit;
    let editing = here && app.mode == Mode::ColorEdit;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::Input;

    fn visible(frame: &str) -> String {
        let mut out = String::with_capacity(frame.len());
        let mut chars = frame.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\u{1b}' {
                out.push(c);
                continue;
            }
            if chars.peek() == Some(&'[') {
                chars.next();
                for f in chars.by_ref() {
                    if f.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        }
        out
    }

    #[test]
    fn the_paths_tab_lists_folders_and_an_add_row() {
        let mut app = App::headless();
        app.cfg.music_paths = vec!["D:/Music".into(), "D:/Lists/night.m3u".into()];
        app.mode = Mode::Settings;
        app.settings_tab = TAB_PATHS;

        let body = visible(&build(&app, 155, 35));
        assert!(body.contains("D:/Music"));
        assert!(body.contains("night.m3u"));
        assert!(body.contains(app.lang.paths_add));
        assert!(body.contains(app.lang.paths_missing));
        assert_eq!(max_row(&app), 3);
    }

    #[test]
    fn typing_a_path_adds_it_and_clearing_one_removes_it() {
        let mut app = App::headless();
        app.cfg.music_paths = vec!["D:/Music".into()];
        app.mode = Mode::Settings;
        app.settings_tab = TAB_PATHS;

        app.settings_row = 1;
        app.edit_buffer = "E:/More".into();
        app.mode = Mode::ColorEdit;
        commit_edit(&mut app);
        assert_eq!(app.cfg.music_paths, vec!["D:/Music", "E:/More"]);

        app.settings_row = 0;
        remove_path(&mut app);
        assert_eq!(app.cfg.music_paths, vec!["E:/More"]);
    }

    #[test]
    fn clicking_a_tab_switches_to_it() {
        let mut app = App::headless();
        app.mode = Mode::Settings;
        app.last_width = 155;
        let strip = tab_strip(&app, 155);
        let (index, x0, _) = strip.top[TAB_PATHS as usize];
        assert_eq!(index, TAB_PATHS as usize);
        handle_pointer(&mut app, Input::Click { col: x0, row: 1 });
        assert_eq!(app.settings_tab, TAB_PATHS);
    }

    #[test]
    fn clicking_a_toggle_flips_it() {
        let mut app = App::headless();
        app.mode = Mode::Settings;
        app.last_width = 155;
        app.settings_tab = TAB_ONOFF;
        let before = app.cfg.show_disk;
        handle_pointer(&mut app, Input::Click { col: 40, row: 3 });
        assert_eq!(app.settings_row, 0);
        assert_ne!(app.cfg.show_disk, before);
    }

    #[test]
    fn the_colour_grid_maps_clicks_to_the_right_cell() {
        let mut app = App::headless();
        app.mode = Mode::Settings;
        app.last_width = 155;
        app.settings_tab = TAB_COLORS;

        // Row 5 on screen is the divider above LIST, so row 6 is LIST itself.
        handle_pointer(&mut app, Input::Click { col: 60, row: 9 });
        assert_eq!(app.settings_row, 5);
        assert_eq!(app.settings_col, 1);

        handle_pointer(&mut app, Input::Click { col: 40, row: 3 });
        assert_eq!(app.settings_row, 0);
        assert_eq!(app.settings_col, 0);
    }

    #[test]
    fn the_eq_tab_draws_a_slider_per_band() {
        let mut app = App::headless();
        app.mode = Mode::Settings;
        app.settings_tab = TAB_EQ;
        app.cfg.eq = [0.0; 10];
        app.cfg.eq[0] = 6.0;

        let body = visible(&build(&app, 155, 35));
        assert!(body.contains("31 Hz"));
        assert!(body.contains("16 kHz"));
        assert!(body.contains("+6.0 dB"));
        assert!(body.contains(app.lang.eq_preset));
        assert_eq!(max_row(&app), 11);
    }

    #[test]
    fn clicking_the_slider_sets_that_band() {
        let mut app = App::headless();
        app.mode = Mode::Settings;
        app.last_width = 155;
        app.settings_tab = TAB_EQ;

        // Row 4 on screen is band index 3 (250 Hz); the far right is max gain.
        handle_pointer(
            &mut app,
            Input::Click {
                col: SLIDER_X + SLIDER_W - 1,
                row: 3 + 4,
            },
        );
        assert_eq!(app.settings_row, 4);
        assert!(app.cfg.eq[3] > 10.0, "band sat at {}", app.cfg.eq[3]);

        handle_pointer(
            &mut app,
            Input::Click {
                col: SLIDER_X + SLIDER_W / 2,
                row: 3 + 4,
            },
        );
        assert_eq!(app.cfg.eq[3], 0.0);
    }

    #[test]
    fn a_preset_fills_the_whole_curve() {
        let mut app = App::headless();
        app.mode = Mode::Settings;
        app.settings_tab = TAB_EQ;
        app.settings_row = 0;
        cycle_preset(&mut app, 1);
        assert_ne!(app.cfg.eq, [0.0; 10]);
        assert_eq!(preset_name(&app), eq::PRESETS[1].0);

        handle_key(&mut app, Key::Char('0'));
        assert_eq!(app.cfg.eq, [0.0; 10]);
        assert_eq!(preset_name(&app), "flat");
    }

    /// Every row the panel moves the cursor to. Only `ESC [ y ; x H` counts;
    /// colour escapes share the prefix and must not be misread as positions.
    fn positioned_rows(frame: &str) -> Vec<i32> {
        let mut rows = Vec::new();
        let bytes: Vec<char> = frame.chars().collect();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] != '\u{1b}' || bytes[i + 1] != '[' {
                i += 1;
                continue;
            }
            let mut j = i + 2;
            let mut body = String::new();
            while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == ';') {
                body.push(bytes[j]);
                j += 1;
            }
            if j < bytes.len() && bytes[j] == 'H' {
                if let Some((y, _)) = body.split_once(';') {
                    if let Ok(y) = y.parse::<i32>() {
                        rows.push(y);
                    }
                }
            }
            i = j.max(i + 2);
        }
        rows
    }

    #[test]
    fn the_panel_never_draws_below_the_window() {
        let mut app = App::headless();
        app.mode = Mode::Settings;
        for height in [20, 28, 35] {
            let frame = build(&app, 155, height);
            let lowest = positioned_rows(&frame).into_iter().max().unwrap_or(0);
            assert!(
                lowest <= height,
                "at height {height} the panel reached row {lowest}"
            );
        }
    }
}
