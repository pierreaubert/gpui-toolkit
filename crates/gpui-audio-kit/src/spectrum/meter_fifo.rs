//! Realtime-safe meter bridge between the audio thread and meter UI.
//!
//! The audio (DSP) thread must never allocate, block on the UI thread, or
//! touch GPUI state. [`MeterFifo`] is a triple-buffered level/meter frame
//! store: the producer publishes fixed-size `&[f32]` frames with a bounded
//! copy, and the UI thread snapshots the latest published frame into a
//! caller-owned buffer it reuses across frames.
//!
//! Triple buffering (instead of a mutex around one buffer) means the
//! producer never waits on a UI-thread snapshot: it always owns a private
//! slot to write into, and publishing is a single atomic index exchange.
//! Per-slot locks are only ever taken on the thread-owned slot, so they are
//! uncontended by construction (lock poisoning is still tolerated by
//! recovering the inner buffer).

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

const MAX_SLOTS: usize = 3;

/// Packed triple-buffer state: write index (bits 0-1), read index (bits 2-3),
/// shared/published index (bits 4-5), dirty flag (bit 6).
///
/// The three indices are always a permutation of `{0, 1, 2}`: the writer
/// exclusively owns `write`, the reader exclusively owns `read`, and they
/// only exchange ownership of `shared` through atomic transitions.
struct FifoState(AtomicU8);

impl FifoState {
    fn new() -> Self {
        // write = 0, read = 1, shared = 2, dirty = 0.
        Self(AtomicU8::new((1 << 2) | (2 << 4)))
    }

    #[inline]
    fn unpack(state: u8) -> (usize, usize, usize, bool) {
        (
            (state & 0b11) as usize,
            ((state >> 2) & 0b11) as usize,
            ((state >> 4) & 0b11) as usize,
            state & 0b0100_0000 != 0,
        )
    }

    #[inline]
    fn pack(write: usize, read: usize, shared: usize, dirty: bool) -> u8 {
        (write as u8) | ((read as u8) << 2) | ((shared as u8) << 4) | u8::from(dirty) << 6
    }

    /// Publish the just-written `write` slot, rotating it into `shared` and
    /// taking the previously shared slot as the next private write slot.
    fn publish(&self) {
        let _ = self
            .0
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                let (write, read, shared, _) = Self::unpack(state);
                Some(Self::pack(shared, read, write, true))
            });
    }

    /// Adopt the latest published slot as the private read slot when a new
    /// frame arrived since the last snapshot. Returns the read slot index.
    fn acquire(&self) -> (usize, bool) {
        let mut fresh = false;
        let _ = self
            .0
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                let (write, read, shared, dirty) = Self::unpack(state);
                if dirty {
                    fresh = true;
                    Some(Self::pack(write, shared, read, false))
                } else {
                    None
                }
            });
        let (_, read, _, _) = Self::unpack(self.0.load(Ordering::Acquire));
        (read, fresh)
    }

    #[cfg(test)]
    fn indices(&self) -> (usize, usize, usize, bool) {
        Self::unpack(self.0.load(Ordering::Relaxed))
    }
}

fn unpoisoned<'a, T>(
    lock: Result<
        std::sync::MutexGuard<'a, T>,
        std::sync::PoisonError<std::sync::MutexGuard<'a, T>>,
    >,
) -> std::sync::MutexGuard<'a, T> {
    lock.unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct MeterFifoInner {
    slots: [Mutex<Vec<f32>>; MAX_SLOTS],
    state: FifoState,
    channels: usize,
}

/// Triple-buffered audio level frame store shared between the DSP thread
/// and meter/spectrum UI.
///
/// Clone shares the same backing buffers (like `Arc`); hand one handle to
/// the audio callback and keep one on the UI side.
#[derive(Clone)]
pub struct MeterFifo {
    inner: Arc<MeterFifoInner>,
}

impl MeterFifo {
    /// Create a FIFO for `channels` meter channels. All slots are
    /// pre-sized so the audio thread never allocates after construction.
    pub fn new(channels: usize) -> Self {
        Self {
            inner: Arc::new(MeterFifoInner {
                slots: std::array::from_fn(|_| Mutex::new(vec![0.0; channels])),
                state: FifoState::new(),
                channels,
            }),
        }
    }

    /// Channel count fixed at construction.
    pub fn channels(&self) -> usize {
        self.inner.channels
    }

