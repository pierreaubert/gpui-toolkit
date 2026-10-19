//! Cell ownership oracles independent of shared-vertex preparation.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet, Field};
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

fn transfer() -> TransferFunction {
    TransferFunction::from_stops(
        [0.0, 1.0],
        &[(0.0, [1.0, 0.0, 0.0, 1.0]), (1.0, [0.0, 0.0, 1.0, 1.0])],
    )
}

fn pair(origin: f64) -> DataSet {
    DataSet::new(
        "pair",
        vec![
            [-1.0, -1.0, 0.0],
            [0.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [-1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ]
        .into_iter()
        .map(|p| p.map(|v| v + origin))
        .collect::<Vec<_>>(),
        Cells::from_cells(&[
            (CellType::Quad, &[0, 1, 4, 3]),
            (CellType::Quad, &[1, 2, 5, 4]),
        ]),
    )
    .with_field(Field::scalar("p", vec![0.0, 1.0]).on_cells())
}

fn colored(data: DataSet, opacity: f32, clip: bool) -> Scene {
    let mut actor = Actor::new(data)
        .coloured_by_cell("p", transfer())
        .opacity(opacity);
    if clip {
        actor = actor.clip(ClipPlane {
            origin: [0.2, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
        });
    }
    let mut scene = Scene::new().with_actor(actor);
    scene.background = [0.0; 4];
    scene
}

// Each reference cell is a separate, flat-colored actor. No shared scalar
// resolution or boundary ownership helper participates in this oracle.
fn reference(data: &DataSet, opacity: f32, clip: bool) -> Scene {
    let field = &data.cell_data[0];
    let mut scene = Scene::new();
    scene.background = [0.0; 4];
    for cell in 0..data.cell_count() {
        if !field.is_valid(cell) {
            continue;
        }
        let mut actor = Actor::new(DataSet::new(
            "independent-cell",
            data.positions.to_vec(),
            Cells::from_cells(&[(data.cells.types[cell], data.cells.cell(cell).unwrap())]),
        ))
        .color(transfer().map(field.as_scalar().unwrap()[cell]))
        .opacity(opacity);
        if clip {
            actor = actor.clip(ClipPlane {
                origin: [0.2, 0.0, 0.0],
                normal: [1.0, 0.0, 0.0],
            });
        }
        scene = scene.with_actor(actor);
    }
    scene
}

fn pixel(frame: &Frame, x: u32, y: u32) -> [u8; 4] {
    frame.rgba[(y * SIZE + x) as usize * 4..(y * SIZE + x + 1) as usize * 4]
        .try_into()
        .unwrap()
}

fn same(actual: &Frame, expected: &Frame, label: &str) {
    assert_eq!(actual.rgba.len(), expected.rgba.len());
    let mismatches = actual
        .rgba
        .iter()
        .zip(&expected.rgba)
        .filter(|(a, b)| a != b)
        .count();
    let first = actual
        .rgba
        .iter()
        .zip(&expected.rgba)
        .position(|(a, b)| a != b);
    assert_eq!(mismatches, 0, "{label}; first differing byte: {first:?}");
}

fn exercise(mut draw: impl FnMut(&Scene, &Camera) -> Frame) {
    for origin in [0.0, 1e12 + 0.125] {
        let data = pair(origin);
        for opacity in [1.0, 0.5] {
            let actual = draw(&colored(data.clone(), opacity, false), &camera(origin));
            let expected = draw(&reference(&data, opacity, false), &camera(origin));
            same(&actual, &expected, "cell values leaked across shared nodes");
            let left = pixel(&actual, 24, 48);
            let right = pixel(&actual, 72, 48);
            assert!(
                left[0] > 170 && left[2] == 0,
                "left cell changed scalar: {left:?}"
            );
            assert!(
                right[2] > 170 && right[0] == 0,
                "right cell changed scalar: {right:?}"
            );
        }
    }
    let data = pair(0.0);
    same(
        &draw(&colored(data.clone(), 1.0, true), &camera(0.0)),
        &draw(&reference(&data, 1.0, true), &camera(0.0)),
        "clipping changed cell values",
    );

    let volume = DataSet::structured_grid("volume", [3, 2, 2], [-1.0, -0.5, -0.5], [1.0; 3])
        .unwrap()
        .with_field(Field::scalar("p", vec![0.0, 1.0]).on_cells());
    let actual = draw(&colored(volume, 1.0, false), &camera(0.0));
    let left = pixel(&actual, 24, 48);
    let right = pixel(&actual, 72, 48);
    assert!(
        left[0] > 50 && left[2] == 0,
        "volume boundary lost left owner: {left:?}"
    );
    assert!(
        right[2] > 50 && right[0] == 0,
        "volume boundary lost right owner: {right:?}"
    );

    let mut wires = colored(pair(0.0), 1.0, false);
    wires.actors[0].wireframe = true;
    wires.actors[0].transfer = Some(TransferFunction::from_stops(
        [0.0, 1.0],
        &[(0.0, [1.0, 0.0, 0.0, 0.5]), (1.0, [0.0, 0.0, 1.0, 0.5])],
    ));
    let frame = draw(&wires, &camera(0.0));
    let alpha = (47..=49)
        .flat_map(|x| (39..=41).map(move |y| (x, y)))
        .map(|(x, y)| pixel(&frame, x, y)[3])
        .max()
        .unwrap();
    assert!(
        (188..=195).contains(&alpha),
        "shared wire alpha composed more than once: {alpha}"
    );
}

fn missing_cell(mut draw: impl FnMut(&Scene, &Camera) -> Frame) {
    // Every center-cell vertex also belongs to valid neighbors. Averaged point
    // values cannot represent the hole; test it independently of color seams.
    let mut positions = Vec::new();
    for y in 0..4 {
        for x in 0..4 {
            positions.push([
                (f64::from(x) - 1.5) * 0.75,
                (f64::from(y) - 1.5) * 0.75,
                0.0,
            ]);
        }
    }
    let mut nodes = Vec::new();
    for y in 0..3u32 {
        for x in 0..3u32 {
            let i = y * 4 + x;
            nodes.push([i, i + 1, i + 5, i + 4]);
        }
    }
    let cells = nodes
        .iter()
        .map(|n| (CellType::Quad, &n[..]))
        .collect::<Vec<_>>();
    let mut values = vec![1.0; 9];
    values[4] = 999.0;
    let mut field = Field::scalar("p", values).on_cells();
    let mut mask = vec![true; 9];
    mask[4] = false;
    field.valid = Some(mask.into());
    let data = DataSet::new("hole", positions, Cells::from_cells(&cells)).with_field(field);
    let actual = draw(&colored(data.clone(), 1.0, false), &camera(0.0));
    assert_eq!(
        pixel(&actual, 48, 48),
        [0; 4],
        "masked cell filled from valid neighbors"
    );
    same(
        &actual,
        &draw(&reference(&data, 1.0, false), &camera(0.0)),
        "masked cell changes other cells",
    );
}

#[test]
fn software_preserves_cell_ownership_and_missing_cells() {
    exercise(|scene, camera| gpui_scene3d::render(scene, camera, SIZE, SIZE).unwrap());
}

#[test]
fn software_masked_cell_cannot_be_recovered_from_neighbors() {
    missing_cell(|scene, camera| gpui_scene3d::render(scene, camera, SIZE, SIZE).unwrap());
}

#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
#[test]
fn gpu_preserves_cell_ownership_and_missing_cells() {
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
    missing_cell(|scene, camera| {
        let mut uploaded = renderer.upload(scene).unwrap();
        renderer.render(&mut uploaded, camera, &target).unwrap();
        renderer.read_rgba(&target).unwrap()
    });
}
