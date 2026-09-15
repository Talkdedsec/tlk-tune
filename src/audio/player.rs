use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};

use super::buffer::PcmStream;
use crate::visual::spectrum::SpectrumSink;

pub struct Output {
    pub sample_rate: u32,
    pub channels: usize,
}

struct Shared {
    cursor: AtomicI64,
    gain: AtomicU32,
    paused: AtomicBool,
    finished: AtomicBool,
}

impl Shared {
    fn gain(&self) -> f32 {
        f32::from_bits(self.gain.load(Ordering::Relaxed))
    }
}

pub struct Player {
    stream: Option<cpal::Stream>,
    shared: Arc<Shared>,
    pcm: Option<Arc<PcmStream>>,
    output: Output,
    volume: i32,
    has_track: bool,
}

impl Player {
    pub fn new() -> Player {
        let output = probe_output().unwrap_or(Output {
            sample_rate: 48000,
            channels: 2,
        });
        Player {
            stream: None,
            shared: Arc::new(Shared {
                cursor: AtomicI64::new(0),
                gain: AtomicU32::new(0.7f32.to_bits()),
                paused: AtomicBool::new(false),
                finished: AtomicBool::new(false),
            }),
            pcm: None,
            output,
            volume: 70,
            has_track: false,
        }
    }

    pub fn output(&self) -> &Output {
        &self.output
    }

    pub fn play(&mut self, pcm: Arc<PcmStream>, start_sec: f64, sink: Option<SpectrumSink>) -> bool {
        self.stop();
        let channels = pcm.channels;
        let rate = pcm.sample_rate;
        self.shared
            .cursor
            .store((start_sec * rate as f64) as i64, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
        self.shared.paused.store(false, Ordering::Relaxed);

        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else {
            return false;
        };
        let config = StreamConfig {
            channels: channels as u16,
            sample_rate: cpal::SampleRate(rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let shared = Arc::clone(&self.shared);
        let source = Arc::clone(&pcm);
        let stream = device.build_output_stream(
            &config,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let frames = out.len() / channels;
                if shared.paused.load(Ordering::Relaxed) {
                    out.fill(0.0);
                    return;
                }
                let cursor = shared.cursor.load(Ordering::Relaxed);
                if cursor < 0 {
                    out.fill(0.0);
                    shared.cursor.store(0, Ordering::Relaxed);
                    return;
                }
                source.read_into(cursor as usize, out, shared.gain());
                if let Some(s) = &sink {
                    s.push(out, rate);
                }
                let next = cursor + frames as i64;
                // Only finished once decoding stopped AND playback caught up
                // with everything it ever produced.
                if source.is_done() && next as usize >= source.available_frames() {
                    shared.finished.store(true, Ordering::Relaxed);
                }
                shared.cursor.store(next, Ordering::Relaxed);
            },
            |_| {},
            None,
        );

        let Ok(stream) = stream else { return false };
        if stream.play().is_err() {
            return false;
        }
        self.stream = Some(stream);
        self.pcm = Some(pcm);
        self.has_track = true;
        true
    }

    pub fn stop(&mut self) {
        self.stream = None;
        self.pcm = None;
        self.has_track = false;
        self.shared.cursor.store(0, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
    }

    pub fn toggle_pause(&self) {
        let paused = self.shared.paused.load(Ordering::Relaxed);
        self.shared.paused.store(!paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.shared.paused.load(Ordering::Relaxed)
    }

    pub fn seek(&self, delta_sec: f64) {
        let Some(pcm) = &self.pcm else { return };
        let delta = (delta_sec * pcm.sample_rate as f64) as i64;
        let cursor = (self.shared.cursor.load(Ordering::Relaxed) + delta).max(0);
        self.shared.cursor.store(cursor, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
    }

    pub fn elapsed(&self) -> f64 {
        let Some(pcm) = &self.pcm else { return 0.0 };
        self.shared.cursor.load(Ordering::Relaxed).max(0) as f64 / pcm.sample_rate as f64
    }

    pub fn finished(&self) -> bool {
        self.shared.finished.load(Ordering::Relaxed)
    }

    pub fn clear_finished(&self) {
        self.shared.finished.store(false, Ordering::Relaxed);
    }

    pub fn volume(&self) -> i32 {
        self.volume
    }

    pub fn set_volume(&mut self, percent: i32) {
        self.volume = percent.clamp(0, 100);
        let gain = self.volume as f32 / 100.0;
        self.shared.gain.store(gain.to_bits(), Ordering::Relaxed);
    }
}

fn probe_output() -> Option<Output> {
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device
        .supported_output_configs()
        .ok()?
        .filter(|c| c.sample_format() == SampleFormat::F32)
        .max_by_key(|c| c.channels())?;
    let channels = config.channels().clamp(1, 2) as usize;
    let rate = device
        .default_output_config()
        .map(|c| c.sample_rate().0)
        .unwrap_or(48000);
    let rate = rate.clamp(config.min_sample_rate().0, config.max_sample_rate().0);
    Some(Output {
        sample_rate: rate,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Opens the real default device. Skipped when the machine has none.
    #[test]
    fn plays_and_advances_the_cursor() {
        if cpal::default_host().default_output_device().is_none() {
            return;
        }
        let mut player = Player::new();
        let out = player.output();
        let (rate, channels) = (out.sample_rate, out.channels);
        let pcm = Arc::new(PcmStream::with_seconds(2.0, rate, channels));
        let tone: Vec<f32> = (0..rate as usize * channels)
            .map(|i| ((i / channels) as f32 * 0.01).sin() * 0.2)
            .collect();
        pcm.append(&tone);
        pcm.mark_done();

        player.set_volume(10);
        assert!(player.play(Arc::clone(&pcm), 0.0, None), "device refused");
        std::thread::sleep(Duration::from_millis(400));
        let elapsed = player.elapsed();
        player.stop();
        assert!(elapsed > 0.1, "cursor stalled at {elapsed}");
    }
}
