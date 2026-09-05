//! CPU-vs-GPU vello snapshot comparison for the backend-compare story.
//!
//! Builds small antialiasing-sensitive [`ChartScene`] presets, renders each
//! through [`CpuRasterizer`] and the headless [`snapshot_scene_gpu`] path at
//! the same physical size and scale, and returns display-ready images plus a
//! [`PixelDiff`]. Pure enough to unit-test without a window; the story render
//! fn in `component_lab.rs` owns caching and layout.

use d3rs::vello2d::kurbo::{Rect, Stroke};
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{
    ChartScene, CpuRasterizer, PixelDiff, compare_rgba, diff_image_rgba, snapshot_scene_gpu,
};
use gpui::RenderImage;
use image::{Frame, RgbaImage};
use std::sync::Arc;
use std::time::Instant;

/// Logical scene size for every compare preset.
pub(super) const COMPARE_LOGICAL_W: f32 = 160.0;
pub(super) const COMPARE_LOGICAL_H: f32 = 120.0;

/// Display-ready result of one CPU-vs-GPU comparison.
#[derive(Clone)]
pub(super) struct BackendCompareResult {
    pub cpu: Arc<RenderImage>,
    pub gpu: Option<Arc<RenderImage>>,
    pub diff: Option<Arc<RenderImage>>,
    pub stats: Option<PixelDiff>,
    pub gpu_error: Option<String>,
    pub cpu_ms: f64,
    pub gpu_ms: f64,
}

/// One of the `preset` story choices.
pub(super) fn compare_preset_scene(preset: &str) -> ChartScene {
    match preset {
        "spectrum" => spectrum_preset(),
        "strokes" => strokes_preset(),
        _ => knob_preset(),
    }
}

fn solid(r: u8, g: u8, b: u8) -> Brush {
    Brush::Solid(Color::from_rgb8(r, g, b))
}

fn translucent(r: u8, g: u8, b: u8, a: u8) -> Brush {
    Brush::Solid(Color::from_rgba8(r, g, b, a))
}

/// Knob-like shapes: thick arc, value wedge, thin tick ring, tick strokes.
fn knob_preset() -> ChartScene {
    let mut scene = ChartScene::new();
    let (cx, cy) = (80.0, 62.0);
    scene.stroke_arc(
        cx,
        cy,
        44.0,
        -2.4,
        4.8,
        Stroke::new(9.0),
        solid(150, 150, 160),
    );
    scene.fill_wedge(cx, cy, 33.0, -1.0, 1.6, solid(70, 170, 240));
    scene.stroke_arc(
        cx,
        cy,
        52.0,
        0.0,
        std::f64::consts::TAU,
        Stroke::new(1.5),
        solid(245, 245, 245),
    );
    // Eleven tick strokes along the arc sweep.
    for index in 0..11 {
        let angle = -2.4 + (4.8 + 2.4) * f64::from(index) / 10.0;
        let (inner, outer) = (56.0, 62.0);
        scene.stroke_polyline(
            &[
                (cx + inner * angle.cos(), cy + inner * angle.sin()),
                (cx + outer * angle.cos(), cy + outer * angle.sin()),
            ],
            Stroke::new(2.0),
            solid(200, 200, 210),
        );
    }
    scene
}

/// Spectrum-like shapes: bars, translucent threshold band, thin peak line.
fn spectrum_preset() -> ChartScene {
    let mut scene = ChartScene::new();
    for (index, level) in [18.0, 42.0, 76.0, 34.0, 92.0, 58.0, 24.0, 70.0]
        .into_iter()
        .enumerate()
    {
        let x = 6.0 + index as f64 * 19.0;
        scene.fill_rect(
            Rect::new(x, 112.0 - level, x + 14.0, 112.0),
            solid(40, 190, 120),
        );
    }
    scene.fill_rect(
        Rect::new(0.0, 30.0, COMPARE_LOGICAL_W as f64, 33.0),
        translucent(240, 180, 40, 150),
    );
    scene.stroke_polyline(
        &[(2.0, 20.0), (158.0, 20.0)],
        Stroke::new(1.0),
        solid(250, 80, 60),
    );
    scene
}

