//! Singleton-dimension grids must render their explicit cell topology.

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet, Field};
use gpui_scene3d::{Actor, Camera, Frame, Projection, Scene, TransferFunction};

const SIZE: u32 = 64;

fn exercise(mut draw: impl FnMut(&Scene, &Camera) -> Frame) {
    for dims in [
        [3, 3, 1],
        [3, 1, 3],
        [1, 3, 3],
        [3, 1, 1],
        [1, 3, 1],
        [1, 1, 3],
        [1, 1, 1],
    ] {
        let data = DataSet::structured_grid("generated", dims, [0.0; 3], [1.0; 3]).unwrap();
        let dimension = dims.iter().filter(|&&n| n > 1).count();
        let cells = match dimension {
            0 => Cells::from_cells(&[(CellType::Vertex, &[0])]),
            1 => Cells::lines(&[[0, 1], [1, 2]]),
            2 => Cells::from_cells(&[
                (CellType::Quad, &[0, 1, 4, 3]),
                (CellType::Quad, &[1, 2, 5, 4]),
                (CellType::Quad, &[3, 4, 7, 6]),
                (CellType::Quad, &[4, 5, 8, 7]),
            ]),
            _ => unreachable!(),
        };
        let reference = DataSet::new("explicit", data.positions.clone(), cells);
        let make = |data: DataSet| {
            let field = Field::scalar("p", vec![0.5; data.point_count()]);
            let mut scene = Scene::new().with_actor(
                Actor::new(data.with_field(field))
                    .coloured_by("p", TransferFunction::viridis([0.0, 1.0])),
            );
            scene.background = [0.0; 4];
            scene
        };
        let camera = Camera {
            target: data.bounds().unwrap().center(),
            azimuth: 0.7,
            elevation: 0.4,
            distance: 5.0,
            projection: Projection::Orthographic { half_height: 2.5 },
            near: 0.1,
            far: 20.0,
        };
        let actual = draw(&make(data), &camera);
        let expected = draw(&make(reference), &camera);
        assert_eq!(actual.rgba.len(), expected.rgba.len());
        let differences = actual
            .rgba
            .iter()
            .zip(&expected.rgba)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            differences, 0,
            "grid topology rendered differently from explicit cells: {dims:?}"
        );
        assert!(
            actual.rgba.as_chunks::<4>().0.iter().any(|p| p[3] > 0),
            "grid disappeared: {dims:?}"
        );
    }
}

#[test]
fn software_renders_grid_planes_lines_and_vertices() {
    exercise(|scene, camera| gpui_scene3d::render(scene, camera, SIZE, SIZE).unwrap());
}

#[cfg(all(feature = "gpu", not(target_family = "wasm")))]
#[test]
fn gpu_renders_grid_planes_lines_and_vertices() {
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
