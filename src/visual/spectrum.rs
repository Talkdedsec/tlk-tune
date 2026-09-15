use std::sync::{Arc, Mutex};

use realfft::{RealFftPlanner, RealToComplex};

const FFT_SIZE: usize = 2048;
const BANDS: usize = 32;
const HALF: usize = BANDS / 2;

const EDGES: [f32; HALF + 1] = [
    30.0, 50.0, 80.0, 120.0, 180.0, 250.0, 350.0, 500.0, 700.0, 1000.0, 1400.0, 2000.0, 2800.0,
    4000.0, 5600.0, 8000.0, 12000.0,
];

struct Ring {
    data: Box<[f32; FFT_SIZE]>,
    write: usize,
    sample_rate: u32,
}

/// Handle held by the audio callback. Writing is a short lock and nothing
/// else, so the callback never waits on the FFT itself.
#[derive(Clone)]
pub struct SpectrumSink(Arc<Mutex<Ring>>);

impl SpectrumSink {
    pub fn push(&self, samples: &[f32], sample_rate: u32) {
        let Ok(mut ring) = self.0.lock() else { return };
        if sample_rate > 0 {
            ring.sample_rate = sample_rate;
        }
        for s in samples {
            let w = ring.write;
            ring.data[w] = *s;
            ring.write = (w + 1) % FFT_SIZE;
        }
    }
}

pub struct Spectrum {
    ring: Arc<Mutex<Ring>>,
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    scratch_in: Vec<f32>,
    scratch_out: Vec<realfft::num_complex::Complex<f32>>,
    bands: [f32; BANDS],
    levels: Vec<f32>,
    velocity: Vec<f32>,
    spring: f32,
    damping: f32,
    release: f32,
    viscosity: f32,
}

