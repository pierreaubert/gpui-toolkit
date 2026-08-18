#![cfg(feature = "gpu-3d")]

use d3rs::gpu3d::{Camera3D, OrbitControls};
use d3rs::mesh::gpu::{
    FieldRevision, GeometryRevision, MeshColorConfig, MeshGpuRenderer, MeshLodController,
    MeshSceneState, WgpuMesh3DRenderer, WgpuMeshRenderer, render_offscreen_wgpu,
    render_offscreen_wgpu_with_camera,
};
use d3rs::mesh::{
    MeshBounds, MeshBvh, MeshTopology, ScalarAssociation, ScalarField, TriangleMesh, prepare_upload,
};
use gpui::{Bounds, Point, Size, lookup_custom_draw, px, unregister_custom_draw};
use gpui_wgpu::{WgpuContext, WgpuCustomDrawAdapter};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

const TARGET_SIZE: [u32; 2] = [128, 96];

fn headless_context() -> Option<WgpuContext> {
    match WgpuContext::headless() {
        Ok(context) => Some(context),
        Err(error) => {
            if std::env::var_os("QA_WGPU_REQUIRED").is_some_and(|value| value == "1") {
                panic!("required WGPU adapter is unavailable: {error:#}");
            }
            eprintln!("SKIP WGPU adapter-backed MeshPlot test: {error:#}");
            None
        }
    }
}

#[test]
fn dropping_retained_renderer_unregisters_its_custom_draw() {
    let id;
    {
        let renderer = WgpuMesh3DRenderer::new(Rc::new(RefCell::new(MeshSceneState::default())));
        id = renderer.custom_id();
        assert!(lookup_custom_draw(id).is_some());
    }
    assert!(lookup_custom_draw(id).is_none());
}

#[test]
fn dropping_retained_2d_renderer_unregisters_its_custom_draw() {
    let id;
    {
        let renderer = WgpuMeshRenderer::new(Rc::new(RefCell::new(MeshSceneState::default())));
        id = renderer.custom_id();
        assert!(lookup_custom_draw(id).is_some());
    }
    assert!(lookup_custom_draw(id).is_none());
}

fn square_upload() -> d3rs::mesh::MeshUpload {
    let mesh = TriangleMesh {
        id: "embedded-wgpu".into(),
        positions: Arc::from([
            [-1.0, -1.0, 0.0],
            [1.0, -1.0, 0.0],
            [1.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0],
        ]),
        triangles: Arc::from([[0, 1, 2], [0, 2, 3]]),
        vertex_ids: None,
        cell_ids: None,
    };
    prepare_upload(&mesh, &MeshTopology::build(&mesh.triangles))
}

fn connected_grid_upload(side: u32) -> d3rs::mesh::MeshUpload {
    let vertex_count = (side + 1) * (side + 1);
    let mut positions = Vec::with_capacity(vertex_count as usize);
    for y in 0..=side {
        for x in 0..=side {
            positions.push([
                -1.0 + 2.0 * f64::from(x) / f64::from(side),
                -1.0 + 2.0 * f64::from(y) / f64::from(side),
                0.0,
            ]);
        }
    }

    let mut triangles = Vec::with_capacity((side * side * 2) as usize);
    for y in 0..side {
        for x in 0..side {
            let top_left = y * (side + 1) + x;
            let top_right = top_left + 1;
            let bottom_left = top_left + side + 1;
            let bottom_right = bottom_left + 1;
            triangles.push([top_left, top_right, bottom_right]);
            triangles.push([top_left, bottom_right, bottom_left]);
        }
    }

    let mesh = TriangleMesh {
        id: "embedded-wgpu-connected-grid".into(),
        positions: Arc::from(positions),
        triangles: Arc::from(triangles),
        vertex_ids: None,
        cell_ids: None,
    };
    let mut upload = prepare_upload(&mesh, &MeshTopology::build(&mesh.triangles));
    upload.values_f32 = Some(
        (0..vertex_count)
            .map(|index| index as f32 / vertex_count.max(1) as f32)
            .collect(),
    );
    upload
}

