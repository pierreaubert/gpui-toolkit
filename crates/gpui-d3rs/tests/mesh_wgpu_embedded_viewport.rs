#![cfg(feature = "gpu-3d")]

use d3rs::mesh::gpu::{
    GeometryRevision, MeshGpuRenderer, MeshSceneState, WgpuMesh3DRenderer, WgpuMeshRenderer,
};
use d3rs::mesh::{MeshTopology, TriangleMesh, prepare_upload};
use gpui::{Bounds, Point, Size, lookup_custom_draw, px, unregister_custom_draw};
use gpui_wgpu::{WgpuContext, WgpuCustomDrawAdapter};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

const TARGET_SIZE: [u32; 2] = [128, 96];

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
    let Ok(ctx) = WgpuContext::headless() else {
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
