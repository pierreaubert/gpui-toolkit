//! Multi-threaded variants of the host/guest policies.
//!
//! The `Cell`-based policies assume main-thread-only use. Hosts that drive
//! editors from several threads (Android JNI: UI thread, `Choreographer`,
//! binder threads) use these `Mutex`-based equivalents, which keep identical
//! semantics: coalescing, debounce convergence, and take–invoke–restore
//! callback storage.

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::{DEFAULT_FRAME_MIN_INTERVAL, DEFAULT_RESIZE_DEBOUNCE_INTERVAL};

/// Recover policy state from a poisoned lock.
///
/// Guarded policy state is plain data (timestamps, sizes, flags), so a
/// panic on another thread never invalidates it; failing open keeps
/// frames flowing instead of wedging the host pump.
fn unpoison<T>(
    guard: std::sync::LockResult<std::sync::MutexGuard<'_, T>>,
) -> std::sync::MutexGuard<'_, T> {
    guard.unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Coalesces host-pump overdrive; safe to share across threads.
#[derive(Debug)]
pub struct SyncFrameThrottle {
    min_interval: Duration,
    last_forward: Mutex<Option<Instant>>,
    coalesced: AtomicUsize,
}

impl SyncFrameThrottle {
    /// Create a throttle with an explicit minimum interval.
    #[must_use]
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last_forward: Mutex::new(None),
            coalesced: AtomicUsize::new(0),
        }
    }

    /// True when this tick should be forwarded to the guest frame
    /// callback. A tick inside the minimum interval is counted as
    /// coalesced and reports false.
    pub fn should_forward(&self) -> bool {
        let mut last_forward = unpoison(self.last_forward.lock());
        // Sample after acquiring the lock so concurrent claims use lock order.
        self.claim(&mut last_forward, Instant::now())
    }

    fn claim(&self, last_forward: &mut Option<Instant>, now: Instant) -> bool {
        if last_forward.is_some_and(|last| now.duration_since(last) < self.min_interval) {
            self.coalesced.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        *last_forward = Some(now);
        true
    }

    /// Ticks coalesced so far. Realtime health signal for the host.
    #[must_use]
    pub fn coalesced(&self) -> usize {
        self.coalesced.load(Ordering::Relaxed)
    }
}

impl Default for SyncFrameThrottle {
    fn default() -> Self {
        Self::new(DEFAULT_FRAME_MIN_INTERVAL)
    }
}

/// Debounce state shared across threads.
#[derive(Debug, Clone, Copy)]
struct SyncDebounceState<S: Copy> {
    last_apply: Option<Instant>,
    pending: Option<S>,
}

/// Debounced surface reconfiguration; safe to share across threads.
///
/// The `apply` closures run outside the lock and return false when the
/// reconfigure could not land; the size is then stashed for a later
/// attempt instead of being lost.
#[derive(Debug)]
pub struct SyncResizeDebounce<S: Copy + Send> {
    interval: Duration,
    state: Mutex<SyncDebounceState<S>>,
}

impl<S: Copy + Send> SyncResizeDebounce<S> {
    /// Create a debounce policy with an explicit interval.
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            state: Mutex::new(SyncDebounceState {
                last_apply: None,
                pending: None,
            }),
        }
    }

    /// Claim the apply slot for `size`, stashing it instead when inside
    /// the window. True means the caller should apply now.
    fn try_claim(&self, now: Instant, size: S) -> bool {
        let mut state = unpoison(self.state.lock());
        if state
            .last_apply
            .is_some_and(|last| now.duration_since(last) < self.interval)
        {
            state.pending = Some(size);
            false
        } else {
            state.last_apply = Some(now);
            true
        }
    }

    /// Submit the newest host size. Applies it immediately unless inside
    /// the debounce window or the apply fails, in which case it is
    /// stashed as pending.
    pub fn submit(&self, size: S, apply: impl FnOnce(S) -> bool) {
        if !self.try_claim(Instant::now(), size) {
            return;
        }
        if !apply(size) {
            unpoison(self.state.lock()).pending = Some(size);
        }
    }

    /// Apply a stashed size ahead of drawing, when the debounce window
    /// has elapsed. A no-op without a pending size.
    pub fn poll(&self, apply: impl FnOnce(S) -> bool) {
        let now = Instant::now();
        let mut state = unpoison(self.state.lock());
        let Some(size) = state.pending.take() else {
            return;
        };
        if state
            .last_apply
            .is_some_and(|last| now.duration_since(last) < self.interval)
        {
            state.pending = Some(size);
            return;
        }
        state.last_apply = Some(now);
        drop(state);
        if !apply(size) {
            unpoison(self.state.lock()).pending = Some(size);
        }
    }

    /// Newest size still waiting for the window to elapse, if any.
    #[must_use]
    pub fn pending(&self) -> Option<S> {
        unpoison(self.state.lock()).pending
    }
}

