//! Defined camera depth behavior when projection coefficients exceed f32.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet};
use gpui_scene3d::{Actor, Camera, Projection, Scene, SceneError, render};

#[test]
fn software_reference_keeps_near_clipping_and_reports_surface_limit() {
    let camera = Camera {
        target: [0.0; 3],
        azimuth: 0.0,
        elevation: 0.0,
        distance: 0.0,
        near: 1e-38,
        far: 1e36,
        projection: Projection::Orthographic { half_height: 1.0 },
    };
    let near = f64::from(camera.near);
    let data = DataSet::new(
        "depth",
        vec![[-0.5, 0.0, 0.0], [0.0, 0.0, -near], [0.5, 0.0, -2.0 * near]],
        Cells::from_cells(&[
            (CellType::Vertex, &[0]),
            (CellType::Vertex, &[1]),
            (CellType::Vertex, &[2]),
        ]),
    );
    let scene = Scene::new().with_actor(Actor::new(data).color([1.0, 0.0, 0.0, 1.0]));
    let frame = render(&scene, &camera, 32, 32).unwrap();
    let pixel = |x: usize| &frame.rgba[(16 * 32 + x) * 4..][..4];
    assert_ne!(pixel(8), &[255, 0, 0, 255], "behind the near plane");
    assert_eq!(pixel(16), &[255, 0, 0, 255], "on the near plane");
    assert_eq!(pixel(24), &[255, 0, 0, 255], "in front of the near plane");
    let line_data = DataSet::new(
        "cross near",
        vec![[-0.5, 0.0, 0.0], [0.5, 0.0, -2.0 * near]],
        Cells::lines(&[[0, 1]]),
    );
    let line_scene = Scene::new().with_actor(Actor::new(line_data).color([1.0, 0.0, 0.0, 1.0]));
    let line_frame = render(&line_scene, &camera, 32, 32).unwrap();
    let pixel = |x: usize| &line_frame.rgba[(16 * 32 + x) * 4..][..4];
    assert_ne!(
        pixel(12),
        &[255, 0, 0, 255],
        "line behind near must be clipped"
    );
    assert_eq!(pixel(20), &[255, 0, 0, 255], "line in front must remain");
    let surface = Scene::new().with_actor(Actor::new(DataSet::surface(
        "surface",
        vec![[-0.5, -0.5, -1.0], [0.5, -0.5, -1.0], [0.0, 0.5, -1.0]],
        &[[0, 1, 2]],
    )));
    assert!(matches!(
        render(&surface, &camera, 32, 32),
        Err(SceneError::UnsupportedPrecision(_))
    ));
    assert_eq!(
        surface.preparation_stats().geometry_builds,
        0,
        "reject before preparing unsupported surfaces"
    );
}
