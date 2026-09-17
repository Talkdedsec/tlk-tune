use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

/// An interleaved PCM buffer filled by one decoder thread and read by the
/// audio callback while it is still filling.
///
/// It is built from fixed blocks rather than one allocation, for two reasons.
/// A stream whose length nobody knows — a live one, or a container that
/// declines to say — used to be cut off wherever the guess ran out. And a
/// three minute track no longer pays for the five minutes somebody guessed it
/// might be.
///
/// The lock-free read works because nothing ever moves. The table of block
/// pointers is allocated once and never resized; a block, once published with
/// a release store, is never reallocated or freed until the whole stream is
/// dropped. So a reader that has seen `available` can read every sample below
/// it, and the block holding that sample is guaranteed to be there.
///
/// Samples are kept as `i16` rather than `f32`. A five minute stereo track at
/// 48 kHz is 57 MB this way instead of 115 MB, and the conversion in the
/// callback is one multiply per sample.
pub struct PcmStream {
    blocks: Box<[AtomicPtr<i16>]>,
    /// Samples per block. Fixed, so an index divides into block and offset
    /// with a shift rather than a division.
    shift: u32,
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
/// 2^18 samples is half a megabyte per block: small enough that a short track
/// wastes almost nothing, large enough that a long one is a few thousand
/// allocations rather than a few million.
const BLOCK_SHIFT: u32 = 18;
/// Enough blocks for about nine hours of stereo at 48 kHz. Past that the
/// stream stops growing, which is the same thing the old buffer did at the
/// end of its guess, only much later.
const MAX_BLOCKS: usize = 16384;

impl PcmStream {
    pub fn with_seconds(seconds: f64, sample_rate: u32, channels: usize) -> PcmStream {
        // The estimate no longer decides what can be played, only how much is
        // reserved before the first sample arrives.
        let frames = (seconds.max(1.0) * sample_rate as f64) as usize;
        let wanted = frames.saturating_mul(channels.max(1));
        let blocks = (wanted >> BLOCK_SHIFT).saturating_add(2).min(MAX_BLOCKS);

        let table: Vec<AtomicPtr<i16>> = (0..MAX_BLOCKS)
            .map(|_| AtomicPtr::new(std::ptr::null_mut()))
            .collect();
        let stream = PcmStream {
            blocks: table.into_boxed_slice(),
            shift: BLOCK_SHIFT,
            available: AtomicUsize::new(0),
            written: AtomicUsize::new(0),
            done: AtomicBool::new(false),
            failed: AtomicBool::new(false),
            sample_rate,
            channels: channels.max(1),
        };
        // Reserving what the duration suggests keeps the decoder off the
        // allocator during the opening seconds, when it is busiest.
        for i in 0..blocks {
            stream.ensure(i);
        }
        stream
    }

    fn block_len(&self) -> usize {
        1 << self.shift
    }