impl<S: Copy + Send> Default for SyncResizeDebounce<S> {
    fn default() -> Self {
        Self::new(DEFAULT_RESIZE_DEBOUNCE_INTERVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn overdrive_is_coalesced_and_counted() {
        let throttle = SyncFrameThrottle::default();
        let start = Instant::now();
        let mut last_forward = unpoison(throttle.last_forward.lock());
        assert!(throttle.claim(&mut last_forward, start));
        assert!(!throttle.claim(&mut last_forward, start));
        assert!(!throttle.claim(&mut last_forward, start + DEFAULT_FRAME_MIN_INTERVAL / 2));
        assert_eq!(throttle.coalesced(), 2);
        assert!(throttle.claim(&mut last_forward, start + DEFAULT_FRAME_MIN_INTERVAL));
        assert_eq!(throttle.coalesced(), 2);
    }

    #[test]
    fn ticks_are_conserved_across_threads() {
        let throttle = Arc::new(SyncFrameThrottle::default());
        let forwarded = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let throttle = Arc::clone(&throttle);
            let forwarded = Arc::clone(&forwarded);
            handles.push(std::thread::spawn(move || {
                for _ in 0..100 {
                    if throttle.should_forward() {
                        forwarded.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }));
        }
        for handle in handles {
            handle.join().expect("worker panicked");
        }
        assert_eq!(
            forwarded.load(Ordering::Relaxed) + throttle.coalesced(),
            800
        );
    }

    #[test]
    fn rapid_submit_stashes_newest_until_window_elapses() {
        let debounce = SyncResizeDebounce::new(DEFAULT_RESIZE_DEBOUNCE_INTERVAL);
        debounce.submit((800u32, 600u32), |_| false);
        debounce.submit((1024u32, 768u32), |_| false);
        assert_eq!(debounce.pending(), Some((1024u32, 768u32)));
    }

    #[test]
    fn poll_applies_pending_once_window_elapsed() {
        let debounce = SyncResizeDebounce::new(Duration::ZERO);
        debounce.submit(1u32, |_| false);
        assert_eq!(debounce.pending(), Some(1u32));
        let applied = Arc::new(Mutex::new(None));
        let capture = Arc::clone(&applied);
        debounce.poll(|size| {
            *unpoison(capture.lock()) = Some(size);
            true
        });
        assert_eq!(*unpoison(applied.lock()), Some(1u32));
        assert_eq!(debounce.pending(), None);
    }

    #[test]
    fn concurrent_submits_keep_exactly_one_pending_size() {
        let debounce = Arc::new(SyncResizeDebounce::new(Duration::from_secs(60)));
        let mut handles = Vec::new();
        for id in 0..8u32 {
            let debounce = Arc::clone(&debounce);
            handles.push(std::thread::spawn(move || {
                debounce.submit(id, |_| false);
            }));
        }
        for handle in handles {
            handle.join().expect("worker panicked");
        }
        // All submits were stashed; exactly one (the last writer's) remains.
        assert!(debounce.pending().is_some_and(|id| id < 8));
    }
}
