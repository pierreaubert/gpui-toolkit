//! Electric Usage 2019 — hourly heatmap.
//! Source: <https://observablehq.com/@mbostock/electric-usage-2019>

use crate::ShowcaseApp;
use d3rs::color::SequentialScheme;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const PGE_CSV: &str = include_str!("../../data/pge-electric-data.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let data = d3rs::examples::electric_usage::load_csv(PGE_CSV);
    let result = d3rs::examples::electric_usage::compute(&data);

    let width = result.width;
    let height = result.height.min(800.0); // cap display height

    let scheme = SequentialScheme::oranges();

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    for cell in &result.cells {
        if cell.y > height {
            continue;
        } // clip to display
        let path = D3PathBuilder::new()
            .move_to(cell.x, cell.y)
            .line_to(cell.x + result.cell_width - 0.5, cell.y)
            .line_to(
                cell.x + result.cell_width - 0.5,
                cell.y + result.cell_height - 0.2,
            )
            .line_to(cell.x, cell.y + result.cell_height - 0.2)
            .close_path()
            .build();
        d3_paths.push(path);
        let t = (cell.usage / result.usage_max).clamp(0.0, 1.0);
        all_colors.push(scheme.get(t).to_rgba().into());
    }

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
                .child("Electric Usage 2019"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@mbostock/electric-usage-2019 — {} hourly readings, {} days",
            data.len(),
            result.unique_dates
        )))
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .relative()
                .overflow_hidden()
                .child(
                    canvas(
                        move |bounds, _, _| {
                            d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(p, bounds, 0.0, 0.0)
                                })
                                .collect::<Vec<_>>()
                        },
                        move |_bounds, paths, window, _| {
                            for (i, path_opt) in paths.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                // X-axis: hour of day labels (0-23)
                .children((0..24).step_by(3).map(|h| {
                    let x = 80.0 + h as f64 * result.cell_width;
                    div()
                        .absolute()
                        .left(px((x + result.cell_width / 2.0 - 5.0) as f32))
                        .top(px(10.0))
                        .text_size(px(8.0))
                        .child(format!("{h}:00"))
                }))
                // Y-axis: month labels (sample every ~30 days)
                .children({
                    let months = [
                        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct",
                        "Nov", "Dec",
                    ];
                    let n_dates = result.unique_dates;
                    (0..12).map(move |m| {
                        let row = (m as f64 / 12.0 * n_dates as f64) as usize;
                        let y = 30.0 + row as f64 * result.cell_height;
                        div()
                            .absolute()
                            .left(px(5.0))
                            .top(px(y as f32))
                            .text_size(px(8.0))
                            .child(months[m].to_string())
                    })
                })
                // Color legend
                .child(
                    div()
                        .absolute()
                        .right(px(5.0))
                        .top(px(10.0))
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().text_size(px(7.0)).child("Low"))
                        .child(
                            div()
                                .flex()
                                .h(px(8.0))
                                .w(px(60.0))
                                .rounded_sm()
                                .overflow_hidden()
                                .children((0..10).map(|i| {
                                    let t = i as f64 / 9.0;
                                    div().flex_1().h_full().bg(scheme.get(t).to_rgba())
                                })),
                        )
                        .child(div().text_size(px(7.0)).child("High")),
                ),
        )
}
