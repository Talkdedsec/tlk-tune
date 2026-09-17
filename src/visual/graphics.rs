//! Drawing a real picture in a terminal, where the terminal allows it.
//!
//! Two protocols are worth supporting. Kitty's takes a PNG and a size in
//! cells, which is exactly what we have, so nothing has to be guessed. Sixel
//! is older and far more widely implemented — Windows Terminal, xterm, foot,
//! WezTerm, Konsole — but it places the image at its own pixel size, so the
//! size of a cell has to come from somewhere. It comes from the config.
//!
//! Neither one is used unless it is known to work: the cover falls back to
//! half blocks, which draw in anything.

use std::fmt::Write as _;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Protocol {
    /// Half blocks and truecolour, which every terminal can do.
    #[default]
    Blocks,
    Kitty,
    Sixel,
}

impl Protocol {
    pub fn name(self) -> &'static str {
        match self {
            Protocol::Blocks => "blocks",
            Protocol::Kitty => "kitty",
            Protocol::Sixel => "sixel",
        }
    }

    pub fn parse(value: &str) -> Option<Protocol> {
        match value.trim().to_lowercase().as_str() {
            "blocks" | "half blocks" => Some(Protocol::Blocks),
            "kitty" => Some(Protocol::Kitty),
            "sixel" => Some(Protocol::Sixel),
            _ => None,
        }
    }
}

/// What the terminal says about itself. Asking it directly means reading a
/// reply, which means owning stdin for a moment; the environment is enough to
/// recognise the terminals that actually implement either protocol, and being
/// wrong here costs a garbled panel rather than a crash.
pub fn detect() -> Protocol {
    let has = |name: &str| std::env::var_os(name).is_some();
    let value = |name: &str| std::env::var(name).unwrap_or_default().to_lowercase();

    if has("KITTY_WINDOW_ID") || value("TERM").contains("kitty") {
        return Protocol::Kitty;
    }
    let program = value("TERM_PROGRAM");
    if program.contains("wezterm") || value("TERM").contains("wezterm") {
        // WezTerm does both; kitty's is the one that takes a size in cells.
        return Protocol::Kitty;
    }
    if program.contains("ghostty") {
        return Protocol::Kitty;
    }
    if has("WT_SESSION") {
        // Windows Terminal has drawn sixel since 1.22.
        return Protocol::Sixel;
    }
    let term = value("TERM");
    if term.contains("foot") || term.contains("mlterm") || term.contains("sixel") {
        return Protocol::Sixel;
    }
    if term.contains("xterm") && has("XTERM_VERSION") {
        return Protocol::Sixel;
    }
    Protocol::Blocks
}

/// Moves the cursor to a 1-based cell and leaves it there.
pub fn move_to(row: usize, col: usize) -> String {
    format!("\x1b[{row};{col}H")
}

/// Kitty's protocol: a PNG, base64, in chunks, scaled by the terminal into
/// `cols` by `rows` cells. `id` lets a later frame replace this image rather
/// than stacking another one on top of it.
pub fn kitty(png: &[u8], cols: usize, rows: usize, id: u32) -> String {
    const CHUNK: usize = 4096;
    let payload = base64(png);
    let mut out = String::with_capacity(payload.len() + 256);
    // Clear whatever was placed under this id before drawing over it.
    let _ = write!(out, "\x1b_Ga=d,d=i,i={id}\x1b\\");

    let mut first = true;
    let mut rest = payload.as_str();
    while !rest.is_empty() {
        let take = CHUNK.min(rest.len());
        let (piece, tail) = rest.split_at(take);
        let more = u8::from(!tail.is_empty());
        if first {
            let _ = write!(
                out,
                "\x1b_Ga=T,f=100,i={id},q=2,c={cols},r={rows},m={more};{piece}\x1b\\"
            );
            first = false;
        } else {
            let _ = write!(out, "\x1b_Gm={more};{piece}\x1b\\");
        }
        rest = tail;
    }
    out
}

