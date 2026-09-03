//! Allocation counting API.
//!
//! This module is always compiled. When the `global-allocator` feature is
//! enabled it reads from the counting global allocator; otherwise it returns
//! zeros so instrumented callers do not need conditional compilation.
//!
//! `AllocProbe::sample` returns the delta since the last sample/reset; it does
//! not print anything. Callers are expected to surface the delta in their own
//! UI or logging.
//!
//! Every sample carries its caller-supplied label: [`AllocProbe::sample`]
//! remembers it for [`AllocProbe::last_label`], and
//! [`AllocProbe::sample_labeled`] returns it alongside the delta so named
//! series can be exported with [`samples_to_csv`] or
//! [`samples_to_chrome_trace`].

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use std::thread::ThreadId;

/// A point-in-time snapshot of allocation counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AllocSnapshot {
    /// Bytes allocated since the counting allocator was installed.
    pub bytes: usize,
    /// Allocation calls since the counting allocator was installed.
    ///
    /// This folds plain allocations and reallocations together; use
    /// [`AllocSnapshot::reallocs`] and [`AllocSnapshot::allocs`] to tell them
    /// apart.
    pub count: usize,
    /// Reallocation calls since the counting allocator was installed.
    ///
    /// Always `<= count`. Split out so growth-driven reallocations (for
    /// example `Vec` regrowth) can be distinguished from fresh allocations.
    pub reallocs: usize,
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
            Self::from_stats(crate::global::GLOBAL.stats())
        }
        #[cfg(not(feature = "global-allocator"))]
        {
            Self::default()
        }
    }

    /// Number of plain (non-reallocation) calls in this snapshot or delta.
    ///
    /// Computed as `count - reallocs`, saturating at zero so mixed
    /// snapshot/delta arithmetic cannot underflow.
    #[must_use]
    pub const fn allocs(self) -> usize {
        self.count.saturating_sub(self.reallocs)
    }

    #[cfg(feature = "global-allocator")]
    fn from_stats(stats: stats_alloc::Stats) -> Self {
        // stats_alloc already includes positive realloc growth in
        // `bytes_allocated`; `bytes_reallocated` is the net diagnostic delta.
        Self {
            bytes: stats.bytes_allocated,
            count: stats.allocations.saturating_add(stats.reallocations),
            reallocs: stats.reallocations,
        }
    }

    /// Difference between two snapshots, saturating at zero per field.
    const fn delta_between(now: Self, baseline: Self) -> Self {
        Self {
            bytes: now.bytes.saturating_sub(baseline.bytes),
            count: now.count.saturating_sub(baseline.count),
            reallocs: now.reallocs.saturating_sub(baseline.reallocs),
        }
    }

    /// Difference between two snapshots.
    pub fn delta_since(start: Self) -> Self {
        let now = Self::now();
        Self::delta_between(now, start)
    }
}
/// An allocation delta tagged with the caller-supplied sample label.
///
/// Returned by [`AllocProbe::sample_labeled`] and
/// [`ThreadAllocProbe::sample_labeled`]; see [`samples_to_csv`] and
/// [`samples_to_chrome_trace`] for export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabeledAllocSample {
    /// Label passed to `sample` / `sample_labeled`.
    pub label: &'static str,
    /// Allocation delta since the previous sample or reset.
    pub snapshot: AllocSnapshot,
}

/// Render labeled samples as CSV with the header
/// `label,bytes,count,allocs,reallocs`.
///
/// Fields containing `,`, `"`, or line breaks are quoted per RFC 4180. An
/// empty series renders as the header line alone.
#[must_use]
pub fn samples_to_csv(samples: &[LabeledAllocSample]) -> String {
    let mut out = String::from("label,bytes,count,allocs,reallocs\n");
    for sample in samples {
        out.push_str(&escape_csv_field(sample.label));
        out.push(',');
        push_number(&mut out, sample.snapshot.bytes);
        out.push(',');
        push_number(&mut out, sample.snapshot.count);
        out.push(',');
        push_number(&mut out, sample.snapshot.allocs());
        out.push(',');
        push_number(&mut out, sample.snapshot.reallocs);
        out.push('\n');
    }
    out
}

