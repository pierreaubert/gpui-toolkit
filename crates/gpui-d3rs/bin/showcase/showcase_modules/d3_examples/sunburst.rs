//! Sunburst — Observable example using d3rs::examples::sunburst
//!
//! Source: <https://observablehq.com/@d3/sunburst/2>

use crate::ShowcaseApp;
use d3rs::color::ColorScheme;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let result = d3rs::examples::sunburst::compute();

    let scheme = ColorScheme::tableau10();
    let width = result.width;
    let height = result.height;

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    for slice in &result.slices {
        d3_paths.push(slice.arc_path.clone());
        let color = scheme.color(slice.depth);
        let alpha = if slice.depth == 1 { 0.9 } else { 0.6 };
        all_colors.push(Hsla::from(color.to_rgba()).opacity(alpha));
    }

    // Labels for slices with enough angular extent
    let labels: Vec<(String, f64, f64)> = result
        .slices
        .iter()
        .filter(|s| s.x1 - s.x0 > 0.1) // only show label if wide enough
        .map(|s| {
            let mid_angle = (s.x0 + s.x1) / 2.0 - std::f64::consts::FRAC_PI_2;
            let mid_r = (s.y0 + s.y1) / 2.0;
            let x = width / 2.0 + mid_r * mid_angle.cos();
            let y = height / 2.0 + mid_r * mid_angle.sin();
            (s.name.clone(), x, y)
        })
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
                .child("Sunburst — Flare Hierarchy"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/sunburst — {} slices",
            result.slices.len()
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
                            d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(
                                        p,
                                        bounds,
                                        (width / 2.0) as f32,
                                        (height / 2.0) as f32,
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
                )
                // Slice labels
                .children(labels.into_iter().map(|(name, x, y)| {
                    div()
                        .absolute()
                        .left(px((x - 15.0) as f32))
                        .top(px((y - 5.0) as f32))
                        .text_size(px(8.0))
                        .child(name)
                })),
        )
}