/// Sixel, from 8 bit RGB. Colours are snapped to a 6x6x6 cube plus a grey
/// ramp, with the error pushed into the neighbours, which is what keeps a
/// gradient from banding into stripes at 256 colours.
pub fn sixel(rgb: &[u8], width: usize, height: usize) -> String {
    if width == 0 || height == 0 || rgb.len() < width * height * 3 {
        return String::new();
    }
    let indexed = quantize(rgb, width, height);

    let mut out = String::with_capacity(width * height / 2);
    out.push_str("\x1bPq");
    let _ = write!(out, "\"1;1;{width};{height}");
    for (i, (r, g, b)) in PALETTE.iter().enumerate() {
        // Sixel colour components are percentages, not bytes.
        let _ = write!(
            out,
            "#{};2;{};{};{}",
            i,
            *r as usize * 100 / 255,
            *g as usize * 100 / 255,
            *b as usize * 100 / 255
        );
    }

    let bands = height.div_ceil(6);
    let mut used = Vec::new();
    let mut column = vec![0u8; width];
    for band in 0..bands {
        let top = band * 6;
        used.clear();
        for row in top..(top + 6).min(height) {
            for x in 0..width {
                let colour = indexed[row * width + x];
                if !used.contains(&colour) {
                    used.push(colour);
                }
            }
        }
        used.sort_unstable();

        for (n, colour) in used.iter().enumerate() {
            for (x, slot) in column.iter_mut().enumerate() {
                let mut bits = 0u8;
                for bit in 0..6 {
                    let row = top + bit;
                    if row < height && indexed[row * width + x] == *colour {
                        bits |= 1 << bit;
                    }
                }
                *slot = bits;
            }
            let _ = write!(out, "#{colour}");
            run_length(&column, &mut out);
            if n + 1 < used.len() {
                out.push('$');
            }
        }
        if band + 1 < bands {
            out.push('-');
        }
    }
    out.push_str("\x1b\\");
    out
}

fn run_length(column: &[u8], out: &mut String) {
    let mut i = 0;
    while i < column.len() {
        let value = column[i];
        let mut run = 1;
        while i + run < column.len() && column[i + run] == value {
            run += 1;
        }
        let glyph = (b'?' + value) as char;
        if run > 3 {
            let _ = write!(out, "!{run}{glyph}");
        } else {
            for _ in 0..run {
                out.push(glyph);
            }
        }
        i += run;
    }
}

const CUBE: [u8; 6] = [0, 51, 102, 153, 204, 255];

/// 216 cube entries then 40 greys, which is the same shape as the xterm 256
/// palette and keeps dark cover art from collapsing into three shades.
static PALETTE: std::sync::LazyLock<Vec<(u8, u8, u8)>> = std::sync::LazyLock::new(|| {
    let mut palette = Vec::with_capacity(256);
    for r in CUBE {
        for g in CUBE {
            for b in CUBE {
                palette.push((r, g, b));
            }
        }
    }
    for i in 0..40 {
        let v = (i * 255 / 39) as u8;
        palette.push((v, v, v));
    }
    palette
});

fn nearest(r: i32, g: i32, b: i32) -> u8 {
    let snap = |v: i32| -> (usize, i32) {
        let mut best = 0;
        let mut best_gap = i32::MAX;
        for (i, level) in CUBE.iter().enumerate() {
            let gap = (v - *level as i32).abs();
            if gap < best_gap {
                best_gap = gap;
                best = i;
            }
        }
        (best, best_gap)
    };
    let (ri, rg) = snap(r);
    let (gi, gg) = snap(g);
    let (bi, bg) = snap(b);
    let cube_index = ri * 36 + gi * 6 + bi;
    let cube_error = rg * rg + gg * gg + bg * bg;

    // A grey ramp is finer than the cube along the diagonal, so near-neutral
    // colours land better on it.
    let grey = (r * 30 + g * 59 + b * 11) / 100;
    let step = (grey * 39 / 255).clamp(0, 39);
    let level = step * 255 / 39;
    let grey_error =
        (r - level) * (r - level) + (g - level) * (g - level) + (b - level) * (b - level);

    if grey_error < cube_error {
        (216 + step) as u8
    } else {
        cube_index as u8
    }
}

