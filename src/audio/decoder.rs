use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, StandardTagKey};
use symphonia::core::probe::Hint;

use super::buffer::PcmStream;

#[derive(Clone, Default)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub duration: f64,
    pub sample_rate: u32,
    pub channels: usize,
    pub bits: String,
}

fn hint_for(path: &Path) -> Hint {
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    hint
}

/// Header-only read: tags, duration and stream format, no audio decoded.
pub fn probe(path: &Path) -> Option<TrackInfo> {
    let file = File::open(path).ok()?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint_for(path),
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;

    let track = probed.format.default_track()?;
    let params = &track.codec_params;

    let mut info = TrackInfo {
        sample_rate: params.sample_rate.unwrap_or(44100),
        channels: params.channels.map(|c| c.count()).unwrap_or(2),
        bits: params
            .bits_per_sample
            .map(|b| format!("{} bit", b))
            .unwrap_or_default(),
        ..Default::default()
    };

    if let (Some(frames), Some(tb)) = (params.n_frames, params.time_base) {
        let t = tb.calc_time(frames);
        info.duration = t.seconds as f64 + t.frac;
    } else if let Some(frames) = params.n_frames {
        info.duration = frames as f64 / info.sample_rate as f64;
    }

    let collect = |revision: &symphonia::core::meta::MetadataRevision, info: &mut TrackInfo| {
        for tag in revision.tags() {
            let value = tag.value.to_string();
            match tag.std_key {
                Some(StandardTagKey::TrackTitle) if info.title.is_empty() => info.title = value,
                Some(StandardTagKey::Artist) if info.artist.is_empty() => info.artist = value,
                Some(StandardTagKey::AlbumArtist) if info.artist.is_empty() => info.artist = value,
                Some(StandardTagKey::Album) if info.album.is_empty() => info.album = value,
                Some(StandardTagKey::Date) | Some(StandardTagKey::OriginalDate)
                    if info.year.is_empty() =>
                {
                    info.year = value.chars().take(4).collect()
                }
                _ => {}
            }
        }
    };

    if let Some(meta) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
        collect(meta, &mut info);
    }
    if let Some(meta) = probed.format.metadata().current() {
        collect(meta, &mut info);
    }

    Some(info)
}

/// Decodes the whole file into `sink`, converting to the sink's channel count
/// and sample rate as it goes. Runs on its own thread; playback reads the
/// buffer while it is still filling.
pub fn decode_into(path: &Path, sink: Arc<PcmStream>) {
    if let Err(()) = run(path, &sink) {
        sink.mark_failed();
        return;
    }
    sink.mark_done();
}

fn run(path: &Path, sink: &PcmStream) -> Result<(), ()> {
    let file = File::open(path).map_err(|_| ())?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint_for(path),
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|_| ())?;

    let track = probed.format.default_track().ok_or(())?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|_| ())?;

    let out_channels = sink.channels;
    let out_rate = sink.sample_rate;
    let mut resampler: Option<Resampler> = None;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut staging: Vec<f32> = Vec::new();

    loop {
        let packet = match probed.format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(_)) => break,
            Err(Error::ResetRequired) => break,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(Error::DecodeError(_)) => continue,
            Err(_) => break,
        };

        let spec = *decoded.spec();
        let frames = decoded.capacity() as u64;
        let buf = sample_buf.get_or_insert_with(|| SampleBuffer::<f32>::new(frames, spec));
        if buf.capacity() < decoded.frames() * spec.channels.count() {
            *buf = SampleBuffer::<f32>::new(decoded.frames() as u64, spec);
        }
        buf.copy_interleaved_ref(decoded);

        let src_channels = spec.channels.count();
        remix(buf.samples(), src_channels, out_channels, &mut staging);

        if spec.rate != out_rate {
            let r = resampler.get_or_insert_with(|| {
                Resampler::new(spec.rate, out_rate, out_channels)
            });
            let resampled = r.process(&staging);
            if !sink.append(resampled) {
                break;
            }
        } else if !sink.append(&staging) {
            break;
        }
    }
    Ok(())
}

fn remix(input: &[f32], src: usize, dst: usize, out: &mut Vec<f32>) {
    out.clear();
    if src == 0 {
        return;
    }
    let frames = input.len() / src;
    out.reserve(frames * dst);
    for f in 0..frames {
        let block = &input[f * src..f * src + src];
        match (src, dst) {
            (a, b) if a == b => out.extend_from_slice(block),
            (1, _) => {
                for _ in 0..dst {
                    out.push(block[0]);
                }
            }
            (_, 1) => out.push(block.iter().sum::<f32>() / src as f32),
            _ => {
                for c in 0..dst {
                    out.push(block[c.min(src - 1)]);
                }
            }
        }
    }
}

