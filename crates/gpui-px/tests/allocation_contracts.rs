//! Allocation contract for same-length streaming chart preparation.

use gpui_profiler::{AllocProbe, AllocationBudget};
use gpui_px::{line, scatter};
use std::hint::black_box;
use std::sync::Arc;

#[test]
fn warmed_line_and_scatter_stream_preparation_is_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured operations.
    }

    const POINTS: usize = 10_000;
    let x_a: Arc<[f64]> = (0..POINTS).map(|value| value as f64).collect();
    let y_a: Arc<[f64]> = (0..POINTS).map(|value| (value as f64).sin()).collect();
    let x_b: Arc<[f64]> = (0..POINTS).map(|value| value as f64 + 0.5).collect();
    let y_b: Arc<[f64]> = (0..POINTS).map(|value| (value as f64).cos()).collect();

    let mut line_chart = line(&x_a, &y_a);
    let mut scatter_chart = scatter(&x_a, &y_a);
    assert_eq!(line_chart.prepare_primary_data(), POINTS);
    assert_eq!(scatter_chart.prepare_primary_data(), POINTS);

    let mut probe = AllocProbe::new();
    probe.reset();
    for iteration in 0..1_000 {
        let (x, y) = if iteration % 2 == 0 {
            (&x_b, &y_b)
        } else {
            (&x_a, &y_a)
        };
        line_chart
            .replace_primary_data_shared(Arc::clone(x), Arc::clone(y))
            .unwrap();
        scatter_chart
            .replace_primary_data_shared(Arc::clone(x), Arc::clone(y))
            .unwrap();
        black_box(line_chart.prepare_primary_data());
        black_box(scatter_chart.prepare_primary_data());
    }

    AllocationBudget::zero("gpui-px-stream-prepare-10000-points-1000x")
        .assert_contains(probe.sample("gpui-px-stream-prepare-10000-points-1000x"));
}
