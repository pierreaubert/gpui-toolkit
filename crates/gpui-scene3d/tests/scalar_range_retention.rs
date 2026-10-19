//! Scalar display ranges reuse accurate source-domain coordinates.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet, Field};
use gpui_scene3d::{Actor, Camera, Frame, Scene, TransferFunction, render};
use std::sync::Arc;

const BASE: [f64; 2] = [1e100, 2e100];
const RANGES: [[f64; 2]; 4] = [
    [1e100, 3e100],
    [0.0, 4e100],
    [-1e100, 3e100],
    [1.45e100, 1.55e100],
];
fn scene(cell: bool, primitives: bool) -> Scene {
    let data = if primitives {
        DataSet::new(
            "line and point",
            vec![[-0.8, 0.0, 0.0], [0.8, 0.0, 0.0], [0.0, 0.5, 0.0]],
            Cells {
                types: vec![CellType::Line, CellType::Vertex].into(),
                offsets: vec![0, 2, 3].into(),
                connectivity: vec![0, 1, 2].into(),
            },
        )
    } else {
        DataSet::surface(
            "surface",
            vec![
                [-0.8, -0.8, 0.0],
                [0.8, -0.8, 0.0],
                [0.8, 0.8, 0.0],
                [-0.8, 0.8, 0.0],
            ],
            &[[0, 1, 2], [0, 2, 3]],
        )
    };
    let values = if cell {
        Field::scalar("p", vec![1.3e100, 1.7e100]).on_cells()
    } else if primitives {
        Field::scalar("p", vec![BASE[0], BASE[1], 1.5e100])
    } else {
        Field::scalar("p", vec![BASE[0], BASE[1], 1.50000001e100, 1.49999999e100])
    };
    let actor = Actor::new(data.with_field(values));
    let tf = TransferFunction::coolwarm(BASE);
    Scene::new().with_actor(if cell {
        actor.coloured_by_cell("p", tf)
    } else {
        actor.coloured_by("p", tf)
    })
}
fn camera() -> Camera {
    Camera::fitted(gpui_mesh::Bounds {
        min: [-1.0, -1.0, -0.1],
        max: [1.0, 1.0, 0.1],
    })
}
fn fresh(scene: &Scene) -> Scene {
    Scene::new().with_actor(scene.actors[0].clone())
}
fn compare(a: &Frame, b: &Frame) {
    assert!(a.rgba.as_chunks::<4>().0.iter().any(|p| p[0] != p[1]));
    for (a, b) in a.rgba.iter().zip(&b.rgba) {
        assert!(
            a.abs_diff(*b) <= 2,
            "range remapping changed a byte: {a} != {b}"
        );
    }
}
#[test]
fn software_reuses_preparation_across_encoded_display_ranges() {
    for cell in [false, true] {
        for primitives in [false, true] {
            let mut scene = scene(cell, primitives);
            let initial = scene.prepare().unwrap();
            for range in RANGES.into_iter().chain([BASE, [1.5e100; 2]]) {
                scene.actors[0].transfer.as_mut().unwrap().range = range;
                compare(
                    &render(&scene, &camera(), 64, 64).unwrap(),
                    &render(&fresh(&scene), &camera(), 64, 64).unwrap(),
                );
                assert!(
                    Arc::ptr_eq(&initial, &scene.prepare().unwrap()),
                    "display range rebuilt accurate coordinates"
                );
            }
            assert_eq!(scene.preparation_stats().geometry_builds, 1);
        }
    }
}
#[test]
fn precision_zoom_constant_expansion_and_source_changes_reencode() {
    let mut scene = scene(false, false);
    let initial = scene.prepare().unwrap();
    scene.actors[0].transfer.as_mut().unwrap().range = [1.49999999e100, 1.50000001e100];
    let zoomed = scene.prepare().unwrap();
    assert!(
        !Arc::ptr_eq(&initial, &zoomed),
        "zoom needs original f64 values"
    );
    compare(
        &render(&scene, &camera(), 64, 64).unwrap(),
        &render(&fresh(&scene), &camera(), 64, 64).unwrap(),
    );
    scene.actors[0].dataset.point_data[0] = Field::scalar("p", vec![1.5e100; 4]);
    assert!(!Arc::ptr_eq(&zoomed, &scene.prepare().unwrap()));
    scene.actors[0].transfer.as_mut().unwrap().range = [1.5e100; 2];
    scene.clear_preparation();
    let constant = scene.prepare().unwrap();
    scene.actors[0].transfer.as_mut().unwrap().range = BASE;
    assert!(
        !Arc::ptr_eq(&constant, &scene.prepare().unwrap()),
        "constant encoding discarded source differences"
    );
}
#[test]
fn mixed_actor_updates_retain_each_compatible_encoding() {
    let mut scene = scene(false, false);
    scene.actors.push(scene.actors[0].clone());
    scene.prepare().unwrap();
    assert_eq!(scene.preparation_stats().geometry_builds, 1);
    scene.actors[0].transfer.as_mut().unwrap().range = RANGES[0];
    scene.actors[1].dataset.point_data[0] = Field::scalar("p", vec![1.5e100; 4]);
    scene.prepare().unwrap();
    assert_eq!(
        scene.preparation_stats().geometry_builds,
        2,
        "unrelated source update rebuilt compatible actor"
    );
    scene.actors.swap(0, 1);
    scene.prepare().unwrap();
    assert_eq!(scene.preparation_stats().geometry_builds, 2);
}
#[test]
#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
fn gpu_reuses_coordinates_and_preserves_rejected_updates() {
    use gpui_scene3d::{GpuRenderError, GpuRenderer, SceneError};
    let renderer = match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
        Ok(r) => r,
        Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => return,
        Err(e) => panic!("GPU required: {e}"),
    };
    let target = renderer.create_target(64, 64).unwrap();
    for cell in [false, true] {
        for primitives in [false, true] {
            let mut scene = scene(cell, primitives);
            let mut uploaded = renderer.upload(&scene).unwrap();
            let bytes = uploaded.geometry_bytes;
            for range in RANGES.into_iter().chain([BASE, [1.5e100; 2]]) {
                scene.actors[0].transfer.as_mut().unwrap().range = range;
                assert!(
                    !renderer.sync(&mut uploaded, &scene).unwrap(),
                    "display range uploaded geometry"
                );
                renderer.render(&mut uploaded, &camera(), &target).unwrap();
                let actual = renderer.read_rgba(&target).unwrap();
                let mut cold = renderer.upload(&fresh(&scene)).unwrap();
                renderer.render(&mut cold, &camera(), &target).unwrap();
                compare(&actual, &renderer.read_rgba(&target).unwrap());
                assert_eq!(uploaded.geometry_bytes, bytes);
                let mut bad = fresh(&scene);
                bad.background = [1.0; 4];
                bad.actors[0].transfer.as_mut().unwrap().range = [0.0, 1.0];
                assert!(matches!(
                    renderer.sync(&mut uploaded, &bad),
                    Err(GpuRenderError::Scene(SceneError::UnsupportedPrecision(_)))
                ));
                renderer.render(&mut uploaded, &camera(), &target).unwrap();
                assert_eq!(renderer.read_rgba(&target).unwrap(), actual);
            }
            scene.actors[0].transfer.as_mut().unwrap().range = [1.49999999e100, 1.50000001e100];
            assert!(
                renderer.sync(&mut uploaded, &scene).unwrap(),
                "precision zoom must reencode"
            );
            renderer.render(&mut uploaded, &camera(), &target).unwrap();
            let zoomed = renderer.read_rgba(&target).unwrap();
            let mut cold = renderer.upload(&fresh(&scene)).unwrap();
            renderer.render(&mut cold, &camera(), &target).unwrap();
            compare(&zoomed, &renderer.read_rgba(&target).unwrap());
        }
    }
}