fn quantize(rgb: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut error = vec![0i32; width * height * 3];
    let mut indexed = vec![0u8; width * height];

    for y in 0..height {
        for x in 0..width {
            let at = y * width + x;
            let mut channel = [0i32; 3];
            for (c, slot) in channel.iter_mut().enumerate() {
                *slot = (rgb[at * 3 + c] as i32 + error[at * 3 + c]).clamp(0, 255);
            }
            let chosen = nearest(channel[0], channel[1], channel[2]);
            indexed[at] = chosen;
            let (pr, pg, pb) = PALETTE[chosen as usize];
            let gaps = [
                channel[0] - pr as i32,
                channel[1] - pg as i32,
                channel[2] - pb as i32,
            ];

            // Floyd-Steinberg, the four neighbours that are still to come.
            let mut spread = |nx: usize, ny: usize, numerator: i32| {
                if nx >= width || ny >= height {
                    return;
                }
                let to = ny * width + nx;
                for c in 0..3 {
                    error[to * 3 + c] += gaps[c] * numerator / 16;
                }
            };
            spread(x + 1, y, 7);
            if x > 0 {
                spread(x - 1, y + 1, 3);
            }
            spread(x, y + 1, 5);
            spread(x + 1, y + 1, 1);
        }
    }
    indexed
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let b = [
            group[0],
            *group.get(1).unwrap_or(&0),
            *group.get(2).unwrap_or(&0),
        ];
        let packed = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(packed >> 18) as usize & 63] as char);
        out.push(B64[(packed >> 12) as usize & 63] as char);
        out.push(if group.len() > 1 {
            B64[(packed >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if group.len() > 2 {
            B64[packed as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_known_answers() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn a_run_collapses_only_when_it_pays() {
        let mut out = String::new();
        run_length(&[0, 0, 0], &mut out);
        assert_eq!(out, "???");

        let mut out = String::new();
        run_length(&[63, 63, 63, 63, 63], &mut out);
        assert_eq!(out, "!5~");
    }

    #[test]
    fn the_palette_is_the_full_256() {
        assert_eq!(PALETTE.len(), 256);
        assert_eq!(PALETTE[0], (0, 0, 0));
        assert_eq!(PALETTE[215], (255, 255, 255));
    }

    #[test]
    fn a_flat_colour_survives_the_round_trip() {
        let index = nearest(255, 0, 0);
        assert_eq!(PALETTE[index as usize], (255, 0, 0));
        let index = nearest(0, 0, 0);
        assert_eq!(PALETTE[index as usize], (0, 0, 0));
    }

    #[test]
    fn sixel_output_is_wrapped_in_the_right_envelope() {
        let rgb = vec![255u8; 4 * 4 * 3];
        let out = sixel(&rgb, 4, 4);
        assert!(out.starts_with("\x1bPq"));
        assert!(out.ends_with("\x1b\\"));
        assert!(out.contains("\"1;1;4;4"));
    }

    #[test]
    fn sixel_refuses_a_buffer_that_is_too_small() {
        assert!(sixel(&[0, 0, 0], 4, 4).is_empty());
        assert!(sixel(&[], 0, 0).is_empty());
    }

    #[test]
    fn kitty_chunks_a_payload_that_does_not_fit_in_one_escape() {
        let png = vec![7u8; 9000];
        let out = kitty(&png, 30, 15, 1);
        assert!(out.contains("a=T,f=100,i=1"));
        assert!(out.contains("c=30,r=15"));
        // More than one continuation, and the last one says it is the last.
        assert!(out.matches("\x1b_Gm=1;").count() >= 1);
        assert!(out.contains("\x1b_Gm=0;"));
    }

    #[test]
    fn protocol_names_survive_a_round_trip() {
        for protocol in [Protocol::Blocks, Protocol::Kitty, Protocol::Sixel] {
            assert_eq!(Protocol::parse(protocol.name()), Some(protocol));
        }
        assert_eq!(Protocol::parse("nonsense"), None);
    }
}
