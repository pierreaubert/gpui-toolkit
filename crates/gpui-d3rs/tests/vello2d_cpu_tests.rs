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
