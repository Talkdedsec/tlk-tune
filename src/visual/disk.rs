const CELLS_W: usize = 30;
const CELLS_H: usize = 15;
const PIX_W: usize = CELLS_W * 2;
const PIX_H: usize = CELLS_H * 4;

const RADIUS: f64 = 29.0;
const LABEL_OUTER: f64 = 11.0;
const LABEL_INNER: f64 = 4.0;
const SPINDLE: f64 = 1.6;
const GROOVE_PITCH: f64 = 3.0;
const GROOVE_INK: f64 = 2.1;

const DOT_BITS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

pub struct Disk;

impl Disk {
    pub fn width(&self) -> usize {
        CELLS_W
    }

    pub fn height(&self) -> usize {
        CELLS_H
    }

    pub fn frame(&self, angle: f64) -> Vec<String> {
        let cx = (PIX_W - 1) as f64 / 2.0;
        let cy = (PIX_H - 1) as f64 / 2.0;
        // The vertical pixel pitch inside a braille cell is half the
        // horizontal one, so y has to be stretched to keep the disc round.
        let y_gain = 1.0;
        let (sin, cos) = (-angle).sin_cos();

        let mut ink = vec![false; PIX_W * PIX_H];
        for py in 0..PIX_H {
            for px in 0..PIX_W {
                let dx = px as f64 - cx;
                let dy = (py as f64 - cy) * y_gain;
                let rx = dx * cos - dy * sin;
                let ry = dx * sin + dy * cos;
                if inked(rx, ry) {
                    ink[py * PIX_W + px] = true;
                }
            }
        }

        let mut rows = Vec::with_capacity(CELLS_H);
        for cell_y in 0..CELLS_H {
            let mut line = String::with_capacity(CELLS_W * 3);
            for cell_x in 0..CELLS_W {
                let mut pattern: u8 = 0;
                for dy in 0..4 {
                    for dx in 0..2 {
                        if ink[(cell_y * 4 + dy) * PIX_W + cell_x * 2 + dx] {
                            pattern |= DOT_BITS[dy][dx];
                        }
                    }
                }
                line.push(char::from_u32(0x2800 + pattern as u32).unwrap_or(' '));
            }
            rows.push(line);
        }
        rows
    }
}

fn inked(x: f64, y: f64) -> bool {
    let r = (x * x + y * y).sqrt();
    if r > RADIUS || r < SPINDLE {
        return false;
    }
    if r > RADIUS - 2.0 {
        return true;
    }
    if r < LABEL_INNER {
        return false;
    }
    if r < LABEL_OUTER + 1.4 {
        return true;
    }

    // Two thin light streaks across the grooved area; without them a ring
    // pattern would look completely static while it turns.
    let theta = y.atan2(x);
    for offset in [0.55, 0.55 - std::f64::consts::PI] {
        let mut d = theta - offset;
        while d > std::f64::consts::PI {
            d -= 2.0 * std::f64::consts::PI;
        }
        while d < -std::f64::consts::PI {
            d += 2.0 * std::f64::consts::PI;
        }
        if d.abs() < 0.075 {
            return false;
        }
    }

    (RADIUS - r) % GROOVE_PITCH < GROOVE_INK
}
