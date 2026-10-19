//! Wire overlays checked against projected edges, alpha and opaque occlusion.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{DataSet, Field};
use gpui_scene3d::{Actor, Camera, ClipPlane, Frame, Projection, Scene, TransferFunction};

const SIZE: u32 = 96;

fn quad(radius: f64, z: f64, origin: f64) -> DataSet {
    DataSet::surface(
        "quad",
        vec![
            [-radius + origin, -radius + origin, z + origin],
            [radius + origin, -radius + origin, z + origin],
            [radius + origin, radius + origin, z + origin],
            [-radius + origin, radius + origin, z + origin],
        ],
        &[[0, 1, 2], [0, 2, 3]],
    )
}

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

fn scene(origin: f64, opacity: f32, variable_alpha: bool, clipped: bool) -> Scene {
    let data = quad(1.0, 0.0, origin).with_field(Field::scalar("p", vec![0.0, 1.0, 1.0, 0.0]));
    let actor = if variable_alpha {
        Actor::new(data).coloured_by(
            "p",
            TransferFunction::from_stops(
                [0.0, 1.0],
                &[(0.0, [1.0, 1.0, 1.0, 0.25]), (1.0, [1.0, 1.0, 1.0, 0.75])],
            ),
        )
    } else {
        Actor::new(data).color([1.0; 4]).opacity(opacity)
    };
    let actor = if clipped {
        actor.clip(ClipPlane {
            origin: [origin; 3],
            normal: [1.0, 0.0, 0.0],
        })
    } else {
        actor
    };
    let mut scene = Scene::new().with_actor(actor.wireframe(true));
    scene.background = [0.0; 4];
    scene
}

fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let t = (((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / (d[0] * d[0] + d[1] * d[1]))
        .clamp(0.0, 1.0);
    (p[0] - a[0] - t * d[0]).hypot(p[1] - a[1] - t * d[1])
}

fn check_opaque_edges(wire: &Frame, plain: &Frame, clipped: bool) {
    // The independent orthographic projection maps the quad to 16..80 pixels.
    let points = [[16.0, 80.0], [80.0, 80.0], [80.0, 16.0], [16.0, 16.0]];
    let edges = [[0, 1], [1, 2], [2, 3], [0, 3], [0, 2]];
    let mut visible = 0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            if wire.rgba[i + 3] > 0
                && wire.rgba[i] < 10
                && (plain.rgba[i] > 40 || plain.rgba[i + 3] == 0)
            {
                let p = [f64::from(x) + 0.5, f64::from(y) + 0.5];
                assert!(
                    edges
                        .iter()
                        .any(|&[a, b]| segment_distance(p, points[a], points[b]) <= 1.5),
                    "wire outside projected edges at {x},{y}"
                );
                if clipped {
                    assert!(p[0] >= 47.5, "wire outside user plane at {p:?}");
                }
                visible += 1;
            }
        }
    }
    assert!(visible > 80, "overlay missing: {visible} edge pixels");
    assert!(
        wire.rgba[((32 * SIZE + 56) * 4) as usize] > 100,
        "wire replaced surface interior"
    );
}

fn check_diagonal_alpha(wire: &Frame, plain: &Frame, variable: bool) {
    let light_z = 0.6f64 / (0.4f64.powi(2) + 0.7f64.powi(2) + 0.6f64.powi(2)).sqrt();
    let intensity = 0.35 + 0.65 * light_z;
    let mut compared = 0;
    for y in 25..71 {
        for x in 25..71 {
            let i = ((y * SIZE + x) * 4) as usize;
            if wire.rgba[i + 3] <= plain.rgba[i + 3].saturating_add(15) {
                continue;
            }
            let scalar = (f64::from(x) + 0.5 - 16.0) / 64.0;
            let alpha = if variable { 0.25 + 0.5 * scalar } else { 0.5 };
            // One black edge over one translucent surface: a + a(1-a).
            // Drawing the shared edge twice would produce visibly different alpha.
            let expected_alpha = ((2.0 * alpha - alpha * alpha) * 255.0).round() as u8;
            let expected_gray = (intensity * (1.0 - alpha) / (2.0 - alpha) * 255.0).round() as u8;
            assert!(
                wire.rgba[i + 3].abs_diff(expected_alpha) <= 3,
                "edge alpha {} expected {expected_alpha} at {x},{y}",
                wire.rgba[i + 3]
            );
            for channel in 0..3 {
                assert!(
                    wire.rgba[i + channel].abs_diff(expected_gray) <= 3,
                    "edge gray {} expected {expected_gray}",
                    wire.rgba[i + channel]
                );
            }
            compared += 1;
        }
    }
    assert!(compared > 20, "diagonal overlay absent: {compared}");
}

