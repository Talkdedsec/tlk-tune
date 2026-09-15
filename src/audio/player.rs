use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, StreamConfig};

use super::buffer::PcmStream;
use crate::visual::spectrum::SpectrumSink;

pub struct Output {
    pub sample_rate: u32,
    pub channels: usize,
    pub name: String,
}

struct Shared {
    cursor: AtomicI64,
    gain: AtomicU32,
    paused: AtomicBool,
    finished: AtomicBool,
    lost: AtomicBool,
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
    sink: Option<SpectrumSink>,
    output: Output,
    preferred: Option<String>,
    volume: i32,
}

impl Player {
    pub fn new(preferred: Option<String>) -> Player {
        let output = probe_output(preferred.as_deref()).unwrap_or(Output {
            sample_rate: 48000,
            channels: 2,
            name: String::new(),
        });
        Player {
            stream: None,
            shared: Arc::new(Shared {
                cursor: AtomicI64::new(0),
                gain: AtomicU32::new(0.7f32.to_bits()),
                paused: AtomicBool::new(false),
                finished: AtomicBool::new(false),
                lost: AtomicBool::new(false),
            }),
            pcm: None,
            sink: None,
            output,
            preferred,
            volume: 70,
        }
    }

    pub fn output(&self) -> &Output {
        &self.output
    }

    /// Names of every output the host can see, current one first.
    pub fn devices(&self) -> Vec<String> {
        let mut names: Vec<String> = cpal::default_host()
            .output_devices()
            .map(|list| list.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default();
        names.dedup();
        names
    }

    /// Switches output. Playback restarts from where it was, because a cpal
    /// stream is bound to one device for its lifetime.
    pub fn use_device(&mut self, name: Option<String>) {
        self.preferred = name;
        let at = self.elapsed();
        let pcm = self.pcm.clone();
        let sink = self.sink.clone();
        self.stop();
        if let Some(output) = probe_output(self.preferred.as_deref()) {
            self.output = output;
        }
        if let Some(pcm) = pcm {
            self.play(pcm, at, sink);
        }
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
        self.shared.lost.store(false, Ordering::Relaxed);

        let Some(device) = pick_device(self.preferred.as_deref()) else {
            return false;
        };
        let config = StreamConfig {
            channels: channels as u16,
            sample_rate: cpal::SampleRate(rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let shared = Arc::clone(&self.shared);
        let errors = Arc::clone(&self.shared);
        let source = Arc::clone(&pcm);
        let feed = sink.clone();
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
                if let Some(s) = &feed {
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
            move |err| {
                if matches!(err, cpal::StreamError::DeviceNotAvailable) {
                    errors.lost.store(true, Ordering::Relaxed);
                }
            },
            None,
        );

        let Ok(stream) = stream else { return false };
        if stream.play().is_err() {
            return false;
        }
        self.stream = Some(stream);
        self.pcm = Some(pcm);
        self.sink = sink;
        true
    }

    /// True once the device disappeared under us — an unplugged headset, a
    /// driver reset. The caller reopens rather than going silently quiet.
    pub fn device_lost(&self) -> bool {
        self.shared.lost.load(Ordering::Relaxed)
    }

    pub fn reopen(&mut self) {
        self.shared.lost.store(false, Ordering::Relaxed);
        let at = self.elapsed();
        let pcm = self.pcm.clone();
        let sink = self.sink.clone();
        self.stream = None;
        if let Some(output) = probe_output(self.preferred.as_deref()) {
            self.output = output;
        }
        if let Some(pcm) = pcm {
            self.play(pcm, at, sink);
        }
    }

    pub fn stop(&mut self) {
        self.stream = None;
        self.pcm = None;
        self.sink = None;
        self.shared.cursor.store(0, Ordering::Relaxed);
        self.shared.finished.store(false, Ordering::Relaxed);
    }

    pub fn toggle_pause(&self) {
        let paused = self.shared.paused.load(Ordering::Relaxed);
        self.shared.paused.store(!paused, Ordering::Relaxed);
    }

    pub fn set_paused(&self, paused: bool) {
        self.shared.paused.store(paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.shared.paused.load(Ordering::Relaxed)
    }

    pub fn seek(&self, delta_sec: f64) {
        self.seek_to(self.elapsed() + delta_sec);
    }

    /// Clamped to what the decoder has actually produced. Jumping past that
    /// would play silence until decoding caught up, which reads as a bug.
    pub fn seek_to(&self, seconds: f64) {
        let Some(pcm) = &self.pcm else { return };
        let limit = pcm.available_frames().saturating_sub(1);
        let frame = (seconds.max(0.0) * pcm.sample_rate as f64) as usize;
        self.shared
            .cursor
            .store(frame.min(limit) as i64, Ordering::Relaxed);
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

fn pick_device(preferred: Option<&str>) -> Option<Device> {
    let host = cpal::default_host();
    if let Some(want) = preferred.filter(|w| !w.is_empty()) {
        if let Ok(mut list) = host.output_devices() {
            if let Some(found) = list.find(|d| d.name().map(|n| n == want).unwrap_or(false)) {
                return Some(found);
            }
        }
    }
    host.default_output_device()
}

fn probe_output(preferred: Option<&str>) -> Option<Output> {
    let device = pick_device(preferred)?;
    let config = device
        .supported_output_configs()
        .ok()?
        .filter(|c| c.sample_format() == SampleFormat::F32)
        .max_by_key(|c| c.channels())?;
    let channels = config.channels().clamp(1, 2) as usize;
    let rate = device
        .default_output_config()
        .map(|c| c.sample_rate().0)
        .unwrap_or(48000)
        .clamp(config.min_sample_rate().0, config.max_sample_rate().0);
    Some(Output {
        sample_rate: rate,
        channels,
        name: device.name().unwrap_or_default(),
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
        let mut player = Player::new(None);
        let (rate, channels) = {
            let out = player.output();
            (out.sample_rate, out.channels)
        };
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
        assert!(elapsed > 0.1, "cursor stalled at {elapsed}");

        player.seek_to(900.0);
        let clamped = player.elapsed();
        player.stop();
        assert!(clamped <= 1.01, "seek ran past the buffer to {clamped}");
    }
}
