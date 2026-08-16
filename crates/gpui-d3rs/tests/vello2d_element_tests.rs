use d3rs::vello2d::kurbo::Rect;
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::wgpu_draw_physical_size;
use d3rs::vello2d::{ChartScene, RasterBackend, VelloChartElement};

fn sample_scene() -> ChartScene {
    let mut scene = ChartScene::new();
    scene.fill_rect(Rect::new(0.0, 0.0, 4.0, 4.0), Brush::Solid(Color::from_rgb8(9, 9, 9)));
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
