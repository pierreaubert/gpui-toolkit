use d3rs::vello2d::kurbo::{Circle, Rect, Shape, Stroke};
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{ChartScene, CpuRasterizer};

fn px(buf: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
    let i = (y * w + x) * 4;
    [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
}

fn assert_deterministic_ink(scene: &ChartScene, width: u16, height: u16, label: &str) {
    let mut first = CpuRasterizer::new(width, height);
    let mut second = CpuRasterizer::new(width, height);
    let first_pixels = first.rasterize(scene, width, height);
    let second_pixels = second.rasterize(scene, width, height);
    assert_eq!(first_pixels, second_pixels, "{label} must be deterministic");
    assert!(
        first_pixels.chunks_exact(4).any(|pixel| pixel[3] > 0),
        "{label} must paint at least one pixel"
    );
}

#[test]
fn filled_rect_paints_interior_and_leaves_exterior_transparent() {
    let mut scene = ChartScene::new();
    scene.fill_rect(
        Rect::new(10.0, 10.0, 50.0, 50.0),
        Brush::Solid(Color::from_rgb8(255, 0, 0)),
    );
    let mut rast = CpuRasterizer::new(100, 100);
    let buf = rast.rasterize(&scene, 100, 100);
    assert_eq!(buf.len(), 100 * 100 * 4);
    let [r, g, b, a] = px(&buf, 100, 30, 30);
    assert!(
        r > 200 && g < 40 && b < 40 && a > 200,
        "interior: {r},{g},{b},{a}"
    );
    assert_eq!(px(&buf, 100, 2, 2)[3], 0, "corner must stay transparent");
}

#[test]
fn stroked_circle_paints_ring() {
    let mut scene = ChartScene::new();
    scene.stroke_path(
        Circle::new((50.0, 50.0), 20.0).to_path(0.1),
        Stroke::new(4.0),
        Brush::Solid(Color::from_rgb8(0, 0, 255)),
    );
    let mut rast = CpuRasterizer::new(100, 100);
    let buf = rast.rasterize(&scene, 100, 100);
    // Point on the ring (rightmost): opaque blue.
    let ring = px(&buf, 100, 70, 50);
    assert!(ring[2] > 150 && ring[3] > 150, "ring: {ring:?}");
    // Center: transparent.
    assert_eq!(px(&buf, 100, 50, 50)[3], 0);
}

#[test]
fn resize_reallocates_and_clears() {
    let mut scene = ChartScene::new();
    scene.fill_rect(
        Rect::new(0.0, 0.0, 20.0, 20.0),
        Brush::Solid(Color::from_rgb8(0, 255, 0)),
    );
    let mut rast = CpuRasterizer::new(32, 32);
    let _ = rast.rasterize(&scene, 32, 32);
    let buf = rast.rasterize(&ChartScene::new(), 64, 48);
    assert_eq!(buf.len(), 64 * 48 * 4);
    assert!(
        buf.iter().all(|&b| b == 0),
        "empty scene must clear the buffer"
    );
}

use d3rs::prelude::*;
use d3rs::shape::{ScatterConfig, ScatterPoint, scatter_chart_scene};

#[test]
fn scatter_scene_golden_pixels() {
    // Fixed 100x80 linear scales, 3 points — the QA oracle for the port.
    // `no_stroke`: ScatterConfig's default stroke would add one batched ring
    // command; this test asserts the fill-only command count.
    let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 100.0);
    let y_scale = LinearScale::new().domain(0.0, 80.0).range(80.0, 0.0);
    let data = vec![
        ScatterPoint::new(50.0, 40.0),
        ScatterPoint::new(10.0, 8.0),
        ScatterPoint::new(90.0, 72.0),
    ];
    let config = ScatterConfig::new()
        .fill_color(D3Color::from_hex(0xff0000))
        .point_radius(3.0)
        .opacity(1.0)
        .no_stroke();
    let scene = scatter_chart_scene(&x_scale, &y_scale, &data, &config, 100.0, 80.0);
    assert_eq!(scene.len(), 1, "all points batched into one fill command");

    let mut rast = CpuRasterizer::new(100, 80);
    let buf = rast.rasterize(&scene, 100, 80);
    // (50,40) is the center point: opaque red within the radius.
    let i = (40 * 100 + 50) * 4;
    assert!(
        buf[i] > 200 && buf[i + 3] > 200,
        "center pixel: {:?}",
        &buf[i..i + 4]
    );
    // Far from all points: transparent.
    assert_eq!(buf[3], 0);
}

