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

/// A named upper bound for allocations in a warmed-up operation.
///
/// Budgets make allocation expectations executable instead of leaving them as
/// benchmark comments. Callers should warm caches and reserve reusable buffers
/// before taking the starting snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationBudget {
    /// Stable operation name used in assertion failures and QA reports.
    pub operation: &'static str,
    /// Maximum allocation/reallocation calls allowed by the operation.
    pub max_count: usize,
    /// Maximum newly allocated bytes allowed by the operation.
    pub max_bytes: usize,
}

impl AllocationBudget {
    /// Require a fully allocation-free steady-state operation.
    pub const fn zero(operation: &'static str) -> Self {
        Self {
            operation,
            max_count: 0,
            max_bytes: 0,
        }
    }

    /// Construct an explicit allocation budget.
    pub const fn new(operation: &'static str, max_count: usize, max_bytes: usize) -> Self {
        Self {
            operation,
            max_count,
            max_bytes,
        }
    }

    /// Return whether a measured allocation delta is within this budget.
    pub const fn contains(self, measured: AllocSnapshot) -> bool {
        measured.count <= self.max_count && measured.bytes <= self.max_bytes
    }

    /// Assert that a measured allocation delta is within this budget.
    #[track_caller]
    pub fn assert_contains(self, measured: AllocSnapshot) {
        assert!(
            self.contains(measured),
            "allocation budget '{}' exceeded: measured {} calls/{} bytes, allowed {} calls/{} bytes",
            self.operation,
            measured.count,
            measured.bytes,
            self.max_count,
            self.max_bytes
        );
    }
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

    #[test]
    fn allocation_budget_accepts_values_at_or_below_limits() {
        let budget = AllocationBudget::new("render", 2, 128);
        assert!(budget.contains(AllocSnapshot {
            count: 2,
            bytes: 128,
        }));
        assert!(budget.contains(AllocSnapshot {
            count: 1,
            bytes: 64,
        }));
    }

    #[test]
    fn allocation_budget_rejects_either_limit_exceeding() {
        let budget = AllocationBudget::new("render", 2, 128);
        assert!(!budget.contains(AllocSnapshot {
            count: 3,
            bytes: 64,
        }));
        assert!(!budget.contains(AllocSnapshot {
            count: 1,
            bytes: 129,
        }));
    }

    #[test]
    #[should_panic(expected = "allocation budget 'steady-state' exceeded")]
    fn allocation_budget_assertion_names_the_operation() {
        AllocationBudget::zero("steady-state").assert_contains(AllocSnapshot {
            count: 1,
            bytes: 8,
        });
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
