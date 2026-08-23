//! Allocation contracts for retained d3rs frame-hot state.

#![cfg(all(feature = "profiler", feature = "gpu-compute"))]

use d3rs::mesh::gpu::compute::shared_mesh_compute;
use d3rs::vello2d::SceneCacheKey;
use gpui_profiler::{AllocProbe, AllocationBudget};
use std::hint::black_box;

#[test]
fn retained_vello_keys_and_shared_mesh_compute_are_allocation_free() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return;
    }

    // Warm the process-wide compute service and all lazy synchronization.
    let compute = shared_mesh_compute();
    black_box(&*compute.lock().unwrap());

    let mut probe = AllocProbe::new();
    probe.reset();
    for frame in 0_u64..1_000 {
        let mut key = SceneCacheKey::new();
        key.add(frame & 7).add(640_u32).add(360_u32);
        black_box(key.finish());
        black_box(&*compute.lock().unwrap());
    }

    AllocationBudget::zero("d3rs-vello-key-mesh-compute-1000x")
        .assert_contains(probe.sample("d3rs-vello-key-mesh-compute-1000x"));
}
