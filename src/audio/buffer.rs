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
pub struct PcmStream {
    data: UnsafeCell<Box<[f32]>>,
    available: AtomicUsize,
    written: AtomicUsize,
    done: AtomicBool,
    failed: AtomicBool,
    pub sample_rate: u32,
    pub channels: usize,
}

unsafe impl Send for PcmStream {}
unsafe impl Sync for PcmStream {}

impl PcmStream {
    pub fn with_seconds(seconds: f64, sample_rate: u32, channels: usize) -> PcmStream {
        let frames = (seconds.max(1.0) * sample_rate as f64 * 1.25) as usize;
        let floor = sample_rate as usize * 5;
        let capacity = frames.max(floor) * channels;
        PcmStream {
            data: UnsafeCell::new(vec![0.0; capacity].into_boxed_slice()),
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
            slot.copy_from_slice(&samples[..n]);
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
        for (i, slot) in out.iter_mut().enumerate() {
            let idx = start + i;
            *slot = if idx < available { data[idx] * gain } else { 0.0 };
        }
    }

    /// Snapshot of everything decoded so far, for the waveform pass.
    pub fn snapshot(&self) -> Vec<f32> {
        let available = self.available.load(Ordering::Acquire);
        let data = unsafe { &*self.data.get() };
        data[..available].to_vec()
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
