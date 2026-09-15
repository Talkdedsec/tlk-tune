const LAT: i32 = 20;
const LON: i32 = 40;
const DOT_BITS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

const BASE_RADIUS: f32 = 2.6;
const CAMERA: f32 = 7.0;
const PITCH: f32 = 0.3;

pub struct Sphere {
    yaw: f32,
}

impl Sphere {
    pub fn new() -> Sphere {
        Sphere { yaw: 0.0 }
    }

    /// Reuses the spectrum bars already computed this frame rather than
    /// running a second analysis. Bass sits at the centre of `bars`.
    pub fn render(&mut self, cols: usize, rows: usize, bars: &[i32], dt: f64) -> Vec<String> {
        let mut out = vec![String::new(); rows];
        if cols == 0 || rows == 0 {
            return out;
        }

        let pix_w = cols * 2;
        let pix_h = rows * 4;
        let mut buf = vec![false; pix_w * pix_h];

        let (bass, macro_band, micro_band) = if bars.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let centre = bars.len() / 2;
            let inner = centre.saturating_sub(bars.len() / 4);
            (
                bars[centre] as f32 / 8.0,
                bars[inner] as f32 / 8.0,
                bars[0] as f32 / 8.0,
            )
        };

        self.yaw += dt as f32 * 0.5;
        let scale = pix_w.min(pix_h) as f32 * 2.8;
        let bass_scale = (bass * 0.9).clamp(0.0, 1.4);

        for lat in 1..LAT {
            let phi =
                (std::f32::consts::PI * lat as f32) / LAT as f32 - std::f32::consts::FRAC_PI_2;
            for lon in 0..LON {
                let theta = (2.0 * std::f32::consts::PI * lon as f32) / LON as f32;

                let surface = phi.cos().abs() * 0.6 + (theta * 2.0).sin().abs() * 0.4;
                let band = if bars.is_empty() {
                    0.0
                } else {
                    let idx = (surface.clamp(0.0, 1.0) * (bars.len() - 1) as f32) as usize;
                    bars[idx] as f32 / 8.0 * 0.6
                };

                let macro_warp = macro_band * 0.35 * (3.0 * theta).sin() * (2.0 * phi).cos();
                let micro_warp = micro_band * 0.22 * (8.0 * theta).cos();
                let r = BASE_RADIUS + bass_scale + band + macro_warp + micro_warp;

                let plane = r * phi.cos();
                let (x, y, z) = (plane * theta.cos(), plane * theta.sin(), r * phi.sin());

                let y1 = y * PITCH.cos() - z * PITCH.sin();
                let z1 = y * PITCH.sin() + z * PITCH.cos();
                let x2 = x * self.yaw.cos() + z1 * self.yaw.sin();
                let z2 = -x * self.yaw.sin() + z1 * self.yaw.cos();

                let depth = 1.0 / (CAMERA - z2);
                let px = (x2 * depth * scale + pix_w as f32 / 2.0) as i32;
                let py = (y1 * depth * scale + pix_h as f32 / 2.0) as i32;
                if px >= 0 && (px as usize) < pix_w && py >= 0 && (py as usize) < pix_h {
                    buf[py as usize * pix_w + px as usize] = true;
                }
            }
        }

        for (row, line) in out.iter_mut().enumerate() {
            for col in 0..cols {
                let mut pattern: u8 = 0;
                for dy in 0..4 {
                    for dx in 0..2 {
                        if buf[(row * 4 + dy) * pix_w + col * 2 + dx] {
                            pattern |= DOT_BITS[dy][dx];
                        }
                    }
                }
                if pattern == 0 {
                    line.push(' ');
                } else {
                    line.push(char::from_u32(0x2800 + pattern as u32).unwrap_or(' '));
                }
            }
        }
        out
    }
}
