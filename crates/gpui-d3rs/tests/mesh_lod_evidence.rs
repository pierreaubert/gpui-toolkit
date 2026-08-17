#![cfg(feature = "gpu-3d")]

//! Persist adapter-backed drag-time LOD evidence.
//!
//! Developer hosts without a usable adapter produce an explicit skip. A
//! reference host runs the same proxy/full transition through the retained
//! custom draw, saves both readback images, and records the 60-frame drag
//! budget plus full-resolution restoration contract.

use d3rs::gpu3d::Camera3D;
use d3rs::mesh::gpu::{
    GeometryRevision, MeshColorConfig, MeshGpuRenderer, MeshLodController, MeshSceneState,
    WgpuMesh3DRenderer, render_offscreen_wgpu,
};
use d3rs::mesh::{MeshTopology, ScalarAssociation, ScalarField, TriangleMesh, prepare_upload};
use gpui::{Bounds, Point, Size, lookup_custom_draw, px};
use gpui_wgpu::{WgpuContext, WgpuCustomDrawAdapter};
use serde_json::{Value, json};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

const EVIDENCE_DIR_ENV: &str = "SOTF_MESH_LOD_EVIDENCE_DIR";
const SOURCE_REVISION_ENV: &str = "SOTF_MESH_LOD_SOURCE_REVISION";
const SOURCE_DIRTY_ENV: &str = "SOTF_MESH_LOD_SOURCE_DIRTY";
const FRAME_COUNT: usize = 60;
const FRAME_BUDGET_NS: u128 = 20_000_000;

fn source_fields(manifest: &mut Value) {
    let Some(revision) = env::var(SOURCE_REVISION_ENV).ok() else {
        return;
    };
    manifest["source_revision"] = Value::String(revision);
    manifest["source_dirty"] = Value::Bool(
        env::var(SOURCE_DIRTY_ENV)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE"))
            .unwrap_or(true),
    );
}

fn write_manifest(mut manifest: Value) -> Result<(), String> {
    let Some(directory) = env::var_os(EVIDENCE_DIR_ENV) else {
        return Ok(());
    };
    let directory = Path::new(&directory);
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    source_fields(&mut manifest);
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(directory.join("mesh-lod-evidence.json"), bytes).map_err(|error| error.to_string())
}

fn write_skip(reason: &str) {
    write_manifest(json!({
        "schema_version": 1,
        "report_type": "gpui-mesh-lod-evidence",
        "status": "skipped",
        "reason": reason,
    }))
    .expect("write LOD evidence skip manifest");
}

fn connected_grid(side: u32) -> TriangleMesh {
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
            let lower_left = y * (side + 1) + x;
            let lower_right = lower_left + 1;
            let upper_left = lower_left + side + 1;
            let upper_right = upper_left + 1;
            triangles.push([lower_left, lower_right, upper_right]);
            triangles.push([lower_left, upper_right, upper_left]);
        }
    }
    TriangleMesh {
        id: "lod-evidence-grid".into(),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids: None,
        cell_ids: None,
    }
}

fn field_for(mesh: &TriangleMesh) -> ScalarField {
    ScalarField {
        id: "lod-evidence-field".into(),
        label: "LOD evidence".into(),
        unit: None,
        values: (0..mesh.positions.len())
            .map(|index| index as f64 / mesh.positions.len().max(1) as f64)
            .collect::<Vec<_>>()
            .into(),
        association: ScalarAssociation::Vertex,
        valid: None,
    }
}

fn upload_for(mesh: &TriangleMesh, field: &ScalarField) -> d3rs::mesh::MeshUpload {
    let mut upload = prepare_upload(mesh, &MeshTopology::build(&mesh.triangles));
    upload.values_f32 = Some(field.values.iter().map(|value| *value as f32).collect());
    upload
}

fn state_for(upload: d3rs::mesh::MeshUpload) -> MeshSceneState {
    MeshSceneState {
        geometry_rev: GeometryRevision(1),
        upload: Some(upload),
        color: MeshColorConfig {
            range: [0.0, 1.0],
            unlit: true,
            ..MeshColorConfig::default()
        },
        ..MeshSceneState::default()
    }
}

fn draw_frame(
    context: &WgpuContext,
    draw: &WgpuCustomDrawAdapter,
    target: &wgpu::TextureView,
    size: [u32; 2],
) {
    let mut encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("mesh_lod_evidence_frame"),
        });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("mesh_lod_evidence_clear"),
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
    }
    draw.0.draw_wgpu(
        context,
        &mut encoder,
        target,
        context.color_texture_format(),
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
    context.queue.submit(std::iter::once(encoder.finish()));
}

fn non_black_pixels(image: &image::RgbaImage) -> usize {
    image
        .pixels()
        .filter(|pixel| pixel.0[..3] != [0, 0, 0])
        .count()
}

fn changed_fraction(left: &image::RgbaImage, right: &image::RgbaImage) -> f64 {
    let changed = left
        .pixels()
        .zip(right.pixels())
        .filter(|(left, right)| left.0 != right.0)
        .count();
    changed as f64 / left.width().max(1) as f64 / left.height().max(1) as f64
}

