//! Native allocation contract for shared cell decomposition.

#![cfg(not(target_family = "wasm"))]

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, tetrahedra};
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

#[test]
fn repeated_cell_decomposition_does_not_allocate() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }
    let nodes = [0, 1, 2, 3, 4, 5, 6, 7];
    let mut probe = AllocProbe::new();
    probe.reset();
    let mut count = 0usize;
    for _ in 0..10_000 {
        for tet in tetrahedra(black_box(CellType::Hexahedron), black_box(&nodes)) {
            black_box(tet);
            count += 1;
        }
    }
    let allocations = probe.sample("mesh-tetrahedra-10000x");
    assert_eq!(count, 60_000);
    AllocationBudget::zero("mesh-tetrahedra-10000x").assert_contains(allocations);
}
