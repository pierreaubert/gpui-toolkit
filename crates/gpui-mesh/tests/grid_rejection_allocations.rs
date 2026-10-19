//! Invalid layouts must be rejected before grid-array reservations.

#![cfg(not(target_family = "wasm"))]

// Rust guideline compliant 2026-02-21

use gpui_mesh::{DataSet, GridBuildLimits, MeshError};
use gpui_profiler::AllocProbe;

#[test]
fn oversized_and_zero_budget_requests_do_not_allocate_grid_arrays() {
    for dims in [[usize::MAX, 2, 2], [256; 3]] {
        let mut probe = AllocProbe::new();
        let result = DataSet::structured_grid("reject", dims, [0.0; 3], [1.0; 3]);
        let allocation = probe.sample("grid-rejection");
        assert!(matches!(result, Err(MeshError::ResourceLimit { .. })));
        assert!(
            allocation.bytes <= 1024,
            "size rejection allocated {} bytes",
            allocation.bytes
        );
    }
    let tiny = GridBuildLimits {
        max_working_bytes: 0,
    };
    let mut probe = AllocProbe::new();
    let result = DataSet::rectilinear_grid_with_limits(
        "reject",
        &[0.0, 1.0],
        &[0.0, 1.0],
        &[0.0, 1.0],
        tiny,
    );
    let allocation = probe.sample("rectilinear-rejection");
    assert!(matches!(result, Err(MeshError::ResourceLimit { .. })));
    assert!(allocation.bytes <= 1024);
    let positions = vec![[0.0; 3]; 8];
    let mut probe = AllocProbe::new();
    let result = DataSet::curvilinear_grid_with_limits("reject", [2; 3], positions, tiny);
    let allocation = probe.sample("curvilinear-rejection");
    assert!(matches!(result, Err(MeshError::ResourceLimit { .. })));
    assert!(allocation.bytes <= 1024);
}