/// Catmull-Rom interpolating resampler. Keeps three frames of history so
/// packet boundaries do not click.
struct Resampler {
    ratio: f64,
    position: f64,
    channels: usize,
    history: Vec<f32>,
    out: Vec<f32>,
}

impl Resampler {
    fn new(from: u32, to: u32, channels: usize) -> Resampler {
        Resampler {
            ratio: from as f64 / to as f64,
            position: 0.0,
            channels,
            history: vec![0.0; channels * 3],
            out: Vec::new(),
        }
    }

    fn process(&mut self, input: &[f32]) -> &[f32] {
        self.out.clear();
        let ch = self.channels;
        if ch == 0 || input.is_empty() {
            return &self.out;
        }

        let mut frames: Vec<f32> = Vec::with_capacity(self.history.len() + input.len());
        frames.extend_from_slice(&self.history);
        frames.extend_from_slice(input);
        let total = frames.len() / ch;
        if total < 4 {
            self.history = frames;
            return &self.out;
        }

        let mut pos = self.position;
        while pos + 3.0 < total as f64 {
            let i = pos.floor() as usize;
            let t = (pos - i as f64) as f32;
            for c in 0..ch {
                let p0 = frames[i * ch + c];
                let p1 = frames[(i + 1) * ch + c];
                let p2 = frames[(i + 2) * ch + c];
                let p3 = frames[(i + 3) * ch + c];
                self.out.push(catmull_rom(p0, p1, p2, p3, t));
            }
            pos += self.ratio;
        }

        let consumed = pos.floor() as usize;
        self.position = pos - consumed as f64;
        self.history = frames[consumed * ch..].to_vec();
        &self.out
    }
}

fn catmull_rom(p0: f32, p1: f32, p2: f32, p3: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_sine_wav(path: &Path, rate: u32, channels: u16, seconds: f64) {
        let frames = (rate as f64 * seconds) as u32;
        let data_len = frames * channels as u32 * 2;
        let mut out: Vec<u8> = Vec::with_capacity(44 + data_len as usize);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&rate.to_le_bytes());
        out.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
        out.extend_from_slice(&(channels * 2).to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for i in 0..frames {
            let t = i as f64 / rate as f64;
            let sample = ((t * 440.0 * std::f64::consts::TAU).sin() * 16000.0) as i16;
            for _ in 0..channels {
                out.extend_from_slice(&sample.to_le_bytes());
            }
        }
        std::fs::write(path, out).unwrap();
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join("tlk-tune-tests");
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn probe_reads_duration_and_format() {
        let path = scratch("probe.wav");
        write_sine_wav(&path, 44100, 2, 1.5);
        let info = probe(&path).expect("probe failed");
        assert_eq!(info.sample_rate, 44100);
        assert_eq!(info.channels, 2);
        assert!((info.duration - 1.5).abs() < 0.05, "duration {}", info.duration);
    }

    #[test]
    fn decodes_at_the_sink_rate_and_channel_count() {
        let path = scratch("decode.wav");
        write_sine_wav(&path, 44100, 1, 1.0);
        let sink = Arc::new(PcmStream::with_seconds(1.0, 48000, 2));
        decode_into(&path, Arc::clone(&sink));
        assert!(sink.is_done() && !sink.has_failed());
        let frames = sink.available_frames();
        assert!(
            (frames as i64 - 48000).abs() < 600,
            "resampled to {} frames",
            frames
        );
        let peak = sink.snapshot().iter().fold(0.0f32, |a, b| a.max(b.abs()));
        assert!(peak > 0.4, "peak {}", peak);
    }

    #[test]
    fn mono_source_fills_both_channels() {
        let path = scratch("mono.wav");
        write_sine_wav(&path, 48000, 1, 0.3);
        let sink = Arc::new(PcmStream::with_seconds(0.3, 48000, 2));
        decode_into(&path, Arc::clone(&sink));
        let pcm = sink.snapshot();
        assert!(pcm.len() > 1000);
        for pair in pcm.chunks(2).take(500) {
            assert_eq!(pair[0], pair[1]);
        }
    }
}
