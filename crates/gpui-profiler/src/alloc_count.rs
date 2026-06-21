//! Allocation counting API.
//!
//! This module is always compiled. When the `global-allocator` feature is
//! enabled it reads from the counting global allocator; otherwise it returns
//! zeros so instrumented callers do not need conditional compilation.
//!
//! `AllocProbe::sample` returns the delta since the last sample/reset; it does
//! not print anything. Callers are expected to surface the delta in their own
//! UI or logging.

/// A point-in-time snapshot of allocation counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AllocSnapshot {
    /// Bytes allocated since the counting allocator was installed.
    pub bytes: usize,
    /// Number of allocation calls since the counting allocator was installed.
    pub count: usize,
}

impl AllocSnapshot {
    /// Capture the current allocation counters.
    ///
    /// Returns zeros when the `global-allocator` feature is disabled.
    pub fn now() -> Self {
        #[cfg(feature = "global-allocator")]
        {
            use std::sync::atomic::Ordering;
            Self {
                bytes: crate::global::ALLOC_BYTES.load(Ordering::Relaxed),
                count: crate::global::ALLOC_COUNT.load(Ordering::Relaxed),
            }
        }
        #[cfg(not(feature = "global-allocator"))]
        {
            Self::default()
        }
    }

    /// Difference between two snapshots.
    pub fn delta_since(start: Self) -> Self {
        let now = Self::now();
        Self {
            bytes: now.bytes.saturating_sub(start.bytes),
            count: now.count.saturating_sub(start.count),
        }
    }
}

/// Helper for measuring allocations around a block of work.
///
/// When the `global-allocator` feature is enabled, `sample` returns the
/// allocation delta since the last sample/reset. When disabled, it is a
/// zero-sized no-op that returns zeros.
pub struct AllocProbe {
    #[cfg(feature = "global-allocator")]
    baseline: AllocSnapshot,
}

impl Default for AllocProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl AllocProbe {
    /// Create a new probe with the current counter values as the baseline.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "global-allocator")]
            baseline: AllocSnapshot::now(),
        }
    }

    /// Reset the baseline to the current counter values.
    pub fn reset(&mut self) {
        #[cfg(feature = "global-allocator")]
        {
            self.baseline = AllocSnapshot::now();
        }
    }

    /// Return the allocation delta since the last sample or reset.
    ///
    /// The returned `AllocSnapshot` contains the bytes and count allocated
    /// since the previous sample (or since the probe was created). The caller
    /// can display it in the UI or log it as desired.
    pub fn sample(&mut self, label: &str) -> AllocSnapshot {
        let _ = label;
        #[cfg(feature = "global-allocator")]
        {
            let now = AllocSnapshot::now();
            let delta = AllocSnapshot {
                bytes: now.bytes.saturating_sub(self.baseline.bytes),
                count: now.count.saturating_sub(self.baseline.count),
            };
            self.baseline = now;
            delta
        }
        #[cfg(not(feature = "global-allocator"))]
        {
            AllocSnapshot::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_api_compiles_and_returns_snapshot() {
        let mut probe = AllocProbe::new();
        let delta = probe.sample("test");
        assert_eq!(delta.count, 0);
    }

    #[test]
    fn probe_default_is_equivalent_to_new() {
        let probe = AllocProbe::default();
        let _ = probe;
    }

    #[test]
    fn probe_reset_can_be_called() {
        let mut probe = AllocProbe::new();
        probe.reset();
        let _ = probe.sample("after-reset");
    }

    #[cfg(not(feature = "global-allocator"))]
    #[test]
    fn snapshot_now_without_global_allocator_is_zero() {
        let snapshot = AllocSnapshot::now();
        assert_eq!(snapshot, AllocSnapshot::default());
    }

    #[test]
    fn snapshot_delta_since_computes_difference() {
        let start = AllocSnapshot { bytes: 3, count: 5 };
        let delta = AllocSnapshot::delta_since(start);
        let now = AllocSnapshot::now();
        assert_eq!(delta.bytes, now.bytes.saturating_sub(start.bytes));
        assert_eq!(delta.count, now.count.saturating_sub(start.count));
    }

    #[cfg(feature = "global-allocator")]
    #[test]
    fn probe_detects_allocations() {
        let mut probe = AllocProbe::new();
        let mut values = Vec::new();
        for i in 0..100 {
            values.push(i);
        }
        let _ = values;
        let delta = probe.sample("test");
        assert!(delta.count > 0, "expected at least one allocation");
    }
}