    /// Decoder thread only. Allocates the block if it is not there yet.
    fn ensure(&self, index: usize) -> Option<*mut i16> {
        let slot = self.blocks.get(index)?;
        let existing = slot.load(Ordering::Acquire);
        if !existing.is_null() {
            return Some(existing);
        }
        let block = vec![0i16; self.block_len()].into_boxed_slice();
        let raw = Box::into_raw(block) as *mut i16;
        match slot.compare_exchange(
            std::ptr::null_mut(),
            raw,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Some(raw),
            Err(won) => {
                // Somebody else got there first; give ours back rather than
                // leaking it. Only the decoder writes, so this is defensive.
                unsafe {
                    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        raw,
                        self.block_len(),
                    )));
                }
                Some(won)
            }
        }
    }

    /// Decoder thread only. Returns false once there is nowhere left to put
    /// the samples, which now means hours rather than a bad guess.
    pub fn append(&self, samples: &[f32]) -> bool {
        let mut at = self.written.load(Ordering::Relaxed);
        let mut taken = 0;

        while taken < samples.len() {
            let index = at >> self.shift;
            let Some(block) = self.ensure(index) else {
                break;
            };
            let offset = at & (self.block_len() - 1);
            let room = self.block_len() - offset;
            let n = room.min(samples.len() - taken);
            unsafe {
                let slot = std::slice::from_raw_parts_mut(block.add(offset), n);
                for (dst, src) in slot.iter_mut().zip(&samples[taken..taken + n]) {
                    *dst = (src.clamp(-1.0, 1.0) * SCALE) as i16;
                }
            }
            at += n;
            taken += n;
        }

        self.written.store(at, Ordering::Relaxed);
        self.available.store(at, Ordering::Release);
        taken == samples.len()
    }

    pub fn available_frames(&self) -> usize {
        self.available.load(Ordering::Acquire) / self.channels
    }

    pub fn available_samples(&self) -> usize {
        self.available.load(Ordering::Acquire)
    }

    /// One sample, with the published count passed in so the audio callback
    /// loads the atomic once per buffer rather than once per sample.
    #[inline]
    pub fn at(&self, index: usize, available: usize) -> f32 {
        if index >= available {
            return 0.0;
        }
        let block = match self.blocks.get(index >> self.shift) {
            Some(slot) => slot.load(Ordering::Acquire),
            None => return 0.0,
        };
        if block.is_null() {
            return 0.0;
        }
        let offset = index & (self.block_len() - 1);
        unsafe { *block.add(offset) as f32 / SCALE }
    }

    /// Feeds the waveform pass one mono sample per frame without ever
    /// materialising a second copy of the track.
    pub fn for_each_mono(&self, mut visit: impl FnMut(f32)) {
        let available = self.available.load(Ordering::Acquire);
        let channels = self.channels;
        let mut index = 0;
        while index + channels <= available {
            let mut sum = 0i32;
            for offset in 0..channels {
                sum += (self.at(index + offset, available) * SCALE) as i32;
            }
            visit(sum as f32 / (channels as f32 * SCALE));
            index += channels;
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

impl Drop for PcmStream {
    fn drop(&mut self) {
        let len = self.block_len();
        for slot in self.blocks.iter() {
            let block = slot.swap(std::ptr::null_mut(), Ordering::AcqRel);
            if !block.is_null() {
                unsafe {
                    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                        block, len,
                    )));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_i16() {
        let pcm = PcmStream::with_seconds(1.0, 8000, 2);
        pcm.append(&[1.0, -1.0, 0.5, -0.5]);
        let available = pcm.available_samples();
        assert!((pcm.at(0, available) - 1.0).abs() < 0.001);
        assert!((pcm.at(1, available) + 1.0).abs() < 0.001);
        assert!((pcm.at(2, available) - 0.5).abs() < 0.001);
        assert!((pcm.at(3, available) + 0.5).abs() < 0.001);
    }

    #[test]
    fn reads_past_the_end_as_silence() {
        let pcm = PcmStream::with_seconds(1.0, 8000, 2);
        pcm.append(&[0.5, 0.5]);
        let available = pcm.available_samples();
        assert_eq!(pcm.at(2, available), 0.0);
        assert_eq!(pcm.at(99, available), 0.0);
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

    #[test]
    fn a_stream_outgrows_the_duration_it_was_told() {
        // One second reserved, ten seconds decoded: the old buffer stopped at
        // the guess, which is what cut live streams off.
        let pcm = PcmStream::with_seconds(1.0, 8000, 1);
        let chunk = vec![0.25f32; 8000];
        for _ in 0..10 {
            assert!(pcm.append(&chunk), "append refused inside the first hour");
        }
        assert_eq!(pcm.available_frames(), 80_000);
        let available = pcm.available_samples();
        assert!((pcm.at(79_999, available) - 0.25).abs() < 0.001);
    }

    #[test]
    fn samples_survive_the_seam_between_blocks() {
        let pcm = PcmStream::with_seconds(0.1, 8000, 1);
        let block = pcm.block_len();
        let mut ramp = vec![0.0f32; block + 16];
        for (i, slot) in ramp.iter_mut().enumerate() {
            *slot = if i % 2 == 0 { 0.5 } else { -0.5 };
        }
        assert!(pcm.append(&ramp));

        let available = pcm.available_samples();
        assert_eq!(available, block + 16);
        for i in [block - 1, block, block + 1, block + 15] {
            let expected = if i % 2 == 0 { 0.5 } else { -0.5 };
            assert!(
                (pcm.at(i, available) - expected).abs() < 0.001,
                "sample {i} came back wrong across the block edge"
            );
        }
    }

    #[test]
    fn nothing_is_published_before_it_is_written() {
        let pcm = PcmStream::with_seconds(1.0, 8000, 1);
        assert_eq!(pcm.available_samples(), 0);
        assert_eq!(pcm.at(0, pcm.available_samples()), 0.0);
        pcm.append(&[0.75]);
        assert_eq!(pcm.available_samples(), 1);
    }
}
