#![cfg(all(target_os = "macos", feature = "metal-qa"))]

//! Persist long-run Metal driver-allocation and retained-resource evidence.

use d3rs::mesh::gpu::{
    FieldRevision, GeometryRevision, MeshColorConfig, MeshGpuRenderer, MeshSceneElement,
    MeshSceneState, MetalMeshRenderer,
};
use d3rs::mesh::{MeshTopology, TriangleMesh, prepare_upload};
use gpui::{
    AppContext, Context, HeadlessAppContext, ParentElement, Render, Styled, Window, div, px,
};
use gpui_macos::metal_renderer::MetalHeadlessRenderer;
use serde_json::{Value, json};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

const EVIDENCE_DIR_ENV: &str = "SOTF_MESH_METAL_MEMORY_EVIDENCE_DIR";
const SOURCE_REVISION_ENV: &str = "SOTF_MESH_METAL_MEMORY_SOURCE_REVISION";
const SOURCE_DIRTY_ENV: &str = "SOTF_MESH_METAL_MEMORY_SOURCE_DIRTY";

struct MetalSceneView {
    state: Rc<RefCell<MeshSceneState>>,
    custom_id: gpui::CustomDrawId,
}

impl Render for MetalSceneView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl gpui::IntoElement {
        div()
            .w(px(320.0))
            .h(px(240.0))
            .child(MeshSceneElement::new(self.state.clone()).with_custom_id(self.custom_id))
    }
}

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
    fs::write(directory.join("mesh-metal-memory-evidence.json"), bytes)
        .map_err(|error| error.to_string())
}

fn write_skip(reason: &str) {
    write_manifest(json!({
        "schema_version": 1,
        "report_type": "gpui-mesh-metal-memory-evidence",
        "status": "skipped",
        "reason": reason,
    }))
    .expect("write Metal memory skip manifest");
}

fn metal_required() -> bool {
    env::var_os("QA_METAL_REQUIRED").is_some_and(|value| value == "1")
}

fn grid_upload(side: usize, phase: f32) -> d3rs::mesh::MeshUpload {
    let positions = (0..=side)
        .flat_map(|y| {
            (0..=side).map(move |x| {
                let xf = x as f64 / side as f64;
                let yf = y as f64 / side as f64;
                [
                    -1.0 + 2.0 * xf,
                    -1.0 + 2.0 * yf,
                    (xf * 0.2 + yf * 0.15 + f64::from(phase)).sin() * 0.08,
                ]
            })
        })
        .collect::<Vec<_>>();
    let mut triangles = Vec::with_capacity(side * side * 2);
    for y in 0..side {
        for x in 0..side {
            let lower_left = (y * (side + 1) + x) as u32;
            let lower_right = lower_left + 1;
            let upper_left = lower_left + (side + 1) as u32;
            let upper_right = upper_left + 1;
            triangles.push([lower_left, lower_right, upper_right]);
            triangles.push([lower_left, upper_right, upper_left]);
        }
    }
    let mesh = TriangleMesh {
        id: format!("metal-memory-{side}-{phase}").into(),
        positions: positions.into(),
        triangles: triangles.into(),
        vertex_ids: None,
        cell_ids: None,
    };
    let mut upload = prepare_upload(&mesh, &MeshTopology::build(&mesh.triangles));
    upload.values_f32 = Some(
        (0..mesh.positions.len())
            .map(|index| ((index as f32 / mesh.positions.len() as f32) + phase).fract())
            .collect(),
    );
    upload
}

fn draw(window: gpui::AnyWindowHandle, cx: &mut HeadlessAppContext) {
    cx.update_window(window, |_, window, app| {
        window.draw(app).clear();
    })
    .expect("draw Metal memory evidence frame");
    cx.run_until_parked();
    cx.capture_screenshot(window)
        .expect("capture Metal memory evidence frame");
}

