use super::buffer::PcmStream;
use super::eq::{Biquad, State};

/// EBU R 128 integrated loudness.
///
/// Tracks ripped from different sources sit at wildly different levels, so
/// without this every other song needs the volume touching. The measurement
/// is the real thing: K-weighting, 400 ms blocks at 75% overlap, an absolute
/// gate at -70 LUFS and the relative gate 10 LU below the ungated mean.
const BLOCK_MS: f64 = 400.0;
const OVERLAP: usize = 4;
const ABSOLUTE_GATE: f64 = -70.0;
const RELATIVE_GATE: f64 = -10.0;
const OFFSET: f64 = -0.691;

/// Stage 1 of K-weighting: the head shelf, given for 48 kHz in the standard
/// and re-derived here for whatever rate the track actually runs at.
fn shelf(sample_rate: f64) -> Biquad {
    let f0 = 1681.974450955533;
    let g = 3.999843853973347;
    let q = 0.7071752369554196;

    let k = (std::f64::consts::PI * f0 / sample_rate).tan();
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.4996667741545416);
    let denom = 1.0 + k / q + k * k;

    Biquad {
        b0: ((vh + vb * k / q + k * k) / denom) as f32,
        b1: ((2.0 * (k * k - vh)) / denom) as f32,
        b2: ((vh - vb * k / q + k * k) / denom) as f32,
        a1: ((2.0 * (k * k - 1.0)) / denom) as f32,
        a2: ((1.0 - k / q + k * k) / denom) as f32,
    }
}

/// Stage 2: the RLB high-pass that removes rumble from the measurement.
fn highpass(sample_rate: f64) -> Biquad {
    let f0 = 38.13547087602444;
    let q = 0.5003270373238773;
    let k = (std::f64::consts::PI * f0 / sample_rate).tan();
    let denom = 1.0 + k / q + k * k;

    Biquad {
        b0: 1.0,
        b1: -2.0,
        b2: 1.0,
        a1: ((2.0 * (k * k - 1.0)) / denom) as f32,
        a2: ((1.0 - k / q + k * k) / denom) as f32,
    }
}

fn block_loudness(mean_square: f64) -> f64 {
    if mean_square <= 0.0 {
        return f64::NEG_INFINITY;
    }
    OFFSET + 10.0 * mean_square.log10()
}

/// Measures whatever the buffer holds. Returns None for anything too short to
/// fill a single 400 ms block.
pub fn integrated(pcm: &PcmStream) -> Option<f64> {
    let channels = pcm.channels.max(1);
    let rate = pcm.sample_rate as f64;
    let available = pcm.available_samples();
    let frames = available / channels;

    let block = (rate * BLOCK_MS / 1000.0) as usize;
    let hop = block / OVERLAP;
    if block == 0 || hop == 0 || frames < block {
        return None;
    }

    let shelf = shelf(rate);
    let highpass = highpass(rate);
    let mut shelf_state = vec![State::default(); channels];
    let mut highpass_state = vec![State::default(); channels];

    // Sum of squares per hop, so a block is just four of them added up.
    let mut hop_energy: Vec<f64> = Vec::with_capacity(frames / hop + 1);
    let mut running = 0.0f64;
    let mut in_hop = 0usize;

    for frame in 0..frames {
        for c in 0..channels {
            let raw = pcm.at(frame * channels + c, available);
            let weighted = highpass_state[c].step(&highpass, shelf_state[c].step(&shelf, raw));
            running += (weighted as f64) * (weighted as f64);
        }
        in_hop += 1;
        if in_hop == hop {
            hop_energy.push(running);
            running = 0.0;
            in_hop = 0;
        }
    }

    if hop_energy.len() < OVERLAP {
        return None;
    }

    let per_block = (block * channels) as f64;
    let mut blocks: Vec<f64> = Vec::with_capacity(hop_energy.len());
    for window in hop_energy.windows(OVERLAP) {
        blocks.push(window.iter().sum::<f64>() / per_block);
    }

    let gated: Vec<f64> = blocks
        .iter()
        .copied()
        .filter(|z| block_loudness(*z) > ABSOLUTE_GATE)
        .collect();
    if gated.is_empty() {
        return None;
    }

    let mean = gated.iter().sum::<f64>() / gated.len() as f64;
    let threshold = block_loudness(mean) + RELATIVE_GATE;
    let kept: Vec<f64> = gated
        .into_iter()
        .filter(|z| block_loudness(*z) > threshold)
        .collect();
    if kept.is_empty() {
        return None;
    }

    Some(block_loudness(kept.iter().sum::<f64>() / kept.len() as f64))
}

