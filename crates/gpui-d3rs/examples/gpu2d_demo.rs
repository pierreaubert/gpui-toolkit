//! Vello-backed 2D chart rendering demo.
//!
//! Run with: `cargo run --example gpu2d_demo --features vello-gpui`

use d3rs::vello2d::kurbo::{Rect, Stroke};
use d3rs::vello2d::peniko::{Brush, Color};
use d3rs::vello2d::{ChartScene, VelloChartElement};
use gpui::*;

struct DemoView;

fn brush(rgba: [f32; 4]) -> Brush {
    Brush::Solid(Color::new(rgba))
}

fn demo_scene(width: f32, height: f32) -> ChartScene {
    let mut scene = ChartScene::new();
    scene.fill_rect(
        Rect::new(0.0, 0.0, width as f64, height as f64),
        brush([0.12, 0.12, 0.12, 1.0]),
    );

    let grid = brush([0.3, 0.3, 0.3, 1.0]);
    let mut x = 50.0;
    while x < width {
        scene.stroke_polyline(
            &[(x as f64, 0.0), (x as f64, height as f64)],
            Stroke::new(1.0),
            grid.clone(),
        );
        x += 50.0;
    }
    let mut y = 50.0;
    while y < height {
        scene.stroke_polyline(
            &[(0.0, y as f64), (width as f64, y as f64)],
            Stroke::new(1.0),
            grid.clone(),
        );
        y += 50.0;
    }

    scene.fill_rounded_rect(
        Rect::new(50.0, 100.0, 150.0, 160.0),
        8.0,
        brush([0.2, 0.6, 0.9, 1.0]),
    );
    scene.fill_rounded_rect(
        Rect::new(200.0, 80.0, 280.0, 180.0),
        4.0,
        brush([0.9, 0.3, 0.3, 1.0]),
    );
    scene.fill_rect(
        Rect::new(330.0, 120.0, 450.0, 170.0),
        brush([0.3, 0.8, 0.3, 1.0]),
    );

    scene.stroke_polyline(
        &[
            (50.0, 250.0),
            (150.0, 300.0),
            (250.0, 260.0),
            (350.0, 320.0),
            (450.0, 280.0),
        ],
        Stroke::new(3.0),
        brush([1.0, 0.8, 0.2, 1.0]),
    );

    let points: Vec<_> = (0..100)
        .map(|i| {
            let x = 50.0 + i as f32 * 6.0;
            let y = 400.0 + (i as f32 * 0.5).sin() * 50.0;
            (x, y)
        })
        .collect();
    let data_path: Vec<_> = points.iter().map(|&(x, y)| (x as f64, y as f64)).collect();
    scene.stroke_polyline(&data_path, Stroke::new(2.0), brush([0.4, 0.8, 1.0, 1.0]));
    for (x, y) in points {
        scene.fill_circle(x as f64, y as f64, 5.0, brush([1.0, 0.5, 0.0, 1.0]));
    }

    scene.fill_circle(500.0, 150.0, 30.0, brush([0.8, 0.2, 0.8, 1.0]));
    scene.fill_circle(580.0, 150.0, 20.0, brush([0.2, 0.8, 0.8, 1.0]));
    scene.fill_circle(640.0, 150.0, 40.0, brush([0.8, 0.8, 0.2, 0.7]));
    scene
}

impl Render for DemoView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x1e1e1e))
            .p_4()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .text_color(rgb(0xffffff))
                    .child("Vello-backed 2D Chart Rendering Demo"),
            )
            .child(
                div()
                    .flex_1()
                    .border_1()
                    .border_color(rgb(0x404040))
                    .rounded_md()
                    .overflow_hidden()
                    .child(VelloChartElement::with_builder(demo_scene).absolute()),
            )
    }
}

fn main() {
    let platform = gpui_miniapp::current_platform().expect("failed to initialize GPUI platform");
    Application::with_platform(platform).run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Vello 2D Chart Demo".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(|_| DemoView),
        )
        .unwrap();

        cx.activate(true);
    });
}
