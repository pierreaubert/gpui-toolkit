//! Scalar units must not change coverage or material interpolation.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet, Field};
use gpui_scene3d::{Actor, Camera, Frame, Projection, Scene, TransferFunction, render};

fn transfer(range: [f64; 2]) -> TransferFunction {
    TransferFunction::from_stops(
        range,
        &[(0.0, [0.1, 0.2, 0.3, 1.0]), (1.0, [0.9, 0.8, 0.7, 1.0])],
    )
}
fn values(range: [f64; 2]) -> Vec<f64> {
    vec![range[0], range[1], range[0] * 0.5 + range[1] * 0.5]
}
fn scene(range: [f64; 2], cell: bool) -> Scene {
    let data = DataSet::surface(
        "units",
        vec![[-0.8, -0.8, 0.0], [0.8, -0.8, 0.0], [0.0, 0.8, 0.0]],
        &[[0, 1, 2]],
    );
    let actor = if cell {
        Actor::new(data.with_field(Field::scalar("p", vec![values(range)[2]]).on_cells()))
            .coloured_by_cell("p", transfer(range))
    } else {
        Actor::new(data.with_field(Field::scalar("p", values(range))))
            .coloured_by("p", transfer(range))
    };
    let mut scene = Scene::new().with_actor(actor);
    scene.background = [0.0, 0.0, 0.0, 1.0];
    scene
}
fn camera() -> Camera {
    Camera {
        target: [0.0; 3],
        azimuth: 0.0,
        elevation: 0.0,
        distance: 3.0,
        near: 1.0,
        far: 6.0,
        projection: Projection::Orthographic { half_height: 1.0 },
    }
}
fn compare(reference: &Frame, actual: &Frame) {
    assert_eq!(actual.rgba.len(), reference.rgba.len());
    let mut covered = 0;
    for (a, b) in actual
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(reference.rgba.as_chunks::<4>().0)
    {
        for channel in 0..4 {
            assert!(
                a[channel].abs_diff(b[channel]) <= 2,
                "scalar units changed a pixel: {a:?} != {b:?}"
            );
        }
        if b[0] > 0 {
            covered += 1;
        }
    }
    assert!(covered > 500);
}
const DOMAINS: [[f64; 2]; 6] = [
    [1e12, 1e12 + 1e5],
    [1e6, 1e6 + 1.0],
    [1e100, 2e100],
    [1e12, 1e12 + 1.0],
    [0.0, 1e-100],
    [-f64::MAX, f64::MAX],
];

#[test]
fn software_surface_point_and_cell_colors_survive_scalar_units() {
    for cell in [false, true] {
        let expected = render(&scene([0.0, 1.0], cell), &camera(), 64, 64).unwrap();
        for domain in DOMAINS {
            compare(
                &expected,
                &render(&scene(domain, cell), &camera(), 64, 64).unwrap(),
            );
        }
    }
}

#[test]
fn finite_unrepresentable_coordinates_are_errors_and_invalid_samples_remain_missing() {
    let mut scene = scene([0.0, 1.0], false);
    scene.actors[0].dataset.point_data[0] = Field::scalar("p", vec![1e100; 3]);
    assert!(
        matches!(
            render(&scene, &camera(), 64, 64),
            Err(gpui_scene3d::SceneError::UnsupportedPrecision(_))
        ),
        "a finite valid sample must not disappear as missing"
    );
    let mut missing = Field::scalar("p", vec![1e100; 3]);
    missing.valid = Some(vec![false; 3].into());
    scene.actors[0].dataset.point_data[0] = missing;
    let frame = render(&scene, &camera(), 64, 64).unwrap();
    assert!(
        frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [0, 0, 0, 255])
    );
}

