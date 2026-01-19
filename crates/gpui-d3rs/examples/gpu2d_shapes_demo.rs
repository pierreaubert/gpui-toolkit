//! GPU-accelerated shape rendering demo
//!
//! Run with: cargo run --example gpu2d_shapes_demo --features gpu-2d

use d3rs::color::D3Color;
use d3rs::gpu2d::{
    AxisConfig, BarConfig, BarDatum, CurveType, GpuAxisTheme, GpuGridConfig, LineConfig, LinePoint,
    ScatterConfig, ScatterPoint, render_axis, render_bars, render_grid, render_line,
    render_scatter,
};
use d3rs::scale::LinearScale;
use gpui::*;

struct DemoView;

impl Render for DemoView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Create scales
        let x_scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 600.0);
        let y_scale = LinearScale::new().domain(0.0, 100.0).range(400.0, 0.0);

        // Create scatter data
        let scatter_data: Vec<ScatterPoint> = (0..30)
            .map(|i| {
                let x = (i as f64 * 3.5) + 5.0;
                let y = 50.0 + (i as f64 * 0.2).sin() * 30.0 + (i as f64 % 7.0) * 3.0;
                ScatterPoint::new(x, y)
            })
            .collect();

        let scatter_config = ScatterConfig::new()
            .fill_color(D3Color::from_hex(0xff6347))
            .point_radius(6.0)
            .opacity(0.9)
            .stroke_color(D3Color::from_hex(0xffffff))
            .stroke_width(1.5);

        // Create bar data
        let bar_x_scale = LinearScale::new().domain(0.0, 8.0).range(0.0, 600.0);
        let bar_y_scale = LinearScale::new().domain(0.0, 100.0).range(400.0, 0.0);

        let bar_data: Vec<BarDatum> = vec![
            BarDatum::new("A", 65.0),
            BarDatum::new("B", 85.0),
            BarDatum::new("C", 42.0),
            BarDatum::new("D", 73.0),
            BarDatum::new("E", 58.0),
            BarDatum::new("F", 91.0),
            BarDatum::new("G", 35.0),
        ];

        let bar_config = BarConfig::new()
            .fill_color(D3Color::from_hex(0x4682b4))
            .opacity(0.85)
            .border_radius(4.0)
            .bar_gap(8.0);

        // Create grid config
        let grid_config = GpuGridConfig::with_lines()
            .line_width(1.0)
            .line_opacity(0.15)
            .line_color([1.0, 1.0, 1.0, 1.0]);

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
                    .child("GPU-Accelerated Shape Rendering Demo"),
            )
            // Scatter plot
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Scatter Plot (GPU)"),
                            )
                            .child(
                                div()
                                    .h(px(200.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .relative()
                                    .child(render_grid(&x_scale, &y_scale, &grid_config))
                                    .child(render_scatter(
                                        &x_scale,
                                        &y_scale,
                                        &scatter_data,
                                        &scatter_config,
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Bar Chart (GPU)"),
                            )
                            .child(
                                div()
                                    .h(px(200.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .relative()
                                    .child(render_grid(&bar_x_scale, &bar_y_scale, &grid_config))
                                    .child(render_bars(
                                        &bar_x_scale,
                                        &bar_y_scale,
                                        &bar_data,
                                        600.0,
                                        200.0,
                                        &bar_config,
                                    )),
                            ),
                    ),
            )
            // Line charts
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Line Chart (GPU)"),
                            )
                            .child({
                                let line_data: Vec<LinePoint> = (0..25)
                                    .map(|i| {
                                        let x = i as f64 * 4.0;
                                        let y = 50.0 + (i as f64 * 0.3).sin() * 35.0;
                                        LinePoint::new(x, y)
                                    })
                                    .collect();

                                let line_config = LineConfig::new()
                                    .stroke_color(D3Color::from_hex(0x00ff88))
                                    .stroke_width(2.5)
                                    .curve(CurveType::Linear)
                                    .show_points(true)
                                    .point_radius(4.0)
                                    .point_fill_color(D3Color::from_hex(0xffffff));

                                let inner_grid = GpuGridConfig::with_lines()
                                    .line_width(1.0)
                                    .line_opacity(0.1)
                                    .line_color([1.0, 1.0, 1.0, 1.0]);

                                div()
                                    .h(px(150.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .relative()
                                    .child(render_grid(&x_scale, &y_scale, &inner_grid))
                                    .child(render_line(
                                        &x_scale,
                                        &y_scale,
                                        &line_data,
                                        &line_config,
                                    ))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Step Chart (GPU)"),
                            )
                            .child({
                                let step_data: Vec<LinePoint> = (0..12)
                                    .map(|i| {
                                        LinePoint::new(
                                            i as f64 * 9.0,
                                            20.0 + (i as f64 * 7.0) % 60.0,
                                        )
                                    })
                                    .collect();

                                let step_config = LineConfig::new()
                                    .stroke_color(D3Color::from_hex(0xff9944))
                                    .stroke_width(2.0)
                                    .curve(CurveType::Step);

                                let inner_grid = GpuGridConfig::with_lines()
                                    .line_width(1.0)
                                    .line_opacity(0.1)
                                    .line_color([1.0, 1.0, 1.0, 1.0]);

                                div()
                                    .h(px(150.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .relative()
                                    .child(render_grid(&x_scale, &y_scale, &inner_grid))
                                    .child(render_line(
                                        &x_scale,
                                        &y_scale,
                                        &step_data,
                                        &step_config,
                                    ))
                            }),
                    ),
            )
            // Grid demos
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Grid with Dots (GPU)"),
                            )
                            .child({
                                let dot_grid = GpuGridConfig::with_lines_and_dots()
                                    .line_width(1.0)
                                    .line_opacity(0.1)
                                    .line_color([0.5, 0.8, 1.0, 1.0]);

                                div()
                                    .h(px(150.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .child(render_grid(&x_scale, &y_scale, &dot_grid))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Dots Only (GPU)"),
                            )
                            .child({
                                let dots_only =
                                    GpuGridConfig::with_dots().line_color([1.0, 0.6, 0.2, 1.0]);

                                div()
                                    .h(px(150.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .child(render_grid(&x_scale, &y_scale, &dots_only))
                            }),
                    ),
            )
            // Axis rendering demo
            .child(
                div()
                    .flex_row()
                    .gap_4()
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Bottom Axis (GPU)"),
                            )
                            .child({
                                let axis_x_scale =
                                    LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);
                                let axis_config = AxisConfig::bottom()
                                    .with_ticks(5)
                                    .with_title("Frequency (Hz)");
                                let theme = GpuAxisTheme::dark();

                                div()
                                    .h(px(50.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .child(render_axis(&axis_x_scale, &axis_config, 500.0, &theme))
                            }),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xaaaaaa))
                                    .child("Left Axis (GPU)"),
                            )
                            .child({
                                let axis_y_scale =
                                    LinearScale::new().domain(0.0, 100.0).range(150.0, 0.0);
                                let axis_config = AxisConfig::left().with_ticks(5);
                                let theme = GpuAxisTheme::dark();

                                div()
                                    .h(px(150.0))
                                    .w(px(80.0))
                                    .border_1()
                                    .border_color(rgb(0x404040))
                                    .rounded_md()
                                    .overflow_hidden()
                                    .child(render_axis(&axis_y_scale, &axis_config, 150.0, &theme))
                            }),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(900.0), px(600.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("GPU 2D Shapes Demo".into()),
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