    /// Publish one frame from the audio thread.
    ///
    /// Copies `min(frame.len(), channels)` samples into the private write
    /// slot and publishes it. Never allocates and never blocks on the UI
    /// thread: the slot lock is uncontended by construction.
    pub fn push_frame(&self, frame: &[f32]) {
        let state = self.inner.state.0.load(Ordering::Acquire);
        let (write, _, _, _) = FifoState::unpack(state);
        {
            let mut slot = unpoisoned(self.inner.slots[write].lock());
            for (dst, src) in slot.iter_mut().zip(frame.iter()) {
                *dst = *src;
            }
        }
        self.inner.state.publish();
    }

    /// Copy the latest published frame into `out`, growing it only when its
    /// capacity is insufficient (reuse the same `out` across frames to stay
    /// allocation-free).
    ///
    /// Returns `true` when a frame published since the previous snapshot was
    /// adopted; `false` means `out` still holds the previous contents.
    pub fn snapshot(&self, out: &mut Vec<f32>) -> bool {
        let (read, fresh) = self.inner.state.acquire();
        if out.len() != self.inner.channels {
            out.resize(self.inner.channels, 0.0);
        }
        if fresh {
            let slot = unpoisoned(self.inner.slots[read].lock());
            out.copy_from_slice(&slot);
        }
        fresh
    }

    /// Snapshot into `scratch` and wrap the result as a shared magnitude
    /// buffer ready for [`super::SpectrumElement::new`].
    ///
    /// The `Arc` wrap allocates once per call; keep one `scratch` buffer in
    /// view state and call this only when [`Self::snapshot`] reports a fresh
    /// frame to avoid per-frame allocation.
    pub fn snapshot_shared(&self, scratch: &mut Vec<f32>) -> Arc<[f32]> {
        self.snapshot(scratch);
        Arc::from(scratch.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::MeterFifo;

    #[test]
    fn new_slots_start_silent_with_distinct_indices() {
        let fifo = MeterFifo::new(2);
        assert_eq!(fifo.channels(), 2);
        let (write, read, shared, dirty) = fifo.inner.state.indices();
        assert!(!dirty);
        let mut sorted = [write, read, shared];
        sorted.sort_unstable();
        assert_eq!(sorted, [0, 1, 2]);
    }

    #[test]
    fn snapshot_reports_freshness_and_reuses_buffer() {
        let fifo = MeterFifo::new(2);
        let mut scratch = Vec::new();

        assert!(!fifo.snapshot(&mut scratch));
        assert_eq!(scratch, vec![0.0, 0.0]);

        fifo.push_frame(&[0.5, -12.0]);
        assert!(fifo.snapshot(&mut scratch));
        assert_eq!(scratch, vec![0.5, -12.0]);

        // No new frame: not fresh, contents preserved.
        assert!(!fifo.snapshot(&mut scratch));
        assert_eq!(scratch, vec![0.5, -12.0]);

        // Latest frame wins when several land between snapshots.
        fifo.push_frame(&[0.1, 0.1]);
        fifo.push_frame(&[0.9, 0.8]);
        assert!(fifo.snapshot(&mut scratch));
        assert_eq!(scratch, vec![0.9, 0.8]);
    }

    #[test]
    fn frames_truncate_to_channel_count() {
        let fifo = MeterFifo::new(1);
        fifo.push_frame(&[0.7, 0.6, 0.5]);
        let mut scratch = Vec::new();
        assert!(fifo.snapshot(&mut scratch));
        assert_eq!(scratch, vec![0.7]);
    }

    #[test]
    fn snapshot_shared_yields_arc_magnitudes() {
        let fifo = MeterFifo::new(3);
        fifo.push_frame(&[-30.0, -12.0, -6.0]);
        let mut scratch = Vec::new();
        let shared = fifo.snapshot_shared(&mut scratch);
        assert_eq!(&*shared, &[-30.0_f32, -12.0, -6.0]);
    }

    #[test]
    fn producer_and_consumer_agree_across_threads() {
        let fifo = MeterFifo::new(4);
        let producer = fifo.clone();
        let handle = std::thread::spawn(move || {
            for i in 0..200 {
                let level = i as f32 / 200.0;
                producer.push_frame(&[level, level, level, level]);
            }
        });
        let mut scratch = Vec::new();
        let mut last = 0.0_f32;
        while last < 199.0 / 200.0 {
            if fifo.snapshot(&mut scratch) {
                assert_eq!(scratch.len(), 4);
                assert!(scratch[0] >= last);
                assert!(scratch.iter().all(|v| v.is_finite()));
                last = scratch[0];
            }
        }
        handle.join().expect("producer thread");
    }
}
