//! Common frame scheduler for foreign-parent bindings (contract §6).
//!
//! One policy object every platform binding drives the same way:
//! host input and guest invalidations call
//! [`invalidate`](FrameScheduler::invalidate); the host pump
//! (display link, rAF, `requestFrame` callback, timer) asks
//! [`next_frame`](FrameScheduler::next_frame) what to do. Clean frames
//! cost nothing (no render is issued), bursts inside the minimum
//! interval coalesce through [`FrameThrottle`], and surface resizes go
//! through [`ResizeDebounce`] so no binding re-implements its own
//! gating. This is the shared replacement for the per-binding
//! always-render pumps.

use crate::{FrameThrottle, ResizeDebounce};
use std::cell::Cell;
use std::time::Duration;

/// What the host pump should do for this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameDecision {
    /// Render one frame now; pending invalidation is consumed.
    RenderNow,
    /// Dirty, but inside the minimum interval: skip this tick. The
    /// invalidation stays pending so a later tick renders it.
    Coalesced,
    /// Nothing changed since the last rendered frame: stay idle.
    Idle,
}

/// Dirty-flag + throttle + resize-debounce policy; main-thread use only.
#[derive(Debug)]
pub struct FrameScheduler<S: Copy> {
    throttle: FrameThrottle,
    resize: ResizeDebounce<S>,
    dirty: Cell<bool>,
    rendered: Cell<u64>,
    idle_ticks: Cell<u64>,
}

impl<S: Copy> FrameScheduler<S> {
    /// Create a scheduler with the reference intervals.
    #[must_use]
    pub fn new() -> Self {
        Self::with_intervals(
            crate::DEFAULT_FRAME_MIN_INTERVAL,
            crate::DEFAULT_RESIZE_DEBOUNCE_INTERVAL,
        )
    }

    /// Create a scheduler with explicit frame/resize intervals.
    #[must_use]
    pub fn with_intervals(frame_min_interval: Duration, resize_debounce: Duration) -> Self {
        Self {
            throttle: FrameThrottle::new(frame_min_interval),
            resize: ResizeDebounce::new(resize_debounce),
            dirty: Cell::new(true),
            rendered: Cell::new(0),
            idle_ticks: Cell::new(0),
        }
    }

    /// Mark the guest content dirty (input, data, animation tick).
    pub fn invalidate(&self) {
        self.dirty.set(true);
    }

    /// Whether a render is owed.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    /// Submit the newest host size through the resize debounce and
    /// mark the next frame dirty (a resize always needs a redraw).
    pub fn resize(&self, size: S, apply: impl FnOnce(S) -> bool) {
        self.resize.submit(size, apply);
        self.invalidate();
    }

    /// Apply a stashed resize ahead of drawing, when due.
    pub fn poll_resize(&self, apply: impl FnOnce(S) -> bool) {
        self.resize.poll(apply);
    }

    /// Decide this pump tick. `RenderNow` consumes the invalidation;
    /// the caller renders exactly one frame.
    pub fn next_frame(&self) -> FrameDecision {
        if !self.dirty.get() {
            self.idle_ticks.set(self.idle_ticks.get().saturating_add(1));
            return FrameDecision::Idle;
        }
        if !self.throttle.should_forward() {
            return FrameDecision::Coalesced;
        }
        self.dirty.set(false);
        self.rendered.set(self.rendered.get().saturating_add(1));
        FrameDecision::RenderNow
    }

    /// Frames decided `RenderNow` so far.
    #[must_use]
    pub fn rendered(&self) -> u64 {
        self.rendered.get()
    }

    /// Pump ticks that found nothing to do (idle ticks cost no render).
    #[must_use]
    pub fn idle_ticks(&self) -> u64 {
        self.idle_ticks.get()
    }

    /// Ticks coalesced by the throttle so far.
    #[must_use]
    pub fn coalesced(&self) -> usize {
        self.throttle.coalesced()
    }
}

impl<S: Copy> Default for FrameScheduler<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_ticks_stay_idle_and_dirty_ticks_render_once() {
        let scheduler =
            FrameScheduler::<(u32, u32)>::with_intervals(Duration::ZERO, Duration::from_millis(16));
        // Starts dirty (first frame must paint).
        assert_eq!(scheduler.next_frame(), FrameDecision::RenderNow);
        assert_eq!(scheduler.next_frame(), FrameDecision::Idle);
        assert_eq!(scheduler.next_frame(), FrameDecision::Idle);
        assert_eq!(scheduler.idle_ticks(), 2);
        scheduler.invalidate();
        assert_eq!(scheduler.next_frame(), FrameDecision::RenderNow);
        assert_eq!(scheduler.rendered(), 2);
        assert_eq!(scheduler.next_frame(), FrameDecision::Idle);
    }

    #[test]
    fn bursts_coalesce_inside_the_minimum_interval() {
        let scheduler = FrameScheduler::<(u32, u32)>::with_intervals(
            Duration::from_secs(60),
            Duration::from_millis(16),
        );
        assert_eq!(scheduler.next_frame(), FrameDecision::RenderNow);
        scheduler.invalidate();
        assert_eq!(scheduler.next_frame(), FrameDecision::Coalesced);
        assert!(scheduler.is_dirty(), "coalesced work stays pending");
        assert_eq!(scheduler.coalesced(), 1);
    }

    #[test]
    fn resize_marks_dirty_and_debounces_application() {
        let scheduler =
            FrameScheduler::<(u32, u32)>::with_intervals(Duration::ZERO, Duration::from_secs(60));
        assert_eq!(scheduler.next_frame(), FrameDecision::RenderNow);
        let applied = std::cell::Cell::new(None);
        scheduler.resize((800, 600), |size| {
            applied.set(Some(size));
            true
        });
        assert_eq!(applied.get(), Some((800, 600)));
        assert!(scheduler.is_dirty());
        scheduler.resize((1024, 768), |size| {
            applied.set(Some(size));
            true
        });
        // Inside the debounce window: stashed, not applied.
        assert_eq!(applied.get(), Some((800, 600)));
        assert_eq!(scheduler.next_frame(), FrameDecision::RenderNow);
    }
}
