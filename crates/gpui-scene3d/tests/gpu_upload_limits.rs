//! Primitive staging partitions must preserve rendered pixels.

#![cfg(all(feature = "gpu", not(target_family = "wasm")))]

// Rust guideline compliant 2026-02-21

use gpui_mesh::{CellType, Cells, DataSet, Field};
use gpui_scene3d::{Actor, Camera, GpuRenderError, GpuRenderer, Scene, TransferFunction};

#[test]
fn tiny_primitive_staging_chunks_preserve_point_line_and_surface_images() {
    let renderer = match GpuRenderer::new(wgpu::TextureFormat::Rgba8Unorm) {
        Ok(renderer) => renderer,
        Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => return,
        Err(error) => panic!("required GPU: {error}"),
    };
    let mut scene = Scene::new();
    for (kind, arity) in [(CellType::Line, 2usize), (CellType::Vertex, 1)] {
        let positions: Vec<_> = (0..18)
            .map(|i| {
                [
                    -0.9 + f64::from(i % 6) * 0.3,
                    -0.6 + f64::from(i / 6) * 0.6,
                    0.0,
                ]
            })
            .collect();
        let cell_nodes: Vec<_> = (0..18 / arity)
            .map(|i| {
                (0..arity)
                    .map(|j| (i * arity + j) as u32)
                    .collect::<Vec<_>>()
            })
            .collect();
        let cells: Vec<_> = cell_nodes
            .iter()
            .map(|nodes| (kind, nodes.as_slice()))
            .collect();
        let data = DataSet::new("primitives", positions, Cells::from_cells(&cells))
            .with_field(Field::scalar("p", vec![0.5; 18]));
        scene.actors.push(
            Actor::new(data)
                .coloured_by("p", TransferFunction::viridis([0.0, 1.0]))
                .opacity(0.5),
        );
    }
    scene.actors.push(
        Actor::new(DataSet::surface(
            "surface",
            vec![[-0.7, -0.5, -0.5], [0.7, -0.5, -0.5], [0.0, 0.8, -0.5]],
            &[[0, 1, 2]],
        ))
        .color([1.0, 0.0, 0.0, 1.0]),
    );
    let camera = Camera::fitted(scene.bounds().unwrap());
    let target = renderer.create_target(96, 64).unwrap();
    let mut uploaded = renderer.upload(&scene).unwrap();
    let expected_bytes = uploaded.resource_bytes();
    renderer.render(&mut uploaded, &camera, &target).unwrap();
    let expected = renderer.read_rgba(&target).unwrap();
    scene.gpu_upload_limits.max_primitive_staging_bytes = 32;
    let mut partitioned = renderer.upload(&scene).unwrap();
    assert_eq!(partitioned.resource_bytes(), expected_bytes);
    assert_eq!(partitioned.geometry_bytes, uploaded.geometry_bytes);
    renderer.render(&mut partitioned, &camera, &target).unwrap();
    assert_eq!(renderer.read_rgba(&target).unwrap().rgba, expected.rgba);
}
