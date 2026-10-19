//! Allocation regression guard for general volume boundary traversal.
//!
//! The 32-MiB bound is cumulative Rust allocation during this fixed traversal,
//! excluding source construction/validation. It is not a peak-memory bound.

#![cfg(not(target_family = "wasm"))]

// Rust guideline compliant 2026-02-21

use gpui_mesh::{DataSet, try_for_each_surface_chunk};
use gpui_profiler::AllocProbe;
use std::hint::black_box;

#[test]
fn general_volume_chunk_visit_stays_under_32_mib_allocated() {
    let mut dataset = DataSet::structured_grid("bounded", [48; 3], [0.0; 3], [1.0; 3]).unwrap();
    // Preserve the volume faces while breaking canonical cell ordering.
    let nodes = std::sync::Arc::make_mut(&mut dataset.cells.connectivity);
    let last = nodes.len() - 8;
    nodes[last..last + 4].rotate_left(1);
    nodes[last + 4..].rotate_left(1);
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
    let allocation = probe.sample("general-volume-boundary");
    assert_eq!(count, 12 * 47 * 47);
    assert!(
        allocation.bytes < 32 * 1024 * 1024,
        "general traversal allocated {} bytes; source and validation excluded",
        allocation.bytes
    );
}
