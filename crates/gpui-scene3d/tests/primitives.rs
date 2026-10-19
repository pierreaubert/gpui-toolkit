//! Line/point scalar association, missing data, clipping and pixel coverage.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{Association, CellType, Cells, DataSet, Field};
use gpui_scene3d::{Actor, Camera, ClipPlane, Frame, Projection, Scene, TransferFunction};

const SIZE: u32 = 96;
fn camera(origin: f64) -> Camera {
    Camera {
        target: [origin; 3],
        azimuth: 0.0,
        elevation: 0.0,
        distance: 3.0,
        projection: Projection::Orthographic { half_height: 1.5 },
        near: 1.0,
        far: 6.0,
    }
}
fn scene(data: DataSet, cell: bool) -> Scene {
    let tf = TransferFunction::from_stops(
        [0.0, 1.0],
        &[(0.0, [1.0, 0.0, 0.0, 1.0]), (1.0, [0.0, 0.0, 1.0, 1.0])],
    );
    let actor = if cell {
        Actor::new(data).coloured_by_cell("p", tf)
    } else {
        Actor::new(data).coloured_by("p", tf)
    };
    let mut scene = Scene::new().with_actor(actor);
    scene.background = [0.0; 4];
    scene
}
fn colored(frame: &Frame) -> Vec<(u32, u32, [u8; 4])> {
    frame
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .enumerate()
        .filter(|(_, p)| p[3] > 0)
        .map(|(i, p)| (i as u32 % SIZE, i as u32 / SIZE, *p))
        .collect()
}
fn near(frame: &Frame, x: i32, y: i32) -> Vec<[u8; 4]> {
    colored(frame)
        .into_iter()
        .filter(|(a, b, _)| (*a as i32 - x).abs() <= 1 && (*b as i32 - y).abs() <= 1)
        .map(|(_, _, p)| p)
        .collect()
}

