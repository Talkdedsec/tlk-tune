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

/// Builds a width-independent RMS envelope so the bar can be re-fitted to any
/// terminal width later without re-reading the PCM.
pub fn envelope(pcm: &[f32], resolution: usize, smooth: bool) -> Vec<f32> {
    let mut out = vec![0.0f32; resolution];
    if pcm.is_empty() || resolution == 0 {
        return out;
    }

    let chunk = (pcm.len() / resolution).max(1);
    let mut rms = vec![0.0f32; resolution];
    for (i, slot) in rms.iter_mut().enumerate() {
        let start = i * chunk;
        if start >= pcm.len() {
            break;
        }
        let end = (start + chunk).min(pcm.len());
        let mut sum = 0.0f32;
        for s in &pcm[start..end] {
            sum += s * s;
        }
        *slot = (sum / (end - start) as f32).sqrt();
    }

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
    for (i, v) in source.iter().enumerate() {
        out[i] = (v / peak).powf(2.5).clamp(0.0, 1.0);
    }
    out
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

    #[test]
    fn envelope_is_normalised() {
        let pcm: Vec<f32> = (0..8000)
            .map(|i| (i as f32 * 0.05).sin() * (i as f32 / 8000.0))
            .collect();
        let model = envelope(&pcm, 256, false);
        assert_eq!(model.len(), 256);
        assert!(model.iter().all(|v| (0.0..=1.0).contains(v)));
        assert!(model.iter().cloned().fold(0.0, f32::max) > 0.99);
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
}