impl Spectrum {
    pub fn new() -> Spectrum {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let scratch_out = fft.make_output_vec();
        let window = (0..FFT_SIZE)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos())
            })
            .collect();
        Spectrum {
            ring: Arc::new(Mutex::new(Ring {
                data: Box::new([0.0; FFT_SIZE]),
                write: 0,
                sample_rate: 44100,
            })),
            fft,
            window,
            scratch_in: vec![0.0; FFT_SIZE],
            scratch_out,
            bands: [0.0; BANDS],
            levels: Vec::new(),
            velocity: Vec::new(),
            spring: 4.0,
            damping: 0.06,
            release: 140.0,
            viscosity: 0.07,
        }
    }

    pub fn sink(&self) -> SpectrumSink {
        SpectrumSink(Arc::clone(&self.ring))
    }

    pub fn reset(&mut self) {
        if let Ok(mut ring) = self.ring.lock() {
            ring.data.fill(0.0);
            ring.write = 0;
        }
        self.bands = [0.0; BANDS];
        self.levels.clear();
        self.velocity.clear();
    }

    /// 1 = snappy, 10 = floaty. Rise only; the fall is governed by `set_decay`.
    pub fn set_fluidity(&mut self, level: i32) {
        let t = (level.clamp(1, 10) - 1) as f32 / 9.0;
        self.spring = 55.0 - t * 51.0;
        self.damping = 0.35 - t * 0.29;
    }

    /// 1 = slow VU-style fade, 10 = near instant cut-off.
    pub fn set_decay(&mut self, level: i32) {
        let t = (level.clamp(1, 10) - 1) as f32 / 9.0;
        self.release = 15.0 + t * 125.0;
    }

    /// Neighbour blending, so bars drag each other along like a liquid surface.
    pub fn set_viscosity(&mut self, level: i32) {
        self.viscosity = level.clamp(0, 10) as f32 / 10.0 * 0.7;
    }

    fn refresh_bands(&mut self) {
        let sample_rate = {
            let Ok(ring) = self.ring.lock() else { return };
            for (i, slot) in self.scratch_in.iter_mut().enumerate() {
                *slot = ring.data[(ring.write + i) % FFT_SIZE] * self.window[i];
            }
            ring.sample_rate
        };

        if self
            .fft
            .process(&mut self.scratch_in, &mut self.scratch_out)
            .is_err()
        {
            return;
        }

        let usable = FFT_SIZE / 2;
        let norm = (FFT_SIZE * FFT_SIZE) as f32;
        let bin_hz = sample_rate as f32 / FFT_SIZE as f32;

        let mut upper = [0.0f32; HALF];
        for (b, slot) in upper.iter_mut().enumerate() {
            let lo = ((EDGES[b] / bin_hz) as usize).clamp(1, usable - 1);
            let hi = ((EDGES[b + 1] / bin_hz) as usize).clamp(lo + 1, usable);
            let mut energy = 0.0f32;
            for k in lo..hi {
                let c = self.scratch_out[k];
                energy += c.re * c.re + c.im * c.im;
            }
            energy /= (hi - lo) as f32;
            let db = 10.0 * (energy / norm + 1e-12).log10();
            *slot = (db + 70.0).clamp(0.0, 70.0);
        }

        // Mirror so bass lands in the middle and treble falls off both edges.
        let mut full = [0.0f32; BANDS];
        for i in 0..HALF {
            full[i] = upper[HALF - 1 - i];
            full[HALF + i] = upper[i];
        }

        for (b, target) in full.iter().enumerate() {
            let a = if *target > self.bands[b] { 0.55 } else { 0.30 };
            self.bands[b] = self.bands[b] * (1.0 - a) + target * a;
        }
    }

    /// Returns one 0..=8 level per bar.
    pub fn bars(&mut self, count: usize, dt: f64) -> Vec<i32> {
        if count == 0 {
            return Vec::new();
        }
        self.refresh_bands();

        let mut targets = vec![0.0f32; count];
        for (i, target) in targets.iter_mut().enumerate() {
            let t = if count > 1 {
                i as f32 / (count - 1) as f32
            } else {
                0.0
            };
            let pos = t * (BANDS - 1) as f32;
            let i0 = (pos.floor() as usize).min(BANDS - 1);
            let i1 = (i0 + 1).min(BANDS - 1);
            let frac = pos - i0 as f32;
            let value = self.bands[i0] * (1.0 - frac) + self.bands[i1] * frac;
            let raw = (value / 70.0).clamp(0.0, 1.0);
            // Half-sine envelope keeps the strip reading as a single hill
            // instead of letting the outermost bars spike on their own.
            let envelope = if count > 1 {
                (std::f32::consts::PI * i as f32 / (count - 1) as f32).sin()
            } else {
                1.0
            };
            *target = raw.powf(0.35) * envelope;
        }

        if self.levels.len() != count {
            self.levels = vec![0.0; count];
            self.velocity = vec![0.0; count];
        }
        let dt = dt.clamp(0.0, 0.5) as f32;

        for (i, target) in targets.iter().enumerate() {
            let target = *target;
            let falling = target < self.levels[i];
            // A bar that was still rising carries upward momentum; cancelling
            // most of it on the flip is what stops drops feeling sluggish.
            if falling && self.velocity[i] > 0.0 {
                self.velocity[i] *= 0.15;
            }
            let k = if falling { self.release } else { self.spring };
            self.velocity[i] += (target - self.levels[i]) * k * dt;
            self.velocity[i] *= 1.0 - self.damping.clamp(0.0, 0.95);
            self.levels[i] = (self.levels[i] + self.velocity[i] * dt).clamp(0.0, 1.0);
        }

        if self.viscosity > 0.0 && count > 2 {
            let mut blended = self.levels.clone();
            for (i, slot) in blended.iter_mut().enumerate() {
                let left = if i > 0 {
                    self.levels[i - 1]
                } else {
                    self.levels[i]
                };
                let right = if i + 1 < count {
                    self.levels[i + 1]
                } else {
                    self.levels[i]
                };
                *slot =
                    self.levels[i] * (1.0 - self.viscosity) + (left + right) * 0.5 * self.viscosity;
            }
            self.levels = blended;
        }

        self.levels[0] = 0.0;
        self.levels[count - 1] = 0.0;

        self.levels
            .iter()
            .map(|v| (v.clamp(0.0, 1.0) * 8.0).round() as i32)
            .collect()
    }
}

/// Glyph set used by the spectrum strip. Deliberately not the waveform's.
pub fn bar_glyph(level: i32) -> char {
    match level.clamp(0, 4) {
        1 => '\u{28C0}',
        2 => '\u{28E4}',
        3 => '\u{28F6}',
        4 => '\u{28FF}',
        _ => ' ',
    }
}
