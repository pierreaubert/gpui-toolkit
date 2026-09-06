//! Ridgeline Plot — Observable example using d3rs::examples::ridgeline
//!
//! Shows monthly temperature distributions as overlapping area charts.
//! Uses weather.csv data grouped by month.
//!
//! Source: <https://observablehq.com/@d3/ridgeline-plot>

use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::color::SequentialScheme;
use d3rs::scale::{LinearScale, Scale};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const WEATHER_CSV: &str = include_str!("../../data/weather.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let monthly = d3rs::examples::ridgeline::load_csv(WEATHER_CSV);
    let result = d3rs::examples::ridgeline::compute(&monthly);

    let width = result.width;
    let height = result.height;

    let scheme = SequentialScheme::blues();
    let n = result.area_paths.len();

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    for (i, (_, path)) in result.area_paths.iter().enumerate() {
        d3_paths.push(path.clone());
        let t = 0.3 + 0.5 * (i as f64 / n.max(1) as f64);
        let color = scheme.get(t);
        all_colors.push(chart_colors::ink_rgba(&ui_theme, color.to_rgba()).opacity(0.7));
    }
    // Dark ridge outlines like the official example.
    let outline_color: Hsla = chart_colors::ink_hex(&ui_theme, 0x2c5f8a);

    // X-axis ticks
    let margin_left = 60.0;
    let margin_right = 20.0;
    let x_scale = LinearScale::new()
        .domain(result.x_domain[0], result.x_domain[1])
        .range(margin_left, width - margin_right);

    let x_range = result.x_domain[1] - result.x_domain[0];
    let x_tick_step = (x_range / 8.0).ceil().max(1.0);
    let x_ticks: Vec<f64> = {
        let start = (result.x_domain[0] / x_tick_step).ceil() * x_tick_step;
        (0..)
            .map(|i| start + i as f64 * x_tick_step)
            .take_while(|&v| v <= result.x_domain[1] + 0.01)
            .collect()
    };

    // Month labels
    let month_labels: Vec<(String, f64)> = result
        .area_paths
        .iter()
        .zip(result.y_offsets.iter())
        .map(|((name, _), &y)| (name.clone(), y))
        .collect();

    div()
        .flex()
        .flex_col()
        .size_full()
        .p_4()
        .child(
            div()
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .mb_2()
                .child("Ridgeline Plot — Monthly Temperature Distribution"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/ridgeline-plot — {} months from weather.csv",
            monthly.len()
        )))
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .relative()
                .child(
                    canvas(
                        move |bounds, _, _| {
                            let fills: Vec<_> = d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(p, bounds, 0.0, 0.0)
                                })
                                .collect();
                            let strokes: Vec<_> = d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_stroke(p, bounds, 1.0)
                                })
                                .collect();
                            (fills, strokes)
                        },
                        move |_bounds, (fills, strokes), window, _| {
                            for (i, path_opt) in fills.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                            for path_opt in strokes.into_iter().flatten() {
                                window.paint_path(path_opt, outline_color);
                            }
                        },
                    )
                    .size_full(),
                )
                // Month labels on left
                .children(month_labels.into_iter().map(|(name, y)| {
                    div()
                        .absolute()
                        .left(px(5.0))
                        .top(px((y - 10.0) as f32))
                        .text_size(px(10.0))
                        .child(name)
                }))
                // X-axis labels
                .children(x_ticks.iter().map(|&tick_val| {
                    let x = x_scale.scale(tick_val);
                    div()
                        .absolute()
                        .left(px((x - 10.0) as f32))
                        .bottom(px(2.0))
                        .text_size(px(9.0))
                        .child(format!("{tick_val:.0}°F"))
                })),
        )
}
