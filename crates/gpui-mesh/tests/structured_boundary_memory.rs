//! Allocation regression guard for canonical-grid boundary traversal.
//!
//! The four-MiB bound is cumulative Rust allocation during this fixed traversal,
//! excluding source construction/validation. It is not a peak-memory bound.

#![cfg(not(target_family = "wasm"))]

// Rust guideline compliant 2026-02-21

use gpui_mesh::{DataSet, try_for_each_surface_chunk};
use gpui_profiler::AllocProbe;
use std::hint::black_box;

#[test]
fn canonical_grid_chunk_visit_stays_under_four_mib_allocated() {
    let dataset = DataSet::structured_grid("bounded", [48; 3], [0.0; 3], [1.0; 3]).unwrap();
    let checked = dataset.validated().unwrap();
    let mut count = 0;
    let mut probe = AllocProbe::new();
    try_for_each_surface_chunk(checked, 4096, |chunk, owners| {
        count += owners.len();
        black_box(chunk);
        black_box(owners);
        Ok::<(), ()>(())
    })
    .unwrap();
    let allocation = probe.sample("canonical-grid-boundary");
    assert_eq!(count, 12 * 47 * 47);
    assert!(
        allocation.bytes < 4 * 1024 * 1024,
        "canonical traversal allocated {} bytes; source and validation excluded",
        allocation.bytes
    );
}
