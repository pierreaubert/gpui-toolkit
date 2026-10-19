//! Area-weighted normals must survive changes of physical units.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{DataSet, Field};
use gpui_scene3d::{Actor, prepare_chunks};

fn actor(scale: f64, cell_colors: bool) -> Actor {
    // Two unequal-area faces share an edge; averaging unit face normals would
    // incorrectly give the shared vertices equal Y/Z contributions.
    let points = [
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 2.0],
    ];
    let data = DataSet::surface(
        "units",
        points
            .into_iter()
            .map(|p| p.map(|v| v * scale))
            .collect::<Vec<_>>(),
        &[[0, 1, 2], [1, 0, 3]],
    );
    if cell_colors {
        Actor::new(data.with_field(Field::scalar("p", vec![0.0, 1.0]).on_cells()))
            .coloured_by_cell("p", gpui_scene3d::TransferFunction::viridis([0.0, 1.0]))
    } else {
        Actor::new(data)
    }
}

#[test]
fn normals_keep_area_weighting_across_units_and_cell_splits() {
    for cell_colors in [false, true] {
        let reference = prepare_chunks(&actor(1.0, cell_colors), 100).unwrap();
        assert_eq!(reference.len(), 1);
        let expected = &reference[0];
        // Analytic shared normal: areas 2 and 4, hence [0,2,1]/sqrt(5).
        let shared = [0.0, 2.0 / 5.0f64.sqrt(), 1.0 / 5.0f64.sqrt()];
        assert!(expected.normals.as_chunks::<3>().0.iter().any(|n| {
            n.iter()
                .zip(shared)
                .all(|(&actual, expected)| (f64::from(actual) - expected).abs() < 1e-6)
        }));
        for scale in [1e-100, 1e-30, 1e-12, 1e12, 1e30] {
            let chunks = prepare_chunks(&actor(scale, cell_colors), 100).unwrap();
            assert_eq!(chunks.len(), 1);
            let actual = &chunks[0];
            assert_eq!(actual.indices, expected.indices);
            assert_eq!(actual.normals.len(), expected.normals.len());
            for (actual, expected) in actual.normals.iter().zip(&expected.normals) {
                assert!(
                    actual.is_finite() && (*actual - *expected).abs() < 1e-6,
                    "scale {scale}, cell colors {cell_colors}: {actual} != {expected}"
                );
            }
        }
    }
}

#[test]
fn degenerate_faces_and_unused_vertices_do_not_create_nan_normals() {
    let data = DataSet::surface(
        "degenerate",
        vec![[0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
    );
    let chunks = prepare_chunks(&Actor::new(data), 100).unwrap();
    assert!(chunks.iter().flat_map(|c| &c.normals).all(|v| *v == 0.0));
}

fn view(scale: f64) -> (gpui_scene3d::Scene, gpui_scene3d::Camera) {
    use gpui_scene3d::{Camera, Projection, Scene};
    let mut scene = Scene::new().with_actor(actor(scale, false).color([0.8, 0.6, 0.4, 1.0]));
    scene.background = [0.0, 0.0, 0.0, 1.0];
    let camera = Camera {
        target: [0.75 * scale, 0.5 * scale, 0.25 * scale],
        azimuth: 0.4,
        elevation: 0.3,
        distance: (4.0 * scale) as f32,
        near: (0.2 * scale) as f32,
        far: (8.0 * scale) as f32,
        projection: Projection::Orthographic {
            half_height: (1.5 * scale) as f32,
        },
    };
    (scene, camera)
}

fn compare_interior(reference: &gpui_scene3d::Frame, actual: &gpui_scene3d::Frame) {
    let mut samples = 0;
    for y in 1..63usize {
        for x in 1..63usize {
            let foreground = |x: usize, y: usize| reference.rgba[(y * 64 + x) * 4] > 50;
            if !((y - 1)..=(y + 1)).all(|y| ((x - 1)..=(x + 1)).all(|x| foreground(x, y))) {
                continue;
            }
            let offset = (y * 64 + x) * 4;
            for channel in 0..4 {
                assert!(
                    reference.rgba[offset + channel].abs_diff(actual.rgba[offset + channel]) <= 2,
                    "changed shading at ({x},{y}), channel {channel}: {} != {}",
                    reference.rgba[offset + channel],
                    actual.rgba[offset + channel]
                );
            }
            samples += 1;
        }
    }
    assert!(
        samples > 100,
        "require a substantial interior, checked {samples}"
    );
}

#[test]
fn software_shading_is_invariant_to_representable_unit_changes() {
    let (scene, camera) = view(1.0);
    let reference = gpui_scene3d::render(&scene, &camera, 64, 64).unwrap();
    for scale in [1e-12, 1e12] {
        let (scene, camera) = view(scale);
        compare_interior(
            &reference,
            &gpui_scene3d::render(&scene, &camera, 64, 64).unwrap(),
        );
    }
}

#[test]
#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
fn gpu_shading_is_invariant_to_representable_unit_changes() {
    use gpui_scene3d::{GpuRenderError, GpuRenderer};
    let renderer = match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
        Ok(renderer) => renderer,
        Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => return,
        Err(error) => panic!("GPU required: {error}"),
    };
    let target = renderer.create_target(64, 64).unwrap();
    let frame = |scale| {
        let (scene, camera) = view(scale);
        let mut uploaded = renderer.upload(&scene).unwrap();
        renderer.render(&mut uploaded, &camera, &target).unwrap();
        renderer.read_rgba(&target).unwrap()
    };
    let reference = frame(1.0);
    for scale in [1e-12, 1e12] {
        compare_interior(&reference, &frame(scale));
    }
}

#[test]
fn unsupported_position_range_reports_precision_instead_of_nan_normals() {
    let data = DataSet::surface(
        "range",
        vec![[-1e100, 0.0, 0.0], [1e100, 0.0, 0.0], [0.0, 1e100, 0.0]],
        &[[0, 1, 2]],
    );
    data.validate().unwrap();
    assert!(matches!(
        prepare_chunks(&Actor::new(data), 100),
        Err(gpui_scene3d::SceneError::UnsupportedPrecision(_))
    ));
}

#[test]
fn a_tiny_face_sharing_a_large_face_keeps_its_unshared_vertex_normals() {
    let data = DataSet::surface(
        "mixed units",
        vec![
            [0.0; 3],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1e-30, 0.0, 0.0],
            [0.0, 0.0, 1e-30],
        ],
        &[[0, 1, 2], [0, 3, 4]],
    );
    let chunks = prepare_chunks(&Actor::new(data), 100).unwrap();
    let normals = chunks[0].normals.as_chunks::<3>().0;
    assert_eq!(
        normals.iter().filter(|n| **n == [0.0, -1.0, 0.0]).count(),
        2
    );
    assert_eq!(normals.iter().filter(|n| n[2] == 1.0).count(), 3);
}
