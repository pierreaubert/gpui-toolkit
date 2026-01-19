//! GPU-accelerated 2D chart rendering demo
//!
//! Run with: cargo run --example gpu2d_demo --features gpu-2d

use d3rs::gpu2d::Chart2DElement;
use d3rs::gpu2d::primitives::Rect;
use gpui::*;

struct DemoView;

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
                    .child("GPU-Accelerated 2D Chart Rendering Demo"),
            )
            .child(
                div()
                    .flex_1()
                    .border_1()
                    .border_color(rgb(0x404040))
                    .rounded_md()
                    .overflow_hidden()
                    .child(
                        Chart2DElement::new(|renderer, bounds| {
                            let width: f32 = bounds.size.width.into();
                            let height: f32 = bounds.size.height.into();

                            // Draw grid
                            let grid_color = [0.3, 0.3, 0.3, 1.0];
                            let grid_spacing = 50.0;
                            let mut x = grid_spacing;
                            while x < width {
                                renderer.draw_line(x, 0.0, x, height, 1.0, grid_color);
                                x += grid_spacing;
                            }
                            let mut y = grid_spacing;
                            while y < height {
                                renderer.draw_line(0.0, y, width, y, 1.0, grid_color);
                                y += grid_spacing;
                            }

                            // Draw some rectangles
                            renderer.draw_rect(
                                Rect::new(50.0, 100.0, 100.0, 60.0),
                                [0.2, 0.6, 0.9, 1.0], // Blue
                                8.0,                  // corner radius
                            );
                            renderer.draw_rect(
                                Rect::new(200.0, 80.0, 80.0, 100.0),
                                [0.9, 0.3, 0.3, 1.0], // Red
                                4.0,
                            );
                            renderer.draw_rect(
                                Rect::new(330.0, 120.0, 120.0, 50.0),
                                [0.3, 0.8, 0.3, 1.0], // Green
                                0.0,                  // sharp corners
                            );

                            // Draw some lines
                            let line_color = [1.0, 0.8, 0.2, 1.0]; // Yellow
                            renderer.draw_line(50.0, 250.0, 150.0, 300.0, 3.0, line_color);
                            renderer.draw_line(150.0, 300.0, 250.0, 260.0, 3.0, line_color);
                            renderer.draw_line(250.0, 260.0, 350.0, 320.0, 3.0, line_color);
                            renderer.draw_line(350.0, 320.0, 450.0, 280.0, 3.0, line_color);

                            // Draw a line chart pattern
                            let data_color = [0.4, 0.8, 1.0, 1.0]; // Cyan
                            let points: Vec<(f32, f32)> = (0..20)
                                .map(|i| {
                                    let x = 50.0 + i as f32 * 20.0;
                                    let y = 400.0 + (i as f32 * 0.5).sin() * 50.0;
                                    (x, y)
                                })
                                .collect();

                            for i in 1..points.len() {
                                renderer.draw_line(
                                    points[i - 1].0,
                                    points[i - 1].1,
                                    points[i].0,
                                    points[i].1,
                                    2.0,
                                    data_color,
                                );
                            }

                            // Draw scatter points
                            for (x, y) in &points {
                                renderer.draw_circle(*x, *y, 5.0, [1.0, 0.5, 0.0, 1.0]);
                                // Orange
                            }

                            // Draw some standalone circles
                            renderer.draw_circle(500.0, 150.0, 30.0, [0.8, 0.2, 0.8, 1.0]); // Purple
                            renderer.draw_circle(580.0, 150.0, 20.0, [0.2, 0.8, 0.8, 1.0]); // Teal
                            renderer.draw_circle(640.0, 150.0, 40.0, [0.8, 0.8, 0.2, 0.7]);
                            // Semi-transparent yellow
                        })
                        .background_color([0.12, 0.12, 0.12, 1.0]),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(800.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("GPU 2D Chart Demo".into()),
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
