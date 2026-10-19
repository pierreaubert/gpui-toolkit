//! Scene preparation propagates face-workspace errors for both associations.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet, Field, MeshError, SurfaceExtractionLimits};
use gpui_scene3d::{Actor, SceneError, TransferFunction, prepare_chunks_with_limits};

#[test]
fn point_and_cell_preparation_respect_explicit_face_allowances() {
    let data = DataSet::new(
        "tet",
        vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
    )
    .with_field(Field::scalar("point", vec![0.0, 0.25, 0.5, 1.0]))
    .with_field(Field::scalar("cell", vec![0.5]).on_cells());
    for actor in [
        Actor::new(data.clone()),
        Actor::new(data.clone()).coloured_by("point", TransferFunction::viridis([0.0, 1.0])),
        Actor::new(data).coloured_by_cell("cell", TransferFunction::viridis([0.0, 1.0])),
    ] {
        assert!(matches!(
            prepare_chunks_with_limits(&actor, 2, SurfaceExtractionLimits { max_face_bytes: 0 }),
            Err(SceneError::Mesh(MeshError::ResourceLimit {
                resource: "surface face workspace bytes",
                limit: 0,
                ..
            }))
        ));
        let chunks =
            prepare_chunks_with_limits(&actor, 2, SurfaceExtractionLimits::default()).unwrap();
        assert_eq!(
            chunks
                .iter()
                .map(gpui_scene3d::GpuChunk::triangle_count)
                .sum::<usize>(),
            4
        );
        assert!(chunks.iter().all(|c| c.triangle_count() <= 2));
    }
}

#[test]
fn retained_scene_applies_host_allowance_to_new_builds_and_keeps_warm_geometry() {
    use gpui_scene3d::{Camera, Scene, render};
    use std::sync::Arc;
    let data = DataSet::new(
        "tet",
        vec![
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
        ],
        Cells::from_cells(&[(CellType::Tetra, &[0, 1, 2, 3])]),
    );
    let mut scene = Scene::new()
        .with_actor(Actor::new(data))
        .with_surface_limits(SurfaceExtractionLimits { max_face_bytes: 0 });
    assert!(matches!(
        scene.prepare(),
        Err(SceneError::Mesh(MeshError::ResourceLimit { .. }))
    ));
    assert_eq!(scene.preparation_stats().geometry_builds, 0);
    scene.surface_limits = SurfaceExtractionLimits::default();
    let prepared = scene.prepare().unwrap();
    let camera = Camera::fitted(scene.bounds().unwrap());
    let expected = render(&scene, &camera, 32, 32).unwrap();
    scene.surface_limits.max_face_bytes = 0;
    assert!(
        Arc::ptr_eq(&prepared, &scene.prepare().unwrap()),
        "no workspace is needed for a cache hit"
    );
    assert_eq!(render(&scene, &camera, 32, 32).unwrap().rgba, expected.rgba);
    assert_eq!(scene.preparation_stats().geometry_builds, 1);
    let mut changed = scene.clone();
    Arc::make_mut(&mut changed.actors[0].dataset.positions)[0][0] -= 0.25;
    assert!(
        changed.prepare().is_err(),
        "new source revision must obey the clone's allowance"
    );
    assert!(
        Arc::ptr_eq(&prepared, &scene.prepare().unwrap()),
        "a failed build preserves cached geometry"
    );
    scene.clear_preparation();
    assert!(
        scene.prepare().is_err(),
        "a cleared cache needs workspace again"
    );
    assert!(
        prepared.matches(&scene).unwrap(),
        "existing external snapshots remain valid"
    );
    scene.surface_limits = SurfaceExtractionLimits::default();
    assert!(!Arc::ptr_eq(&prepared, &scene.prepare().unwrap()));
}