fn exercise(mut draw: impl FnMut(&Scene, &Camera) -> Frame) {
    for origin in [0.0, 1e12 + 0.125] {
        let positions = vec![
            [-1.0 + origin, origin, origin],
            [1.0 + origin, origin, origin],
        ];
        let line = DataSet::new(
            "line",
            positions.clone(),
            Cells::from_cells(&[(CellType::Line, &[0, 1])]),
        )
        .with_field(Field::scalar("p", vec![0.0, 1.0]));
        let frame = draw(&scene(line, false), &camera(origin));
        assert!(colored(&frame).len() > 50, "line missing");
        let left = near(&frame, 24, 48);
        let right = near(&frame, 72, 48);
        let middle = near(&frame, 48, 48);
        assert!(
            left.iter().any(|p| p[0] > 190 && p[2] < 65),
            "point scalar line lost left gradient: {left:?}"
        );
        assert!(
            right.iter().any(|p| p[2] > 190 && p[0] < 65),
            "point scalar line lost right gradient"
        );
        assert!(
            middle
                .iter()
                .any(|p| p[0].abs_diff(p[2]) < 10 && p[0] > 110),
            "line is lit or does not interpolate"
        );
        for mask in [vec![false, true], vec![true, false]] {
            let mut field = Field::scalar("p", vec![0.0, 999.0]);
            field.valid = Some(mask.into());
            let missing = DataSet::new(
                "missing",
                positions.clone(),
                Cells::from_cells(&[(CellType::Line, &[0, 1])]),
            )
            .with_field(field);
            assert!(
                colored(&draw(&scene(missing, false), &camera(origin))).is_empty(),
                "invalid endpoint entered interpolation"
            );
        }
        let mut field = Field::scalar("p", vec![0.0, 1.0, 999.0]);
        field.valid = Some(vec![true, true, false].into());
        let points = DataSet::new(
            "points",
            vec![
                [-0.5 + origin, origin, origin],
                [0.5 + origin, origin, origin],
                [origin, origin + 0.5, origin],
            ],
            Cells::from_cells(&[
                (CellType::Vertex, &[0]),
                (CellType::Vertex, &[1]),
                (CellType::Vertex, &[2]),
            ]),
        )
        .with_field(field);
        let frame = draw(&scene(points, false), &camera(origin));
        assert_eq!(
            colored(&frame).len(),
            2,
            "point cells must cover one pixel each"
        );
        assert!(near(&frame, 32, 48).iter().any(|p| p[0] > 250 && p[2] == 0));
        assert!(near(&frame, 64, 48).iter().any(|p| p[2] > 250 && p[0] == 0));
    }
    let positions = vec![[-1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0]];
    let cells = Cells::from_cells(&[
        (CellType::Line, &[0, 1]),
        (CellType::Line, &[1, 2]),
        (CellType::Vertex, &[1]),
    ]);
    let data = DataSet::new("cell-values", positions, cells)
        .with_field(Field::scalar("p", vec![0.0, 1.0, 0.5]).on_cells());
    let frame = draw(&scene(data.clone(), true), &camera(0.0));
    assert!(
        near(&frame, 24, 48).contains(&[255, 0, 0, 255]),
        "cell scalar was averaged into shared node"
    );
    assert!(near(&frame, 72, 48).contains(&[0, 0, 255, 255]));
    assert!(
        near(&frame, 48, 48)
            .iter()
            .any(|p| p[0].abs_diff(p[2]) <= 1 && (126..=128).contains(&p[0])),
        "point cell must draw after coincident line cells"
    );
    let mut missing = data;
    missing.cell_data[0].valid = Some(vec![false, true, false].into());
    let mut clipped = scene(missing, true);
    clipped.actors[0] = clipped.actors[0].clone().clip(ClipPlane {
        origin: [0.25, 0.0, 0.0],
        normal: [1.0, 0.0, 0.0],
    });
    let frame = draw(&clipped, &camera(0.0));
    assert!(colored(&frame).len() > 10);
    assert!(
        colored(&frame)
            .iter()
            .all(|&(x, _, p)| x >= 55 && p[2] > 250 && p[0] == 0),
        "cell mask or clipping was ignored"
    );
    let point = DataSet::new(
        "half",
        vec![[0.0; 3]],
        Cells::from_cells(&[(CellType::Vertex, &[0])]),
    )
    .with_field(Field::scalar("p", vec![0.0]));
    let mut half = scene(point, false);
    half.actors[0].opacity = 0.5;
    let pixels = colored(&draw(&half, &camera(0.0)));
    assert_eq!(pixels.len(), 1);
    assert!(
        pixels[0].2[3].abs_diff(128) <= 1 && pixels[0].2[0] > 250,
        "point alpha or unlit color lost"
    );
    for z in [2.5, -4.0] {
        let point = DataSet::new(
            "outside",
            vec![[0.0, 0.0, z]],
            Cells::from_cells(&[(CellType::Vertex, &[0])]),
        )
        .with_field(Field::scalar("p", vec![0.0]));
        assert!(
            colored(&draw(&scene(point, false), &camera(0.0))).is_empty(),
            "point outside near/far was drawn"
        );
    }
    // Independent pinhole model: eye z=3, fov=90 degrees, x_screen=48+48*x/(3-z).
    // The first endpoint crosses the near plane. Evaluate the original scalar,
    // rather than interpolating colors or values after clipping/clamping.
    let data = DataSet::new(
        "perspective",
        vec![[-1.0, 0.0, 2.5], [1.0, 0.0, -1.0]],
        Cells::from_cells(&[(CellType::Line, &[0, 1])]),
    )
    .with_field(Field::scalar("p", vec![-2.0, 2.0]));
    let mut perspective = camera(0.0);
    perspective.projection = Projection::Perspective {
        fov_y: std::f32::consts::FRAC_PI_2,
    };
    let mut line = scene(data, false);
    for clipped in [false, true] {
        if clipped {
            line.actors[0] = line.actors[0].clone().clip(ClipPlane {
                origin: [-0.5, 0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
            });
        }
        let frame = draw(&line, &perspective);
        for x in [34, 44, 55] {
            let pixels = colored(&frame)
                .into_iter()
                .filter(|&(a, b, _)| a == x && (47..=49).contains(&b))
                .collect::<Vec<_>>();
            assert!(!pixels.is_empty(), "near-clipped line lost pixel {x}");
            let r = (f64::from(x) + 0.5 - 48.0) / 48.0;
            let t = (-1.0 - 0.5 * r) / (3.5 * r - 2.0);
            let scalar = (-2.0 + 4.0 * t).clamp(0.0, 1.0);
            let expected = [255.0 * (1.0 - scalar), 255.0 * scalar];
            for (_, _, p) in pixels {
                assert!(
                    (f64::from(p[0]) - expected[0]).abs() <= 6.0
                        && (f64::from(p[2]) - expected[1]).abs() <= 6.0,
                    "perspective scalar at {x}, clip={clipped}: {p:?}, expected {expected:?}"
                );
            }
        }
    }
    let points = DataSet::new(
        "viewport",
        vec![
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.9999, 0.25, 0.0],
        ],
        Cells::from_cells(&[
            (CellType::Vertex, &[0]),
            (CellType::Vertex, &[1]),
            (CellType::Vertex, &[2]),
            (CellType::Vertex, &[3]),
            (CellType::Vertex, &[4]),
        ]),
    )
    .with_field(Field::scalar("p", vec![0.0; 5]));
    let mut ortho = camera(0.0);
    ortho.projection = Projection::Orthographic { half_height: 1.0 };
    let frame = draw(&scene(points, false), &ortho);
    let pixels = colored(&frame);
    assert_eq!(
        pixels.len(),
        3,
        "point viewport must be half-open: {pixels:?}"
    );
    for expected in [(0, 48), (48, 0), (95, 36)] {
        assert!(
            pixels.iter().any(|&(x, y, _)| (x, y) == expected),
            "missing {expected:?}"
        );
    }
    // An opaque face hides both primitive kinds in either actor order. Front
    // primitives must remain visible; the surface's lighting is irrelevant.
    let occluder = Actor::new(DataSet::new(
        "face",
        vec![
            [-1.25, -1.25, 0.0],
            [1.25, -1.25, 0.0],
            [1.25, 1.25, 0.0],
            [-1.25, 1.25, 0.0],
        ],
        Cells::from_cells(&[(CellType::Quad, &[0, 1, 2, 3])]),
    ));
    let mut reference = Scene::new().with_actor(occluder.clone());
    reference.background = [0.0; 4];
    let reference = draw(&reference, &camera(0.0));
    for z in [-0.5, 0.5] {
        let primitives = Actor::new(DataSet::new(
            "depth",
            vec![[-0.75, 0.0, z], [0.75, 0.0, z], [0.0, 0.5, z]],
            Cells::from_cells(&[(CellType::Line, &[0, 1]), (CellType::Vertex, &[2])]),
        ))
        .color([0.0, 0.0, 1.0, 1.0]);
        for reversed in [false, true] {
            let actors = if reversed {
                [primitives.clone(), occluder.clone()]
            } else {
                [occluder.clone(), primitives.clone()]
            };
            let mut depth_scene = Scene::new()
                .with_actor(actors[0].clone())
                .with_actor(actors[1].clone());
            depth_scene.background = [0.0; 4];
            let frame = draw(&depth_scene, &camera(0.0));
            if z < 0.0 {
                assert_eq!(frame.rgba, reference.rgba, "hidden primitives leaked");
            } else {
                assert!(near(&frame, 48, 48).contains(&[0, 0, 255, 255]));
                assert!(near(&frame, 48, 32).contains(&[0, 0, 255, 255]));
            }
        }
    }
}

