//! Difference Chart — Observable example using d3rs::examples::difference_chart
//!
//! Shows the difference between two temperature series (daily high vs normal)
//! from sfo-temperature.csv. Areas are colored by which series is higher.
//!
//! Source: <https://observablehq.com/@d3/difference-chart/2>

use crate::ShowcaseApp;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const SFO_CSV: &str = include_str!("../../data/sfo-temperature.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let data = d3rs::examples::difference_chart::load_csv(SFO_CSV);
    let result = d3rs::examples::difference_chart::compute(&data);

    let width = result.width;
    let height = result.height;

    // 4 paths: above area, below area, line0, line1
    let d3_paths = vec![
        result.above_path,
        result.below_path,
        result.line0_path,
        result.line1_path,
    ];

    // D3 schemeRdYlBu[3]: orange (#fc8d59) where SF warmer, blue (#91bfdb) where NY warmer
    let above_color = Hsla::from(rgb(0xfc8d59)).opacity(0.7); // SF warmer = orange
    let below_color = Hsla::from(rgb(0x91bfdb)).opacity(0.7); // NY warmer = blue
    let line0_color = hsla(0.0, 0.0, 0.15, 1.0); // black line for SF
    let line1_color = hsla(0.0, 0.0, 0.15, 0.0); // invisible (Observable only shows value0 line)
    let all_colors = vec![above_color, below_color, line0_color, line1_color];

    // Y-axis ticks
    let margin_left = 40.0;
    let margin_top = 20.0;
    let margin_bottom = 30.0;
    let y_scale = LinearScale::new()
        .domain(result.y_domain[0], result.y_domain[1])
        .range(height - margin_bottom, margin_top);

    let y_range = result.y_domain[1] - result.y_domain[0];
    let y_tick_step = (y_range / 6.0).ceil().max(1.0);
    let y_ticks: Vec<f64> = {
        let start = (result.y_domain[0] / y_tick_step).ceil() * y_tick_step;
        (0..)
            .map(|i| start + i as f64 * y_tick_step)
            .take_while(|&v| v <= result.y_domain[1] + 0.01)
            .collect()
    };

    // Grid lines
    let mut grid_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let chart_width = width - margin_left - 20.0;
    for &tick_val in &y_ticks {
        let y = y_scale.scale(tick_val);
        grid_paths.push(
            D3PathBuilder::new()
                .move_to(margin_left, y)
                .line_to(margin_left + chart_width, y)
                .build(),
        );
    }

    // Order: grid lines first (background), then areas, then lines
    let mut all_paths = grid_paths;
    let mut all_c: Vec<Hsla> =
        std::iter::repeat_n(hsla(0.0, 0.0, 0.9, 1.0), y_ticks.len()).collect();
    all_paths.extend(d3_paths);
    all_c.extend(all_colors);

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
                .child("Difference Chart — SFO Temperature"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/difference-chart — {} daily readings",
            data.len()
        )))
        .child(
            div()
                .flex()
                .items_center()
                .gap_4()
                .mb_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size_3().bg(rgb(0xfc8d59)).rounded_sm())
                        .child(div().text_xs().child("San Francisco warmer")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size_3().bg(rgb(0x91bfdb)).rounded_sm())
                        .child(div().text_xs().child("New York warmer")),
                ),
        )
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
                            all_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(p, bounds, 0.0, 0.0)
                                })
                                .collect::<Vec<_>>()
                        },
                        move |_bounds, paths, window, _| {
                            for (i, path_opt) in paths.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_c[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                // Y-axis labels
                .children(y_ticks.iter().map(|&tick_val| {
                    let y = y_scale.scale(tick_val);
                    div()
                        .absolute()
                        .left(px(5.0))
                        .top(px((y - 6.0) as f32))
                        .text_size(px(9.0))
                        .child(format!("{tick_val:.0}°F"))
                })),
        )
}
