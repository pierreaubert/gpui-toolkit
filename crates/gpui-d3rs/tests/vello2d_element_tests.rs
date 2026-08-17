use d3rs::vello2d::kurbo::Rect;
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{ChartScene, RasterBackend, VelloChartElement};
use d3rs::vello2d::{wgpu_draw_clip_src_rect, wgpu_draw_physical_size, wgpu_draw_scene_scale};
use gpui::{Bounds, Point, Size, px};

fn sample_scene() -> ChartScene {
    let mut scene = ChartScene::new();
    scene.fill_rect(
        Rect::new(0.0, 0.0, 4.0, 4.0),
        Brush::Solid(Color::from_rgb8(9, 9, 9)),
    );
    scene
}

#[test]
fn default_backend_is_auto() {
    let element = VelloChartElement::new(ChartScene::new());
    assert!(format!("{element:?}").contains("Auto"));
}

#[test]
fn explicit_cpu_backend_shows_in_debug() {
    let element = VelloChartElement::new(sample_scene()).backend(RasterBackend::Cpu);
    assert!(format!("{element:?}").contains("Cpu"));
}

#[test]
fn builder_supplies_scene_lazily() {
    // Scene starts empty; the builder fills it at first paint with real bounds.
    let element = VelloChartElement::with_builder(|w, h| {
        let mut scene = ChartScene::new();
        scene.fill_rect(
            Rect::new(0.0, 0.0, w as f64, h as f64),
            Brush::Solid(Color::from_rgb8(1, 2, 3)),
        );
        scene
    });
    assert!(format!("{element:?}").contains("builder"));
}

#[test]
fn physical_size_scales_and_clamps() {
    assert_eq!(wgpu_draw_physical_size(100.0, 50.0, 2.0), [200, 100]);
    assert_eq!(wgpu_draw_physical_size(0.0, -3.0, 1.0), [1, 1]);
}

#[test]
fn scene_scale_maps_logical_to_physical() {
    // Retina: 400x300 logical element drawn into an 800x600 texture.
    assert_eq!(
        wgpu_draw_scene_scale(400.0, 300.0, 800.0, 600.0),
        [2.0, 2.0]
    );
    // Zero logical size (element not painted yet) falls back to unit scale.
    assert_eq!(wgpu_draw_scene_scale(0.0, 0.0, 800.0, 600.0), [1.0, 1.0]);
}

#[test]
fn clip_src_rect_offsets_into_full_element() {
    let full = Bounds::new(
        Point::new(px(10.0), px(20.0)),
        Size::new(px(100.0), px(80.0)),
    );
    // Content mask cuts 10 px off the left and shrinks height.
    let clipped = Bounds::new(
        Point::new(px(20.0), px(20.0)),
        Size::new(px(90.0), px(40.0)),
    );
    let (origin, size) = wgpu_draw_clip_src_rect(full, clipped, 2.0);
    assert_eq!(origin, [20.0, 0.0]);
    assert_eq!(size, [180.0, 80.0]);
}

#[test]
fn clip_src_rect_unclipped_covers_whole_texture() {
    let full = Bounds::new(Point::new(px(-5.0), px(7.0)), Size::new(px(64.0), px(48.0)));
    let (origin, size) = wgpu_draw_clip_src_rect(full, full, 1.0);
    assert_eq!(origin, [0.0, 0.0]);
    assert_eq!(size, [64.0, 48.0]);
}