#[test]
fn scatter_scene_respects_opacity() {
    let x_scale = LinearScale::new().domain(0.0, 10.0).range(0.0, 20.0);
    let y_scale = LinearScale::new().domain(0.0, 10.0).range(20.0, 0.0);
    let data = vec![ScatterPoint::new(5.0, 5.0)];
    let config = ScatterConfig::new()
        .fill_color(D3Color::from_hex(0x00ff00))
        .point_radius(4.0)
        .opacity(0.5);
    let scene = scatter_chart_scene(&x_scale, &y_scale, &data, &config, 20.0, 20.0);
    let mut rast = CpuRasterizer::new(20, 20);
    let buf = rast.rasterize(&scene, 20, 20);
    let alpha = buf[(10 * 20 + 10) * 4 + 3];
    assert!(
        (100..=160).contains(&alpha),
        "premultiplied alpha ~128, got {alpha}"
    );
}

#[test]
fn scatter_scene_stroke_ring_paints_before_fill() {
    // Mirrors render_scatter's legacy stroke ring: one batched stroke path of
    // circles with radius r + w/2 emitted before the single batched fill,
    // using the stroke color un-opacified.
    let x_scale = LinearScale::new().domain(0.0, 10.0).range(0.0, 40.0);
    let y_scale = LinearScale::new().domain(0.0, 10.0).range(40.0, 0.0);
    let data = vec![ScatterPoint::new(5.0, 5.0)];
    let config = ScatterConfig::new()
        .fill_color(D3Color::from_hex(0xff0000))
        .stroke_color(D3Color::from_hex(0x0000ff))
        .stroke_width(2.0)
        .point_radius(4.0)
        .opacity(1.0);
    let scene = scatter_chart_scene(&x_scale, &y_scale, &data, &config, 40.0, 40.0);
    assert_eq!(
        scene.len(),
        2,
        "one batched stroke command + one batched fill command"
    );
    assert!(
        matches!(scene.commands()[0], d3rs::vello2d::ChartCmd::Stroke { .. }),
        "stroke ring must be emitted before the fill"
    );
    assert!(
        matches!(scene.commands()[1], d3rs::vello2d::ChartCmd::Fill { .. }),
        "fill must follow the stroke"
    );

    let mut rast = CpuRasterizer::new(40, 40);
    let buf = rast.rasterize(&scene, 40, 40);
    // Center: red fill wins over the ring.
    let center = px(&buf, 40, 20, 20);
    assert!(center[0] > 200 && center[2] < 60, "center: {center:?}");
    // On the ring (radius 5 -> point at (25, 20)): blue stroke visible.
    let ring = px(&buf, 40, 25, 20);
    assert!(ring[2] > 150 && ring[3] > 150, "ring: {ring:?}");
}

#[test]
fn scatter_scene_overlapping_translucent_points_blend_once() {
    // Regression oracle for legacy compositing parity: all circles are
    // batched into ONE fill path, so a pixel covered by two overlapping
    // translucent markers must reach the same premultiplied alpha as a pixel
    // covered by only one — not be blended twice (darker overlap).
    let x_scale = LinearScale::new().domain(0.0, 40.0).range(0.0, 40.0);
    let y_scale = LinearScale::new().domain(0.0, 32.0).range(32.0, 0.0);
    let data = vec![ScatterPoint::new(16.0, 16.0), ScatterPoint::new(24.0, 16.0)];
    let config = ScatterConfig::new()
        .fill_color(D3Color::from_hex(0xff0000))
        .point_radius(8.0)
        .opacity(0.5)
        .no_stroke();
    let scene = scatter_chart_scene(&x_scale, &y_scale, &data, &config, 40.0, 32.0);
    assert_eq!(scene.len(), 1, "both circles batched into one fill command");

    let mut rast = CpuRasterizer::new(40, 32);
    let buf = rast.rasterize(&scene, 40, 32);
    // (12,16): inside only the first circle (center 16, r 8).
    let single = px(&buf, 40, 12, 16);
    // (20,16): inside BOTH circles (4px from each center).
    let overlap = px(&buf, 40, 20, 16);
    assert!(
        (100..=160).contains(&single[3]),
        "single-covered alpha ~128, got {single:?}"
    );
    assert!(
        (100..=160).contains(&overlap[3]),
        "overlap must blend once, got {overlap:?}"
    );
    assert!(
        single[3].abs_diff(overlap[3]) <= 4,
        "overlap alpha must match single-covered alpha: {single:?} vs {overlap:?}"
    );
}

