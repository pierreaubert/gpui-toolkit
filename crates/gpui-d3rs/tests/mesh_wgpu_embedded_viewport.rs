#![cfg(feature = "gpu-3d")]

use d3rs::gpu3d::OrbitControls;
use d3rs::mesh::gpu::{
    FieldRevision, GeometryRevision, MeshColorConfig, MeshGpuRenderer, MeshSceneState,
    WgpuMesh3DRenderer, WgpuMeshRenderer, render_offscreen_wgpu,
};
use d3rs::mesh::{MeshTopology, TriangleMesh, prepare_upload};
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
        size,
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

    let mut state = MeshSceneState::default();
    state.geometry_rev = GeometryRevision(1);
    state.upload = Some(upload);
    state.color = MeshColorConfig {
        range: [0.0, 1.0],
        unlit: true,
        ..MeshColorConfig::default()
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
    let initial = state.borrow().clone();
    assert_eq!(initial.gpu_geometry_upload_count, 1);
    assert_eq!(initial.gpu_field_write_count, 0);
    assert!(initial.gpu_geometry_upload_time_ns > 0);
    assert!(initial.gpu_frame_time_ns > 0);
    assert_eq!(initial.gpu_frame_count, 1);
    assert!(initial.gpu_field_capacity_bytes >= 4 * std::mem::size_of::<f32>() as u64);

    renderer.write_field(FieldRevision(2), &[1.0, 0.25, 0.75, 0.5]);
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
        TARGET_SIZE,
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
        resized_target,
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
        TARGET_SIZE,
        Bounds::new(Point::new(px(0.0), px(12.0)), Size::new(px(48.0), px(72.0))),
        1.0,
    );
    draw.0.draw_wgpu(
        &ctx,
        &mut sibling_encoder,
        &sibling_view,
        TARGET_SIZE,
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
