use d3rs::vello2d::kurbo::{Circle, Rect, Shape, Stroke};
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{ChartScene, CpuRasterizer};

fn px(buf: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
    let i = (y * w + x) * 4;
    [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
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
    assert!(r > 200 && g < 40 && b < 40 && a > 200, "interior: {r},{g},{b},{a}");
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
    scene.fill_rect(Rect::new(0.0, 0.0, 20.0, 20.0), Brush::Solid(Color::from_rgb8(0, 255, 0)));
    let mut rast = CpuRasterizer::new(32, 32);
    let _ = rast.rasterize(&scene, 32, 32);
    let buf = rast.rasterize(&ChartScene::new(), 64, 48);
    assert_eq!(buf.len(), 64 * 48 * 4);
    assert!(buf.iter().all(|&b| b == 0), "empty scene must clear the buffer");
}

use d3rs::prelude::*;
use d3rs::shape::{ScatterConfig, ScatterPoint, scatter_chart_scene};

#[test]
fn scatter_scene_golden_pixels() {
    // Fixed 100x80 linear scales, 3 points — the QA oracle for the port.
    // `no_stroke`: ScatterConfig's default stroke would add one ring command
    // per point; this test asserts the fill-only command count.
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
    assert_eq!(scene.len(), 3, "one fill command per point");

    let mut rast = CpuRasterizer::new(100, 80);
    let buf = rast.rasterize(&scene, 100, 80);
    // (50,40) is the center point: opaque red within the radius.
    let i = (40 * 100 + 50) * 4;
    assert!(buf[i] > 200 && buf[i + 3] > 200, "center pixel: {:?}", &buf[i..i + 4]);
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
    assert!((100..=160).contains(&alpha), "premultiplied alpha ~128, got {alpha}");
}

#[test]
fn scatter_scene_stroke_ring_paints_before_fill() {
    // Mirrors render_scatter's legacy stroke ring: a stroked circle of radius
    // r + w/2 emitted before the fill, using the stroke color un-opacified.
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
    assert_eq!(scene.len(), 2, "one stroke ring + one fill per point");
    assert!(
        matches!(scene.commands()[0], d3rs::vello2d::ChartCmd::Stroke { .. }),
        "stroke ring must be emitted before the fill"
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
fn deterministic_for_fixed_input() {
    // QA-oracle property: same scene -> identical bytes across runs.
    let mut scene = ChartScene::new();
    scene.fill_circle(25.0, 25.0, 10.0, Brush::Solid(Color::from_rgba8(10, 200, 100, 128)));
    scene.stroke_polyline(
        &[(0.0, 0.0), (49.0, 49.0)],
        Stroke::new(1.5),
        Brush::Solid(Color::from_rgb8(1, 2, 3)),
    );
    let mut a = CpuRasterizer::new(50, 50);
    let mut b = CpuRasterizer::new(50, 50);
    assert_eq!(a.rasterize(&scene, 50, 50), b.rasterize(&scene, 50, 50));
}
