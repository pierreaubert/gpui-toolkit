//! Surface fragments checked against independent f64 ray/triangle interpolation.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{DataSet, Field};
use gpui_scene3d::{Actor, Camera, ClipPlane, Frame, Projection, Scene, TransferFunction, render};

const WIDTH: u32 = 96;
const HEIGHT: u32 = 80;
const BACKGROUND: [f32; 4] = [0.125, 0.25, 0.375, 0.5];

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn normalized(v: [f64; 3]) -> [f64; 3] {
    let length = dot(v, v).sqrt();
    v.map(|v| v / length)
}

struct Fixture {
    points: Vec<[f64; 3]>,
    indices: Vec<[u32; 3]>,
    scalars: Vec<f64>,
    normals: Vec<[f64; 3]>,
    clipped: bool,
    transparent: bool,
    flat: bool,
}

fn fixture(case: usize) -> Fixture {
    let (points, indices, scalars) = if case == 2 {
        (
            vec![
                [-1.0, -1.0, 0.0],
                [1.0, -1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, -1.0, 1.0],
            ],
            vec![[0, 1, 2], [1, 0, 3]],
            vec![0.0, 1.0, 0.25, 0.75],
        )
    } else {
        (
            vec![
                [-1.0, -1.0, if case == 1 { 2.5 } else { 0.0 }],
                [1.0, -1.0, 1.0],
                [0.0, 1.0, -1.0],
            ],
            vec![[0, 1, 2]],
            vec![0.0, 1.0, 0.25],
        )
    };
    // Area-weighted unit normals are derived independently from world edges.
    let mut normals = vec![[0.0; 3]; points.len()];
    for triangle in &indices {
        let [a, b, c] = triangle.map(|i| points[i as usize]);
        let n = cross(sub(b, a), sub(c, a));
        for &i in triangle {
            for axis in 0..3 {
                normals[i as usize][axis] += n[axis];
            }
        }
    }
    for n in &mut normals {
        *n = normalized(*n);
    }
    Fixture {
        points,
        indices,
        scalars,
        normals,
        clipped: case == 1,
        transparent: case == 3,
        flat: case == 2,
    }
}

fn scene_and_camera(f: &Fixture, projection: Projection, origin: f64) -> (Scene, Camera) {
    let data = DataSet::surface(
        "analytic",
        f.points
            .iter()
            .map(|p| p.map(|v| v + origin))
            .collect::<Vec<_>>(),
        &f.indices,
    )
    .with_field(Field::scalar("p", f.scalars.clone()));
    let mut actor = if f.flat {
        Actor::new(data).color([0.8, 0.6, 0.4, 1.0])
    } else {
        Actor::new(data).coloured_by(
            "p",
            TransferFunction::from_stops(
                [0.0, 1.0],
                &[
                    (0.0, [0.0, 0.0, 0.0, if f.transparent { 0.25 } else { 1.0 }]),
                    (1.0, [1.0, 1.0, 1.0, if f.transparent { 0.75 } else { 1.0 }]),
                ],
            ),
        )
    };
    if f.clipped {
        actor = actor.clip(ClipPlane {
            origin: [origin - 0.125, origin, origin],
            normal: [1.0, 0.0, 0.0],
        });
    }
    let mut scene = Scene::new().with_actor(actor);
    scene.background = BACKGROUND;
    let camera = Camera {
        target: [origin; 3],
        azimuth: 0.0,
        elevation: 0.0,
        distance: 3.0,
        projection,
        near: 1.0,
        far: 6.0,
    };
    (scene, camera)
}

fn intersection(
    origin: [f64; 3],
    direction: [f64; 3],
    points: [[f64; 3]; 3],
) -> Option<(f64, [f64; 3])> {
    let [a, b, c] = points;
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let p = cross(direction, e2);
    let determinant = dot(e1, p);
    if determinant.abs() < 1e-12 {
        return None;
    }
    let t = sub(origin, a);
    let u = dot(t, p) / determinant;
    let q = cross(t, e1);
    let v = dot(direction, q) / determinant;
    let distance = dot(e2, q) / determinant;
    if u < 0.0 || v < 0.0 || u + v > 1.0 || distance <= 0.0 {
        return None;
    }
    Some((distance, [1.0 - u - v, u, v]))
}

