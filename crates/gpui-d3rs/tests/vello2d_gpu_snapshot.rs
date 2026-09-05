//! CPU-vs-GPU pixel agreement for the vello scene IR, split by primitive.
//!
//! Each scene renders one primitive class so the printed table attributes
//! divergence precisely: axis-aligned opaque bars pin pixel-center and
//! coverage conventions, the translucent band pins premultiplied-alpha
//! agreement, the wedge pins curve flattening, and thin strokes pin
//! fringe-encoding agreement. The engines agree to UNORM rounding on all of
//! these — the dominant historical divergence was straight (GPU) vs
//! premultiplied (CPU) alpha, fixed by converting the snapshot readback on
//! capture. The downsampled metric adds structural-error sensitivity on top.
//!
//! On any bound failure the CPU, GPU, and diff images land in the temp dir
//! for eyeballing. Skips gracefully where no wgpu adapter exists; the
//! component-lab backend-compare view is the interactive consumer.

use d3rs::vello2d::kurbo::{Rect, Stroke};
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{
    ChartScene, CpuRasterizer, SnapshotError, compare_rgba, compare_rgba_downsampled2,
    diff_image_rgba, snapshot_scene_gpu,
};

const SCALE: f32 = 2.0;
const PHYS: u32 = 128;

fn bars_scene() -> ChartScene {
    let mut scene = ChartScene::new();
    for (index, level) in [10.0, 22.0, 40.0, 18.0, 48.0].into_iter().enumerate() {
        let x = 4.0 + index as f64 * 11.0;
        scene.fill_rect(
            Rect::new(x, 60.0 - level, x + 8.0, 60.0),
            Brush::Solid(Color::from_rgb8(40, 190, 120)),
        );
    }
    scene
}

fn band_scene() -> ChartScene {
    let mut scene = ChartScene::new();
    scene.fill_rect(
        Rect::new(0.0, 18.0, 64.0, 20.0),
        Brush::Solid(Color::from_rgba8(240, 180, 40, 150)),
    );
    scene
}

fn wedge_scene() -> ChartScene {
    let mut scene = ChartScene::new();
    scene.fill_wedge(
        32.0,
        32.0,
        16.0,
        -1.0,
        1.6,
        Brush::Solid(Color::from_rgb8(70, 170, 240)),
    );
    scene
}

fn strokes_scene() -> ChartScene {
    let mut scene = ChartScene::new();
    scene.stroke_arc(
        32.0,
        32.0,
        22.0,
        -2.4,
        4.8,
        Stroke::new(5.0),
        Brush::Solid(Color::from_rgb8(150, 150, 160)),
    );
    scene.stroke_arc(
        32.0,
        32.0,
        26.0,
        0.0,
        std::f64::consts::TAU,
        Stroke::new(1.0),
        Brush::Solid(Color::from_rgb8(245, 245, 245)),
    );
    scene.stroke_polyline(
        &[(2.0, 62.0), (62.0, 2.0)],
        Stroke::new(1.0),
        Brush::Solid(Color::from_rgb8(250, 80, 60)),
    );
    scene
}

struct ShapeStats {
    std_mean: f64,
    std_frac: f64,
    down_mean: f64,
    down_frac: f64,
}

fn dump_png(name: &str, kind: &str, rgba: &[u8]) {
    let path = std::env::temp_dir().join(format!("vello_compare_{name}_{kind}.png"));
    match image::RgbaImage::from_raw(PHYS, PHYS, rgba.to_vec()) {
        Some(image) => {
            if let Err(err) = image.save(&path) {
                eprintln!("  {name}: dump {kind} failed: {err}");
            } else {
                eprintln!("  {name}: {kind} -> {}", path.display());
            }
        }
        None => eprintln!("  {name}: dump {kind} failed: bad dims"),
    }
}

