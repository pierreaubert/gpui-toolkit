//! Debounced surface reconfiguration (contract §§5–6).
//!
//! Bounds, the native layer, and the GPUI resize callback track the
//! newest host size immediately; only the expensive surface reconfigure
//! is gated. A size that arrives inside the window is stashed and
//! applied from the next drawn frame, so the surface always converges
//! to the latest host size.

use std::cell::Cell;
use std::time::Duration;
use web_time::Instant;

/// Reference minimum interval between surface reconfigures.
///
/// Carried over from the `AUv3` baseline: 16 ms is roughly one 60 Hz
/// frame, so a host resize-drag reconfigures at most once per frame
/// while bounds already track the newest size. Lowering it risks GPU
/// `surface.configure` churn per resize event; raising it delays
/// visible convergence after the drag ends.
pub const DEFAULT_RESIZE_DEBOUNCE_INTERVAL: Duration = Duration::from_millis(16);

/// Debounce policy over a `Copy` size type; main-thread use only.
///
/// The `apply` closures return false when the reconfigure could not
/// land (no renderer yet, contended locks); the size is then stashed
/// for a later attempt instead of being lost.
#[derive(Debug)]
pub struct ResizeDebounce<S: Copy> {
    interval: Duration,
    last_apply: Cell<Option<Instant>>,
    pending: Cell<Option<S>>,
}

impl<S: Copy> ResizeDebounce<S> {
    /// Create a debounce policy with an explicit interval.
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_apply: Cell::new(None),
            pending: Cell::new(None),
        }
    }

    /// Submit the newest host size. Applies it immediately unless inside
    /// the debounce window or the apply fails, in which case it is
    /// stashed as pending.
    pub fn submit(&self, size: S, apply: impl FnOnce(S) -> bool) {
        let now = Instant::now();
        if self
            .last_apply
            .get()
            .is_some_and(|last| now.duration_since(last) < self.interval)
        {
            self.pending.set(Some(size));
            return;
        }
        if apply(size) {
            self.last_apply.set(Some(now));
        } else {
            self.pending.set(Some(size));
        }
    }

    /// Apply a stashed size ahead of drawing, when the debounce window
    /// has elapsed. A no-op without a pending size.
    pub fn poll(&self, apply: impl FnOnce(S) -> bool) {
        let Some(size) = self.pending.take() else {
            return;
        };
        let now = Instant::now();
        if self
            .last_apply
            .get()
            .is_some_and(|last| now.duration_since(last) < self.interval)
        {
            self.pending.set(Some(size));
            return;
        }
        if apply(size) {
            self.last_apply.set(Some(now));
        } else {
            self.pending.set(Some(size));
        }
    }

    /// Newest size still waiting for the window to elapse, if any.
    #[must_use]
    pub fn pending(&self) -> Option<S> {
        self.pending.get()
    }
}

impl<S: Copy> Default for ResizeDebounce<S> {
    fn default() -> Self {
        Self::new(DEFAULT_RESIZE_DEBOUNCE_INTERVAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn rapid_submit_stashes_newest_until_window_elapses() {
        let debounce = ResizeDebounce::new(DEFAULT_RESIZE_DEBOUNCE_INTERVAL);
        // No renderer: applies cannot land, so the newest size is
        // stashed for the next frame.
        debounce.submit((800u32, 600u32), |_| false);
        debounce.submit((1024u32, 768u32), |_| false);
        assert_eq!(debounce.pending(), Some((1024u32, 768u32)));
    }

    #[test]
    fn applied_size_records_the_timestamp() {
        let debounce = ResizeDebounce::new(DEFAULT_RESIZE_DEBOUNCE_INTERVAL);
        let applied = Rc::new(RefCell::new(Vec::new()));
        let capture = Rc::clone(&applied);
        debounce.submit(1u32, |size| {
            capture.borrow_mut().push(size);
            true
        });
        // Inside the window now: stashed, not applied.
        debounce.submit(2u32, |_| {
            unreachable!("must not apply inside the window");
        });
        assert_eq!(&*applied.borrow(), &[1u32]);
        assert_eq!(debounce.pending(), Some(2u32));
    }

    #[test]
    fn poll_applies_pending_once_window_elapsed() {
        let debounce = ResizeDebounce::new(Duration::ZERO);
        // Zero interval: the window has always elapsed.
        debounce.submit(1u32, |_| false);
        assert_eq!(debounce.pending(), Some(1u32));
        let applied = Rc::new(Cell::new(None));
        let capture = Rc::clone(&applied);
        debounce.poll(|size| {
            capture.set(Some(size));
            true
        });
        assert_eq!(applied.get(), Some(1u32));
        assert_eq!(debounce.pending(), None);
    }

    #[test]
    fn poll_without_pending_is_a_noop() {
        let debounce = ResizeDebounce::<u32>::new(DEFAULT_RESIZE_DEBOUNCE_INTERVAL);
        debounce.poll(|_| {
            unreachable!("must not apply without a pending size");
        });
    }
}