/// Render labeled samples as a Chrome-Trace/Perfetto JSON array of complete
/// (`ph: "X"`) events, one per sample.
///
/// The probe records cumulative counters rather than wall time, so `ts` is the
/// sample index (ordering only, in microseconds) and `dur` is zero; the
/// measured `bytes`, `count`, `allocs`, and `reallocs` travel in `args`. An
/// empty series renders as `[]`.
#[must_use]
pub fn samples_to_chrome_trace(samples: &[LabeledAllocSample]) -> String {
    let mut out = String::from("[");
    for (index, sample) in samples.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"name\":\"");
        out.push_str(&escape_json_string(sample.label));
        out.push_str("\",\"cat\":\"alloc\",\"ph\":\"X\",\"pid\":1,\"tid\":0,\"ts\":");
        push_number(&mut out, index);
        out.push_str(",\"dur\":0,\"args\":{\"bytes\":");
        push_number(&mut out, sample.snapshot.bytes);
        out.push_str(",\"count\":");
        push_number(&mut out, sample.snapshot.count);
        out.push_str(",\"allocs\":");
        push_number(&mut out, sample.snapshot.allocs());
        out.push_str(",\"reallocs\":");
        push_number(&mut out, sample.snapshot.reallocs);
        out.push_str("}}");
    }
    out.push(']');
    out
}

fn push_number(out: &mut String, value: usize) {
    // `usize::to_string` never fails; one shared push path for CSV and JSON.
    out.push_str(&value.to_string());
}

fn escape_csv_field(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        let mut quoted = String::with_capacity(field.len() + 2);
        quoted.push('"');
        for ch in field.chars() {
            if ch == '"' {
                quoted.push('"');
            }
            quoted.push(ch);
        }
        quoted.push('"');
        quoted
    } else {
        field.to_owned()
    }
}

fn escape_json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                escaped.push_str(&format!("\\u{:04x}", ch as u32));
            }
            ch => escaped.push(ch),
        }
    }
    escaped
}

/// Helper for measuring allocations around a block of work.
///
/// When the `global-allocator` feature is enabled, `sample` returns the
/// allocation delta since the last sample/reset. When disabled, it is a
/// near-zero-cost shim that returns zeros but still records the label and
/// keeps the peak at zero, so label/export plumbing can be exercised without
/// the counting allocator.
pub struct AllocProbe {
    #[cfg(feature = "global-allocator")]
    baseline: AllocSnapshot,
    /// Most recent label passed to [`AllocProbe::sample`].
    last_label: Option<&'static str>,
    /// High-water mark of per-sample `bytes` deltas since creation or the
    /// last [`AllocProbe::reset_peak`] call. Survives [`AllocProbe::reset`].
    peak_bytes: usize,
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
            last_label: None,
            peak_bytes: 0,
        }
    }

    /// Reset the baseline to the current counter values.
    ///
    /// Does not clear the peak; call [`AllocProbe::reset_peak`] for that.
    pub fn reset(&mut self) {
        #[cfg(feature = "global-allocator")]
        {
            self.baseline = AllocSnapshot::now();
        }
    }

    /// Return the allocation delta since the last sample or reset.
    ///
    /// The returned `AllocSnapshot` contains the bytes and count allocated
    /// since the previous sample (or since the probe was created). The label
    /// is recorded for [`AllocProbe::last_label`] and
    /// [`AllocProbe::sample_labeled`]; the caller can display the delta in
    /// the UI or log it as desired.
    ///
    /// The label must be `'static` (a string literal at every in-repo call
    /// site) so recording it performs no allocation and never pollutes the
    /// counters being measured.
    pub fn sample(&mut self, label: &'static str) -> AllocSnapshot {
        self.last_label = Some(label);
        #[cfg(feature = "global-allocator")]
        {
            let now = AllocSnapshot::now();
            let delta = AllocSnapshot::delta_between(now, self.baseline);
            self.baseline = now;
            self.peak_bytes = self.peak_bytes.max(delta.bytes);
            delta
        }
        #[cfg(not(feature = "global-allocator"))]
        {
            AllocSnapshot::default()
        }
    }

    /// Return the allocation delta tagged with its label for named series.
    ///
    /// Equivalent to [`AllocProbe::sample`] plus the label, ready to collect
    /// and export with [`samples_to_csv`] or [`samples_to_chrome_trace`].
    pub fn sample_labeled(&mut self, label: &'static str) -> LabeledAllocSample {
        let snapshot = self.sample(label);
        LabeledAllocSample { label, snapshot }
    }

    /// Most recent label passed to [`AllocProbe::sample`], if any.
    #[must_use]
    pub const fn last_label(&self) -> Option<&'static str> {
        self.last_label
    }

    /// High-water mark of per-sample `bytes` deltas since creation or the
    /// last [`AllocProbe::reset_peak`] call.
    #[must_use]
    pub const fn peak_bytes(&self) -> usize {
        self.peak_bytes
    }

    /// Clear the high-water mark tracked by [`AllocProbe::peak_bytes`].
    pub fn reset_peak(&mut self) {
        self.peak_bytes = 0;
    }
}

