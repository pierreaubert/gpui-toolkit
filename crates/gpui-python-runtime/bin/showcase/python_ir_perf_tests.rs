//! Headless measurements of the Python host's real table frame path.

// Rust guideline compliant 2026-10-09

use super::{PresentationStore, PythonAppIr, PythonIrShowcase};
use gpui::{
    AnyWindowHandle, AppContext as _, HeadlessAppContext, PlatformTextSystem as _, ScrollStrategy,
    px, size,
};
use gpui_design::{DesignSystem, DesignSystemState};
use gpui_python_runtime::session::{Patch, PatchOp};
use gpui_python_runtime::ui_ir::UiNode;
use gpui_ui_kit::accessibility::AccessibilityTree;
use gpui_ui_kit::i18n::I18nState;
use gpui_ui_kit::theme::{ThemeState, ThemeVariant};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;

fn table_app(row_count: usize) -> PythonAppIr {
    let rows = (0..row_count)
        .map(|index| {
            serde_json::json!({
                "id": format!("row-{index}"),
                "cells": [format!("Name {index}"), format!("user{index}@example.com"), index, "active"],
            })
        })
        .collect::<Vec<_>>();
    let app: PythonAppIr = serde_json::from_value(serde_json::json!({
        "title": "Table benchmark",
        "sections": [{
            "id": "table",
            "label": "Table",
            "content": {
                "kind": "table",
                "id": "table",
                "columns": [
                    {"id": "name", "label": "Name"},
                    {"id": "email", "label": "Email"},
                    {"id": "index", "label": "Index"},
                    {"id": "state", "label": "State"}
                ],
                "typed_rows": rows,
                "selected_row": format!("row-{}", row_count - 1),
                "selection_action": "select"
            }
        }, {
            "id": "overview",
            "label": "Overview",
            "content": {"kind": "text", "text": "Overview"}
        }]
    }))
    .expect("table fixture must deserialize");
    app.validate().expect("table fixture must validate");
    app
}

fn draw(cx: &mut HeadlessAppContext, window: AnyWindowHandle, metal: bool) {
    cx.update_window(window, |_, window, app| {
        let _ = std::hint::black_box(window.draw(app));
    })
    .expect("headless frame must draw");
    if metal {
        let image = cx
            .capture_screenshot(window)
            .expect("Metal frame must render");
        std::hint::black_box(image);
    }
}

fn screenshot_hash(cx: &mut HeadlessAppContext, window: AnyWindowHandle) -> u64 {
    let image = cx
        .capture_screenshot(window)
        .expect("Metal frame must render");
    let pixels = image.as_raw();
    let (pixels_rgba, _) = pixels.as_chunks::<4>();
    assert!(
        pixels_rgba.iter().any(|pixel| pixel != &pixels_rgba[0]),
        "Metal frame must contain visible content"
    );
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pixels.hash(&mut hasher);
    hasher.finish()
}