/// How much to turn a track by to land on `target`, held inside a range that
/// cannot wreck a quiet recording or blow up a loud one.
pub fn gain_db(measured: f64, target: f64) -> f32 {
    ((target - measured).clamp(-15.0, 15.0)) as f32
}

/// Keeps a boosted track from clipping. Linear below the knee, curving into
/// the ceiling above it, so normal material passes through untouched.
#[inline]
pub fn soft_clip(sample: f32) -> f32 {
    const KNEE: f32 = 0.7;
    let magnitude = sample.abs();
    if magnitude <= KNEE {
        return sample;
    }
    let over = (magnitude - KNEE) / (1.0 - KNEE);
    let shaped = KNEE + (1.0 - KNEE) * (over / (1.0 + over * over).sqrt());
    shaped.min(0.999) * sample.signum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn sine(rate: u32, channels: usize, seconds: f64, amplitude: f32) -> Arc<PcmStream> {
        let pcm = Arc::new(PcmStream::with_seconds(seconds, rate, channels));
        let frames = (rate as f64 * seconds) as usize;
        let mut block = Vec::with_capacity(frames * channels);
        for f in 0..frames {
            let t = f as f32 / rate as f32;
            let value = (2.0 * std::f32::consts::PI * 1000.0 * t).sin() * amplitude;
            for _ in 0..channels {
                block.push(value);
            }
        }
        pcm.append(&block);
        pcm.mark_done();
        pcm
    }

    #[test]
    fn a_full_scale_tone_lands_near_minus_three() {
        let measured = integrated(&sine(48000, 2, 2.0, 1.0)).expect("no measurement");
        assert!(
            (measured + 3.0).abs() < 2.0,
            "full scale tone read {measured} LUFS"
        );
    }

    #[test]
    fn ten_db_quieter_reads_ten_lu_quieter() {
        let loud = integrated(&sine(48000, 2, 2.0, 1.0)).unwrap();
        let quiet = integrated(&sine(48000, 2, 2.0, 0.3162)).unwrap();
        assert!(
            ((loud - quiet) - 10.0).abs() < 0.5,
            "difference was {} LU",
            loud - quiet
        );
    }

    #[test]
    fn silence_and_short_clips_measure_nothing() {
        assert!(integrated(&sine(48000, 2, 0.1, 1.0)).is_none());
        assert!(integrated(&sine(48000, 2, 2.0, 0.0)).is_none());
    }

    #[test]
    fn gain_is_capped() {
        assert_eq!(gain_db(-18.0, -18.0), 0.0);
        assert!((gain_db(-28.0, -18.0) - 10.0).abs() < 0.001);
        assert_eq!(gain_db(-90.0, -18.0), 15.0);
        assert_eq!(gain_db(0.0, -18.0), -15.0);
    }

    #[test]
    fn the_limiter_leaves_normal_levels_alone() {
        for v in [-0.7, -0.3, 0.0, 0.25, 0.7] {
            assert!((soft_clip(v) - v).abs() < 1e-6, "{v} was altered");
        }
        assert!(soft_clip(4.0) < 1.0);
        assert!(soft_clip(-4.0) > -1.0);
        assert!(soft_clip(0.95) > 0.7 && soft_clip(0.95) < 1.0);
    }
}
