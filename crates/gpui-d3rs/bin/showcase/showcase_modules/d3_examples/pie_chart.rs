//! Pie Chart -- Observable example using d3rs::examples::pie_chart
//!
//! Demonstrates idiomatic d3rs usage: `Pie` layout + `Arc` generator + `d3rs_path_to_gpui_simple`.
use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::color::ColorScheme;
use d3rs::shape::arc::Arc as D3Arc;
use d3rs::shape::pie::Pie;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let result = d3rs::examples::pie_chart::compute(d3rs::examples::pie_chart::DEFAULT_DATA);

    let scheme = ColorScheme::tableau10();

    let width = 700.0_f64;
    let height = 450.0_f64;
    let cx_center = width / 2.0;
    let cy_center = height / 2.0;
    let radius = width.min(height) / 2.0 - 20.0;

    // Use d3rs Pie layout to compute slices, then Arc generator to build d3rs Paths.
    // padAngle(1 / radius) matches the official example's slice separation.
    let values: Vec<f64> = result.slices.iter().map(|s| s.value).collect();
    let pie = Pie::new()
        .outer_radius(radius)
        .pad_angle(1.0 / radius)
        .sort(false);
    let slices = pie.generate(&values, |v| *v);

    let arc_gen = D3Arc::new().center(cx_center, cy_center);

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut slice_names: Vec<String> = Vec::new();
    let mut slice_pcts: Vec<f64> = Vec::new();
    // In-slice labels at arc centroids, like the official example (only for
    // slices wide enough to fit their name).
    let mut slice_labels: Vec<(f64, f64, String)> = Vec::new();
    for (i, s) in slices.iter().enumerate() {
        let path = arc_gen.generate(&s.arc);
        d3_paths.push(path);
        slice_names.push(result.slices[i].name.clone());
        let pct = (s.arc.end_angle - s.arc.start_angle) / std::f64::consts::TAU * 100.0;
        slice_pcts.push(pct);
        if s.arc.end_angle - s.arc.start_angle > 0.35 {
            let c = s.arc.centroid();
            slice_labels.push((
                cx_center + c.x,
                cy_center + c.y,
                result.slices[i].name.clone(),
            ));
        }
    }

    let legend_items: Vec<Div> = slice_names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(div().size_3().bg(chart_colors::categorical(&ui_theme, &scheme, i)))
                .child(
                    div()
                        .text_xs()
                        .child(format!("{}: {:.1}%", name, slice_pcts[i])),
                )
        })
        .collect();

    let colors: Vec<Hsla> = (0..scheme.len())
        .map(|i| chart_colors::categorical(&ui_theme, &scheme, i))
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
                .child("Pie Chart"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child("Source: observablehq.com/@d3/pie-chart"),
        )
        .child(
            div()
                .flex()
                .gap_4()
                .mb_2()
                .flex_wrap()
                .children(legend_items),
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
                                    window.paint_path(path, colors[i % colors.len()]);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                // In-slice labels anchored at arc centroids
                .children(slice_labels.iter().map(|(x, y, name)| {
                    div()
                        .absolute()
                        .left(px((*x - 40.0) as f32))
                        .top(px((*y - 8.0) as f32))
                        .w(px(80.0))
                        .flex()
                        .justify_center()
                        .text_color(white())
                        .child(div().text_xs().child(name.clone()))
                })),
        )
}
