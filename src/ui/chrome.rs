use crate::config::Config;
use crate::text;

pub const RESET: &str = "\x1b[0m";

/// Every panel is built against its full outer width, borders included, so two
/// panels sitting side by side always add up to the width the caller asked for.
pub struct Chrome<'a> {
    pub cfg: &'a Config,
}

impl<'a> Chrome<'a> {
    pub fn new(cfg: &'a Config) -> Chrome<'a> {
        Chrome { cfg }
    }

    pub fn top(&self, label: &str, width: usize, colour: &str) -> String {
        let tag = if label.is_empty() {
            String::new()
        } else {
            format!(" {} ", label)
        };
        let prefix = format!("{}{}{}", self.cfg.corner_tl, self.cfg.edge_h, tag);
        let used = text::width(&prefix);
        let fill = width.saturating_sub(used + 1);
        let line = format!(
            "{}{}{}",
            prefix,
            self.cfg.edge_h.repeat(fill),
            self.cfg.corner_tr
        );
        self.paint(&text::pad_right(&line, width), colour)
    }

    pub fn bottom(&self, width: usize, footer: &str, colour: &str) -> String {
        let prefix = if footer.is_empty() {
            format!("{}{}", self.cfg.corner_bl, self.cfg.edge_h)
        } else {
            format!("{}{} {} ", self.cfg.corner_bl, self.cfg.edge_h, footer)
        };
        let used = text::width(&prefix);
        let fill = width.saturating_sub(used + 1);
        let line = format!(
            "{}{}{}",
            prefix,
            self.cfg.edge_h.repeat(fill),
            self.cfg.corner_br
        );
        self.paint(&text::pad_right(&line, width), colour)
    }

    pub fn line(&self, content: &str, width: usize, colour: &str) -> String {
        let inner = width.saturating_sub(4);
        let body = text::pad_right(&text::truncate(content, inner), inner);
        let bar = self.bar(colour);
        format!("{} {} {}", bar, body, bar)
    }

    pub fn bar(&self, colour: &str) -> String {
        if colour.is_empty() {
            self.cfg.edge_v.clone()
        } else {
            format!("{}{}{}", colour, self.cfg.edge_v, RESET)
        }
    }

    fn paint(&self, line: &str, colour: &str) -> String {
        if colour.is_empty() {
            line.to_string()
        } else {
            format!("{}{}{}", colour, line, RESET)
        }
    }
}
