//! Warm-frame allocation acceptance for retained scene preparation.
//!
//! After the first (cold) preparation, camera-only frames must not
//! allocate in proportion to mesh size: allocation traffic for the
//! same orbit sequence at two very different mesh sizes stays within a
//! small tolerance, and no geometry is rebuilt. Measured in one serial
//! test because the probe is process-global.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{DataSet, Field};
use gpui_profiler::AllocProbe;
use gpui_scene3d::{Actor, Camera, Scene, TransferFunction, render};
use std::hint::black_box;

fn grid_scene(cells: usize) -> Scene {
    let dataset = DataSet::structured_grid("grid", [cells, cells, cells], [-1.0; 3], [0.1; 3])
        .unwrap()
        .with_field(Field::scalar("p", vec![0.5; cells * cells * cells]));
    Scene::new()
        .with_actor(Actor::new(dataset).coloured_by("p", TransferFunction::viridis([0.0, 1.0])))
}

fn orbit_allocations(scene: &Scene, frames: usize) -> (usize, usize) {
    let bounds = scene.bounds().unwrap();
    // Cold preparation + first frame, outside the measurement.
    let camera = Camera::fitted(bounds);
    black_box(render(scene, &camera, 64, 64).unwrap());
    let builds_before = scene.preparation_stats().geometry_builds;

    let mut probe = AllocProbe::new();
    probe.reset();
    let mut camera = Camera::fitted(bounds);
    for _ in 0..frames {
        camera.orbit(std::f32::consts::FRAC_PI_8, 0.05);
        black_box(render(scene, &camera, 64, 64).unwrap());
    }
    let sample = probe.sample("warm-orbit");
    assert_eq!(
        scene.preparation_stats().geometry_builds,
        builds_before,
        "camera-only frames must not rebuild geometry"
    );
    (sample.bytes, sample.count)
}

#[test]
fn warm_camera_frames_do_not_allocate_per_mesh_vertex() {
    if std::env::var_os("CARGO_LLVM_COV").is_some() {
        return; // Coverage instrumentation allocates inside measured work.
    }
    // The probe is process-global; other tests in this binary do not
    // exist, and this test measures both sizes back to back.
    let small = grid_scene(4); // 125 points
    let (small_bytes, small_calls) = orbit_allocations(&small, 8);
    let large = grid_scene(24); // 15,625 points (125x the vertices)
    let (large_bytes, large_calls) = orbit_allocations(&large, 8);
    assert!(
        large_bytes <= small_bytes + 64 * 1024,
        "warm orbit bytes grew with mesh size: small={small_bytes} large={large_bytes}"
    );
    assert!(
        large_calls <= small_calls + 64,
        "warm orbit allocation calls grew with mesh size: small={small_calls} large={large_calls}"
    );
}
