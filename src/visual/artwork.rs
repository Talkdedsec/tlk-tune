use image::imageops::FilterType;

use crate::audio::decoder::Source;
use crate::visual::graphics;

const SIDECAR_NAMES: [&str; 6] = [
    "cover.jpg",
    "cover.png",
    "folder.jpg",
    "folder.png",
    "front.jpg",
    "album.jpg",
];

/// A cover sitting next to the track, the way ripped albums usually store it
/// when the tag itself has no picture.
pub fn beside_the_track(source: &Source) -> Option<Vec<u8>> {
    let Source::File(path) = source else {
        return None;
    };
    let folder = path.parent()?;
    for name in SIDECAR_NAMES {
        let candidate = folder.join(name);
        if let Ok(bytes) = std::fs::read(&candidate) {
            return Some(bytes);
        }
    }
    None
}

/// The same cover, but as a real picture for a terminal that can draw one.
/// Kitty is given a PNG and a size in cells and does its own scaling; sixel
/// needs the pixels, so the cell size has to be known.
pub fn as_image(
    bytes: &[u8],
    cols: usize,
    rows: usize,
    protocol: graphics::Protocol,
    cell_px: (u32, u32),
) -> Option<String> {
    if cols == 0 || rows == 0 {
        return None;
    }
    let width = cols as u32 * cell_px.0.max(1);
    let height = rows as u32 * cell_px.1.max(1);
    let image = image::load_from_memory(bytes).ok()?;

    match protocol {
        graphics::Protocol::Blocks => None,
        graphics::Protocol::Kitty => {
            let scaled = image.resize_exact(width, height, FilterType::Lanczos3);
            let mut png = Vec::new();
            scaled
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .ok()?;
            Some(graphics::kitty(&png, cols, rows, ART_IMAGE_ID))
        }
        graphics::Protocol::Sixel => {
            let scaled = image
                .resize_exact(width, height, FilterType::Lanczos3)
                .to_rgb8();
            Some(graphics::sixel(
                scaled.as_raw(),
                width as usize,
                height as usize,
            ))
        }
    }
}

/// One id for the cover means a new track replaces the old picture instead of
/// piling another one on top of it.
const ART_IMAGE_ID: u32 = 7;

/// Renders cover art into terminal cells using the upper half block, so each
/// cell carries two pixels: the foreground paints the top, the background the
/// bottom. That doubles the vertical resolution and keeps the aspect square,
/// because a cell is about twice as tall as it is wide.
pub fn render(bytes: &[u8], cols: usize, rows: usize) -> Option<Vec<String>> {
    if cols == 0 || rows == 0 {
        return None;
    }
    let image = image::load_from_memory(bytes).ok()?;
    let scaled = image
        .resize_exact(cols as u32, (rows * 2) as u32, FilterType::Lanczos3)
        .to_rgb8();

    let mut out = Vec::with_capacity(rows);
    for row in 0..rows {
        let mut line = String::with_capacity(cols * 24);
        for col in 0..cols {
            let top = scaled.get_pixel(col as u32, (row * 2) as u32).0;
            let bottom = scaled.get_pixel(col as u32, (row * 2 + 1) as u32).0;
            line.push_str(&format!(
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m\u{2580}",
                top[0], top[1], top[2], bottom[0], bottom[1], bottom[2]
            ));
        }
        line.push_str("\x1b[0m");
        out.push(line);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkerboard(size: u32) -> Vec<u8> {
        let mut buffer = image::RgbImage::new(size, size);
        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            *pixel = if (x + y) % 2 == 0 {
                image::Rgb([255, 0, 0])
            } else {
                image::Rgb([0, 0, 255])
            };
        }
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(buffer)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        png.into_inner()
    }

    #[test]
    fn renders_one_row_per_cell_row() {
        let rows = render(&checkerboard(64), 30, 15).expect("render failed");
        assert_eq!(rows.len(), 15);
        for row in &rows {
            assert_eq!(row.matches('\u{2580}').count(), 30);
            assert!(row.ends_with("\x1b[0m"), "row does not reset its colours");
        }
    }

    #[test]
    fn carries_real_colour() {
        let rows = render(&checkerboard(64), 8, 4).expect("render failed");
        assert!(rows[0].contains("\x1b[38;2;"));
        assert!(rows[0].contains("\x1b[48;2;"));
    }

    #[test]
    fn rejects_bytes_that_are_not_an_image() {
        assert!(render(b"not an image at all", 10, 5).is_none());
        assert!(render(&checkerboard(8), 0, 5).is_none());
    }
}
