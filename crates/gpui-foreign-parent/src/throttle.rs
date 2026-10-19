//! Host-pump frame-request throttling (contract §6).
//!
//! Display-link ticks can arrive faster than the guest can usefully
//! forward them; ticks inside the minimum interval are coalesced and
//! counted instead of queueing redundant frame work.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Reference minimum interval between forwarded frame requests.
///
/// Carried over from the `AUv3` baseline: 4 ms absorbs `CVDisplayLink`
/// overdrive on high-refresh displays without delaying real frames.
/// Lowering it queues redundant GPUI frame work per tick; raising it
/// toward the display period visibly throttles animation smoothness.
pub const DEFAULT_FRAME_MIN_INTERVAL: Duration = Duration::from_millis(4);

/// Coalesces host-pump overdrive; main-thread use only.
///
/// `Cell`/`AtomicUsize` interior mutability mirrors the baseline window:
/// the throttle lives in the guest window and is touched through shared
/// references on the host UI thread.
#[derive(Debug)]
pub struct FrameThrottle {
    min_interval: Duration,
    last_forward: Cell<Option<Instant>>,
    coalesced: AtomicUsize,
}

impl FrameThrottle {
    /// Create a throttle with an explicit minimum interval.
    #[must_use]
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last_forward: Cell::new(None),
            coalesced: AtomicUsize::new(0),
        }
    }

    /// True when this tick should be forwarded to the guest frame
    /// callback. A tick inside the minimum interval is counted as
    /// coalesced and reports false.
    pub fn should_forward(&self) -> bool {
        self.should_forward_at(Instant::now())
    }

    fn should_forward_at(&self, now: Instant) -> bool {
        if self
            .last_forward
            .get()
            .is_some_and(|last| now.duration_since(last) < self.min_interval)
        {
            self.coalesced.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        self.last_forward.set(Some(now));
        true
    }

    /// Ticks coalesced so far. Realtime health signal for the host.
    #[must_use]
    pub fn coalesced(&self) -> usize {
        self.coalesced.load(Ordering::Relaxed)
    }
}

impl Default for FrameThrottle {
    fn default() -> Self {
        Self::new(DEFAULT_FRAME_MIN_INTERVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overdrive_is_coalesced_and_counted() {
        let throttle = FrameThrottle::default();
        let start = Instant::now();
        assert!(throttle.should_forward_at(start));
        assert!(!throttle.should_forward_at(start));
        assert!(!throttle.should_forward_at(start + DEFAULT_FRAME_MIN_INTERVAL / 2));
        assert_eq!(throttle.coalesced(), 2);
        // Rejected ticks do not postpone the exact next eligible boundary.
        assert!(throttle.should_forward_at(start + DEFAULT_FRAME_MIN_INTERVAL));
        assert_eq!(throttle.coalesced(), 2);
    }

    #[test]
    fn zero_interval_forwards_every_tick() {
        let throttle = FrameThrottle::new(Duration::ZERO);
        assert!(throttle.should_forward());
        assert!(throttle.should_forward());
        assert_eq!(throttle.coalesced(), 0);
    }
}
