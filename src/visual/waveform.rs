pub const RESOLUTION: usize = 4096;

pub struct Column {
    pub top: char,
    pub mid: char,
    pub bot: char,
}

pub fn column(level: i32) -> Column {
    match level {
        1 => Column {
            top: ' ',
            mid: '\u{28FF}',
            bot: ' ',
        },
        2 => Column {
            top: '\u{28C0}',
            mid: '\u{28FF}',
            bot: '\u{2809}',
        },
        3 => Column {
            top: '\u{28E4}',
            mid: '\u{28FF}',
            bot: '\u{281B}',
        },
        4 => Column {
            top: '\u{28F6}',
            mid: '\u{28FF}',
            bot: '\u{283F}',
        },
        5 => Column {
            top: '\u{28FF}',
            mid: '\u{28FF}',
            bot: '\u{28FF}',
        },
        _ => Column {
            top: ' ',
            mid: '\u{2836}',
            bot: ' ',
        },
    }
}

/// Accumulates the envelope one sample at a time.
///
/// The whole track never exists as a second copy this way: an hour of audio
/// would otherwise mean a 690 MB `Vec<f32>` just to draw a bar a few hundred
/// columns wide.
pub struct Builder {
    sums: Vec<f32>,
    counts: Vec<u32>,
    chunk: usize,
    slot: usize,
    in_slot: usize,
}

impl Builder {
    pub fn new(total_frames: usize, resolution: usize) -> Builder {
        let resolution = resolution.max(1);
        Builder {
            sums: vec![0.0; resolution],
            counts: vec![0; resolution],
            chunk: (total_frames / resolution).max(1),
            slot: 0,
            in_slot: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, sample: f32) {
        if self.slot >= self.sums.len() {
            return;
        }
        self.sums[self.slot] += sample * sample;
        self.counts[self.slot] += 1;
        self.in_slot += 1;
        if self.in_slot == self.chunk {
            self.slot += 1;
            self.in_slot = 0;
        }
    }

    pub fn finish(self, smooth: bool) -> Vec<f32> {
        let resolution = self.sums.len();
        let mut rms = vec![0.0f32; resolution];
        for (i, slot) in rms.iter_mut().enumerate() {
            if self.counts[i] > 0 {
                *slot = (self.sums[i] / self.counts[i] as f32).sqrt();
            }
        }
        shape(rms, smooth)
    }
}

fn shape(rms: Vec<f32>, smooth: bool) -> Vec<f32> {
    let resolution = rms.len();
    let source = if smooth {
        const W: [f32; 3] = [0.15, 0.70, 0.15];
        let mut blurred = vec![0.0f32; resolution];
        for (i, slot) in blurred.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            let mut weight = 0.0f32;
            for j in -1i32..=1 {
                let idx = i as i32 + j;
                if idx >= 0 && (idx as usize) < resolution {
                    sum += rms[idx as usize] * W[(j + 1) as usize];
                    weight += W[(j + 1) as usize];
                }
            }
            *slot = sum / weight;
        }
        blurred
    } else {
        rms
    };

    let peak = source.iter().copied().fold(0.0001f32, f32::max);
    source
        .iter()
        .map(|v| (v / peak).powf(2.5).clamp(0.0, 1.0))
        .collect()
}

/// Peak decimation down to one 0..=5 level per terminal column, so a short
/// transient survives instead of being averaged into silence.
pub fn fit(model: &[f32], width: usize) -> Vec<i32> {
    let mut out = vec![0; width];
    if width == 0 || model.is_empty() {
        return out;
    }
    let ratio = model.len() as f32 / width as f32;
    for (i, slot) in out.iter_mut().enumerate() {
        let start = ((i as f32 * ratio) as usize).min(model.len());
        let mut end = (((i + 1) as f32 * ratio) as usize).min(model.len());
        if end == start && start < model.len() {
            end = start + 1;
        }
        let mut peak = 0.0f32;
        for v in &model[start..end] {
            if *v > peak {
                peak = *v;
            }
        }
        if peak < 0.08 {
            peak = 0.0;
        }
        *slot = (peak * 5.0).round() as i32;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(pcm: &[f32], resolution: usize, smooth: bool) -> Vec<f32> {
        let mut builder = Builder::new(pcm.len(), resolution);
        for sample in pcm {
            builder.push(*sample);
        }
        builder.finish(smooth)
    }

    #[test]
    fn envelope_is_normalised() {
        let pcm: Vec<f32> = (0..8000)
            .map(|i| (i as f32 * 0.05).sin() * (i as f32 / 8000.0))
            .collect();
        let model = build(&pcm, 256, false);
        assert_eq!(model.len(), 256);
        assert!(model.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(model.iter().cloned().fold(0.0, f32::max) > 0.99);
    }

    #[test]
    fn the_envelope_follows_the_signal() {
        // Quiet first half, loud second half.
        let mut pcm = vec![0.05f32; 20_000];
        pcm.extend(std::iter::repeat_n(0.9f32, 20_000));
        let model = build(&pcm, 100, false);
        assert!(model[10] < 0.05, "quiet half read {}", model[10]);
        assert!(model[90] > 0.9, "loud half read {}", model[90]);
    }

    #[test]
    fn fit_keeps_the_loudest_column() {
        let mut model = vec![0.0f32; 100];
        model[42] = 1.0;
        let levels = fit(&model, 10);
        assert_eq!(levels.len(), 10);
        assert_eq!(levels[4], 5);
        assert_eq!(levels[0], 0);
    }

    #[test]
    fn gate_drops_near_silence() {
        let levels = fit(&[0.05f32; 20], 4);
        assert!(levels.iter().all(|l| *l == 0));
    }

    #[test]
    fn smoothing_takes_the_edge_off_a_single_spike() {
        let mut pcm = vec![0.2f32; 10_000];
        for sample in pcm.iter_mut().skip(5_000).take(100) {
            *sample = 1.0;
        }
        let raw = build(&pcm, 100, false);
        let smooth = build(&pcm, 100, true);
        let sharpest = raw.iter().cloned().fold(0.0f32, f32::max);
        let softened = smooth.iter().cloned().fold(0.0f32, f32::max);
        assert!(softened <= sharpest);
        assert_eq!(raw.len(), smooth.len());
    }

    #[test]
    fn extra_samples_do_not_run_off_the_end() {
        let mut builder = Builder::new(100, 10);
        for _ in 0..500 {
            builder.push(0.5);
        }
        let model = builder.finish(false);
        assert_eq!(model.len(), 10);
        assert!(model.iter().all(|v| v.is_finite()));
    }
}
