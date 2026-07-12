//! Steady-state allocation contracts for frame-hot audio UI helpers.
//!
//! Keep these contracts in a dedicated integration-test binary: the counting
//! allocator is process-wide, so unrelated parallel tests would make deltas
//! noisy. Caches are explicitly warmed before the probe baseline is reset.

use gpui_audio_kit::meter::format_meter_value;
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

#[test]
fn cached_meter_formatting_is_allocation_free() {
    const ITERATIONS: usize = 1_000;

    // Warm the thread-local cache and any one-time SharedString machinery.
    black_box(format_meter_value(-18.0));

    let mut probe = AllocProbe::new();
    probe.reset();
    for _ in 0..ITERATIONS {
        black_box(format_meter_value(-18.0));
    }

    AllocationBudget::zero("cached-meter-formatting-1000x")
        .assert_contains(probe.sample("cached-meter-formatting-1000x"));
}