fn exercise(mut draw: impl FnMut(&Scene, &Camera) -> Frame) {
    for origin in [0.0, 1e12 + 0.125] {
        for clipped in [false, true] {
            let wire = scene(origin, 1.0, false, clipped);
            let mut plain = wire.clone();
            plain.actors[0].wireframe = false;
            check_opaque_edges(
                &draw(&wire, &camera(origin)),
                &draw(&plain, &camera(origin)),
                clipped,
            );
        }
    }
    for variable in [false, true] {
        let wire = scene(0.0, 0.5, variable, false);
        let mut plain = wire.clone();
        plain.actors[0].wireframe = false;
        check_diagonal_alpha(
            &draw(&wire, &camera(0.0)),
            &draw(&plain, &camera(0.0)),
            variable,
        );
    }
    for opacity in [0.5, 1.0] {
        for front_z in [1.0, 0.00001] {
            for projection in [
                Projection::Orthographic { half_height: 1.5 },
                Projection::Perspective {
                    fov_y: std::f32::consts::FRAC_PI_2,
                },
            ] {
                let mut wire = scene(0.0, opacity, false, false);
                wire.actors
                    .push(Actor::new(quad(1.25, front_z, 0.0)).color([1.0, 0.0, 0.0, 1.0]));
                let mut front = wire.clone();
                front.actors.remove(0);
                let view = Camera {
                    projection,
                    ..camera(0.0)
                };
                let expected = draw(&front, &view);
                assert_eq!(
                    draw(&wire, &view)
                        .rgba
                        .iter()
                        .zip(&expected.rgba)
                        .filter(|(a, b)| a != b)
                        .count(),
                    0,
                    "hidden wire leaked through front surface"
                );
                wire.actors.reverse();
                assert_eq!(
                    draw(&wire, &view)
                        .rgba
                        .iter()
                        .zip(&expected.rgba)
                        .filter(|(a, b)| a != b)
                        .count(),
                    0,
                    "occlusion depends on actor order"
                );
            }
        }
    }
    let mut missing = scene(0.0, 1.0, true, false);
    missing.actors[0].dataset.point_data[0].valid = Some(vec![false; 4].into());
    assert!(
        draw(&missing, &camera(0.0)).rgba.iter().all(|&v| v == 0),
        "invalid field created wire edges"
    );
    let data = DataSet::surface(
        "near",
        vec![[-1.0, -0.5, 2.5], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]],
        &[[0, 1, 2]],
    );
    let data = data.with_field(Field::scalar("p", vec![0.0, 1.0, 0.5]));
    let mut near_scene = Scene::new().with_actor(Actor::new(data).color([1.0; 4]).wireframe(true));
    near_scene.background = [0.0; 4];
    let mut wire = near_scene.clone();
    wire.background = [0.0; 4];
    let mut plain = wire.clone();
    plain.actors[0].wireframe = false;
    let camera = Camera {
        projection: Projection::Perspective {
            fov_y: std::f32::consts::FRAC_PI_2,
        },
        ..camera(0.0)
    };
    let (wire, plain) = (draw(&wire, &camera), draw(&plain, &camera));
    // Intersect z=2 (distance 1 from the eye) analytically on the two
    // crossing edges, then project. The generated cut edge is not a wire edge.
    let edges = [
        ([19.2, 76.8], [64.0, 64.0]),
        ([9.6, 57.6], [48.0, 32.0]),
        ([64.0, 64.0], [48.0, 32.0]),
    ];
    let mut visible = 0;
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            if wire.rgba[i + 3] > 0
                && wire.rgba[i] < 10
                && (plain.rgba[i] > 40 || plain.rgba[i + 3] == 0)
            {
                let p = [f64::from(x) + 0.5, f64::from(y) + 0.5];
                assert!(
                    edges.iter().any(|&(a, b)| segment_distance(p, a, b) <= 1.5),
                    "incorrect near-clipped edge at {p:?}"
                );
                visible += 1;
            }
        }
    }
    assert!(visible > 40, "near-clipped wires disappeared: {visible}");
    for (a, b) in edges {
        for t in [0.25, 0.5, 0.75] {
            let point = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
            let mut found = false;
            for y in 0..SIZE {
                for x in 0..SIZE {
                    if (f64::from(x) + 0.5 - point[0]).hypot(f64::from(y) + 0.5 - point[1]) > 1.8 {
                        continue;
                    }
                    let i = ((y * SIZE + x) * 4) as usize;
                    found |= wire.rgba[i + 3] > 0 && wire.rgba[i] < 10;
                }
            }
            assert!(found, "near-clipped edge missing around {point:?}");
        }
    }
    near_scene.actors[0] = near_scene.actors[0].clone().coloured_by(
        "p",
        TransferFunction::from_stops(
            [0.0, 1.0],
            &[(0.0, [1.0, 1.0, 1.0, 0.25]), (1.0, [1.0, 1.0, 1.0, 0.75])],
        ),
    );
    let mut plain = near_scene.clone();
    plain.actors[0].wireframe = false;
    let (wire, plain) = (draw(&near_scene, &camera), draw(&plain, &camera));
    let mut compared = [0usize; 3];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            if wire.rgba[i + 3] <= plain.rgba[i + 3].saturating_add(5) {
                continue;
            }
            let p = [f64::from(x) + 0.5, f64::from(y) + 0.5];
            let (edge, _) = edges
                .iter()
                .enumerate()
                .map(|(i, &(a, b))| (i, segment_distance(p, a, b)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            let (a, b) = edges[edge];
            let delta = [b[0] - a[0], b[1] - a[1]];
            let t = ((p[0] - a[0]) * delta[0] + (p[1] - a[1]) * delta[1])
                / (delta[0] * delta[0] + delta[1] * delta[1]);
            if !(0.08..0.92).contains(&t) {
                continue;
            }
            // Near clipping gives scalar/clip-w endpoints ([.2,1],[1,3]),
            // ([.1,.5],[1,3]), and ([1,.5],[3,3]). Projected t must be
            // corrected by reciprocal clip w before alpha is evaluated.
            let (scalars, ws) = match edge {
                0 => ([0.2, 1.0], [1.0, 3.0]),
                1 => ([0.1, 0.5], [1.0, 3.0]),
                _ => ([1.0, 0.5], [3.0, 3.0]),
            };
            let q = [(1.0 - t) / ws[0], t / ws[1]];
            let scalar = (q[0] * scalars[0] + q[1] * scalars[1]) / (q[0] + q[1]);
            let alpha = 0.25 + 0.5 * scalar;
            let destination = f64::from(plain.rgba[i + 3]) / 255.0;
            let expected = ((alpha + destination * (1.0 - alpha)) * 255.0).round() as u8;
            assert!(
                wire.rgba[i + 3].abs_diff(expected) <= 3,
                "perspective edge alpha {} expected {expected} at {p:?}",
                wire.rgba[i + 3]
            );
            compared[edge] += 1;
        }
    }
    assert!(
        compared.iter().all(|&count| count > 5),
        "perspective alpha edges missing: {compared:?}"
    );
}

#[test]
fn software_wireframe_matches_edge_alpha_clipping_and_occlusion_contract() {
    exercise(|scene, camera| gpui_scene3d::render(scene, camera, SIZE, SIZE).unwrap());
}

#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
#[test]
fn gpu_wireframe_matches_edge_alpha_clipping_and_occlusion_contract() {
    use gpui_scene3d::{GpuRenderError, GpuRenderer};
    let renderer = match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
        Ok(renderer) => renderer,
        Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => {
            eprintln!("SKIP: no GPU adapter");
            return;
        }
        Err(error) => panic!("GPU required: {error}"),
    };
    let target = renderer.create_target(SIZE, SIZE).unwrap();
    exercise(|scene, camera| {
        let mut uploaded = renderer.upload(scene).unwrap();
        renderer.render(&mut uploaded, camera, &target).unwrap();
        renderer.read_rgba(&target).unwrap()
    });
}