fn lock_probe<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().expect("allocation probe lock is poisoned")
}

/// Per-thread allocation probe.
///
/// An `AllocProbe` baseline is per instance: sharing one across threads mixes
/// every thread's allocations into a single delta. `ThreadAllocProbe` is
/// `Sync` and keeps an independent baseline (plus label and peak) per calling
/// thread, so one instance can be shared while each thread measures its own
/// deltas.
///
/// The underlying counters are still process-wide: another thread allocating
/// between this thread's `reset` and `sample` inflates this thread's delta.
/// Treat deltas as regression signals rather than exact per-thread budgets.
/// The first `sample` on a thread behaves like a reset followed by a sample
/// and reports a zero delta.
#[derive(Debug, Default)]
pub struct ThreadAllocProbe {
    baselines: Mutex<HashMap<ThreadId, AllocSnapshot>>,
    labels: Mutex<HashMap<ThreadId, &'static str>>,
    peaks: Mutex<HashMap<ThreadId, usize>>,
}

impl ThreadAllocProbe {
    /// Create a new probe with empty per-thread state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset the calling thread's baseline to the current counter values.
    pub fn reset(&self) {
        #[cfg(feature = "global-allocator")]
        {
            lock_probe(&self.baselines).insert(std::thread::current().id(), AllocSnapshot::now());
        }
    }

    /// Return the calling thread's allocation delta since its last sample or
    /// reset, recording `label` for [`ThreadAllocProbe::last_label`].
    pub fn sample(&self, label: &'static str) -> AllocSnapshot {
        let thread = std::thread::current().id();
        // Warm-up note: the first sample/reset on a thread grows these maps
        // (one bookkeeping allocation); sample once before measuring, exactly
        // like any other cache warm-up, and steady-state samples stay clean.
        lock_probe(&self.labels).insert(thread, label);
        // Baseline bookkeeping runs in both configurations so label and peak
        // plumbing can be exercised without the counting allocator; without
        // `global-allocator` every snapshot is zero, so deltas stay zero.
        let now = AllocSnapshot::now();
        #[cfg(feature = "global-allocator")]
        let baseline = lock_probe(&self.baselines)
            .get(&thread)
            .copied()
            .unwrap_or(now);
        lock_probe(&self.baselines).insert(thread, now);
        #[cfg(feature = "global-allocator")]
        {
            let delta = AllocSnapshot::delta_between(now, baseline);
            let mut peaks = lock_probe(&self.peaks);
            let peak = peaks.entry(thread).or_insert(0);
            *peak = (*peak).max(delta.bytes);
            delta
        }
        #[cfg(not(feature = "global-allocator"))]
        {
            AllocSnapshot::default()
        }
    }

    /// Return the calling thread's delta tagged with its label.
    pub fn sample_labeled(&self, label: &'static str) -> LabeledAllocSample {
        let snapshot = self.sample(label);
        LabeledAllocSample { label, snapshot }
    }