#[test]
fn deterministic_for_fixed_input() {
    // QA-oracle property: same scene -> identical bytes across runs.
    let mut scene = ChartScene::new();
    scene.fill_circle(
        25.0,
        25.0,
        10.0,
        Brush::Solid(Color::from_rgba8(10, 200, 100, 128)),
    );
    scene.stroke_polyline(
        &[(0.0, 0.0), (49.0, 49.0)],
        Stroke::new(1.5),
        Brush::Solid(Color::from_rgb8(1, 2, 3)),
    );
    let mut a = CpuRasterizer::new(50, 50);
    let mut b = CpuRasterizer::new(50, 50);
    assert_eq!(a.rasterize(&scene, 50, 50), b.rasterize(&scene, 50, 50));
}

#[test]
fn cpu_fixture_covers_line_area_bars_and_grouped_bars() {
    let mut line = ChartScene::new();
    line.stroke_polyline(
        &[(4.0, 28.0), (12.0, 12.0), (20.0, 20.0), (28.0, 6.0)],
        Stroke::new(2.0),
        Brush::Solid(Color::from_rgb8(0, 80, 220)),
    );
    assert_deterministic_ink(&line, 32, 32, "line");

    let mut area = ChartScene::new();
    let mut area_path = d3rs::vello2d::kurbo::BezPath::new();
    area_path.move_to((2.0, 28.0));
    area_path.line_to((2.0, 20.0));
    area_path.line_to((10.0, 10.0));
    area_path.line_to((18.0, 18.0));
    area_path.line_to((26.0, 8.0));
    area_path.line_to((30.0, 28.0));
    area_path.close_path();
    area.fill_path(
        area_path,
        Brush::Solid(Color::from_rgba8(20, 180, 100, 180)),
    );
    assert_deterministic_ink(&area, 32, 32, "area");

    let mut bars = ChartScene::new();
    for (index, value) in [8.0, 16.0, 24.0].into_iter().enumerate() {
        let x = 2.0 + index as f64 * 9.0;
        bars.fill_rect(
            Rect::new(x, 30.0 - value, x + 6.0, 30.0),
            Brush::Solid(Color::from_rgb8(220, 80, 40)),
        );
    }
    for (index, value) in [12.0, 20.0, 10.0].into_iter().enumerate() {
        let x = 4.0 + index as f64 * 9.0;
        bars.fill_rect(
            Rect::new(x, 30.0 - value, x + 2.5, 30.0),
            Brush::Solid(Color::from_rgb8(40, 120, 220)),
        );
    }
    assert_deterministic_ink(&bars, 32, 32, "bars and grouped bars");
}

#[test]
fn cpu_fixture_covers_boxplot_pie_donut_and_treemap() {
    let mut boxplot = ChartScene::new();
    boxplot.fill_rect(
        Rect::new(10.0, 10.0, 22.0, 22.0),
        Brush::Solid(Color::from_rgb8(220, 120, 50)),
    );
    boxplot.stroke_polyline(
        &[(16.0, 4.0), (16.0, 28.0)],
        Stroke::new(1.0),
        Brush::Solid(Color::from_rgb8(40, 40, 40)),
    );
    assert_deterministic_ink(&boxplot, 40, 40, "boxplot");

    let mut pie = ChartScene::new();
    pie.fill_wedge(
        20.0,
        20.0,
        16.0,
        0.0,
        std::f64::consts::FRAC_PI_2,
        Brush::Solid(Color::from_rgb8(240, 80, 80)),
    );
    pie.fill_wedge(
        20.0,
        20.0,
        16.0,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
        Brush::Solid(Color::from_rgb8(80, 160, 240)),
    );
    pie.stroke_arc(
        20.0,
        20.0,
        8.0,
        0.0,
        std::f64::consts::TAU,
        Stroke::new(3.0),
        Brush::Solid(Color::from_rgb8(250, 200, 40)),
    );
    assert_deterministic_ink(&pie, 40, 40, "pie and donut");

    let mut treemap = ChartScene::new();
    treemap.fill_rounded_rect(
        Rect::new(1.0, 1.0, 39.0, 39.0),
        3.0,
        Brush::Solid(Color::from_rgb8(80, 100, 180)),
    );
    treemap.fill_rounded_rect(
        Rect::new(4.0, 4.0, 20.0, 36.0),
        2.0,
        Brush::Solid(Color::from_rgba8(220, 180, 60, 220)),
    );
    assert_deterministic_ink(&treemap, 40, 40, "treemap");
}

