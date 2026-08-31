//! Steady-state allocation contracts for frame-hot audio UI helpers.
//!
//! Keep these contracts in a dedicated integration-test binary: the counting
//! allocator is process-wide, so unrelated parallel tests would make deltas
//! noisy. Caches are explicitly warmed before the probe baseline is reset.

use gpui_audio_kit::meter::format_meter_value;
use gpui_audio_kit::scale::Scale;
use gpui_audio_kit::spectrum::MeterData;
use gpui_audio_kit::{DragState, InteractionConfig, handle_drag};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

fn cached_meter_formatting_is_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }
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

fn warmed_spectrum_meter_updates_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }
    const BINS: usize = 1_024;
    const ITERATIONS: usize = 1_000;

    let levels = vec![0.5; BINS];
    let mut spectrum = MeterData::new(BINS);
    let mut probe = AllocProbe::new();
    // Warm the exact retained state through a complete update cycle before
    // measuring. This absorbs allocator and CPU-specific first-use effects
    // while preserving the zero-allocation steady-state contract below.
    for _ in 0..ITERATIONS {
        spectrum.update(black_box(&levels), 0.8);
    }
    probe.reset();
    for _ in 0..ITERATIONS {
        spectrum.update(black_box(&levels), 0.8);
    }

    assert_eq!(spectrum.levels.len(), BINS);
    assert_eq!(spectrum.peaks.len(), BINS);
    AllocationBudget::zero("spectrum-meter-update-1024-bins-1000x")
        .assert_contains(probe.sample("spectrum-meter-update-1024-bins-1000x"));
}

fn warmed_knob_drag_math_is_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }
    let drag = DragState {
        start_pos: 120.0,
        start_value: 0.5,
    };
    let config = InteractionConfig::rotational(0.0, 1.0, Scale::Linear, 48.0);
    black_box(handle_drag(119.0, &drag, &config));
    let mut probe = AllocProbe::new();
    probe.reset();
    for frame in 0..1_000 {
        black_box(handle_drag(120.0 - frame as f32 * 0.01, &drag, &config));
    }
    AllocationBudget::zero("audio-knob-drag-1000x")
        .assert_contains(probe.sample("audio-knob-drag-1000x"));
}

#[test]
fn allocation_contracts_run_serially() {
    // Allocation counters are process-wide, so these measurements must not
    // execute concurrently in the same integration-test binary.
    cached_meter_formatting_is_allocation_free();
    warmed_spectrum_meter_updates_are_allocation_free();
    warmed_knob_drag_math_is_allocation_free();
}