fn connected_grid_mesh(side: u32) -> TriangleMesh {
    let vertex_count = (side + 1) * (side + 1);
    let positions = (0..=side)
        .flat_map(|y| {
            (0..=side).map(move |x| {
                [
                    -1.0 + 2.0 * f64::from(x) / f64::from(side),
                    -1.0 + 2.0 * f64::from(y) / f64::from(side),
                    0.0,
                ]
            })
        })
        .collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity((side * side * 2) as usize);
    for y in 0..side {
        for x in 0..side {
            let top_left = y * (side + 1) + x;
            let top_right = top_left + 1;
            let bottom_left = top_left + side + 1;
            let bottom_right = bottom_left + 1;
            triangles.push([top_left, top_right, bottom_right]);
            triangles.push([top_left, bottom_right, bottom_left]);
        }
    }
    assert_eq!(positions.len(), vertex_count as usize);
    TriangleMesh {
        id: "embedded-wgpu-lod-grid".into(),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn draw_adapter_frame(
    ctx: &WgpuContext,
    draw: &WgpuCustomDrawAdapter,
    target: &wgpu::TextureView,
    size: [u32; 2],
) {
    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mesh_3d_field_update_encoder"),
        });
    draw_2d_clear(ctx, target, &mut encoder);
    draw.0.draw_wgpu(
        ctx,
        &mut encoder,
        target,
        ctx.color_texture_format(),
        size,
        Bounds::new(
            Point::new(px(0.0), px(0.0)),
            Size::new(px(size[0] as f32), px(size[1] as f32)),
        ),
        Bounds::new(
            Point::new(px(0.0), px(0.0)),
            Size::new(px(size[0] as f32), px(size[1] as f32)),
        ),
        1.0,
    );
    ctx.queue.submit(std::iter::once(encoder.finish()));
}

