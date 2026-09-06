//! Donut Chart -- Observable example using d3rs::examples::donut_chart
//!
//! Demonstrates idiomatic d3rs usage: `Pie` with inner_radius + `Arc` generator + `d3rs_path_to_gpui_simple`.
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
    let result = d3rs::examples::donut_chart::compute(d3rs::examples::donut_chart::DEFAULT_DATA);

    let scheme = ColorScheme::tableau10();

    let width = 700.0_f64;
    let height = 450.0_f64;
    let cx_center = width / 2.0;
    let cy_center = height / 2.0;
    let outer_radius = width.min(height) / 2.0 - 20.0;
    let inner_radius = outer_radius * 0.67;
    let pad_angle = 1.0 / outer_radius;

    // Use d3rs Pie layout with inner radius and pad angle for donut slices
    let values: Vec<f64> = result.slices.iter().map(|s| s.value).collect();
    let pie = Pie::new()
        .inner_radius(inner_radius)
        .outer_radius(outer_radius)
        .pad_angle(pad_angle)
        .sort(false);
    let slices = pie.generate(&values, |v| *v);

    let arc_gen = D3Arc::new().center(cx_center, cy_center);

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    // Outside labels with leader lines like the official example:
    // (name, label x, label y, label on right side).
    let mut slice_labels: Vec<(String, f64, f64, bool)> = Vec::new();
    // Leader polylines in chart coordinates (inner centroid -> outer arc -> label).
    let mut leader_lines: Vec<Vec<(f32, f32)>> = Vec::new();
    for (i, s) in slices.iter().enumerate() {
        let path = arc_gen.generate(&s.arc);
        d3_paths.push(path);
        let mid = (s.arc.start_angle + s.arc.end_angle) / 2.0;
        let std_angle = mid - std::f64::consts::FRAC_PI_2;
        let (dx, dy) = (std_angle.cos(), std_angle.sin());
        let c = s.arc.centroid();
        let p_inner = (cx_center + c.x, cy_center + c.y);
        let p_outer = (
            cx_center + dx * (outer_radius + 2.0),
            cy_center + dy * (outer_radius + 2.0),
        );
        let p_label = (
            cx_center + dx * (outer_radius + 12.0),
            cy_center + dy * (outer_radius + 12.0),
        );
        leader_lines.push(vec![
            (p_inner.0 as f32, p_inner.1 as f32),
            (p_outer.0 as f32, p_outer.1 as f32),
            (p_label.0 as f32, p_label.1 as f32),
        ]);
        slice_labels.push((result.slices[i].name.clone(), p_label.0, p_label.1, dx >= 0.0));
    }

    let colors: Vec<Hsla> = (0..scheme.len())
        .map(|i| chart_colors::categorical(&ui_theme, &scheme, i))
        .collect();
    let leader_color: Hsla = hsla(0.0, 0.0, 0.5, 1.0);
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
                .child("Donut Chart"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child("Source: observablehq.com/@d3/donut-chart"),
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
                        move |bounds, paths, window, _| {
                            for (i, path_opt) in paths.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, colors[i % colors.len()]);
                                }
                            }
                            // Leader lines as true strokes
                            let origin = bounds.origin;
                            for line in &leader_lines {
                                let mut builder = gpui::PathBuilder::stroke(px(1.0));
                                for (j, &(x, y)) in line.iter().enumerate() {
                                    let pt = origin + point(px(x), px(y));
                                    if j == 0 {
                                        builder.move_to(pt);
                                    } else {
                                        builder.line_to(pt);
                                    }
                                }
                                if let Ok(path) = builder.build() {
                                    window.paint_path(path, leader_color);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                // Outside slice labels anchored by side
                .children(slice_labels.iter().map(|(name, x, y, on_right)| {
                    if *on_right {
                        div()
                            .absolute()
                            .left(px(*x as f32))
                            .top(px((*y - 7.0) as f32))
                            .flex()
                            .text_size(px(10.0))
                            .child(name.clone())
                    } else {
                        div()
                            .absolute()
                            .left(px((*x - 100.0) as f32))
                            .top(px((*y - 7.0) as f32))
                            .w(px(96.0))
                            .flex()
                            .justify_end()
                            .text_size(px(10.0))
                            .child(name.clone())
                    }
                })),
        )
}