    /// Label of the calling thread's most recent sample, if any.
    #[must_use]
    pub fn last_label(&self) -> Option<&'static str> {
        lock_probe(&self.labels)
            .get(&std::thread::current().id())
            .copied()
    }

    /// High-water mark of the calling thread's per-sample `bytes` deltas.
    #[must_use]
    pub fn peak_bytes(&self) -> usize {
        lock_probe(&self.peaks)
            .get(&std::thread::current().id())
            .copied()
            .unwrap_or(0)
    }

    /// Clear the calling thread's high-water mark.
    pub fn reset_peak(&self) {
        lock_probe(&self.peaks).remove(&std::thread::current().id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Allocation counters are process-wide, so tests that measure exact
    // deltas and tests that allocate must not run concurrently (see the
    // allocation-contract template in README.md). Every test below holds
    // this lock; poisoning is ignored because `should_panic` tests panic
    // while holding it.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn test_lock() -> MutexGuard<'static, ()> {
        TEST_LOCK
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    fn labeled(
        label: &'static str,
        bytes: usize,
        count: usize,
        reallocs: usize,
    ) -> LabeledAllocSample {
        LabeledAllocSample {
            label,
            snapshot: AllocSnapshot {
                bytes,
                count,
                reallocs,
            },
        }
    }

    #[test]
    fn probe_api_compiles_and_returns_snapshot() {
        let _guard = test_lock();
        let mut probe = AllocProbe::new();
        let delta = probe.sample("test");
        assert_eq!(delta.count, 0);
    }

    #[test]
    fn probe_default_is_equivalent_to_new() {
        let _guard = test_lock();
        let probe = AllocProbe::default();
        let _ = probe;
    }

    #[test]
    fn probe_reset_can_be_called() {
        let _guard = test_lock();
        let mut probe = AllocProbe::new();
        probe.reset();
        let _ = probe.sample("after-reset");
    }

    #[test]
    fn probe_records_sample_label() {
        let _guard = test_lock();
        let mut probe = AllocProbe::new();
        assert_eq!(probe.last_label(), None);
        let _ = probe.sample("render");
        assert_eq!(probe.last_label(), Some("render"));
        let labeled_sample = probe.sample_labeled("input");
        assert_eq!(labeled_sample.label, "input");
        assert_eq!(probe.last_label(), Some("input"));
    }

    #[test]
    fn peak_tracks_max_sample_bytes_and_resets() {
        let _guard = test_lock();
        let mut probe = AllocProbe::new();
        assert_eq!(probe.peak_bytes(), 0);
        let delta = probe.sample("first");
        assert!(probe.peak_bytes() >= delta.bytes);
        probe.reset();
        // The peak is a high-water mark: a baseline reset must not clear it.
        assert!(probe.peak_bytes() >= delta.bytes);
        probe.reset_peak();
        assert_eq!(probe.peak_bytes(), 0);
    }

    #[test]
    fn snapshot_allocs_excludes_reallocs() {
        let _guard = test_lock();
        let snapshot = AllocSnapshot {
            bytes: 96,
            count: 5,
            reallocs: 2,
        };
        assert_eq!(snapshot.allocs(), 3);
        let saturated = AllocSnapshot {
            bytes: 0,
            count: 1,
            reallocs: 9,
        };
        assert_eq!(saturated.allocs(), 0);
    }

    #[cfg(not(feature = "global-allocator"))]
    #[test]
    fn snapshot_now_without_global_allocator_is_zero() {
        let _guard = test_lock();
        let snapshot = AllocSnapshot::now();
        assert_eq!(snapshot, AllocSnapshot::default());
    }

    #[cfg(not(feature = "global-allocator"))]
    #[test]
    fn delta_since_without_counters_reports_zeros() {
        let _guard = test_lock();
        let delta = AllocSnapshot::delta_since(AllocSnapshot {
            bytes: 3,
            count: 5,
            reallocs: 2,
        });
        assert_eq!(delta, AllocSnapshot::default());
    }

    #[test]
    fn snapshot_delta_since_computes_difference() {
        let _guard = test_lock();
        let start = AllocSnapshot {
            bytes: 3,
            count: 5,
            reallocs: 1,
        };
        let delta = AllocSnapshot::delta_since(start);
        let now = AllocSnapshot::now();
        assert_eq!(delta.bytes, now.bytes.saturating_sub(start.bytes));
        assert_eq!(delta.count, now.count.saturating_sub(start.count));
        assert_eq!(delta.reallocs, now.reallocs.saturating_sub(start.reallocs));
    }

    #[cfg(feature = "global-allocator")]
    #[test]
    fn snapshot_does_not_double_count_reallocation_growth() {
        let _guard = test_lock();
        let snapshot = AllocSnapshot::from_stats(stats_alloc::Stats {
            allocations: 3,
            deallocations: 1,
            reallocations: 2,
            bytes_allocated: 96,
            bytes_deallocated: 16,
            bytes_reallocated: 64,
        });

        assert_eq!(snapshot.bytes, 96);
        assert_eq!(snapshot.count, 5);
        assert_eq!(snapshot.reallocs, 2);
        assert_eq!(snapshot.allocs(), 3);
    }

    #[test]
    fn allocation_budget_accepts_values_at_or_below_limits() {
        let _guard = test_lock();
        let budget = AllocationBudget::new("render", 2, 128);
        assert!(budget.contains(AllocSnapshot {
            count: 2,
            bytes: 128,
            reallocs: 0,
        }));
        assert!(budget.contains(AllocSnapshot {
            count: 1,
            bytes: 64,
            reallocs: 1,
        }));
    }

    #[test]
    fn allocation_budget_rejects_either_limit_exceeding() {
        let _guard = test_lock();
        let budget = AllocationBudget::new("render", 2, 128);
        assert!(!budget.contains(AllocSnapshot {
            count: 3,
            bytes: 64,
            reallocs: 0,
        }));
        assert!(!budget.contains(AllocSnapshot {
            count: 1,
            bytes: 129,
            reallocs: 0,
        }));
    }

    #[test]
    #[should_panic(expected = "allocation budget 'steady-state' exceeded")]
    fn allocation_budget_assertion_names_the_operation() {
        let _guard = test_lock();
        AllocationBudget::zero("steady-state").assert_contains(AllocSnapshot {
            count: 1,
            bytes: 8,
            reallocs: 0,
        });
    }

    #[cfg(feature = "global-allocator")]
    #[test]
    fn probe_detects_allocations() {
        let _guard = test_lock();
        let mut probe = AllocProbe::new();
        let mut values = Vec::new();
        for i in 0..100 {
            values.push(i);
        }
        let _ = values;
        let delta = probe.sample("test");
        assert!(delta.count > 0, "expected at least one allocation");
    }

    #[cfg(feature = "global-allocator")]
    #[test]
    fn probe_peak_records_allocating_sample() {
        let _guard = test_lock();
        let mut probe = AllocProbe::new();
        probe.reset_peak();
        let mut values = Vec::new();
        for i in 0..100 {
            values.push(i);
        }
        std::hint::black_box(values);
        let delta = probe.sample("allocating");
        assert!(delta.bytes > 0, "expected allocated bytes");
        assert_eq!(probe.peak_bytes(), delta.bytes);
    }

    #[test]
    fn csv_export_matches_expected_output() {
        let _guard = test_lock();
        let samples = vec![
            labeled("render", 128, 2, 0),
            labeled("input", 64, 3, 2),
            labeled("weird,label\"q", 0, 0, 0),
        ];
        assert_eq!(
            samples_to_csv(&samples),
            "label,bytes,count,allocs,reallocs\n\
             render,128,2,2,0\n\
             input,64,3,1,2\n\
             \"weird,label\"\"q\",0,0,0,0\n"
        );
    }

    #[test]
    fn csv_export_of_empty_series_is_header_only() {
        let _guard = test_lock();
        assert_eq!(samples_to_csv(&[]), "label,bytes,count,allocs,reallocs\n");
    }

    #[test]
    fn chrome_trace_export_matches_expected_output() {
        let _guard = test_lock();
        let samples = vec![labeled("render", 128, 2, 0)];
        assert_eq!(
            samples_to_chrome_trace(&samples),
            "[{\"name\":\"render\",\"cat\":\"alloc\",\"ph\":\"X\",\"pid\":1,\"tid\":0,\
             \"ts\":0,\"dur\":0,\"args\":{\"bytes\":128,\"count\":2,\
             \"allocs\":2,\"reallocs\":0}}]"
        );
    }

    #[test]
    fn chrome_trace_export_escapes_labels_and_handles_empty_series() {
        let _guard = test_lock();
        assert_eq!(samples_to_chrome_trace(&[]), "[]");
        let samples = vec![labeled("a\"b\\c", 1, 1, 0)];
        let trace = samples_to_chrome_trace(&samples);
        assert!(
            trace.contains("\"name\":\"a\\\"b\\\\c\""),
            "label must be JSON-escaped: {trace}"
        );
    }

    #[test]
    fn thread_probe_is_sync_and_tracks_labels_per_thread() {
        let _guard = test_lock();
        fn assert_sync<T: Sync>() {}
        assert_sync::<ThreadAllocProbe>();

        let probe = ThreadAllocProbe::new();
        assert_eq!(probe.last_label(), None);
        assert_eq!(probe.peak_bytes(), 0);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                probe.reset();
                let _ = probe.sample("worker-a");
                assert_eq!(probe.last_label(), Some("worker-a"));
            });
            scope.spawn(|| {
                probe.reset();
                let labeled_sample = probe.sample_labeled("worker-b");
                assert_eq!(labeled_sample.label, "worker-b");
                assert_eq!(probe.last_label(), Some("worker-b"));
            });
        });
        // Labels are per thread: the main thread never sampled.
        assert_eq!(probe.last_label(), None);
        assert_eq!(probe.peak_bytes(), 0);
        probe.reset_peak();
    }
}