fn draw_2d_clear(
    ctx: &WgpuContext,
    target: &wgpu::TextureView,
    encoder: &mut wgpu::CommandEncoder,
) {
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("mesh_3d_field_update_clear"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let _ = ctx;
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn offscreen_wgpu_readback_contains_retained_scalar_mesh() {
    let mut upload = square_upload();
    upload.values_f32 = Some(vec![0.0, 0.5, 1.0, 0.25]);

    let state = MeshSceneState {
        geometry_rev: GeometryRevision(1),
        upload: Some(upload),
        color: MeshColorConfig {
            range: [0.0, 1.0],
            unlit: true,
            ..MeshColorConfig::default()
        },
        ..MeshSceneState::default()
    };

    let image = match render_offscreen_wgpu(&state, 64, 64) {
        Ok(image) => image,
        Err(error) if error.starts_with("Failed to request a headless GPU adapter") => return,
        Err(error) => panic!("WGPU offscreen readback failed: {error}"),
    };

    assert_eq!(image.dimensions(), (64, 64));
    let opaque_pixels = image
        .pixels()
        .filter(|pixel| pixel.0[3] != 0)
        .collect::<Vec<_>>();
    assert!(
        opaque_pixels.len() > 32,
        "retained mesh should render opaque pixels, got {}",
        opaque_pixels.len()
    );
    let scalar_colors = opaque_pixels
        .iter()
        .map(|pixel| [pixel.0[0], pixel.0[1], pixel.0[2]])
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        scalar_colors.len() >= 2,
        "scalar field should produce more than one RGB color"
    );
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn offscreen_wgpu_explicit_camera_changes_projection_without_mutating_state() {
    let mut upload = square_upload();
    upload.values_f32 = Some(vec![0.0, 0.5, 1.0, 0.25]);
    let state = MeshSceneState {
        geometry_rev: GeometryRevision(1),
        upload: Some(upload),
        color: MeshColorConfig {
            range: [0.0, 1.0],
            unlit: true,
            ..MeshColorConfig::default()
        },
        ..MeshSceneState::default()
    };
    let original_transform = state.view_transform;
    let default_image = match render_offscreen_wgpu(&state, 96, 72) {
        Ok(image) => image,
        Err(error) if error.starts_with("Failed to request a headless GPU adapter") => return,
        Err(error) => panic!("default WGPU readback failed: {error}"),
    };
    let camera = Camera3D::default()
        .with_position(glam::Vec3::new(1.25, 2.75, 3.5))
        .with_target(glam::Vec3::new(0.15, 0.0, 0.0))
        .with_aspect(96.0 / 72.0);
    let camera_image = render_offscreen_wgpu_with_camera(&state, 96, 72, &camera)
        .expect("explicit-camera WGPU readback should succeed after adapter probe");
    assert_ne!(default_image.as_raw(), camera_image.as_raw());
    assert_eq!(state.view_transform, original_transform);
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn offscreen_wgpu_displayed_range_and_nan_mask_change_only_expected_pixels() {
    let mut upload = square_upload();
    upload.values_f32 = Some(vec![0.0, 0.5, 1.0, 0.25]);
    let state = MeshSceneState {
        geometry_rev: GeometryRevision(1),
        upload: Some(upload),
        color: MeshColorConfig {
            range: [0.0, 1.0],
            unlit: true,
            ..MeshColorConfig::default()
        },
        ..MeshSceneState::default()
    };
    let render = |scene: &MeshSceneState| match render_offscreen_wgpu(scene, 96, 72) {
        Ok(image) => image,
        Err(error) if error.starts_with("Failed to request a headless GPU adapter") => {
            panic!("adapter probe should be handled before rendering: {error}")
        }
        Err(error) => panic!("WGPU readback failed: {error}"),
    };
    let base = match render_offscreen_wgpu(&state, 96, 72) {
        Ok(image) => image,
        Err(error) if error.starts_with("Failed to request a headless GPU adapter") => return,
        Err(error) => panic!("base WGPU readback failed: {error}"),
    };
    let mut ranged = state.clone();
    ranged.color.range = [0.25, 0.75];
    let ranged_image = render(&ranged);
    let colored_pixels = |image: &image::RgbaImage| {
        image
            .pixels()
            .filter(|pixel| pixel.0[..3] != [0, 0, 0])
            .count()
    };
    assert_eq!(colored_pixels(&base), colored_pixels(&ranged_image));
    assert_ne!(base.as_raw(), ranged_image.as_raw());

    let mut masked = state;
    masked.upload.as_mut().expect("upload").values_f32 = Some(vec![0.0, f32::NAN, 1.0, 0.25]);
    let masked_image = render(&masked);
    assert!(colored_pixels(&masked_image) < colored_pixels(&base));
    assert!(colored_pixels(&masked_image) > 0);
}

#[cfg(not(target_family = "wasm"))]
#[test]
fn adapter_drag_lod_renders_visible_proxy_and_restores_full_mesh_for_picking() {
    let mesh = connected_grid_mesh(32);
    let mut controller = MeshLodController::with_lod_threshold(mesh.clone(), 128);
    let field = ScalarField {
        id: "embedded-wgpu-lod-field".into(),
        label: "lod".into(),
        unit: None,
        values: (0..mesh.positions.len())
            .map(|index| index as f64 / mesh.positions.len().max(1) as f64)
            .collect::<Vec<_>>()
            .into(),
        association: ScalarAssociation::Vertex,
        valid: None,
    };
    let proxy = controller
        .proxy_mesh()
        .expect("large mesh should have a drag proxy")
        .clone();
    assert!(proxy.triangles.len() < mesh.triangles.len());
    assert_eq!(
        MeshBounds::from_positions(&mesh.positions),
        MeshBounds::from_positions(&proxy.positions),
        "drag proxy must preserve the full mesh bounds"
    );

    let render = |mesh: &TriangleMesh, field: &ScalarField| {
        let upload = prepare_upload(mesh, &MeshTopology::build(&mesh.triangles));
        let mut state = MeshSceneState {
            geometry_rev: GeometryRevision(1),
            upload: Some(upload),
            color: MeshColorConfig {
                range: [0.0, 1.0],
                unlit: true,
                ..MeshColorConfig::default()
            },
            ..MeshSceneState::default()
        };
        state.upload.as_mut().expect("upload").values_f32 =
            Some(field.values.iter().map(|value| *value as f32).collect());
        render_offscreen_wgpu(&state, 128, 96)
    };

    controller.begin_camera_drag();
    let proxy_field = controller
        .active_field(&field)
        .expect("proxy field should map to active proxy positions");
    let proxy_image = match render(controller.active_mesh(), &proxy_field) {
        Ok(image) => image,
        Err(error) if error.starts_with("Failed to request a headless GPU adapter") => return,
        Err(error) => panic!("proxy WGPU readback failed: {error}"),
    };
    assert!(
        proxy_image.pixels().any(|pixel| pixel.0[3] != 0),
        "drag proxy must remain visibly renderable"
    );

    controller.end_camera_drag();
    assert_eq!(
        controller.active_mesh().triangles.len(),
        mesh.triangles.len()
    );
    let full_image = render(controller.active_mesh(), &field)
        .expect("full-resolution WGPU readback should succeed after drag");
    assert!(
        full_image.pixels().any(|pixel| pixel.0[3] != 0),
        "full-resolution restore must remain visibly renderable"
    );
    let full_bvh = MeshBvh::build(controller.full_mesh());
    assert_eq!(
        MeshBounds::from_positions(&controller.full_mesh().positions),
        MeshBounds::from_positions(&controller.active_mesh().positions),
        "picking must remain attached to the restored full mesh"
    );
    assert!(
        full_bvh
            .ray_cast([0.0, 0.0, 2.0], [0.0, 0.0, -1.0])
            .is_some(),
        "full-resolution BVH must remain pickable after drag restoration"
    );
}

#[test]
fn adapter_3d_field_update_writes_scalar_storage_without_geometry_reupload() {
    let Some(ctx) = headless_context() else {
        return;
    };

    let state = Rc::new(RefCell::new(MeshSceneState::default()));
    let mut renderer = WgpuMesh3DRenderer::new(state.clone());
    let mut upload = square_upload();
    upload.values_f32 = Some(vec![0.0, 0.5, 1.0, 0.25]);
    renderer.upload_geometry(GeometryRevision(1), &upload);

    let size = [96, 96];
    let (_texture, view) = target(&ctx, size);
    let draw = lookup_custom_draw(renderer.custom_id()).expect("3D custom draw must be registered");
    let draw = draw
        .as_any()
        .downcast_ref::<WgpuCustomDrawAdapter>()
        .expect("MeshPlot 3D custom draw must use the WGPU adapter");
    draw_adapter_frame(&ctx, draw, &view, size);
    if std::env::var_os("SOTF_GPU_TIMESTAMPS").is_some_and(|value| value == "1")
        && ctx.supports_timestamp_queries()
    {
        ctx.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(1)),
            })
            .expect("timestamp query submission should complete");
    }
    let initial = state.borrow().clone();
    assert_eq!(initial.gpu_geometry_upload_count, 1);
    assert_eq!(initial.gpu_field_write_count, 0);
    assert!(initial.gpu_geometry_upload_time_ns > 0);
    assert!(initial.gpu_frame_time_ns > 0);
    assert_eq!(initial.gpu_frame_count, 1);
    assert!(initial.gpu_field_capacity_bytes >= 4 * std::mem::size_of::<f32>() as u64);

    renderer.write_field(FieldRevision(2), &[1.0, 0.25, 0.75, 0.5]);
    draw_adapter_frame(&ctx, draw, &view, size);
    // Readback is intentionally asynchronous: this frame starts mapping the
    // first submission, and the following frame polls its completed result.
    draw_adapter_frame(&ctx, draw, &view, size);
    let updated = state.borrow().clone();
    assert_eq!(updated.gpu_geometry_upload_count, 1);
    assert_eq!(
        updated.gpu_geometry_upload_bytes,
        initial.gpu_geometry_upload_bytes
    );
    assert_eq!(updated.gpu_field_write_count, 1);
    assert!(updated.gpu_field_write_time_ns > 0);
    assert!(updated.gpu_frame_count >= 2);
    if std::env::var_os("SOTF_GPU_TIMESTAMPS").is_some_and(|value| value == "1")
        && ctx.supports_timestamp_queries()
    {
        assert!(updated.gpu_frame_gpu_time_count > 0);
        assert!(updated.gpu_frame_gpu_time_ns > 0);
    }
    assert_eq!(
        updated.gpu_field_write_bytes,
        4 * std::mem::size_of::<f32>() as u64
    );
    assert_eq!(
        updated.gpu_field_capacity_bytes,
        initial.gpu_field_capacity_bytes
    );

    let mut cell_upload = square_upload();
    cell_upload.cell_values_f32 = Some(vec![0.2, 0.8]);
    renderer.upload_geometry(GeometryRevision(2), &cell_upload);
    draw_adapter_frame(&ctx, draw, &view, size);
    let layout_changed = state.borrow();
    assert_eq!(layout_changed.gpu_geometry_upload_count, 2);
    assert!(
        layout_changed.gpu_geometry_upload_bytes > updated.gpu_geometry_upload_bytes,
        "changing from vertex to cell field storage must rebuild the retained geometry resource"
    );
    drop(layout_changed);
    renderer.write_field(FieldRevision(3), &[0.9, 0.1]);
    draw_adapter_frame(&ctx, draw, &view, size);
    let cell_updated = state.borrow().clone();
    assert_eq!(cell_updated.gpu_geometry_upload_count, 2);
    assert_eq!(cell_updated.gpu_field_write_count, 2);
    assert_eq!(
        cell_updated.gpu_field_write_bytes,
        (4 + 6) * std::mem::size_of::<f32>() as u64,
        "a same-layout cell update must write expanded triangle-local values"
    );
}

