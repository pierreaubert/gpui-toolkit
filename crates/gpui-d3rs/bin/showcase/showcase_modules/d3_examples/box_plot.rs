//! Box Plot -- Observable example using d3rs::examples::box_plot
//!
//! Demonstrates idiomatic d3rs usage: `BandScale` for groups, `LinearScale` for y-axis,
//! `PathBuilder` for box/whisker/outlier paths, `d3rs_path_to_gpui_simple` for rendering.
use crate::ShowcaseApp;
use d3rs::color::ColorScheme;
use d3rs::scale::{BandScale, LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const DIAMONDS_CSV: &str = include_str!("../../data/diamonds.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    // Load real diamonds data, bin by carat range for box plot groups
    let rows = d3rs::fetch::parse_csv(DIAMONDS_CSV).expect("valid diamonds CSV");
    let mut binned: std::collections::BTreeMap<String, Vec<f64>> =
        std::collections::BTreeMap::new();
    for row in &rows {
        let carat: f64 = row.get("carat").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let price: f64 = row.get("price").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        if carat <= 0.0 || price <= 0.0 {
            continue;
        }
        // Bin by carat range: 0-0.5, 0.5-1, 1-1.5, 1.5-2, 2-3, 3+
        let group = if carat < 0.5 {
            "0-0.5"
        } else if carat < 1.0 {
            "0.5-1"
        } else if carat < 1.5 {
            "1-1.5"
        } else if carat < 2.0 {
            "1.5-2"
        } else if carat < 3.0 {
            "2-3"
        } else {
            "3+"
        };
        binned.entry(group.to_string()).or_default().push(price);
    }
    let data: Vec<(String, Vec<f64>)> = binned.into_iter().collect();
    let result = d3rs::examples::box_plot::compute(&data);

    let scheme = ColorScheme::tableau10();
    let box_color = scheme.color(0).to_rgba(); // Blue
    let median_color = scheme.color(2).to_rgba(); // Red
    let whisker_color = rgb(0x333333);
    let outlier_color = scheme.color(1).to_rgba(); // Orange

    let width = 700.0_f64;
    let height = 450.0_f64;
    let margin_top = 20.0_f64;
    let margin_bottom = 40.0_f64;
    let margin_left = 50.0_f64;
    let margin_right = 20.0_f64;
    let chart_width = width - margin_left - margin_right;
    let chart_height = height - margin_top - margin_bottom;

    // Use d3rs BandScale for group positions
    let group_names: Vec<String> = result.groups.iter().map(|g| g.group.clone()).collect();
    let band = BandScale::new()
        .domain(group_names.clone())
        .range(0.0, chart_width)
        .padding_inner(0.2);
    let bw = band.bandwidth();

    let y_scale = LinearScale::new()
        .domain(result.y_domain[0] - 5.0, result.y_domain[1] + 5.0)
        .range(chart_height, 0.0);

    // Build all the geometry as d3rs Paths using D3PathBuilder
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    for group in &result.groups {
        let band_x = band.scale(&group.group).unwrap_or(0.0);
        let box_x = band_x + bw * 0.15;
        let box_w = bw * 0.7;

        // Box rectangle (q1 to q3)
        let q1_y = y_scale.scale(group.q1);
        let q3_y = y_scale.scale(group.q3);
        let top = q1_y.min(q3_y);
        let bottom = q1_y.max(q3_y);
        let box_h = (bottom - top).max(1.0);
        d3_paths.push(
            D3PathBuilder::new()
                .move_to(box_x, top)
                .line_to(box_x + box_w, top)
                .line_to(box_x + box_w, top + box_h)
                .line_to(box_x, top + box_h)
                .close_path()
                .build(),
        );
        all_colors.push(box_color.into());

        // Median line
        let med_y = y_scale.scale(group.median);
        let line_h = 2.0;
        d3_paths.push(
            D3PathBuilder::new()
                .move_to(box_x, med_y - line_h / 2.0)
                .line_to(box_x + box_w, med_y - line_h / 2.0)
                .line_to(box_x + box_w, med_y + line_h / 2.0)
                .line_to(box_x, med_y + line_h / 2.0)
                .close_path()
                .build(),
        );
        all_colors.push(median_color.into());

        let whisker_x = band_x + bw * 0.5;
        let whisker_w = 1.5;

        // Whisker: low vertical line
        let wl_y = y_scale.scale(group.whisker_low);
        let wl_top = wl_y.min(bottom);
        let wl_bottom = wl_y.max(bottom);
        d3_paths.push(
            D3PathBuilder::new()
                .move_to(whisker_x - whisker_w / 2.0, wl_top)
                .line_to(whisker_x + whisker_w / 2.0, wl_top)
                .line_to(whisker_x + whisker_w / 2.0, wl_bottom)
                .line_to(whisker_x - whisker_w / 2.0, wl_bottom)
                .close_path()
                .build(),
        );
        all_colors.push(whisker_color.into());

        // Whisker: high vertical line
        let wh_y = y_scale.scale(group.whisker_high);
        let wh_top = wh_y.min(top);
        let wh_bottom = wh_y.max(top);
        d3_paths.push(
            D3PathBuilder::new()
                .move_to(whisker_x - whisker_w / 2.0, wh_top)
                .line_to(whisker_x + whisker_w / 2.0, wh_top)
                .line_to(whisker_x + whisker_w / 2.0, wh_bottom)
                .line_to(whisker_x - whisker_w / 2.0, wh_bottom)
                .close_path()
                .build(),
        );
        all_colors.push(whisker_color.into());

        // Whisker caps (horizontal lines)
        let cap_w = bw * 0.3;
        let cap_h = 1.5;
        // Low cap
        d3_paths.push(
            D3PathBuilder::new()
                .move_to(whisker_x - cap_w / 2.0, wl_y - cap_h / 2.0)
                .line_to(whisker_x + cap_w / 2.0, wl_y - cap_h / 2.0)
                .line_to(whisker_x + cap_w / 2.0, wl_y + cap_h / 2.0)
                .line_to(whisker_x - cap_w / 2.0, wl_y + cap_h / 2.0)
                .close_path()
                .build(),
        );
        all_colors.push(whisker_color.into());
        // High cap
        d3_paths.push(
            D3PathBuilder::new()
                .move_to(whisker_x - cap_w / 2.0, wh_y - cap_h / 2.0)
                .line_to(whisker_x + cap_w / 2.0, wh_y - cap_h / 2.0)
                .line_to(whisker_x + cap_w / 2.0, wh_y + cap_h / 2.0)
                .line_to(whisker_x - cap_w / 2.0, wh_y + cap_h / 2.0)
                .close_path()
                .build(),
        );
        all_colors.push(whisker_color.into());

        // Outlier dots (small diamonds)
        for &val in &group.outliers {
            let oy = y_scale.scale(val);
            let dot_r = 3.0;
            d3_paths.push(
                D3PathBuilder::new()
                    .move_to(whisker_x, oy - dot_r)
                    .line_to(whisker_x + dot_r, oy)
                    .line_to(whisker_x, oy + dot_r)
                    .line_to(whisker_x - dot_r, oy)
                    .close_path()
                    .build(),
            );
            all_colors.push(outlier_color.into());
        }
    }

    // Y-axis ticks
    let y_min = result.y_domain[0] - 5.0;
    let y_max = result.y_domain[1] + 5.0;
    let y_range = y_max - y_min;
    let y_step = (y_range / 6.0).ceil();
    let y_min_tick = (y_min / y_step).floor() * y_step;
    let y_ticks: Vec<f64> = (0..=8)
        .map(|i| y_min_tick + i as f64 * y_step)
        .filter(|v| *v >= y_min - 0.1 && *v <= y_max + 0.1)
        .collect();

    // Group labels
    let group_labels: Vec<Div> = group_names
        .iter()
        .map(|name| {
            let band_x = margin_left + band.scale(name).unwrap_or(0.0);
            div()
                .absolute()
                .left(px(band_x as f32))
                .top(px((margin_top + chart_height + 4.0) as f32))
                .w(px(bw as f32))
                .flex()
                .justify_center()
                .text_xs()
                .child(format!("Group {}", name))
        })
        .collect();

    let legend_items = vec![
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(div().size_3().bg(box_color))
            .child(div().text_xs().child("IQR (Q1-Q3)")),
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(div().size_3().bg(median_color))
            .child(div().text_xs().child("Median")),
        div()
            .flex()
            .items_center()
            .gap_1()
            .child(div().size_3().bg(outlier_color))
            .child(div().text_xs().child("Outliers")),
    ];

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
                .child("Box Plot"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child("Source: observablehq.com/@d3/box-plot"),
        )
        .child(div().flex().gap_4().mb_2().children(legend_items))
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .relative()
                .children(group_labels)
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(chart_width as f32))
                        .h(px(chart_height as f32))
                        .child(
                            canvas(
                                move |bounds, _, _| {
                                    d3_paths
                                        .iter()
                                        .map(|p| {
                                            super::path_utils::d3rs_path_to_gpui_simple(
                                                p, bounds, 0.0, 0.0,
                                            )
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
                        ),
                )
                // X-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + chart_height) as f32))
                        .w(px(chart_width as f32))
                        .h(px(1.0))
                        .bg(rgb(0x000000)),
                )
                // Y-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(1.0))
                        .h(px(chart_height as f32))
                        .bg(rgb(0x000000)),
                )
                // Y-axis tick labels
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px((margin_top + y - 6.0) as f32))
                        .w(px(margin_left as f32))
                        .flex()
                        .justify_end()
                        .pr_1()
                        .child(div().text_xs().child(format!("{:.0}", val)))
                }))
                // Y grid lines
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + y) as f32))
                        .w(px(chart_width as f32))
                        .h(px(1.0))
                        .bg(ui_theme.surface)
                })),
        )
}