fn primitive_scene(range: [f64; 2]) -> Scene {
    let data = DataSet::new(
        "primitives",
        vec![[-0.8, 0.0, 0.0], [0.8, 0.0, 0.0], [0.0, 0.5, 0.0]],
        Cells {
            types: vec![CellType::Line, CellType::Vertex].into(),
            offsets: vec![0, 2, 3].into(),
            connectivity: vec![0, 1, 2].into(),
        },
    )
    .with_field(Field::scalar("p", values(range)));
    let mut scene = Scene::new().with_actor(Actor::new(data).coloured_by("p", transfer(range)));
    scene.background = [0.0, 0.0, 0.0, 1.0];
    scene
}

#[test]
#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
fn gpu_surface_and_primitive_materials_survive_scalar_units() {
    use gpui_scene3d::{GpuRenderError, GpuRenderer};
    let renderer = match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
        Ok(r) => r,
        Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => return,
        Err(e) => panic!("GPU required: {e}"),
    };
    let target = renderer.create_target(64, 64).unwrap();
    let frame = |scene: &Scene| {
        let mut uploaded = renderer.upload(scene).unwrap();
        renderer.render(&mut uploaded, &camera(), &target).unwrap();
        renderer.read_rgba(&target).unwrap()
    };
    for cell in [false, true] {
        let expected = frame(&scene([0.0, 1.0], cell));
        for domain in DOMAINS {
            compare(&expected, &frame(&scene(domain, cell)));
        }
    }
    for cell in [false, true] {
        compare(
            &frame(&scene([0.0, 0.0], cell)),
            &frame(&scene([1e100, 1e100], cell)),
        );
        let mut missing_scene = scene([0.0, 1.0], cell);
        let mut field = Field::scalar("p", vec![1e100; if cell { 1 } else { 3 }]);
        field.valid = Some(vec![false; if cell { 1 } else { 3 }].into());
        if cell {
            missing_scene.actors[0].dataset.cell_data[0] = field.on_cells();
        } else {
            missing_scene.actors[0].dataset.point_data[0] = field;
        }
        assert!(
            frame(&missing_scene)
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| *p == [0, 0, 0, 255])
        );
    }
    let expected = frame(&primitive_scene([0.0, 1.0]));
    for domain in DOMAINS {
        let actual = frame(&primitive_scene(domain));
        assert_eq!(actual.rgba.len(), expected.rgba.len());
        for (a, b) in actual.rgba.iter().zip(&expected.rgba) {
            assert!(a.abs_diff(*b) <= 2);
        }
    }
}

fn clipped_scene(range: [f64; 2]) -> Scene {
    let mut scene = scene(range, false);
    // Values lie outside the display domain. Clamping before clipping or
    // perspective interpolation would change the visible interior gradient.
    let [lo, hi] = range;
    scene.actors[0].dataset.point_data[0] = Field::scalar(
        "p",
        vec![
            lo - (hi - lo) * 2.0,
            lo + (hi - lo) * 2.0,
            lo * 0.5 + hi * 0.5,
        ],
    );
    std::sync::Arc::make_mut(&mut scene.actors[0].dataset.positions)[0][2] = 2.5;
    scene.actors[0].clip_planes.push(gpui_scene3d::ClipPlane {
        origin: [-0.15, 0.0, 0.0],
        normal: [1.0, 0.0, 0.0],
    });
    scene
}
fn clipped_camera() -> Camera {
    Camera {
        projection: Projection::Perspective {
            fov_y: std::f32::consts::FRAC_PI_2,
        },
        ..camera()
    }
}
fn compare_pixels(reference: &Frame, actual: &Frame) {
    assert_eq!(reference.rgba.len(), actual.rgba.len());
    for (a, b) in actual.rgba.iter().zip(&reference.rgba) {
        assert!(
            a.abs_diff(*b) <= 2,
            "clipped domain changed color: {a} != {b}"
        );
    }
    assert!(
        reference
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 0)
            .count()
            > 30
    );
}
#[test]
fn software_linear_coordinates_remain_unclamped_through_clipping() {
    let expected = render(&clipped_scene([0.0, 1.0]), &clipped_camera(), 64, 64).unwrap();
    compare_pixels(
        &expected,
        &render(&clipped_scene([1e100, 2e100]), &clipped_camera(), 64, 64).unwrap(),
    );
}