#[test]
fn adapter_3d_timestamp_queries_recover_async_gpu_duration() {
    if std::env::var_os("SOTF_GPU_TIMESTAMPS").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }
    let Some(ctx) = headless_context() else {
        return;
    };
    if !ctx.supports_timestamp_queries() {
        eprintln!("SKIP WGPU timestamp test: adapter has no encoder timestamp support");
        return;
    }

    let state = Rc::new(RefCell::new(MeshSceneState::default()));
    let mut renderer = WgpuMesh3DRenderer::new(state.clone());
    let mut upload = square_upload();
    upload.values_f32 = Some(vec![0.0, 0.5, 1.0, 0.25]);
    renderer.upload_geometry(GeometryRevision(1), &upload);

    let size = [96, 96];
    let (_texture, view) = target(&ctx, size);
    let draw = lookup_custom_draw(renderer.custom_id()).expect("3D custom draw must be registered");
    let draw = draw
        .as_any()
        .downcast_ref::<WgpuCustomDrawAdapter>()
        .expect("MeshPlot 3D custom draw must use the WGPU adapter");

    for _ in 0..4 {
        draw_adapter_frame(&ctx, draw, &view, size);
        ctx.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(1)),
            })
            .expect("timestamp query submission should complete");
    }

    let stats = state.borrow();
    assert!(stats.gpu_frame_gpu_time_count > 0);
    assert!(stats.gpu_frame_gpu_time_ns > 0);
}

