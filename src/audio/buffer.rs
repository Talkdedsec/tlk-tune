use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// A fixed-capacity interleaved PCM buffer filled by one decoder thread and
/// read by the audio callback while it is still filling.
///
/// Capacity is reserved up front from the container's duration and never
/// grows. That is what makes the lock-free read safe: the backing allocation
/// can never move, so once `available` is published with a release store,
/// every sample below it is known to be fully written. A duration estimate
/// that is badly wrong costs the tail of a track rather than risking a
/// dangling read mid-playback.
///
/// Samples are kept as `i16` rather than `f32`. A five minute stereo track at
/// 48 kHz is 57 MB this way instead of 115 MB, and the conversion in the
/// callback is one multiply per sample.
pub struct PcmStream {
    data: UnsafeCell<Box<[i16]>>,
    available: AtomicUsize,
    written: AtomicUsize,
    done: AtomicBool,
    failed: AtomicBool,
    pub sample_rate: u32,
    pub channels: usize,
}

unsafe impl Send for PcmStream {}
unsafe impl Sync for PcmStream {}

const SCALE: f32 = 32767.0;

impl PcmStream {
    pub fn with_seconds(seconds: f64, sample_rate: u32, channels: usize) -> PcmStream {
        let frames = (seconds.max(1.0) * sample_rate as f64 * 1.25) as usize;
        let floor = sample_rate as usize * 5;
        let capacity = frames.max(floor) * channels;
        PcmStream {
            data: UnsafeCell::new(vec![0; capacity].into_boxed_slice()),
            available: AtomicUsize::new(0),
            written: AtomicUsize::new(0),
            done: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            sample_rate,
            channels,
        }
    }

    pub fn capacity(&self) -> usize {
        unsafe { (&*self.data.get()).len() }
    }

    /// Decoder thread only. Returns false once the reserved capacity is full.
    pub fn append(&self, samples: &[f32]) -> bool {
        let at = self.written.load(Ordering::Relaxed);
        let capacity = self.capacity();
        if at >= capacity {
            return false;
        }
        let n = samples.len().min(capacity - at);
        unsafe {
            let slot = &mut (&mut *self.data.get())[at..at + n];
            for (dst, src) in slot.iter_mut().zip(&samples[..n]) {
                *dst = (src.clamp(-1.0, 1.0) * SCALE) as i16;
            }
        }
        self.written.store(at + n, Ordering::Relaxed);
        self.available.store(at + n, Ordering::Release);
        n == samples.len()
    }

    pub fn available_frames(&self) -> usize {
        self.available.load(Ordering::Acquire) / self.channels
    }

    /// Copies `out.len()` samples starting at `frame`, zero-filling anything
    /// the decoder has not produced yet.
    pub fn read_into(&self, frame: usize, out: &mut [f32], gain: f32) {
        let available = self.available.load(Ordering::Acquire);
        let start = frame * self.channels;
        let data = unsafe { &*self.data.get() };
        let scale = gain / SCALE;
        for (i, slot) in out.iter_mut().enumerate() {
            let idx = start + i;
            *slot = if idx < available {
                data[idx] as f32 * scale
            } else {
                0.0
            };
        }
    }

    /// Feeds the waveform pass one mono sample per frame without ever
    /// materialising a second copy of the track.
    pub fn for_each_mono(&self, mut visit: impl FnMut(f32)) {
        let available = self.available.load(Ordering::Acquire);
        let data = unsafe { &*self.data.get() };
        let channels = self.channels.max(1);
        for frame in data[..available].chunks(channels) {
            let sum: i32 = frame.iter().map(|s| *s as i32).sum();
            visit(sum as f32 / (channels as f32 * SCALE));
        }
    }

    pub fn mark_done(&self) {
        self.done.store(true, Ordering::Release);
    }

    pub fn mark_failed(&self) {
        self.failed.store(true, Ordering::Release);
        self.done.store(true, Ordering::Release);
    }

    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    pub fn has_failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_i16() {
        let pcm = PcmStream::with_seconds(1.0, 8000, 2);
        pcm.append(&[1.0, -1.0, 0.5, -0.5]);
        let mut out = [0.0f32; 4];
        pcm.read_into(0, &mut out, 1.0);
        assert!((out[0] - 1.0).abs() < 0.001);
        assert!((out[1] + 1.0).abs() < 0.001);
        assert!((out[2] - 0.5).abs() < 0.001);
        assert!((out[3] + 0.5).abs() < 0.001);
    }

    #[test]
    fn reads_past_the_end_as_silence() {
        let pcm = PcmStream::with_seconds(1.0, 8000, 2);
        pcm.append(&[0.5, 0.5]);
        let mut out = [9.0f32; 6];
        pcm.read_into(0, &mut out, 1.0);
        assert!(out[2..].iter().all(|s| *s == 0.0));
    }

    #[test]
    fn refuses_to_grow_past_capacity() {
        let pcm = PcmStream::with_seconds(1.0, 8000, 1);
        let capacity = pcm.capacity();
        assert!(!pcm.append(&vec![0.1f32; capacity + 10]));
        assert_eq!(pcm.available_frames(), capacity);
    }

    #[test]
    fn downmixes_for_the_waveform() {
        let pcm = PcmStream::with_seconds(1.0, 8000, 2);
        pcm.append(&[1.0, -1.0, 0.5, 0.5]);
        let mut seen = Vec::new();
        pcm.for_each_mono(|v| seen.push(v));
        assert_eq!(seen.len(), 2);
        assert!(seen[0].abs() < 0.001);
        assert!((seen[1] - 0.5).abs() < 0.001);
    }
}