#[test]
fn cpu_fixture_covers_heatmap_contours_and_isolines() {
    let mut heatmap = ChartScene::new();
    for row in 0..4 {
        for column in 0..4 {
            let color = if (row + column) % 2 == 0 {
                Color::from_rgb8(40, 160, 220)
            } else {
                Color::from_rgb8(220, 80, 120)
            };
            let x = column as f64 * 8.0;
            let y = row as f64 * 8.0;
            heatmap.fill_rect(Rect::new(x, y, x + 8.0, y + 8.0), Brush::Solid(color));
        }
    }
    assert_deterministic_ink(&heatmap, 32, 32, "heatmap");

    let mut isolines = ChartScene::new();
    for offset in [4.0, 12.0, 20.0, 28.0] {
        isolines.stroke_polyline(
            &[
                (2.0, offset),
                (10.0, offset - 3.0),
                (20.0, offset + 3.0),
                (30.0, offset),
            ],
            Stroke::new(1.0),
            Brush::Solid(Color::from_rgb8(30, 30, 30)),
        );
    }
    isolines.fill_wedge(
        16.0,
        16.0,
        8.0,
        0.0,
        std::f64::consts::PI,
        Brush::Solid(Color::from_rgba8(30, 200, 100, 100)),
    );
    assert_deterministic_ink(&isolines, 32, 32, "contours and isolines");
}

#[test]
fn cpu_fixture_covers_audio_spectrum_meters_and_controls() {
    let mut audio = ChartScene::new();
    // Spectrum bars with threshold bands.
    for (index, level) in [8.0, 14.0, 22.0, 12.0, 26.0].into_iter().enumerate() {
        let x = 2.0 + index as f64 * 6.0;
        audio.fill_rect(
            Rect::new(x, 30.0 - level, x + 4.0, 30.0),
            Brush::Solid(Color::from_rgb8(40, 190, 120)),
        );
    }
    audio.fill_rect(
        Rect::new(0.0, 10.0, 32.0, 11.0),
        Brush::Solid(Color::from_rgba8(240, 180, 40, 150)),
    );
    // Vertical and horizontal meter segments/peak lines.
    audio.fill_rect(
        Rect::new(24.0, 4.0, 28.0, 28.0),
        Brush::Solid(Color::from_rgb8(70, 130, 230)),
    );
    audio.fill_rect(
        Rect::new(2.0, 34.0, 28.0, 38.0),
        Brush::Solid(Color::from_rgb8(70, 130, 230)),
    );
    audio.stroke_polyline(
        &[(24.0, 8.0), (28.0, 8.0)],
        Stroke::new(1.0),
        Brush::Solid(Color::from_rgb8(250, 80, 60)),
    );
    // Potentiometer arc/ticks and a volume knob ring/value.
    audio.stroke_arc(
        16.0,
        58.0,
        12.0,
        -2.4,
        4.8,
        Stroke::new(3.0),
        Brush::Solid(Color::from_rgb8(150, 150, 160)),
    );
    audio.fill_wedge(
        16.0,
        58.0,
        9.0,
        -1.0,
        1.6,
        Brush::Solid(Color::from_rgb8(70, 170, 240)),
    );
    audio.stroke_arc(
        16.0,
        58.0,
        5.0,
        0.0,
        std::f64::consts::TAU,
        Stroke::new(1.0),
        Brush::Solid(Color::from_rgb8(245, 245, 245)),
    );
    assert_deterministic_ink(&audio, 40, 72, "audio visuals");
}