#[test]
fn adapter_3d_camera_only_updates_preserve_retained_geometry_and_scalar_buffers() {
    let Some(ctx) = headless_context() else {
        return;
    };

    let state = Rc::new(RefCell::new(MeshSceneState::default()));
    let mut renderer = WgpuMesh3DRenderer::new(state.clone());
    let mut upload = square_upload();
    upload.values_f32 = Some(vec![0.0, 0.5, 1.0, 0.25]);
    renderer.upload_geometry(GeometryRevision(1), &upload);

    let size = [96, 96];
    let (_texture, view) = target(&ctx, size);
    let draw = lookup_custom_draw(renderer.custom_id()).expect("3D custom draw must be registered");
    let draw = draw
        .as_any()
        .downcast_ref::<WgpuCustomDrawAdapter>()
        .expect("MeshPlot 3D custom draw must use the WGPU adapter");
    draw_adapter_frame(&ctx, draw, &view, size);
    let initial = state.borrow().clone();
    assert_eq!(initial.gpu_geometry_upload_count, 1);
    assert_eq!(initial.gpu_field_write_count, 0);

    let mut controls = OrbitControls::default();
    let camera_frames_started = Instant::now();
    for frame in 0..60 {
        controls.rotate(1.0, -0.4);
        controls.zoom(if frame % 2 == 0 { 0.002 } else { -0.001 });
        let mut camera = renderer.camera();
        controls.update_camera(&mut camera);
        renderer.set_camera(&camera);
        draw_adapter_frame(&ctx, draw, &view, size);
    }
    let camera_frames_elapsed = camera_frames_started.elapsed();
    assert!(
        camera_frames_elapsed < std::time::Duration::from_secs(1),
        "60 camera-only frames exceeded the 1 s (60 Hz average) budget: {camera_frames_elapsed:?}"
    );

    let final_state = state.borrow();
    assert_eq!(final_state.gpu_geometry_upload_count, 1);
    assert_eq!(
        final_state.gpu_geometry_upload_bytes,
        initial.gpu_geometry_upload_bytes
    );
    assert_eq!(final_state.gpu_field_write_count, 0);
    assert_eq!(final_state.gpu_field_write_bytes, 0);
    assert!(final_state.gpu_frame_count >= 61);
    assert!(final_state.gpu_frame_time_ns >= initial.gpu_frame_time_ns);
    assert_eq!(
        final_state.gpu_field_capacity_bytes,
        initial.gpu_field_capacity_bytes
    );
    assert_eq!(final_state.gpu_resident_bytes, initial.gpu_resident_bytes);
}

#[test]
fn adapter_3d_thousand_field_updates_keep_geometry_and_memory_bounded() {
    let Some(ctx) = headless_context() else {
        return;
    };

    let state = Rc::new(RefCell::new(MeshSceneState::default()));
    let mut renderer = WgpuMesh3DRenderer::new(state.clone());
    let upload = connected_grid_upload(32);
    let field_len = upload.values_f32.as_ref().expect("grid field").len();
    renderer.upload_geometry(GeometryRevision(1), &upload);

    let size = [128, 128];
    let (_texture, view) = target(&ctx, size);
    let draw = lookup_custom_draw(renderer.custom_id()).expect("3D custom draw must be registered");
    let draw = draw
        .as_any()
        .downcast_ref::<WgpuCustomDrawAdapter>()
        .expect("MeshPlot 3D custom draw must use the WGPU adapter");
    draw_adapter_frame(&ctx, draw, &view, size);
    let initial = state.borrow().clone();
    assert_eq!(initial.gpu_geometry_upload_count, 1);
    assert!(initial.gpu_geometry_upload_time_ns > 0);
    assert!(initial.gpu_frame_time_ns > 0);
    assert!(initial.gpu_resident_bytes > 0);
    assert!(initial.gpu_field_capacity_bytes >= (field_len * std::mem::size_of::<f32>()) as u64);

    for revision in 2..=1_001 {
        let phase = revision as f32 / 1_000.0;
        let values = (0..field_len)
            .map(|index| ((index as f32 / field_len as f32) + phase).fract())
            .collect::<Vec<_>>();
        renderer.write_field(FieldRevision(revision), &values);
        draw_adapter_frame(&ctx, draw, &view, size);
    }

    let final_state = state.borrow();
    assert_eq!(final_state.gpu_geometry_upload_count, 1);
    assert_eq!(
        final_state.gpu_geometry_upload_bytes,
        initial.gpu_geometry_upload_bytes
    );
    assert_eq!(final_state.gpu_field_write_count, 1_000);
    assert_eq!(
        final_state.gpu_field_write_bytes,
        1_000 * (field_len * std::mem::size_of::<f32>()) as u64
    );
    assert_eq!(
        final_state.gpu_field_capacity_bytes, initial.gpu_field_capacity_bytes,
        "alternating field updates must reuse the retained scalar capacity"
    );
    assert_eq!(
        final_state.gpu_resident_bytes, initial.gpu_resident_bytes,
        "alternating field updates must not grow resident adapter memory"
    );
}

