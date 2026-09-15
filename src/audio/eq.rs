/// Ten octave-spaced peaking filters, the layout every hardware equaliser has
/// used since the seventies.
pub const BANDS: [f32; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];
pub const MAX_GAIN: f32 = 12.0;
const Q: f32 = 1.0;

pub const PRESETS: [(&str, [f32; 10]); 7] = [
    ("flat", [0.0; 10]),
    (
        "bass",
        [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    ),
    (
        "treble",
        [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.5, 4.0, 5.0, 5.0],
    ),
    (
        "vocal",
        [-2.0, -2.0, -1.0, 1.0, 3.0, 4.0, 3.0, 1.0, 0.0, -1.0],
    ),
    (
        "rock",
        [4.0, 3.0, 1.0, -1.0, -2.0, 0.0, 2.0, 3.0, 4.0, 4.0],
    ),
    (
        "loudness",
        [5.0, 4.0, 2.0, 0.0, -1.0, -1.0, 0.0, 2.0, 4.0, 5.0],
    ),
    (
        "night",
        [-4.0, -3.0, -1.0, 1.0, 2.0, 2.0, 1.0, 0.0, -2.0, -3.0],
    ),
];

#[derive(Clone, Copy, Default)]
pub struct Biquad {
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl Biquad {
    /// Robert Bristow-Johnson's peaking EQ, normalised by a0.
    fn peaking(freq: f32, gain_db: f32, sample_rate: f32) -> Biquad {
        let a = 10f32.powf(gain_db / 40.0);
        let w0 = 2.0 * std::f32::consts::PI * freq / sample_rate;
        let alpha = w0.sin() / (2.0 * Q);
        let cos = w0.cos();

        let a0 = 1.0 + alpha / a;
        Biquad {
            b0: (1.0 + alpha * a) / a0,
            b1: (-2.0 * cos) / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: (-2.0 * cos) / a0,
            a2: (1.0 - alpha / a) / a0,
        }
    }
}

/// One filter's memory, per channel.
#[derive(Clone, Copy, Default)]
pub struct State {
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl State {
    #[inline]
    pub fn step(&mut self, filter: &Biquad, input: f32) -> f32 {
        let out = filter.b0 * input + filter.b1 * self.x1 + filter.b2 * self.x2
            - filter.a1 * self.y1
            - filter.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = out;
        out
    }

}

/// A whole curve, ready for the audio callback. `bypass` is set when every
/// band sits at zero so the common case costs nothing.
pub struct Curve {
    pub filters: [Biquad; 10],
    pub bypass: bool,
}

pub fn build(gains: &[f32; 10], sample_rate: u32) -> Curve {
    let bypass = gains.iter().all(|g| g.abs() < 0.05);
    let mut filters = [Biquad::default(); 10];
    for (i, slot) in filters.iter_mut().enumerate() {
        *slot = Biquad::peaking(
            BANDS[i],
            gains[i].clamp(-MAX_GAIN, MAX_GAIN),
            sample_rate as f32,
        );
    }
    Curve { filters, bypass }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_at(gains: &[f32; 10], freq: f32, rate: u32) -> f32 {
        let curve = build(gains, rate);
        let mut states = [State::default(); 10];
        let mut peak = 0.0f32;
        let total = rate as usize / 4;
        for n in 0..total {
            let t = n as f32 / rate as f32;
            let mut sample = (2.0 * std::f32::consts::PI * freq * t).sin();
            for (filter, state) in curve.filters.iter().zip(states.iter_mut()) {
                sample = state.step(filter, sample);
            }
            // Ignore the settling transient.
            if n > total / 2 {
                peak = peak.max(sample.abs());
            }
        }
        20.0 * peak.log10()
    }

    #[test]
    fn a_flat_curve_bypasses() {
        assert!(build(&[0.0; 10], 48000).bypass);
        assert!(!build(&[0.0, 0.0, 3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], 48000).bypass);
    }

    #[test]
    fn a_boosted_band_actually_boosts() {
        let mut gains = [0.0f32; 10];
        gains[5] = 9.0; // 1 kHz
        let db = response_at(&gains, 1000.0, 48000);
        assert!(db > 7.0 && db < 11.0, "1 kHz came out at {db} dB");
    }

    #[test]
    fn a_cut_band_cuts_and_leaves_neighbours_alone() {
        let mut gains = [0.0f32; 10];
        gains[5] = -9.0;
        let cut = response_at(&gains, 1000.0, 48000);
        assert!(cut < -7.0, "1 kHz came out at {cut} dB");

        let far = response_at(&gains, 62.0, 48000);
        assert!(far.abs() < 1.5, "62 Hz moved by {far} dB");
    }

    #[test]
    fn presets_stay_inside_the_range() {
        for (name, gains) in PRESETS {
            for g in gains {
                assert!(g.abs() <= MAX_GAIN, "{name} exceeds the range");
            }
        }
        assert_eq!(PRESETS[0].0, "flat");
    }
}
