use crate::app::App;

/// Screen rectangles of the player view, in 1-based terminal coordinates.
///
/// Rendering and hit testing both derive from this, so a click always lands on
/// the thing that was actually drawn there.
pub struct Layout {
    pub disk_x: (usize, usize),
    pub panel_y: (usize, usize),
    pub wave_x: (usize, usize),
    pub wave_y: (usize, usize),
    pub buttons: [(usize, usize); 3],
    pub button_y: usize,
    pub volume_x: (usize, usize),
    pub volume_y: usize,
    pub search_y: usize,
    pub sigil_x: (usize, usize),
    pub rows_y: (usize, usize),
    pub list_x: (usize, usize),
    pub queue_x: (usize, usize),
}

const BUTTON_INNER: usize = 9;
const BUTTON_TOTAL: usize = BUTTON_INNER + 4;

pub fn layout_for(app: &App, width: usize) -> Layout {
    let panel_h = if app.compact { 0 } else { app.disk.height() };
    let disk_w = if app.cfg.show_disk {
        app.disk.width()
    } else {
        0
    };

    // In compact mode the metadata panel is not drawn at all, so the progress
    // panel starts at the top and the disk has no rows to hit.
    let (panel_first, panel_last, progress_top) = if app.compact {
        (0, 0, 1)
    } else {
        (2, 1 + panel_h, 3 + panel_h)
    };

    let side_w = if app.cfg.show_buttons {
        BUTTON_TOTAL * 3
    } else {
        38
    };
    let main_w = width.saturating_sub(side_w).max(24);
    let wave_w = main_w.saturating_sub(4);

    let button_y = progress_top + 1;
    let mut buttons = [(0usize, 0usize); 3];
    for (i, slot) in buttons.iter_mut().enumerate() {
        let start = main_w + 1 + BUTTON_TOTAL * i;
        *slot = (start, start + BUTTON_TOTAL - 1);
    }

    let volume_y = progress_top + 3;
    let volume_body = 20usize;
    let visible = crate::text::width(&format!(
        "{}:[{}] {}%",
        app.lang.volume_bar,
        "#".repeat(volume_body),
        app.player.volume()
    ));
    let side = width.saturating_sub(main_w);
    let pad = side.saturating_sub(visible);
    let label_len = crate::text::width(app.lang.volume_bar) + 2;
    let volume_start = main_w + pad + label_len + 1;

    let search_top = progress_top + 5;
    let list_top = search_top + 3;

    let (list_x, queue_x) = if app.cfg.show_queue {
        let half = width / 2;
        ((1, half), (half + 1, width))
    } else {
        ((1, width), (0, 0))
    };

    Layout {
        disk_x: (3, 2 + disk_w),
        panel_y: (panel_first, panel_last),
        wave_x: (3, 2 + wave_w),
        wave_y: (progress_top + 1, progress_top + 3),
        buttons,
        button_y,
        volume_x: (volume_start, volume_start + volume_body - 1),
        volume_y,
        search_y: search_top + 1,
        sigil_x: (width.saturating_sub(4), width),
        rows_y: (list_top + 1, list_top + app.list_rows.max(1)),
        list_x,
        queue_x,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    None,
    PlayPause,
    Previous,
    Next,
    Seek(u16),
    Volume(u16),
    Search,
    Settings,
    ListRow(usize),
    QueueRow(usize),
}

fn inside(x: usize, range: (usize, usize)) -> bool {
    range.1 >= range.0 && x >= range.0 && x <= range.1
}

pub fn hit(layout: &Layout, col: usize, row: usize) -> Target {
    if inside(col, layout.disk_x) && row >= layout.panel_y.0 && row <= layout.panel_y.1 {
        return Target::PlayPause;
    }
    if row == layout.button_y {
        if inside(col, layout.buttons[0]) {
            return Target::Previous;
        }
        if inside(col, layout.buttons[1]) {
            return Target::PlayPause;
        }
        if inside(col, layout.buttons[2]) {
            return Target::Next;
        }
    }
    if row >= layout.wave_y.0 && row <= layout.wave_y.1 && inside(col, layout.wave_x) {
        let span = (layout.wave_x.1 - layout.wave_x.0 + 1).max(1);
        let offset = col - layout.wave_x.0;
        return Target::Seek(((offset * 1000) / span) as u16);
    }
    if row == layout.volume_y && inside(col, layout.volume_x) {
        let span = (layout.volume_x.1 - layout.volume_x.0 + 1).max(1);
        let offset = col - layout.volume_x.0 + 1;
        return Target::Volume(((offset * 100) / span) as u16);
    }
    if row == layout.search_y {
        if inside(col, layout.sigil_x) {
            return Target::Settings;
        }
        return Target::Search;
    }
    if row >= layout.rows_y.0 && row <= layout.rows_y.1 {
        let index = row - layout.rows_y.0;
        if inside(col, layout.queue_x) {
            return Target::QueueRow(index);
        }
        if inside(col, layout.list_x) {
            return Target::ListRow(index);
        }
    }
    Target::None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(app: &App, col: usize, row: usize) -> Target {
        hit(&layout_for(app, 155), col, row)
    }

    #[test]
    fn transport_buttons_are_clickable() {
        let app = App::new();
        let l = layout_for(&app, 155);
        assert_eq!(
            probe(&app, l.buttons[0].0 + 2, l.button_y),
            Target::Previous
        );
        assert_eq!(
            probe(&app, l.buttons[1].0 + 2, l.button_y),
            Target::PlayPause
        );
        assert_eq!(probe(&app, l.buttons[2].0 + 2, l.button_y), Target::Next);
    }

    #[test]
    fn the_waveform_maps_left_to_right() {
        let app = App::new();
        let l = layout_for(&app, 155);
        assert_eq!(probe(&app, l.wave_x.0, l.wave_y.0), Target::Seek(0));
        match probe(&app, l.wave_x.1, l.wave_y.1) {
            Target::Seek(v) => assert!(v > 980, "right edge mapped to {v}"),
            other => panic!("expected a seek, got {other:?}"),
        }
    }

    #[test]
    fn the_volume_bar_maps_to_percent() {
        let app = App::new();
        let l = layout_for(&app, 155);
        assert_eq!(probe(&app, l.volume_x.1, l.volume_y), Target::Volume(100));
        match probe(&app, l.volume_x.0, l.volume_y) {
            Target::Volume(v) => assert!(v <= 5, "left edge mapped to {v}"),
            other => panic!("expected a volume, got {other:?}"),
        }
    }

    #[test]
    fn rows_split_between_list_and_queue() {
        let app = App::new();
        let l = layout_for(&app, 155);
        assert_eq!(probe(&app, 4, l.rows_y.0), Target::ListRow(0));
        assert_eq!(
            probe(&app, 4, l.rows_y.1),
            Target::ListRow(app.list_rows - 1)
        );
        assert_eq!(
            probe(&app, l.queue_x.0 + 4, l.rows_y.0),
            Target::QueueRow(0)
        );
    }

    #[test]
    fn the_sigil_box_opens_settings() {
        let app = App::new();
        let l = layout_for(&app, 155);
        assert_eq!(probe(&app, l.sigil_x.0 + 2, l.search_y), Target::Settings);
        assert_eq!(probe(&app, 10, l.search_y), Target::Search);
    }

    #[test]
    fn the_disk_toggles_playback() {
        let app = App::new();
        let l = layout_for(&app, 155);
        assert_eq!(
            probe(&app, l.disk_x.0 + 5, l.panel_y.0 + 5),
            Target::PlayPause
        );
    }

    #[test]
    fn a_short_window_drops_the_metadata_panel() {
        let mut app = App::new();

        app.fit_to_height(40);
        assert!(!app.compact);
        assert_eq!(app.list_rows, crate::app::LIST_ROWS);

        // 29 rows go on chrome and the record, so 34 still fits five entries.
        app.fit_to_height(34);
        assert!(!app.compact);
        assert_eq!(app.list_rows, 5);

        app.fit_to_height(30);
        assert!(app.compact, "30 rows cannot hold the record and a list");

        let l = layout_for(&app, 155);
        assert_eq!(l.panel_y, (0, 0));
        assert_eq!(hit(&l, l.wave_x.0, l.wave_y.0), Target::Seek(0));
    }

    #[test]
    fn rows_follow_the_fitted_height() {
        let mut app = App::new();
        app.fit_to_height(31);
        let l = layout_for(&app, 155);
        assert_eq!(l.rows_y.1 - l.rows_y.0 + 1, app.list_rows);
        assert_eq!(hit(&l, 4, l.rows_y.1), Target::ListRow(app.list_rows - 1));
    }
}

#[cfg(test)]
mod frame_tests {
    use super::*;

    /// Strips escapes and returns the visible grid, 0-indexed.
    fn grid(frame: &str) -> Vec<Vec<char>> {
        let mut plain = String::with_capacity(frame.len());
        let mut chars = frame.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\u{1b}' {
                plain.push(c);
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
        plain.lines().map(|l| l.chars().collect()).collect()
    }

    fn find(rows: &[Vec<char>], needle: &str) -> (usize, usize) {
        for (y, row) in rows.iter().enumerate() {
            let line: String = row.iter().collect();
            if let Some(byte) = line.find(needle) {
                let col = line[..byte].chars().count();
                return (col + 1, y + 1);
            }
        }
        panic!("{needle:?} is not in the frame");
    }

    /// The point of this one: hit testing and drawing must not drift apart.
    #[test]
    fn targets_land_on_what_was_drawn() {
        let mut app = App::new();
        let frame = app.render_frame(155);
        let rows = grid(&frame);
        let layout = layout_for(&app, 155);

        let (x, y) = find(&rows, "<<<");
        assert_eq!(hit(&layout, x, y), Target::Previous);

        let (x, y) = find(&rows, ">>>");
        assert_eq!(hit(&layout, x, y), Target::Next);

        let (x, y) = find(&rows, app.lang.play);
        assert_eq!(hit(&layout, x, y), Target::PlayPause);

        let (x, y) = find(&rows, "\u{2726}");
        assert_eq!(hit(&layout, x, y), Target::Settings);

        let bar = format!("{}:[", app.lang.volume_bar);
        let (x, y) = find(&rows, &bar);
        let first = x + bar.chars().count();
        assert_eq!(hit(&layout, first, y), Target::Volume(5));
        assert_eq!(hit(&layout, first + 19, y), Target::Volume(100));
    }
}
