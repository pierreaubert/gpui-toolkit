//! GPU camera coefficients must fail before encoding invalid uniforms.

#![cfg(all(feature = "gpu", not(target_family = "wasm")))]

// Rust guideline compliant 2026-02-21

use gpui_mesh::DataSet;
use gpui_scene3d::{Actor, Camera, GpuRenderError, GpuRenderer, Projection, Scene, SceneError};

fn renderer() -> Option<(GpuRenderer, std::sync::Arc<wgpu::Queue>)> {
    let result = pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                ..Default::default()
            })
            .await
            .map_err(|_| GpuRenderError::NoAdapter)?;
        let (device, queue) = adapter
            .request_device(&Default::default())
            .await
            .map_err(|error| GpuRenderError::Device(error.to_string()))?;
        let queue = std::sync::Arc::new(queue);
        let renderer = GpuRenderer::from_shared_device(
            std::sync::Arc::new(device),
            queue.clone(),
            wgpu::TextureFormat::Rgba8Unorm,
        )?;
        Ok((renderer, queue))
    });
    match result {
        Ok(result) => Some(result),
        Err(GpuRenderError::NoAdapter) if std::env::var_os("GPUI_REQUIRE_GPU").is_none() => None,
        Err(error) => panic!("required GPU: {error}"),
    }
}

#[test]
fn relative_camera_overflow_is_rejected_without_changing_encoded_or_resident_frames() {
    let Some((renderer, queue)) = renderer() else {
        return;
    };
    let scene = Scene::new().with_actor(Actor::new(
        DataSet::structured_grid("camera", [2; 3], [-0.5; 3], [1.0; 3]).unwrap(),
    ));
    let camera = Camera::fitted(scene.bounds().unwrap());
    let target = renderer.create_target(48, 48).unwrap();
    let mut uploaded = renderer.upload(&scene).unwrap();
    renderer.render(&mut uploaded, &camera, &target).unwrap();
    let expected = renderer.read_rgba(&target).unwrap();
    let background = scene.background.map(|value| (value * 255.0) as u8);
    assert!(
        expected
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel != &background),
        "the preserved reference must contain actual scene pixels"
    );
    let bad = Camera {
        target: [1e100; 3],
        ..camera
    };
    bad.validate().unwrap(); // Valid parameters; only the actor-relative GPU matrix fails.
    assert!(
        bad.view_projection_relative_to([0.0; 3], 1.0)
            .iter()
            .any(|v| !v.is_finite())
    );
    assert!(matches!(
        renderer.render(&mut uploaded, &bad, &target),
        Err(GpuRenderError::Scene(SceneError::UnsupportedPrecision(_)))
    ));
    assert_eq!(renderer.read_rgba(&target).unwrap().rgba, expected.rgba);
    let mut encoder = renderer
        .device()
        .create_command_encoder(&Default::default());
    renderer
        .encode(&mut encoder, &mut uploaded, &camera, &target)
        .unwrap();
    assert!(
        renderer
            .encode(&mut encoder, &mut uploaded, &bad, &target)
            .is_err()
    );
    let mut mixed = scene.clone();
    mixed.actors.push(Actor::new(DataSet::new(
        "far point",
        vec![[1e100; 3]],
        gpui_mesh::Cells::from_cells(&[(gpui_mesh::CellType::Vertex, &[0])]),
    )));
    let mut mixed_upload = renderer.upload(&mixed).unwrap();
    assert!(
        matches!(
            renderer.encode(&mut encoder, &mut mixed_upload, &camera, &target),
            Err(GpuRenderError::Scene(SceneError::UnsupportedPrecision(_)))
        ),
        "a valid first actor must not hide a later invalid matrix"
    );
    queue.submit([encoder.finish()]);
    assert_eq!(
        renderer.read_rgba(&target).unwrap().rgba,
        expected.rgba,
        "rejection must leave the preceding encoded pass intact"
    );
    renderer.render(&mut uploaded, &camera, &target).unwrap();
    assert_eq!(renderer.read_rgba(&target).unwrap().rgba, expected.rgba);
}

