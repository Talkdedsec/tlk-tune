use unicode_width::UnicodeWidthChar;

const VIRAMAS: [char; 9] = [
    '\u{094D}', '\u{09CD}', '\u{0A4D}', '\u{0ACD}', '\u{0B4D}', '\u{0BCD}', '\u{0C4D}', '\u{0CCD}',
    '\u{0D4D}',
];

fn is_indic(c: char) -> bool {
    (0x0900..=0x0DFF).contains(&(c as u32))
}

fn cell_width(c: char) -> usize {
    if c == '\0' {
        return 0;
    }
    if is_indic(c) {
        let cp = c as u32;
        let combining = matches!(cp,
            0x0900..=0x0903 | 0x093A..=0x093C | 0x093E..=0x094F | 0x0951..=0x0957 | 0x0962..=0x0963 |
            0x0981..=0x0983 | 0x09BC | 0x09BE..=0x09CD | 0x09D7 | 0x09E2..=0x09E3 |
            0x0A01..=0x0A03 | 0x0A3C | 0x0A3E..=0x0A4D |
            0x0A81..=0x0A83 | 0x0ABC | 0x0ABE..=0x0ACD |
            0x0B01..=0x0B03 | 0x0B3C | 0x0B3E..=0x0B4D |
            0x0B82 | 0x0BBE..=0x0BCD | 0x0BD7 |
            0x0C00..=0x0C04 | 0x0C3E..=0x0C4D | 0x0C55..=0x0C56 |
            0x0C81..=0x0C83 | 0x0CBC | 0x0CBE..=0x0CCD | 0x0CD5..=0x0CD6 |
            0x0D00..=0x0D03 | 0x0D3B..=0x0D3C | 0x0D3E..=0x0D4D | 0x0D57
        );
        return if combining { 0 } else { 1 };
    }
    UnicodeWidthChar::width(c).unwrap_or(0)
}

/// A virama followed by a consonant forms a conjunct that shares one cell.
fn conjunct_merges(prev: char, cur: char, w: usize) -> bool {
    is_indic(cur) && VIRAMAS.contains(&prev) && w == 1
}

pub fn width(s: &str) -> usize {
    let mut cols: isize = 0;
    let mut prev = '\0';
    for c in s.chars() {
        let w = cell_width(c);
        if conjunct_merges(prev, c, w) {
            cols -= 1;
        }
        cols += w as isize;
        prev = c;
    }
    cols.max(0) as usize
}

/// Longest prefix occupying at most `target` columns, never splitting a glyph.
pub fn take_cols(s: &str, target: usize) -> String {
    let mut out = String::new();
    let mut used: isize = 0;
    let mut prev = '\0';
    for c in s.chars() {
        let w = cell_width(c);
        if conjunct_merges(prev, c, w) {
            used -= 1;
        }
        if used + w as isize > target as isize {
            if w == 0 {
                out.push(c);
                prev = c;
                continue;
            }
            break;
        }
        out.push(c);
        used += w as isize;
        prev = c;
    }
    out
}

pub fn pad_right(s: &str, w: usize) -> String {
    if w == 0 {
        return String::new();
    }
    let cur = width(s);
    if cur >= w {
        return take_cols(s, w);
    }
    let mut out = s.to_string();
    out.push_str(&" ".repeat(w - cur));
    out
}

pub fn pad_left(s: &str, w: usize) -> String {
    if w == 0 {
        return String::new();
    }
    let cur = width(s);
    if cur >= w {
        return take_cols(s, w);
    }
    let mut out = " ".repeat(w - cur);
    out.push_str(s);
    out
}

pub fn truncate(s: &str, w: usize) -> String {
    if w == 0 {
        return String::new();
    }
    if width(s) <= w {
        return s.to_string();
    }
    if w <= 3 {
        return take_cols(s, w);
    }
    let mut out = take_cols(s, w - 3);
    out.push_str("...");
    out
}

pub fn center(s: &str, w: usize) -> String {
    let t = truncate(s, w);
    let pad = w.saturating_sub(width(&t));
    let left = pad / 2;
    format!("{}{}{}", " ".repeat(left), t, " ".repeat(pad - left))
}

pub fn mmss(seconds: f64) -> String {
    if seconds < 0.0 {
        return "--:--".to_string();
    }
    let t = seconds as i64;
    format!("{:02}:{:02}", t / 60, t % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_to_exact_columns() {
        assert_eq!(width(&pad_right("abc", 8)), 8);
        assert_eq!(width(&pad_left("abc", 8)), 8);
        assert_eq!(width(&center("abc", 9)), 9);
    }

    #[test]
    fn counts_wide_glyphs_as_two() {
        assert_eq!(width("日本"), 4);
        assert_eq!(width(&pad_right("日本", 10)), 10);
    }

    #[test]
    fn never_splits_a_glyph() {
        assert_eq!(take_cols("日本", 3), "日");
        assert_eq!(width(&take_cols("日本語", 5)), 4);
    }

    #[test]
    fn truncation_keeps_the_budget() {
        assert_eq!(truncate("abcdefghij", 6), "abc...");
        assert_eq!(width(&truncate("abcdefghij", 6)), 6);
    }

    #[test]
    fn formats_timestamps() {
        assert_eq!(mmss(0.0), "00:00");
        assert_eq!(mmss(75.4), "01:15");
        assert_eq!(mmss(-1.0), "--:--");
    }
}