fn verify(frame: &Frame, f: &Fixture, projection: Projection, tolerance: u8) {
    let light = normalized([-0.4, 0.7, 0.6]);
    let aspect = f64::from(WIDTH) / f64::from(HEIGHT);
    let mut checked = 0;
    let mut minimum = 255;
    let mut maximum = 0;
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let screen = [
                (2.0 * (f64::from(x) + 0.5) / f64::from(WIDTH) - 1.0) * aspect,
                1.0 - 2.0 * (f64::from(y) + 0.5) / f64::from(HEIGHT),
            ];
            let (origin, direction) = match projection {
                Projection::Perspective { fov_y } => {
                    let tangent = (f64::from(fov_y) * 0.5).tan();
                    (
                        [0.0, 0.0, 3.0],
                        [screen[0] * tangent, screen[1] * tangent, -1.0],
                    )
                }
                Projection::Orthographic { half_height } => (
                    [
                        screen[0] * f64::from(half_height),
                        screen[1] * f64::from(half_height),
                        3.0,
                    ],
                    [0.0, 0.0, -1.0],
                ),
            };
            let hit = f
                .indices
                .iter()
                .filter_map(|nodes| {
                    intersection(origin, direction, nodes.map(|i| f.points[i as usize]))
                        .map(|hit| (hit, nodes))
                })
                .min_by(|a, b| a.0.0.total_cmp(&b.0.0));
            let Some(((distance, weights), nodes)) = hit else {
                continue;
            };
            let point: [f64; 3] = std::array::from_fn(|i| origin[i] + distance * direction[i]);
            // Stay clear of coverage and clipping boundaries: this tests fragment
            // attributes, not the backend-specific subpixel edge coverage rule.
            if weights.iter().any(|&v| v < 0.05)
                || !(1.05..5.95).contains(&(3.0 - point[2]))
                || (f.clipped && point[0] < -0.075)
            {
                continue;
            }
            let scalar: f64 = (0..3)
                .map(|i| weights[i] * f.scalars[nodes[i] as usize])
                .sum();
            let normal = normalized(std::array::from_fn(|axis| {
                (0..3)
                    .map(|i| weights[i] * f.normals[nodes[i] as usize][axis])
                    .sum()
            }));
            let intensity = 0.35 + 0.65 * dot(normal, light).abs();
            let alpha = if f.transparent {
                0.25 + 0.5 * scalar
            } else {
                1.0
            };
            let output_alpha = alpha + f64::from(BACKGROUND[3]) * (1.0 - alpha);
            let mut expected = [0u8; 4];
            for channel in 0..3 {
                let color = if f.flat {
                    [0.8, 0.6, 0.4][channel]
                } else {
                    scalar
                };
                expected[channel] = (((color * intensity * alpha
                    + f64::from(BACKGROUND[channel]) * f64::from(BACKGROUND[3]) * (1.0 - alpha))
                    / output_alpha)
                    * 255.0)
                    .round() as u8;
            }
            expected[3] = (output_alpha * 255.0).round() as u8;
            let actual = &frame.rgba[((y * WIDTH + x) * 4) as usize..][..4];
            for channel in 0..4 {
                assert!(
                    actual[channel].abs_diff(expected[channel]) <= tolerance,
                    "pixel {x},{y} channel {channel}: {actual:?} expected {expected:?}, scalar {scalar}, weights {weights:?}"
                );
            }
            minimum = minimum.min(actual[0]);
            maximum = maximum.max(actual[0]);
            checked += 1;
        }
    }
    assert!(checked > 100, "only {checked} interior samples");
    assert!(
        maximum - minimum > 20,
        "gradient missing: {minimum}..{maximum}"
    );
}

#[test]
fn software_fragments_match_ray_reference_through_perspective_clipping_and_smooth_normals() {
    for projection in [
        Projection::Perspective {
            fov_y: std::f32::consts::FRAC_PI_2,
        },
        Projection::Orthographic { half_height: 1.5 },
    ] {
        for case in 0..4 {
            let f = fixture(case);
            for origin in [0.0, 1e12 + 0.125] {
                let (scene, camera) = scene_and_camera(&f, projection, origin);
                verify(
                    &render(&scene, &camera, WIDTH, HEIGHT).unwrap(),
                    &f,
                    projection,
                    if f.transparent { 2 } else { 1 },
                );
            }
        }
    }
}

#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
#[test]
fn gpu_fragments_match_the_same_independent_reference() {
    use gpui_scene3d::gpu_render::{GpuRenderError, GpuRenderer};
    let renderer = match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
        Ok(renderer) => renderer,
        Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => {
            eprintln!("SKIP: no GPU adapter");
            return;
        }
        Err(error) => panic!("GPU required: {error}"),
    };
    let target = renderer.create_target(WIDTH, HEIGHT).unwrap();
    for projection in [
        Projection::Perspective {
            fov_y: std::f32::consts::FRAC_PI_2,
        },
        Projection::Orthographic { half_height: 1.5 },
    ] {
        for case in 0..4 {
            let f = fixture(case);
            let (scene, camera) = scene_and_camera(&f, projection, 1e12 + 0.125);
            let mut uploaded = renderer.upload(&scene).unwrap();
            renderer.render(&mut uploaded, &camera, &target).unwrap();
            verify(&renderer.read_rgba(&target).unwrap(), &f, projection, 3);
        }
    }
}