#[test]
fn adapter_3d_geometry_generation_replacement_keeps_resident_memory_bounded() {
    let Some(ctx) = headless_context() else {
        return;
    };

    let state = Rc::new(RefCell::new(MeshSceneState::default()));
    let mut renderer = WgpuMesh3DRenderer::new(state.clone());
    let size = [96, 96];
    let (_texture, view) = target(&ctx, size);
    let draw = lookup_custom_draw(renderer.custom_id()).expect("3D custom draw must be registered");
    let draw = draw
        .as_any()
        .downcast_ref::<WgpuCustomDrawAdapter>()
        .expect("MeshPlot 3D custom draw must use the WGPU adapter");

    let mut max_resident_bytes = 0;
    for revision in 1..=100 {
        let mut upload = square_upload();
        if revision % 2 == 0 {
            upload.cell_values_f32 = Some(vec![0.2, 0.8]);
        } else {
            upload.values_f32 = Some(vec![0.0, 0.5, 1.0, 0.25]);
        }
        renderer.upload_geometry(GeometryRevision(revision), &upload);
        draw_adapter_frame(&ctx, draw, &view, size);
        max_resident_bytes = max_resident_bytes.max(state.borrow().gpu_resident_bytes);
    }

    let final_state = state.borrow();
    assert_eq!(final_state.gpu_geometry_upload_count, 100);
    assert!(final_state.gpu_resident_bytes > 0);
    assert_eq!(
        final_state.gpu_peak_resident_bytes, max_resident_bytes,
        "peak telemetry must retain the largest observed resident generation"
    );
    assert!(
        final_state.gpu_peak_resident_bytes < 64 * 1024,
        "replacing 100 tiny retained generations must not grow resident adapter memory: {max_resident_bytes} bytes"
    );
}

#[test]
fn adapter_3d_renderer_drop_releases_current_memory_but_keeps_peak_evidence() {
    let Some(ctx) = headless_context() else {
        return;
    };

    let state = Rc::new(RefCell::new(MeshSceneState::default()));
    let mut renderer = WgpuMesh3DRenderer::new(state.clone());
    renderer.upload_geometry(GeometryRevision(1), &square_upload());
    let size = [96, 96];
    let (_texture, view) = target(&ctx, size);
    {
        let draw =
            lookup_custom_draw(renderer.custom_id()).expect("3D custom draw must be registered");
        let draw = draw
            .as_any()
            .downcast_ref::<WgpuCustomDrawAdapter>()
            .expect("MeshPlot 3D custom draw must use the WGPU adapter");
        draw_adapter_frame(&ctx, draw, &view, size);
    }

    let before_drop = state.borrow().clone();
    assert!(before_drop.gpu_resident_bytes > 0);
    assert!(before_drop.gpu_peak_resident_bytes >= before_drop.gpu_resident_bytes);
    drop(renderer);

    let after_drop = state.borrow();
    assert_eq!(after_drop.gpu_resident_bytes, 0);
    assert_eq!(after_drop.gpu_field_capacity_bytes, 0);
    assert_eq!(after_drop.gpu_memory_release_count, 1);
    assert_eq!(
        after_drop.gpu_peak_resident_bytes,
        before_drop.gpu_peak_resident_bytes
    );
}