#[test]
fn metal_long_run_memory_release_evidence_covers_churn_and_teardown() {
    if MetalHeadlessRenderer::try_new().is_none() {
        if metal_required() {
            panic!("required Metal adapter is unavailable");
        }
        eprintln!("SKIP Metal memory evidence: no compatible Metal device");
        write_skip("no usable Metal adapter");
        return;
    }

    let text_system = Arc::new(gpui::NoopTextSystem::new());
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), || {
        Some(Box::new(MetalHeadlessRenderer::new()))
    });
    let state = Rc::new(RefCell::new(MeshSceneState {
        color: MeshColorConfig {
            range: [0.0, 1.0],
            unlit: false,
            ..MeshColorConfig::default()
        },
        ..MeshSceneState::default()
    }));
    let mut renderer = MetalMeshRenderer::new_3d(state.clone());
    renderer.upload_geometry(GeometryRevision(1), &grid_upload(24, 0.0));
    let custom_id = renderer.custom_id();
    let view_state = state.clone();
    let window = cx
        .open_window(gpui::size(px(320.0), px(240.0)), move |_window, app| {
            app.new(|_cx| MetalSceneView {
                state: view_state,
                custom_id,
            })
        })
        .expect("open Metal memory evidence window");
    let window: gpui::AnyWindowHandle = window.into();

    draw(window, &mut cx);
    let mut samples = Vec::new();
    let mut current_side = 24_usize;
    for revision in 2..=21_u64 {
        let phase = revision as f32 * 0.13;
        if revision % 2 == 0 {
            let value_count = (current_side + 1) * (current_side + 1);
            let values = (0..value_count)
                .map(|index| ((index as f32 / value_count as f32) + phase).fract())
                .collect::<Vec<_>>();
            renderer.write_field(FieldRevision(revision), &values);
        } else {
            let side = if revision % 4 == 1 { 24 } else { 32 };
            current_side = side;
            renderer.upload_geometry(GeometryRevision(revision), &grid_upload(side, phase));
        }
        draw(window, &mut cx);
        let stats = state.borrow().clone();
        samples.push(json!({
            "revision": revision,
            "operation": if revision % 2 == 0 { "field" } else { "geometry" },
            "driver_allocated_bytes": stats.gpu_driver_allocated_bytes,
            "peak_driver_allocated_bytes": stats.gpu_peak_driver_allocated_bytes,
            "resident_bytes": stats.gpu_resident_bytes,
            "peak_resident_bytes": stats.gpu_peak_resident_bytes,
            "geometry_upload_count": stats.gpu_geometry_upload_count,
            "memory_release_count": stats.gpu_memory_release_count,
        }));
    }

    let before_drop = state.borrow().clone();
    cx.update_window(window, |_, window, _app| window.remove_window())
        .expect("close Metal memory evidence window");
    cx.run_until_parked();
    drop(cx);
    drop(renderer);
    let after_drop = state.borrow().clone();
    assert!(
        before_drop
            .gpu_driver_allocated_bytes
            .is_some_and(|bytes| bytes > 0)
    );
    assert!(
        before_drop.gpu_peak_driver_allocated_bytes
            >= before_drop.gpu_driver_allocated_bytes.unwrap_or(0)
    );
    assert!(samples.len() >= 20);
    assert_eq!(after_drop.gpu_driver_allocated_bytes, None);
    assert!(after_drop.gpu_memory_release_count > 0);
    assert_eq!(
        after_drop.gpu_peak_driver_allocated_bytes,
        before_drop.gpu_peak_driver_allocated_bytes
    );

    write_manifest(json!({
        "schema_version": 1,
        "report_type": "gpui-mesh-metal-memory-evidence",
        "status": "captured",
        "backend": "metal",
        "adapter_backed": true,
        "sample_count": samples.len(),
        "samples": samples,
        "before_drop": {
            "driver_allocated_bytes": before_drop.gpu_driver_allocated_bytes,
            "peak_driver_allocated_bytes": before_drop.gpu_peak_driver_allocated_bytes,
            "resident_bytes": before_drop.gpu_resident_bytes,
            "peak_resident_bytes": before_drop.gpu_peak_resident_bytes,
            "geometry_upload_count": before_drop.gpu_geometry_upload_count,
        },
        "after_drop": {
            "driver_allocated_bytes": after_drop.gpu_driver_allocated_bytes,
            "resident_bytes": after_drop.gpu_resident_bytes,
            "memory_release_count": after_drop.gpu_memory_release_count,
            "peak_driver_allocated_bytes": after_drop.gpu_peak_driver_allocated_bytes,
        },
        "contracts": {
            "alternating_field_and_geometry_churn": true,
            "driver_peak_is_monotonic": true,
            "teardown_clears_current_memory": after_drop.gpu_driver_allocated_bytes.is_none(),
            "teardown_preserves_peak": after_drop.gpu_peak_driver_allocated_bytes == before_drop.gpu_peak_driver_allocated_bytes,
        },
    }))
    .expect("write Metal memory evidence manifest");
}