#[test]
fn viewport_projection_collapse_is_rejected_before_the_render_target_changes() {
    let Some((renderer, _queue)) = renderer() else {
        return;
    };
    let scene = Scene::new().with_actor(Actor::new(DataSet::surface(
        "surface",
        vec![[-0.5, -0.5, 0.0], [0.5, -0.5, 0.0], [0.0, 0.5, 0.0]],
        &[[0, 1, 2]],
    )));
    let target = renderer.create_target(4096, 1).unwrap();
    let mut uploaded = renderer.upload(&scene).unwrap();
    let camera = Camera::fitted(scene.bounds().unwrap());
    renderer.render(&mut uploaded, &camera, &target).unwrap();
    let expected = renderer.read_rgba(&target).unwrap();
    let bad = Camera {
        projection: Projection::Orthographic { half_height: 1e36 },
        ..camera
    };
    bad.validate().unwrap();
    assert!(bad.projection_matrix(1.0)[0].is_normal());
    assert_eq!(bad.projection_matrix(4096.0)[0], 0.0);
    assert!(matches!(
        renderer.render(&mut uploaded, &bad, &target),
        Err(GpuRenderError::Scene(SceneError::UnsupportedPrecision(_)))
    ));
    assert_eq!(renderer.read_rgba(&target).unwrap().rgba, expected.rgba);
    let tiny = renderer.create_target(1, 2).unwrap();
    let bad = Camera {
        projection: Projection::Perspective { fov_y: 1e-38 },
        ..camera
    };
    bad.validate().unwrap();
    assert!(bad.projection_matrix(0.5)[0].is_infinite());
    assert!(matches!(
        renderer.render(&mut uploaded, &bad, &tiny),
        Err(GpuRenderError::Scene(SceneError::UnsupportedPrecision(_)))
    ));
}

#[test]
fn orthographic_near_bias_underflow_cannot_leak_a_point_behind_the_near_plane() {
    let Some((renderer, _queue)) = renderer() else {
        return;
    };
    let data = DataSet::new(
        "behind near",
        vec![[0.0; 3]],
        gpui_mesh::Cells::from_cells(&[(gpui_mesh::CellType::Vertex, &[0])]),
    );
    let mut scene = Scene::new().with_actor(Actor::new(data).color([1.0, 0.0, 0.0, 1.0]));
    scene.background = [0.0, 0.0, 0.0, 1.0];
    let camera = Camera {
        distance: 0.0,
        near: 1e-38,
        far: 1e36,
        projection: Projection::Orthographic { half_height: 1.0 },
        ..Default::default()
    };
    camera.validate().unwrap();
    assert_eq!(camera.projection_matrix(1.0)[14], 0.0);
    let cpu = gpui_scene3d::render(&scene, &camera, 32, 32).unwrap();
    let cpu_foreground = cpu
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| **p != [0, 0, 0, 255])
        .count();
    let target = renderer.create_target(32, 32).unwrap();
    let mut uploaded = renderer.upload(&scene).unwrap();
    match renderer.render(&mut uploaded, &camera, &target) {
        Err(GpuRenderError::Scene(SceneError::UnsupportedPrecision(_))) => {
            assert_eq!(cpu_foreground, 0);
        }
        Err(error) => panic!("unexpected rendering error: {error}"),
        Ok(()) => {
            let gpu = renderer.read_rgba(&target).unwrap();
            let gpu_foreground = gpu
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| **p != [0, 0, 0, 255])
                .count();
            assert_eq!(
                (cpu_foreground, gpu_foreground),
                (0, 0),
                "collapsed near offset leaked a behind-near point in software/GPU"
            );
            panic!("a required projection coefficient collapsed without a precision error");
        }
    }
}