#[test]
#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
fn gpu_domain_updates_remap_coordinates_and_fail_transactionally() {
    use gpui_scene3d::{GpuRenderError, GpuRenderer, SceneError};
    let renderer = match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
        Ok(r) => r,
        Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => return,
        Err(e) => panic!("GPU required: {e}"),
    };
    let target = renderer.create_target(64, 64).unwrap();
    let clipped = |range| {
        let mut uploaded = renderer.upload(&clipped_scene(range)).unwrap();
        renderer
            .render(&mut uploaded, &clipped_camera(), &target)
            .unwrap();
        renderer.read_rgba(&target).unwrap()
    };
    compare_pixels(&clipped([0.0, 1.0]), &clipped([1e100, 2e100]));
    let mut wide = scene([1e100, 2e100], false);
    let mut uploaded = renderer.upload(&wide).unwrap();
    wide.actors[0].transfer = Some(transfer([1e100, 3e100]));
    assert!(
        !renderer.sync(&mut uploaded, &wide).unwrap(),
        "compatible display ranges retain encoded scalar coordinates"
    );
    renderer.render(&mut uploaded, &camera(), &target).unwrap();
    let actual = renderer.read_rgba(&target).unwrap();
    let mut unit = scene([0.0, 1.0], false);
    unit.actors[0].dataset.point_data[0] = Field::scalar("p", vec![0.0, 0.5, 0.25]);
    let mut reference = renderer.upload(&unit).unwrap();
    renderer.render(&mut reference, &camera(), &target).unwrap();
    compare(&renderer.read_rgba(&target).unwrap(), &actual);
    let mut bad = wide.clone();
    bad.background = [1.0; 4];
    bad.actors[0].transfer = Some(transfer([0.0, 1.0]));
    assert!(matches!(
        renderer.sync(&mut uploaded, &bad),
        Err(GpuRenderError::Scene(SceneError::UnsupportedPrecision(_)))
    ));
    renderer.render(&mut uploaded, &camera(), &target).unwrap();
    assert_eq!(renderer.read_rgba(&target).unwrap().rgba, actual.rgba);
}

#[test]
fn constant_large_domains_keep_the_start_color() {
    for cell in [false, true] {
        compare(
            &render(&scene([0.0, 0.0], cell), &camera(), 64, 64).unwrap(),
            &render(&scene([1e100, 1e100], cell), &camera(), 64, 64).unwrap(),
        );
    }
}

#[test]
fn software_primitives_retain_scalar_unit_invariance() {
    let reference = render(&primitive_scene([0.0, 1.0]), &camera(), 64, 64).unwrap();
    for domain in DOMAINS {
        compare_pixels(
            &reference,
            &render(&primitive_scene(domain), &camera(), 64, 64).unwrap(),
        );
    }
}

#[test]
fn linear_lut_positions_do_not_lose_resolution_in_original_scalar_units() {
    let make = |range| {
        TransferFunction::from_stops(
            range,
            &[(0.0, [0.1, 0.2, 0.3, 0.2]), (1.0, [0.9, 0.8, 0.7, 0.8])],
        )
    };
    for bands in [None, Some(255)] {
        let mut unit = make([0.0, 1.0]);
        unit.bands = bands;
        let expected = unit.lut_bytes();
        for domain in DOMAINS {
            let mut shifted = make(domain);
            shifted.bands = bands;
            let actual = shifted.lut_bytes();
            let first = expected
                .iter()
                .zip(&actual)
                .enumerate()
                .find(|(_, (a, b))| a != b)
                .map(|(i, (a, b))| (i, *a, *b));
            assert_eq!(
                first, None,
                "linear LUT changed with domain {domain:?}, bands {bands:?}"
            );
        }
    }
}