/// Renders one scene on both engines, prints both metrics, dumps artifacts.
/// `None` when no adapter exists so the test can skip gracefully.
fn compare_shape(name: &str, scene: &ChartScene) -> Option<ShapeStats> {
    let cpu_pixels = CpuRasterizer::new(PHYS as u16, PHYS as u16).rasterize(
        scene,
        PHYS as u16,
        PHYS as u16,
        SCALE,
    );
    let gpu_pixels = match snapshot_scene_gpu(scene, PHYS, PHYS, SCALE) {
        Ok(pixels) => pixels,
        Err(SnapshotError::NoAdapter) => return None,
        Err(err) => panic!("GPU snapshot failed: {err}"),
    };
    let std = compare_rgba(&cpu_pixels, &gpu_pixels, 8).expect("same-shape buffers compare");
    let down = compare_rgba_downsampled2(
        &cpu_pixels,
        &gpu_pixels,
        PHYS as usize,
        PHYS as usize,
        8.0,
    )
    .expect("even dims compare");
    eprintln!(
        "  {name:8} std(mean={:6.3} max={:3} frac={:.4}) down(mean={:6.3} frac={:.4})",
        std.mean_abs, std.max_abs, std.frac_over_tol, down.mean_abs, down.frac_over_tol
    );
    dump_png(name, "cpu", &cpu_pixels);
    dump_png(name, "gpu", &gpu_pixels);
    if let Some(diff) = diff_image_rgba(&cpu_pixels, &gpu_pixels, 4.0) {
        dump_png(name, "diff", &diff);
    }
    Some(ShapeStats {
        std_mean: std.mean_abs,
        std_frac: std.frac_over_tol,
        down_mean: down.mean_abs,
        down_frac: down.frac_over_tol,
    })
}

#[test]
fn gpu_snapshot_agrees_with_cpu_per_shape() {
    eprintln!("cpu-vs-gpu per primitive (tol 8):");
    let scenes = [
        ("bars", bars_scene()),
        ("band", band_scene()),
        ("wedge", wedge_scene()),
        ("strokes", strokes_scene()),
    ];
    let mut stats = Vec::new();
    for (name, scene) in &scenes {
        match compare_shape(name, scene) {
            Some(stat) => stats.push(stat),
            None => {
                eprintln!("SKIP: no wgpu adapter for GPU snapshot agreement test");
                return;
            }
        }
    }
    let [bars, band, wedge, strokes] = stats.as_slice() else {
        unreachable!("four scenes compared");
    };
    // Observed post-fix floors (Apple Silicon): bars/band 0.000, wedge
    // 0.002, strokes 0.043/0.0017. Bounds sit ~10x above with room for
    // cross-GPU float variance; a broken scale/offset/alpha transform reads
    // orders of magnitude higher (pre-fix: band 1.48, strokes 8.2).
    assert!(bars.std_mean < 0.5, "bars std mean {}", bars.std_mean);
    assert!(bars.std_frac < 0.01, "bars std frac {}", bars.std_frac);
    assert!(bars.down_mean < 0.2, "bars down mean {}", bars.down_mean);
    assert!(band.std_mean < 0.5, "band std mean {}", band.std_mean);
    assert!(band.std_frac < 0.01, "band std frac {}", band.std_frac);
    assert!(band.down_mean < 0.2, "band down mean {}", band.down_mean);
    assert!(wedge.std_mean < 0.5, "wedge std mean {}", wedge.std_mean);
    assert!(wedge.std_frac < 0.01, "wedge std frac {}", wedge.std_frac);
    assert!(wedge.down_mean < 0.2, "wedge down mean {}", wedge.down_mean);
    assert!(
        strokes.std_mean < 1.0,
        "strokes std mean {}",
        strokes.std_mean
    );
    assert!(
        strokes.std_frac < 0.02,
        "strokes std frac {}",
        strokes.std_frac
    );
    assert!(
        strokes.down_mean < 0.5,
        "strokes down mean {}",
        strokes.down_mean
    );
    assert!(
        strokes.down_frac < 0.05,
        "strokes down frac {}",
        strokes.down_frac
    );
}
