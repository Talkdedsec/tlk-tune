use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;

use arc_swap::{ArcSwap, ArcSwapOption};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, StreamConfig};

use super::buffer::PcmStream;
use super::eq;
use super::loudness;
use crate::visual::spectrum::SpectrumSink;

pub struct Output {
    pub sample_rate: u32,
    pub channels: usize,
    pub name: String,
}

/// One playing buffer and where in it we are. Decks are swapped in and out
/// while the device stays open, which is what makes track changes gapless.
struct Deck {
    pcm: Arc<PcmStream>,
    cursor: AtomicI64,
}

impl Deck {
    fn new(pcm: Arc<PcmStream>, start_frame: i64) -> Deck {
        Deck {
            pcm,
            cursor: AtomicI64::new(start_frame),
        }
    }
}

struct Mix {
    current: ArcSwapOption<Deck>,
    outgoing: ArcSwapOption<Deck>,
    fade_total: AtomicUsize,
    fade_left: AtomicUsize,
    curve: ArcSwap<eq::Curve>,
    gain: AtomicU32,
    track_gain: AtomicU32,
    paused: AtomicBool,
    finished: AtomicBool,
    lost: AtomicBool,
    sink: ArcSwapOption<SpectrumSink>,
}

impl Mix {
    fn gain(&self) -> f32 {
        f32::from_bits(self.gain.load(Ordering::Relaxed))
            * f32::from_bits(self.track_gain.load(Ordering::Relaxed))
    }
}

pub struct Player {
    stream: Option<cpal::Stream>,
    mix: Arc<Mix>,
    output: Output,
    preferred: Option<String>,
    volume: i32,
    crossfade_ms: u32,
    gains: [f32; 10],
}

impl Player {
    pub fn new(preferred: Option<String>) -> Player {
        let output = probe_output(preferred.as_deref()).unwrap_or(Output {
            sample_rate: 48000,
            channels: 2,
            name: String::new(),
        });
        let mix = Arc::new(Mix {
            current: ArcSwapOption::empty(),
            outgoing: ArcSwapOption::empty(),
            fade_total: AtomicUsize::new(0),
            fade_left: AtomicUsize::new(0),
            curve: ArcSwap::from_pointee(eq::build(&[0.0; 10], output.sample_rate)),
            gain: AtomicU32::new(0.7f32.to_bits()),
            track_gain: AtomicU32::new(1.0f32.to_bits()),
            paused: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            lost: AtomicBool::new(false),
            sink: ArcSwapOption::empty(),
        });
        let mut player = Player {
            stream: None,
            mix,
            output,
            preferred,
            volume: 70,
            crossfade_ms: 0,
            gains: [0.0; 10],
        };
        player.open_stream();
        player
    }

    pub fn output(&self) -> &Output {
        &self.output
    }