/// Run with `cargo test --release -p gpui-python-runtime --features perf-test
/// --bin gpui-python-showcase python_table_frames -- --ignored --nocapture`.
#[::core::prelude::v1::test]
#[ignore = "performance measurement; run in isolation"]
fn python_table_frames() {
    let row_count = std::env::var("GPUI_PERF_ROWS")
        .ok()
        .map_or(10_000, |value| value.parse().expect("valid row count"));
    let frames = std::env::var("GPUI_PERF_FRAMES")
        .ok()
        .map_or(30, |value| value.parse().expect("valid frame count"));
    let section = std::env::var("GPUI_PERF_SECTION").unwrap_or_else(|_| "table".into());
    let mode = std::env::var("GPUI_PERF_MODE").unwrap_or_else(|_| "refresh".into());
    let renderer = std::env::var("GPUI_PERF_RENDERER").unwrap_or_else(|_| "headless".into());
    let metal = renderer == "metal";
    let mut showcase = PythonIrShowcase::new_empty(PresentationStore::open());
    showcase.apply_snapshot_message(table_app(row_count));
    assert!(showcase.load_error.is_none());
    showcase.current_section.clone_from(&section);

    let text_system = Arc::new(gpui_wgpu::CosmicTextSystem::new_without_system_fonts(
        "IBM Plex Sans",
    ));
    text_system
        .add_fonts(gpui_miniapp::bundled_ui_fonts())
        .expect("bundled fonts must load");
    let mut cx = HeadlessAppContext::with_platform(text_system, Arc::new(()), move || {
        #[cfg(all(target_os = "macos", feature = "native-qa"))]
        if metal {
            return Ok(Some(Box::new(
                gpui_macos::metal_renderer::MetalHeadlessRenderer::new(),
            )));
        }
        Ok(None)
    });
    cx.update(|app| {
        app.set_global(ThemeState::with_variant(ThemeVariant::Dark));
        app.set_global(DesignSystemState::with_system(DesignSystem::neutral()));
        app.set_global(AccessibilityTree::new());
        app.set_global(I18nState::new());
    });
    let cold_start = Instant::now();
    let handle = cx
        .open_window(size(px(1200.0), px(900.0)), move |_, app| {
            app.new(|_| showcase)
        })
        .expect("table window must open");
    let window: AnyWindowHandle = *handle;
    draw(&mut cx, window, metal);
    let cold_ns = cold_start.elapsed().as_nanos();
    let initial_image_hash = metal.then(|| screenshot_hash(&mut cx, window));
    for _ in 0..5 {
        handle.update(&mut cx, |_, _, cx| cx.notify()).unwrap();
        draw(&mut cx, window, metal);
    }
    let scroll = (mode == "scroll").then(|| {
        handle
            .update(&mut cx, |view, _, _| {
                view.table_scrolls.get("table").unwrap().clone()
            })
            .unwrap()
    });
    cx.update_window(window, |_, window, _| window.reset_layout_stats())
        .unwrap();
    let mut samples = Vec::with_capacity(frames);
    let allocation_start = gpui_profiler::AllocSnapshot::now();
    for frame in 0..frames {
        let start = Instant::now();
        if let Some(scroll) = &scroll {
            scroll.scroll_to_item_strict((frame * 3) % row_count, ScrollStrategy::Top);
        }
        handle
            .update(&mut cx, |view, _, cx| {
                if mode == "patch" {
                    view.apply_patch_message(Patch {
                        revision: (frame + 1) as u64,
                        request_id: None,
                        ops: vec![PatchOp::Set {
                            id: "table".into(),
                            property: "selected_row".into(),
                            value: serde_json::json!(format!(
                                "row-{}",
                                if frame.is_multiple_of(2) {
                                    0
                                } else {
                                    row_count - 1
                                }
                            )),
                        }],
                    });
                }
                cx.notify();
            })
            .unwrap();
        draw(&mut cx, window, metal);
        samples.push(start.elapsed().as_nanos());
    }
    let allocations = gpui_profiler::AllocSnapshot::delta_since(allocation_start);
    let image_hash = if metal {
        let hash = screenshot_hash(&mut cx, window);
        if mode == "scroll" || (mode == "patch" && !frames.is_multiple_of(2)) {
            assert_ne!(
                Some(hash),
                initial_image_hash,
                "interaction must change the rendered frame"
            );
        }
        hash
    } else {
        0
    };
    let stats = cx
        .update_window(window, |_, window, _| window.layout_stats())
        .unwrap();
    assert!(stats.frames >= frames as u64);
    assert!(stats.views_built >= frames as u64);
    if let Some(scroll) = scroll {
        let offset = scroll.0.borrow().base_handle.offset().y;
        assert!(offset < px(0.0), "table did not scroll: {offset:?}");
    }
    handle
        .update(&mut cx, |view, _, _| {
            if mode == "patch" {
                assert_eq!(view.session_state.revision(), frames as u64);
                let UiNode::Table(table) = &view.app.as_ref().unwrap().sections[0].content else {
                    panic!("fixture must contain a table");
                };
                let selected = if (frames - 1).is_multiple_of(2) {
                    0
                } else {
                    row_count - 1
                };
                assert_eq!(
                    table.selected_row.as_deref(),
                    Some(format!("row-{selected}").as_str())
                );
            }
            assert_eq!(
                view.app_value,
                view.app
                    .as_ref()
                    .map(|app| serde_json::to_value(app).unwrap())
            );
        })
        .unwrap();
    samples.sort_unstable();
    println!(
        "section={section} mode={mode} renderer={renderer} rows={row_count} iterations={frames} cold_ns={cold_ns} median_ns={} min_ns={} max_ns={} frames={} views_built={} alloc_bytes_per_iter={} alloc_calls_per_iter={} image_hash={image_hash}",
        samples[frames / 2],
        samples[0],
        samples[frames - 1],
        stats.frames,
        stats.views_built,
        allocations.bytes / frames,
        allocations.count / frames
    );
}