#[test]
fn adapter_2d_field_update_writes_scalar_storage_without_geometry_reupload() {
    let Some(ctx) = headless_context() else {
        return;
    };

    let state = Rc::new(RefCell::new(MeshSceneState::default()));
    let mut renderer = WgpuMeshRenderer::new(state.clone());
    let mut upload = square_upload();
    upload.values_f32 = Some(vec![0.0, 0.5, 1.0, 0.25]);
    renderer.upload_geometry(GeometryRevision(1), &upload);

    let size = [96, 96];
    let (_texture, view) = target(&ctx, size);
    let draw = lookup_custom_draw(renderer.custom_id()).expect("2D custom draw must be registered");
    let draw = draw
        .as_any()
        .downcast_ref::<WgpuCustomDrawAdapter>()
        .expect("MeshPlot 2D custom draw must use the WGPU adapter");
    draw_adapter_frame(&ctx, draw, &view, size);
    let initial = state.borrow().clone();
    assert_eq!(initial.gpu_geometry_upload_count, 1);

    renderer.write_field(FieldRevision(2), &[0.8, 0.4, 0.2, 0.6]);
    draw_adapter_frame(&ctx, draw, &view, size);
    let updated = state.borrow().clone();
    assert_eq!(updated.gpu_geometry_upload_count, 1);
    assert_eq!(
        updated.gpu_geometry_upload_bytes,
        initial.gpu_geometry_upload_bytes
    );
    assert_eq!(updated.gpu_field_write_count, 1);
    assert!(updated.gpu_field_write_time_ns > 0);
    assert_eq!(
        updated.gpu_field_write_bytes,
        4 * std::mem::size_of::<f32>() as u64
    );

    let mut cell_upload = square_upload();
    cell_upload.cell_values_f32 = Some(vec![0.2, 0.8]);
    renderer.upload_geometry(GeometryRevision(2), &cell_upload);
    draw_adapter_frame(&ctx, draw, &view, size);
    let cell = state.borrow().clone();
    assert_eq!(cell.gpu_geometry_upload_count, 2);
    assert!(
        cell.gpu_geometry_upload_bytes > updated.gpu_geometry_upload_bytes,
        "changing from vertex to cell field storage must rebuild the retained geometry resource"
    );
    assert_eq!(
        cell.gpu_field_write_bytes,
        (4 + 6) * std::mem::size_of::<f32>() as u64,
        "cell updates must write one value per expanded triangle-local vertex"
    );

    drop(cell);
    renderer.write_field(FieldRevision(3), &[0.9, 0.1]);
    draw_adapter_frame(&ctx, draw, &view, size);
    let cell_updated = state.borrow().clone();
    assert_eq!(cell_updated.gpu_geometry_upload_count, 2);
    assert_eq!(cell_updated.gpu_field_write_count, 3);
    assert_eq!(
        cell_updated.gpu_field_write_bytes,
        (4 + 6 + 6) * std::mem::size_of::<f32>() as u64
    );

    let (cell_texture, cell_view) = target(&ctx, size);
    draw_adapter_frame(&ctx, draw, &cell_view, size);
    let upper_left = read_pixel(&ctx, &cell_texture, size, 24, 24);
    let lower_right = read_pixel(&ctx, &cell_texture, size, 72, 72);
    assert_ne!(
        upper_left, lower_right,
        "different cell values must produce different rendered scalar colors"
    );
}

fn target(ctx: &WgpuContext, size: [u32; 2]) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mesh_embedded_viewport_target"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: ctx.color_texture_format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    (texture, view)
}

fn clear_background(
    ctx: &WgpuContext,
    target: &wgpu::TextureView,
    encoder: &mut wgpu::CommandEncoder,
) {
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("mesh_embedded_viewport_background"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.15,
                    g: 0.35,
                    b: 0.65,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let _ = ctx;
}

fn read_pixel(
    ctx: &WgpuContext,
    texture: &wgpu::Texture,
    size: [u32; 2],
    x: u32,
    y: u32,
) -> [u8; 4] {
    let row_bytes = size[0] * 4;
    let padded_row_bytes = row_bytes.div_ceil(256) * 256;
    let staging = ctx.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("mesh_embedded_viewport_readback"),
        size: (padded_row_bytes * size[1]) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mesh_embedded_viewport_readback_encoder"),
        });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &staging,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row_bytes),
                rows_per_image: Some(size[1]),
            },
        },
        wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
    );
    ctx.queue.submit(std::iter::once(encoder.finish()));
    let slice = staging.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = ctx.device.poll(wgpu::PollType::Wait {
        submission_index: Default::default(),
        timeout: Some(std::time::Duration::from_secs(5)),
    });
    assert!(matches!(
        receiver.recv_timeout(std::time::Duration::from_secs(5)),
        Ok(Ok(()))
    ));
    let data = slice.get_mapped_range();
    let offset = (y * padded_row_bytes + x * 4) as usize;
    let pixel = data[offset..offset + 4].try_into().unwrap();
    drop(data);
    staging.unmap();
    pixel
}