    /// Names of every output the host can see.
    pub fn devices(&self) -> Vec<String> {
        let mut names: Vec<String> = cpal::default_host()
            .output_devices()
            .map(|list| list.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default();
        names.dedup();
        names
    }

    fn open_stream(&mut self) -> bool {
        self.stream = None;
        let Some(device) = pick_device(self.preferred.as_deref()) else {
            return false;
        };
        let channels = self.output.channels;
        let rate = self.output.sample_rate;
        let config = StreamConfig {
            channels: channels as u16,
            sample_rate: cpal::SampleRate(rate),
            buffer_size: cpal::BufferSize::Default,
        };

        let mix = Arc::clone(&self.mix);
        let errors = Arc::clone(&self.mix);
        let mut states = vec![[eq::State::default(); 10]; channels];

        let stream = device.build_output_stream(
            &config,
            move |out: &mut [f32], _: &cpal::OutputCallbackInfo| {
                render(&mix, out, channels, rate, &mut states);
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
        true
    }

    /// Switches output. The whole mix moves across; playback keeps its place.
    pub fn use_device(&mut self, name: Option<String>) {
        self.preferred = name;
        if let Some(output) = probe_output(self.preferred.as_deref()) {
            self.output = output;
        }
        self.mix
            .curve
            .store(Arc::new(eq::build(&self.gains, self.output.sample_rate)));
        self.open_stream();
    }

    pub fn play(
        &mut self,
        pcm: Arc<PcmStream>,
        start_sec: f64,
        sink: Option<SpectrumSink>,
    ) -> bool {
        let rate = pcm.sample_rate;
        let deck = Arc::new(Deck::new(pcm, (start_sec * rate as f64) as i64));

        match sink {
            Some(s) => self.mix.sink.store(Some(Arc::new(s))),
            None => self.mix.sink.store(None),
        }
        self.mix.finished.store(false, Ordering::Relaxed);
        self.mix.paused.store(false, Ordering::Relaxed);

        let fade = (self.crossfade_ms as usize * rate as usize) / 1000;
        let previous = self.mix.current.swap(Some(deck));
        if fade > 0 && previous.is_some() {
            self.mix.outgoing.store(previous);
            self.mix.fade_total.store(fade, Ordering::Relaxed);
            self.mix.fade_left.store(fade, Ordering::Relaxed);
        } else {
            self.mix.outgoing.store(None);
            self.mix.fade_left.store(0, Ordering::Relaxed);
        }

        if self.stream.is_none() {
            return self.open_stream();
        }
        true
    }

    /// True once the device disappeared under us — an unplugged headset, a
    /// driver reset. The caller reopens rather than going silently quiet.
    pub fn device_lost(&self) -> bool {
        self.mix.lost.load(Ordering::Relaxed)
    }

    pub fn reopen(&mut self) {
        self.mix.lost.store(false, Ordering::Relaxed);
        if let Some(output) = probe_output(self.preferred.as_deref()) {
            self.output = output;
        }
        self.open_stream();
    }

    pub fn stop(&mut self) {
        self.mix.current.store(None);
        self.mix.outgoing.store(None);
        self.mix.fade_left.store(0, Ordering::Relaxed);
        self.mix.finished.store(false, Ordering::Relaxed);
    }

    /// Releases the device. The stream otherwise stays open for the whole run
    /// so that swapping decks never costs a gap.
    pub fn close(&mut self) {
        self.stop();
        self.stream = None;
    }

    pub fn toggle_pause(&self) {
        let paused = self.mix.paused.load(Ordering::Relaxed);
        self.mix.paused.store(!paused, Ordering::Relaxed);
    }

    pub fn set_paused(&self, paused: bool) {
        self.mix.paused.store(paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.mix.paused.load(Ordering::Relaxed)
    }

    pub fn seek(&self, delta_sec: f64) {
        self.seek_to(self.elapsed() + delta_sec);
    }

    /// Clamped to what the decoder has actually produced. Jumping past that
    /// would play silence until decoding caught up, which reads as a bug.
    pub fn seek_to(&self, seconds: f64) {
        let Some(deck) = self.mix.current.load_full() else {
            return;
        };
        let limit = deck.pcm.available_frames().saturating_sub(1);
        let frame = (seconds.max(0.0) * deck.pcm.sample_rate as f64) as usize;
        deck.cursor
            .store(frame.min(limit) as i64, Ordering::Relaxed);
        self.mix.finished.store(false, Ordering::Relaxed);
    }

    pub fn elapsed(&self) -> f64 {
        let Some(deck) = self.mix.current.load_full() else {
            return 0.0;
        };
        deck.cursor.load(Ordering::Relaxed).max(0) as f64 / deck.pcm.sample_rate as f64
    }

    pub fn finished(&self) -> bool {
        self.mix.finished.load(Ordering::Relaxed)
    }

    pub fn clear_finished(&self) {
        self.mix.finished.store(false, Ordering::Relaxed);
    }

    pub fn volume(&self) -> i32 {
        self.volume
    }

    pub fn set_volume(&mut self, percent: i32) {
        self.volume = percent.clamp(0, 100);
        let gain = self.volume as f32 / 100.0;
        self.mix.gain.store(gain.to_bits(), Ordering::Relaxed);
    }

    /// Per-track replay gain, in dB. Reset to zero whenever a track starts
    /// so a measured one never leaks its correction onto the next.
    pub fn set_track_gain_db(&self, db: f32) {
        let linear = 10f32.powf(db.clamp(-15.0, 15.0) / 20.0);
        self.mix
            .track_gain
            .store(linear.to_bits(), Ordering::Relaxed);
    }

    pub fn set_crossfade_ms(&mut self, ms: u32) {
        self.crossfade_ms = ms.min(12_000);
    }

    pub fn set_eq(&mut self, gains: [f32; 10]) {
        self.gains = gains;
        self.mix
            .curve
            .store(Arc::new(eq::build(&self.gains, self.output.sample_rate)));
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.close();
    }
}

/// The whole audio path: two decks, a crossfade between them, the equaliser,
/// then the master gain. Allocation-free by construction.
fn render(mix: &Mix, out: &mut [f32], channels: usize, rate: u32, states: &mut [[eq::State; 10]]) {
    out.fill(0.0);
    if mix.paused.load(Ordering::Relaxed) {
        return;
    }
    let frames = out.len() / channels.max(1);
    let master = mix.gain();

    let outgoing = mix.outgoing.load_full();
    let fade_total = mix.fade_total.load(Ordering::Relaxed);
    let mut fade_left = mix.fade_left.load(Ordering::Relaxed);

    if let (Some(deck), true) = (&outgoing, fade_left > 0 && fade_total > 0) {
        let taken = frames.min(fade_left);
        let cursor = deck.cursor.load(Ordering::Relaxed).max(0) as usize;
        let available = deck.pcm.available_samples();
        for f in 0..taken {
            let level = (fade_left - f) as f32 / fade_total as f32;
            let base = (cursor + f) * channels;
            for c in 0..channels {
                out[f * channels + c] += deck.pcm.at(base + c, available) * level;
            }
        }
        deck.cursor
            .store((cursor + taken) as i64, Ordering::Relaxed);
        fade_left -= taken;
        mix.fade_left.store(fade_left, Ordering::Relaxed);
        if fade_left == 0 {
            mix.outgoing.store(None);
        }
    }

    if let Some(deck) = mix.current.load_full() {
        let cursor = deck.cursor.load(Ordering::Relaxed);
        if cursor < 0 {
            deck.cursor.store(0, Ordering::Relaxed);
        } else {
            let cursor = cursor as usize;
            let fading = fade_total > 0 && fade_left > 0;
            let available = deck.pcm.available_samples();
            for f in 0..frames {
                let ramp = if fading {
                    1.0 - fade_left.saturating_sub(f) as f32 / fade_total as f32
                } else {
                    1.0
                };
                let base = (cursor + f) * channels;
                for c in 0..channels {
                    out[f * channels + c] += deck.pcm.at(base + c, available) * ramp;
                }
            }
            let next = cursor + frames;
            if deck.pcm.is_done() && next >= deck.pcm.available_frames() {
                mix.finished.store(true, Ordering::Relaxed);
            }
            deck.cursor.store(next as i64, Ordering::Relaxed);
        }
    }

    let curve = mix.curve.load();
    if !curve.bypass {
        for f in 0..frames {
            for (c, state) in states.iter_mut().enumerate().take(channels) {
                let slot = f * channels + c;
                let mut sample = out[slot];
                for (filter, band) in curve.filters.iter().zip(state.iter_mut()) {
                    sample = band.step(filter, sample);
                }
                out[slot] = sample;
            }
        }
    }

    for sample in out.iter_mut() {
        *sample = loudness::soft_clip(*sample * master);
    }

    if let Some(sink) = mix.sink.load_full() {
        sink.push(out, rate);
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
        player.close();
        assert!(clamped <= 1.01, "seek ran past the buffer to {clamped}");
    }

    fn deck(rate: u32, channels: usize, value: f32, seconds: f64) -> Arc<PcmStream> {
        let pcm = Arc::new(PcmStream::with_seconds(seconds, rate, channels));
        let samples = vec![value; (rate as f64 * seconds) as usize * channels];
        pcm.append(&samples);
        pcm.mark_done();
        pcm
    }

    #[test]
    fn the_mixer_crossfades_between_two_decks() {
        let rate = 48000u32;
        let channels = 2usize;
        let mix = Arc::new(Mix {
            current: ArcSwapOption::empty(),
            outgoing: ArcSwapOption::empty(),
            fade_total: AtomicUsize::new(0),
            fade_left: AtomicUsize::new(0),
            curve: ArcSwap::from_pointee(eq::build(&[0.0; 10], rate)),
            gain: AtomicU32::new(1.0f32.to_bits()),
            track_gain: AtomicU32::new(1.0f32.to_bits()),
            paused: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            lost: AtomicBool::new(false),
            sink: ArcSwapOption::empty(),
        });

        let old = Arc::new(Deck::new(deck(rate, channels, 0.8, 1.0), 0));
        let new = Arc::new(Deck::new(deck(rate, channels, -0.8, 1.0), 0));
        let fade = 4800usize;
        mix.outgoing.store(Some(old));
        mix.current.store(Some(new));
        mix.fade_total.store(fade, Ordering::Relaxed);
        mix.fade_left.store(fade, Ordering::Relaxed);

        let mut states = vec![[eq::State::default(); 10]; channels];
        let mut out = vec![0.0f32; 480 * channels];

        render(&mix, &mut out, channels, rate, &mut states);
        // Right at the start the outgoing deck still dominates.
        assert!(out[0] > 0.5, "fade started at {}", out[0]);

        // 4800 frames of fade at 480 frames a buffer: nine more finishes it.
        for _ in 0..9 {
            render(&mix, &mut out, channels, rate, &mut states);
        }
        assert_eq!(mix.fade_left.load(Ordering::Relaxed), 0);
        assert!(mix.outgoing.load_full().is_none());

        render(&mix, &mut out, channels, rate, &mut states);
        // Once the fade is done only the new deck is heard.
        assert!(out[0] < -0.7, "fade ended at {}", out[0]);
    }

    #[test]
    fn a_paused_mix_is_silent() {
        let rate = 48000u32;
        let mix = Arc::new(Mix {
            current: ArcSwapOption::from(Some(Arc::new(Deck::new(deck(rate, 2, 0.9, 0.5), 0)))),
            outgoing: ArcSwapOption::empty(),
            fade_total: AtomicUsize::new(0),
            fade_left: AtomicUsize::new(0),
            curve: ArcSwap::from_pointee(eq::build(&[0.0; 10], rate)),
            gain: AtomicU32::new(1.0f32.to_bits()),
            track_gain: AtomicU32::new(1.0f32.to_bits()),
            paused: AtomicBool::new(true),
            finished: AtomicBool::new(false),
            lost: AtomicBool::new(false),
            sink: ArcSwapOption::empty(),
        });
        let mut states = vec![[eq::State::default(); 10]; 2];
        let mut out = vec![9.0f32; 256];
        render(&mix, &mut out, 2, rate, &mut states);
        assert!(out.iter().all(|s| *s == 0.0));
    }
}
