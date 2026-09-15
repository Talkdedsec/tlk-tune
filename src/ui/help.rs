use crate::app::App;
use crate::config;
use crate::text;
use crate::ui::chrome::{Chrome, RESET};

const KEY_WIDTH: usize = 18;

/// Everything the player responds to, on one screen. With this many bindings
/// the alternative is a README nobody has open while listening.
fn rows(app: &App) -> Vec<(String, &'static str)> {
    let s = app.lang;
    let key = |action: &str| app.cfg.key(action).to_string();
    vec![
        (key("HKeySearch"), s.help_search),
        ("/s: …".to_string(), s.help_search_online),
        (key("HKeyPlay"), s.help_play),
        (key("HKeyTogglePlayPause"), s.help_pause),
        (
            format!(
                "{} / {}",
                key("HKeyPlayPreviousSong"),
                key("HKeyPlayNextSong")
            ),
            s.help_skip,
        ),
        ("\u{2190} / \u{2192}".to_string(), s.help_seek),
        (
            format!(
                "{} / {}",
                key("HKeyDecreaseVolume"),
                key("HKeyIncreaseVolume")
            ),
            s.help_volume,
        ),
        (
            format!("{} / {}", key("HKeyToggleShuffle"), key("HKeyToggleRepeat")),
            s.help_modes,
        ),
        (
            format!(
                "{} / {}",
                key("HKeyAddHoveringSongToQueue"),
                key("HKeyRemoveHoveringSongFromQueue")
            ),
            s.help_queue,
        ),
        (key("HKeySwitchBetweenCards"), s.help_panel),
        ("Shift+\u{2191}\u{2193}".to_string(), s.help_reorder),
        ("l".to_string(), s.help_like),
        ("v".to_string(), s.help_view),
        ("o".to_string(), s.help_sort),
        (key("HKeyFilterForFolder"), s.help_folder),
        ("g".to_string(), s.help_artist),
        (key("HKeyClearFilter"), s.help_clear),
        ("t".to_string(), s.help_sleep),
        ("[  ]".to_string(), s.help_lyrics),
        (key("HKeyDownloadStream"), s.help_download),
        (key("HKeySetting"), s.help_settings),
        ("?".to_string(), s.help_help),
        (key("HKeyQuit"), s.help_quit),
    ]
}

fn mouse_rows(app: &App) -> Vec<(&'static str, &'static str)> {
    let s = app.lang;
    vec![
        ("\u{2b24}", s.help_mouse_disk),
        ("<<<  >>>", s.help_mouse_skip),
        ("\u{28ff}\u{28ff}\u{28ff}", s.help_mouse_seek),
        ("[####----]", s.help_mouse_volume),
        ("\u{2937}", s.help_mouse_row),
        ("\u{2937}\u{2937}", s.help_mouse_queue),
        ("\u{2195}", s.help_mouse_scroll),
        ("\u{2726}", s.help_mouse_settings),
    ]
}

pub fn build(app: &App, width: usize, height: usize) -> String {
    let cfg = &app.cfg;
    let chrome = Chrome::new(cfg);
    let border = config::fg(&cfg.border);
    let border_bottom = config::fg(&cfg.border_bottom);
    let bar = chrome.bar(&border);
    let inner = width.saturating_sub(4);
    let dim = config::fg("240");

    let keys = rows(app);
    let mouse = mouse_rows(app);
    let half = inner / 2;

    let mut frame = String::from("\x1b[2J\x1b[H\x1b[?25l");
    frame.push_str(&chrome.top(app.lang.help_title, width, &border));
    frame.push('\n');

    let body_rows = height.saturating_sub(4).max(1);
    let left_count = keys.len().min(body_rows);
    let spare = body_rows.saturating_sub(1);

    let accent = config::fg(&cfg.list_liked_fg);
    // Each cell pads and truncates itself, so a long translation cannot push
    // the right border off the line.
    let cell = |key: &str, what: &str, key_width: usize, total: usize| -> String {
        let room = total.saturating_sub(key_width + 1);
        format!(
            "{}{}{} {}",
            accent,
            text::pad_right(key, key_width),
            RESET,
            text::pad_right(&text::truncate(what, room), room)
        )
    };

    for row in 0..body_rows {
        let left = match keys.get(row) {
            Some((key, what)) if row < left_count => cell(key, what, KEY_WIDTH, half),
            _ => " ".repeat(half),
        };
        let right_width = inner - half;
        let right = if row == 0 {
            format!(
                "{}{}{}",
                dim,
                text::pad_right(app.lang.help_mouse, right_width),
                RESET
            )
        } else {
            match mouse.get(row - 1).filter(|_| row <= spare) {
                Some((gesture, what)) => cell(gesture, what, 12, right_width),
                None => " ".repeat(right_width),
            }
        };
        frame.push_str(&format!("{} {}{} {}\n", bar, left, right, bar));
    }

    frame.push_str(&chrome.bottom(width, app.lang.help_close, &border_bottom));
    frame.push('\n');
    frame.push_str("\x1b[0J");
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(body: &str) -> String {
        let mut out = String::with_capacity(body.len());
        let mut chars = body.chars().peekable();
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
    fn every_line_fills_the_width() {
        let app = App::headless();
        let frame = build(&app, 120, 30);
        for line in strip(&frame).lines().filter(|l| !l.trim().is_empty()) {
            assert_eq!(text::width(line), 120, "line: {line:?}");
        }
    }

    #[test]
    fn it_lists_the_configured_keys_not_hardcoded_ones() {
        let mut app = App::headless();
        app.cfg
            .keys
            .insert("HKeyTogglePlayPause".to_string(), "z".to_string());
        let body = strip(&build(&app, 120, 30));
        assert!(body.contains('z'));
        assert!(body.contains(app.lang.help_pause));
        assert!(body.contains(app.lang.help_mouse));
    }
}