/// Stroke-heavy shapes: thin diagonals, overlapping translucent rects, ring.
fn strokes_preset() -> ChartScene {
    let mut scene = ChartScene::new();
    scene.stroke_polyline(
        &[(4.0, 116.0), (156.0, 4.0)],
        Stroke::new(1.0),
        solid(250, 80, 60),
    );
    scene.stroke_polyline(
        &[(4.0, 4.0), (156.0, 116.0)],
        Stroke::new(2.5),
        solid(40, 80, 220),
    );
    scene.fill_rect(Rect::new(20.0, 30.0, 90.0, 90.0), translucent(220, 80, 40, 128));
    scene.fill_rect(Rect::new(60.0, 50.0, 130.0, 110.0), translucent(40, 120, 220, 128));
    scene.stroke_arc(
        80.0,
        60.0,
        24.0,
        0.0,
        std::f64::consts::TAU,
        Stroke::new(1.0),
        solid(30, 30, 30),
    );
    scene
}

fn swizzle_to_bgra(pixels: &mut [u8]) {
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
}

fn display_image(mut rgba: Vec<u8>, width: u32, height: u32) -> Arc<RenderImage> {
    // GPUI image atlases expect premultiplied BGRA (Metal is BGRA8Unorm);
    // both snapshot paths yield premultiplied RGBA, so swap R<->B.
    swizzle_to_bgra(&mut rgba);
    let image = RgbaImage::from_raw(width, height, rgba).expect("snapshot dims match payload");
    Arc::new(RenderImage::new(vec![Frame::new(image)]))
}

/// Render `preset` through both rasterizers at `scale` and compare.
/// `gpu_ms` only covers a successful snapshot; failures record `gpu_error`.
pub(super) fn run_backend_compare(preset: &str, scale: f32) -> BackendCompareResult {
    let scene = compare_preset_scene(preset);
    let width = ((COMPARE_LOGICAL_W * scale).ceil() as u32).max(1);
    let height = ((COMPARE_LOGICAL_H * scale).ceil() as u32).max(1);

    let cpu_start = Instant::now();
    let mut rasterizer = CpuRasterizer::new(width as u16, height as u16);
    let cpu_pixels = rasterizer.rasterize(&scene, width as u16, height as u16, scale);
    let cpu_ms = cpu_start.elapsed().as_secs_f64() * 1000.0;

    let gpu_start = Instant::now();
    let gpu_pixels = snapshot_scene_gpu(&scene, width, height, scale);
    let gpu_ms = gpu_start.elapsed().as_secs_f64() * 1000.0;

    let cpu = display_image(cpu_pixels.clone(), width, height);
    match gpu_pixels {
        Ok(pixels) => {
            let stats = compare_rgba(&cpu_pixels, &pixels, 8);
            let diff = stats
                .and(diff_image_rgba(&cpu_pixels, &pixels, 4.0))
                .map(|rgba| display_image(rgba, width, height));
            BackendCompareResult {
                cpu,
                gpu: Some(display_image(pixels, width, height)),
                diff,
                stats,
                gpu_error: None,
                cpu_ms,
                gpu_ms,
            }
        }
        Err(err) => BackendCompareResult {
            cpu,
            gpu: None,
            diff: None,
            stats: None,
            gpu_error: Some(err.to_string()),
            cpu_ms,
            gpu_ms,
        },
    }
}

#[cfg(test)]
mod backend_compare_tests {
    use super::{compare_preset_scene, run_backend_compare};
    use d3rs::vello2d::CpuRasterizer;

    #[test]
    fn presets_build_non_empty_scenes() {
        for preset in ["knob", "spectrum", "strokes", "unknown-falls-back-to-knob"] {
            let scene = compare_preset_scene(preset);
            assert!(!scene.is_empty(), "{preset} must build a scene");
            assert!(!scene.commands().is_empty());
        }
    }

    #[test]
    fn cpu_leg_matches_physical_size() {
        // GPU may be absent; the CPU leg alone must still fill the pixmap.
        let scene = compare_preset_scene("knob");
        let pixels = CpuRasterizer::new(320, 240).rasterize(&scene, 320, 240, 2.0);
        assert_eq!(pixels.len(), 320 * 240 * 4);
        assert!(
            pixels.chunks_exact(4).any(|pixel| pixel[3] > 0),
            "knob preset must paint"
        );
    }

    #[test]
    fn compare_runner_survives_missing_gpu() {
        // No adapter in CI: result still carries the CPU image and an error.
        let result = run_backend_compare("strokes", 1.0);
        assert!(result.stats.is_some() || result.gpu_error.is_some());
    }
}
