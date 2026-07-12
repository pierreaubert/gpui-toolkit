//! Steady-state allocation contracts for cached text measurement.
//!
//! This is a dedicated integration-test process because allocation counters
//! are global. Every measured operation is warmed before the probe resets.

use gpui_pretext::measurement::{MeasureCache, TextMeasure};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

struct FixedMeasure;

impl TextMeasure for FixedMeasure {
    fn measure_width(&self, text: &str) -> f64 {
        text.chars().count() as f64 * 8.0
    }
}

#[test]
fn warmed_measurement_cache_hits_are_allocation_free() {
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

#[test]
fn warmed_grapheme_metric_cache_hits_are_allocation_free() {
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