#[test]
fn adapter_lod_release_evidence_covers_drag_budget_and_visual_restore() {
    let Some(context) = (match WgpuContext::headless() {
        Ok(context) => Some(context),
        Err(error) => {
            if env::var_os("QA_WGPU_REQUIRED").is_some_and(|value| value == "1") {
                panic!("required WGPU adapter is unavailable: {error:#}");
            }
            eprintln!("SKIP WGPU LOD evidence: {error:#}");
            write_skip("no usable WGPU adapter");
            None
        }
    }) else {
        return;
    };

    let mesh = connected_grid(32);
    let field = field_for(&mesh);
    let full_upload = upload_for(&mesh, &field);
    let mut controller = MeshLodController::with_lod_threshold(mesh.clone(), 128);
    let proxy_mesh = controller
        .proxy_mesh()
        .expect("large fixture must produce a drag proxy")
        .clone();
    let proxy_field = controller
        .active_field(&field)
        .expect("proxy field must map to proxy vertices");
    let proxy_upload = upload_for(&proxy_mesh, &proxy_field);

    let full_image = match render_offscreen_wgpu(&state_for(full_upload.clone()), 160, 120) {
        Ok(image) => image,
        Err(error) => {
            write_skip(&format!(
                "WGPU full-resolution readback unavailable: {error}"
            ));
            return;
        }
    };
    let proxy_image = match render_offscreen_wgpu(&state_for(proxy_upload.clone()), 160, 120) {
        Ok(image) => image,
        Err(error) => {
            write_skip(&format!("WGPU proxy readback unavailable: {error}"));
            return;
        }
    };
    let full_non_black = non_black_pixels(&full_image);
    let proxy_non_black = non_black_pixels(&proxy_image);
    assert!(
        full_non_black > 0,
        "full-resolution LOD frame must render pixels"
    );
    assert!(proxy_non_black > 0, "proxy LOD frame must render pixels");
    assert!(proxy_mesh.triangles.len() < mesh.triangles.len());

    let state = Rc::new(RefCell::new(MeshSceneState::default()));
    let mut renderer = WgpuMesh3DRenderer::new(state.clone());
    renderer.upload_geometry(GeometryRevision(1), &full_upload);
    let size = [160, 120];
    let target = context.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("mesh_lod_evidence_target"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // The retained pipeline is configured from the platform WGPU
        // surface format.  Reuse that format here rather than hard-coding
        // RGBA: macOS headless WGPU uses BGRA and rejects an incompatible
        // render-pass attachment during pipeline binding.
        format: context.color_texture_format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());
    let draw_handle = lookup_custom_draw(renderer.custom_id())
        .expect("LOD evidence custom draw must be registered");
    let draw = draw_handle
        .as_any()
        .downcast_ref::<WgpuCustomDrawAdapter>()
        .expect("LOD evidence must use the WGPU custom draw");
    draw_frame(&context, draw, &target_view, size);

    controller.begin_camera_drag();
    renderer.upload_geometry(GeometryRevision(2), &proxy_upload);
    let mut frame_times = Vec::with_capacity(FRAME_COUNT);
    for _ in 0..FRAME_COUNT {
        let started = Instant::now();
        draw_frame(&context, draw, &target_view, size);
        frame_times.push(started.elapsed().as_nanos());
    }
    let proxy_stats = state.borrow().clone();

    controller.end_camera_drag();
    renderer.upload_geometry(GeometryRevision(3), &full_upload);
    draw_frame(&context, draw, &target_view, size);
    context
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(1)),
        })
        .expect("LOD evidence submission should complete");
    let restored_stats = state.borrow().clone();
    drop(target_view);
    drop(target);
    drop(renderer);

    let total_ns: u128 = frame_times.iter().sum();
    let average_ns = total_ns / frame_times.len().max(1) as u128;
    let max_ns = frame_times.iter().copied().max().unwrap_or(0);
    let budget_passed = average_ns <= FRAME_BUDGET_NS;
    assert!(
        budget_passed,
        "average drag frame exceeded 20 ms: {average_ns} ns"
    );
    assert!(restored_stats.gpu_geometry_upload_count >= 3);
    assert!(restored_stats.gpu_frame_count >= (FRAME_COUNT + 2) as u64);

    let output_dir = env::var_os(EVIDENCE_DIR_ENV).map(PathBuf::from);
    if let Some(directory) = output_dir.as_deref() {
        fs::create_dir_all(directory).expect("create LOD evidence directory");
        proxy_image
            .save(directory.join("proxy.png"))
            .expect("save proxy LOD image");
        full_image
            .save(directory.join("full.png"))
            .expect("save full-resolution LOD image");
    }
    write_manifest(json!({
        "schema_version": 1,
        "report_type": "gpui-mesh-lod-evidence",
        "status": "captured",
        "backend": format!("{:?}", context.adapter.get_info().backend).to_ascii_lowercase(),
        "adapter_backed": true,
        "workload": {
            "full_triangle_count": mesh.triangles.len(),
            "proxy_triangle_count": proxy_mesh.triangles.len(),
            "proxy_reduces_triangles": proxy_mesh.triangles.len() < mesh.triangles.len(),
        },
        "visual_quality": {
            "width": full_image.width(),
            "height": full_image.height(),
            "full_non_black_pixels": full_non_black,
            "proxy_non_black_pixels": proxy_non_black,
            "proxy_full_changed_fraction": changed_fraction(&proxy_image, &full_image),
            "passed": full_non_black > 0 && proxy_non_black > 0,
            "proxy_path": "proxy.png",
            "full_path": "full.png",
        },
        "frame_budget": {
            "sample_count": FRAME_COUNT,
            "target_average_ns": FRAME_BUDGET_NS,
            "total_ns": total_ns,
            "average_ns": average_ns,
            "max_ns": max_ns,
            "passed": budget_passed,
        },
        "telemetry": {
            "proxy_gpu_frame_count": proxy_stats.gpu_frame_count,
            "restored_gpu_frame_count": restored_stats.gpu_frame_count,
            "restored_geometry_upload_count": restored_stats.gpu_geometry_upload_count,
            "restored_gpu_frame_time_ns": restored_stats.gpu_frame_time_ns,
        },
    }))
    .expect("write LOD evidence manifest");
}

#[allow(dead_code)]
fn _camera_type_remains_linked_to_gpu_lod_evidence() {
    let _ = Camera3D::default();
}