#[test]
fn software_line_point_material_and_validity_contract() {
    exercise(|scene, camera| gpui_scene3d::render(scene, camera, SIZE, SIZE).unwrap());
}

#[test]
fn software_keeps_f64_scalar_domains() {
    let high = 1.0 + 2.0f64.powi(-40);
    let data = DataSet::new(
        "fine",
        vec![[-0.5, 0.0, 0.0], [0.5, 0.0, 0.0]],
        Cells::from_cells(&[(CellType::Vertex, &[0]), (CellType::Vertex, &[1])]),
    )
    .with_field(Field::scalar("p", vec![1.0, high]));
    let mut scene = scene(data, false);
    scene.actors[0].transfer = Some(TransferFunction::from_stops(
        [1.0, high],
        &[(0.0, [1.0, 0.0, 0.0, 1.0]), (1.0, [0.0, 0.0, 1.0, 1.0])],
    ));
    let frame = gpui_scene3d::render(&scene, &camera(0.0), SIZE, SIZE).unwrap();
    assert_eq!(near(&frame, 32, 48), vec![[255, 0, 0, 255]]);
    assert_eq!(near(&frame, 64, 48), vec![[0, 0, 255, 255]]);
    assert_eq!(
        scene.actors[0]
            .dataset
            .field(Association::Point, "p")
            .unwrap()
            .as_scalar()
            .unwrap()[1],
        high
    );
}

#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
#[test]
fn gpu_line_point_material_and_validity_contract() {
    use gpui_scene3d::{GpuRenderError, GpuRenderer};
    let renderer = match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
        Ok(v) => v,
        Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => {
            eprintln!("SKIP no GPU");
            return;
        }
        Err(e) => panic!("required GPU: {e}"),
    };
    let target = renderer.create_target(SIZE, SIZE).unwrap();
    exercise(|scene, camera| {
        let mut uploaded = renderer.upload(scene).unwrap();
        renderer.render(&mut uploaded, camera, &target).unwrap();
        renderer.read_rgba(&target).unwrap()
    });
}
