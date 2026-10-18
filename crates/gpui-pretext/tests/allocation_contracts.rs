//! Steady-state allocation contracts for cached text measurement.
//!
//! This is a dedicated integration-test process because allocation counters
//! are global. Every measured operation is warmed before the probe resets.

use gpui_pretext::measurement::{MeasureCache, TextMeasure};
use gpui_pretext::{
    EngineProfile, KnuthPlassParams, PrepareOptions, prepare_with_segments,
    walk_line_ranges_optimal,
};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

struct FixedMeasure;

impl TextMeasure for FixedMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * 8.0
    }

    fn cache_key(&self) -> u64 {
        8.0f64.to_bits()
    }
    fn cache_key_is_stable(&self) -> bool {
        true
    }
}

fn warmed_measurement_cache_hits_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }
    const ITERATIONS: usize = 1_000;
    let mut cache = MeasureCache::new();
    let measure = FixedMeasure;

    black_box(cache.get_width("cache me", &measure));

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..ITERATIONS {
        black_box(cache.get_width("cache me", &measure));
    }

    AllocationBudget::zero("pretext-width-cache-hit-1000x")
        .assert_contains(probe.sample("pretext-width-cache-hit-1000x"));
}

fn warmed_grapheme_metric_cache_hits_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }
    const ITERATIONS: usize = 1_000;
    let mut cache = MeasureCache::new();
    let measure = FixedMeasure;

    black_box(cache.get_grapheme_widths("Ame\u{301}lie", &measure));
    black_box(cache.get_grapheme_prefix_widths("Ame\u{301}lie", &measure));

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..ITERATIONS {
        black_box(cache.get_grapheme_widths("Ame\u{301}lie", &measure));
        black_box(cache.get_grapheme_prefix_widths("Ame\u{301}lie", &measure));
    }

    AllocationBudget::zero("pretext-grapheme-cache-hits-1000x")
        .assert_contains(probe.sample("pretext-grapheme-cache-hits-1000x"));
}

fn unkeyed_scoped_measurement_hits_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }
    struct Unkeyed;
    impl TextMeasure for Unkeyed {
        fn measure_width(&self, text: &str) -> f64 {
            text.chars().count() as f64 * 8.0
        }
    }
    let measure = Unkeyed;
    let mut storage = MeasureCache::new();
    let mut session = storage.scope(&measure);
    black_box(session.get_width("Ame\u{301}lie"));
    black_box(session.get_grapheme_widths("Ame\u{301}lie"));
    black_box(session.get_grapheme_prefix_widths("Ame\u{301}lie"));
    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..1000 {
        black_box(session.get_width("Ame\u{301}lie"));
        black_box(session.get_grapheme_widths("Ame\u{301}lie"));
        black_box(session.get_grapheme_prefix_widths("Ame\u{301}lie"));
    }
    AllocationBudget::zero("pretext-unkeyed-scoped-hits-1000x")
        .assert_contains(probe.sample("pretext-unkeyed-scoped-hits-1000x"));
}

fn charged_cache_memory_covers_allocator_requests() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }
    let measure = FixedMeasure;
    // Warm reusable scratch outside the cache-allocation measurement.
    let mut warmup = MeasureCache::new();
    black_box(warmup.get_grapheme_prefix_widths("scratch warmup", &measure));
    let mut probe = AllocProbe::new();
    probe.reset();
    let mut cache = MeasureCache::new();
    black_box(cache.get_width("abc", &measure));
    let cold = probe.sample("pretext-cold-cache-storage");
    let base = cache.stats();
    assert!(
        base.retained_bytes >= cold.bytes,
        "{base:?} must cover {cold:?}"
    );
    black_box(cache.get_grapheme_widths("abc", &measure));
    let arrays = probe.sample("pretext-grapheme-storage");
    let graphemes = cache.stats();
    assert!(graphemes.retained_bytes - base.retained_bytes >= arrays.bytes);
    black_box(cache.get_grapheme_prefix_widths("abc", &measure));
    let prefix = probe.sample("pretext-prefix-storage");
    assert!(cache.stats().retained_bytes - graphemes.retained_bytes >= prefix.bytes);
}

/// Optimal multi-chunk walks buffer lines by value (`Copy`, no `clone`)
/// with an upfront capacity reserve: after warming the Knuth-Plass item
/// cache, a repeated walk must not grow the line buffer. The probe asserts a
/// small fixed budget (one replay pass, no reallocation) rather than zero,
/// since the callback itself is caller-controlled.
fn warmed_optimal_multichunk_walk_does_not_reallocate_lines() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }
    let measure = FixedMeasure;
    let profile = EngineProfile::default();
    let options = PrepareOptions::default();
    let params = KnuthPlassParams::default();
    // Hard breaks => multiple chunks => buffered optimal path.
    let text = "alpha bravo charlie\ndelta echo foxtrot\ngolf hotel india";
    let prepared = prepare_with_segments(text, &measure, &profile, &options);

    // Warm the KP item cache and the Knuth-Plass thread-local scratch.
    for _ in 0..5 {
        black_box(walk_line_ranges_optimal(
            &prepared,
            60.0,
            &profile,
            &params,
            |_| {},
        ));
    }

    let mut probe = AllocProbe::new();
    probe.reset();
    let mut total = 0;
    for _ in 0..200 {
        let mut n = 0;
        black_box(walk_line_ranges_optimal(
            &prepared,
            60.0,
            &profile,
            &params,
            |_| {
                n += 1;
            },
        ));
        total += n;
    }
    black_box(total);

    // Known per-walk allocations (3 chunks): 1 capacity-reserved line buffer
    // + 1 single-chunk view Vec per chunk. Nothing per line: lines move by
    // value and breakpoints remap in place. Budget = 5 allocs/walk + slack.
    AllocationBudget::new("pretext-optimal-multichunk-walk-200x", 5 * 200, 512 * 200)
        .assert_contains(probe.sample("pretext-optimal-multichunk-walk-200x"));
}

/// Bounded `MeasureCache` never retains more than its entry budget, even for
/// adversarial per-grapheme CJK input that would grow an unbounded map.
fn bounded_cache_caps_adversarial_growth() {
    let measure = FixedMeasure;
    let mut cache = MeasureCache::with_capacity(16);
    for i in 0..256 {
        black_box(cache.get_width(&format!("\u{6f22}{i}"), &measure));
    }
    assert!(
        cache.len() <= 16,
        "bounded cache grew to {} entries",
        cache.len()
    );
}

#[test]
fn allocation_contracts_run_serially() {
    // Allocation counters are process-wide, so these measurements must not
    // execute concurrently in the same integration-test binary.
    warmed_measurement_cache_hits_are_allocation_free();
    warmed_grapheme_metric_cache_hits_are_allocation_free();
    unkeyed_scoped_measurement_hits_are_allocation_free();
    charged_cache_memory_covers_allocator_requests();
    warmed_optimal_multichunk_walk_does_not_reallocate_lines();
    bounded_cache_caps_adversarial_growth();
}