#[test]
fn adapter_draw_preserves_content_outside_an_embedded_clipped_viewport() {
    let Some(ctx) = headless_context() else {
        return;
    };

    let (baseline_texture, baseline_view) = target(&ctx, TARGET_SIZE);
    let mut baseline_encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mesh_embedded_viewport_baseline_encoder"),
        });
    clear_background(&ctx, &baseline_view, &mut baseline_encoder);
    ctx.queue.submit(std::iter::once(baseline_encoder.finish()));
    let expected_background = read_pixel(&ctx, &baseline_texture, TARGET_SIZE, 0, 0);

    let state = Rc::new(RefCell::new(MeshSceneState::default()));
    state.borrow_mut().color.wireframe = true;
    let mut renderer = WgpuMesh3DRenderer::new(state);
    renderer.upload_geometry(GeometryRevision(1), &square_upload());
    let draw = lookup_custom_draw(renderer.custom_id()).expect("custom draw must be registered");
    let draw = draw
        .as_any()
        .downcast_ref::<WgpuCustomDrawAdapter>()
        .expect("MeshPlot custom draw must use the WGPU adapter");
    let (texture, view) = target(&ctx, TARGET_SIZE);
    let mut encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mesh_embedded_viewport_encoder"),
        });
    clear_background(&ctx, &view, &mut encoder);
    draw.0.draw_wgpu(
        &ctx,
        &mut encoder,
        &view,
        ctx.color_texture_format(),
        TARGET_SIZE,
        Bounds::new(
            Point::new(px(-16.0), px(20.0)),
            Size::new(px(64.0), px(48.0)),
        ),
        Bounds::new(
            Point::new(px(-16.0), px(20.0)),
            Size::new(px(64.0), px(48.0)),
        ),
        1.0,
    );
    ctx.queue.submit(std::iter::once(encoder.finish()));

    assert_eq!(
        read_pixel(&ctx, &texture, TARGET_SIZE, 120, 80),
        expected_background
    );

    // Reuse the same retained custom draw on a smaller target. This exercises
    // resource resize plus chart clipping against a different target extent.
    let resized_target = [80, 64];
    let (resized_baseline, resized_baseline_view) = target(&ctx, resized_target);
    let mut resized_baseline_encoder =
        ctx.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mesh_embedded_viewport_resized_baseline_encoder"),
            });
    clear_background(&ctx, &resized_baseline_view, &mut resized_baseline_encoder);
    ctx.queue
        .submit(std::iter::once(resized_baseline_encoder.finish()));
    let resized_background = read_pixel(&ctx, &resized_baseline, resized_target, 0, 0);
    let (resized_texture, resized_view) = target(&ctx, resized_target);
    let mut resized_encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mesh_embedded_viewport_resized_encoder"),
        });
    clear_background(&ctx, &resized_view, &mut resized_encoder);
    draw.0.draw_wgpu(
        &ctx,
        &mut resized_encoder,
        &resized_view,
        ctx.color_texture_format(),
        resized_target,
        Bounds::new(
            Point::new(px(20.0), px(8.0)),
            Size::new(px(100.0), px(80.0)),
        ),
        Bounds::new(
            Point::new(px(20.0), px(8.0)),
            Size::new(px(100.0), px(80.0)),
        ),
        1.0,
    );
    ctx.queue.submit(std::iter::once(resized_encoder.finish()));
    assert_eq!(
        read_pixel(&ctx, &resized_texture, resized_target, 5, 5),
        resized_background
    );

    // A dashboard can issue more than one retained custom draw into one
    // target. Each must load the prior GPUI/sibling content and constrain its
    // own viewport/scissor rather than clearing the full attachment.
    let (sibling_texture, sibling_view) = target(&ctx, TARGET_SIZE);
    let mut sibling_encoder = ctx
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mesh_embedded_viewport_sibling_encoder"),
        });
    clear_background(&ctx, &sibling_view, &mut sibling_encoder);
    draw.0.draw_wgpu(
        &ctx,
        &mut sibling_encoder,
        &sibling_view,
        ctx.color_texture_format(),
        TARGET_SIZE,
        Bounds::new(Point::new(px(0.0), px(12.0)), Size::new(px(48.0), px(72.0))),
        Bounds::new(Point::new(px(0.0), px(12.0)), Size::new(px(48.0), px(72.0))),
        1.0,
    );
    draw.0.draw_wgpu(
        &ctx,
        &mut sibling_encoder,
        &sibling_view,
        ctx.color_texture_format(),
        TARGET_SIZE,
        Bounds::new(
            Point::new(px(80.0), px(12.0)),
            Size::new(px(48.0), px(72.0)),
        ),
        Bounds::new(
            Point::new(px(80.0), px(12.0)),
            Size::new(px(48.0), px(72.0)),
        ),
        1.0,
    );
    ctx.queue.submit(std::iter::once(sibling_encoder.finish()));

    for (x, y) in [(63, 48), (64, 48), (65, 48), (64, 4), (64, 92)] {
        assert_eq!(
            read_pixel(&ctx, &sibling_texture, TARGET_SIZE, x, y),
            expected_background,
            "sibling viewport gap/background pixel ({x}, {y}) must remain GPUI-owned"
        );
    }
    unregister_custom_draw(renderer.custom_id());
}
