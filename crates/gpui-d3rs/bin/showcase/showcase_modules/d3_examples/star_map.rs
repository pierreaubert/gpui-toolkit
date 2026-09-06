//! Star Map — stereographic star chart.
//! Source: <https://observablehq.com/@d3/star-map>

use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const STARS_CSV: &str = include_str!("../../data/stars.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let stars_data = d3rs::examples::star_map::load_csv(STARS_CSV);
    let result = d3rs::examples::star_map::compute(&stars_data);

    let width = result.width;
    let height = result.height;

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Background follows the theme; star ink adapts to stay visible on it.
    d3_paths.push(result.outline_path.clone());
    all_colors.push(Hsla::from(ui_theme.background));

    // Graticule as a true stroke (not ribbon fills).
    let graticule_path = result.graticule_path.clone();
    let graticule_color: Hsla = chart_colors::grid(&ui_theme);

    // Stars as circles sized by magnitude
    let n_sides = 12;
    for star in &result.stars {
        if star.radius < 0.3 {
            continue;
        }
        let mut builder = D3PathBuilder::new();
        for v in 0..n_sides {
            let angle = std::f64::consts::TAU * v as f64 / n_sides as f64;
            let x = star.px + star.radius * angle.cos();
            let y = star.py + star.radius * angle.sin();
            if v == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
        // Brighter stars are whiter, dimmer are more yellow
        let brightness = ((6.0 - star.magnitude) / 7.0).clamp(0.0, 1.0);
        all_colors.push(chart_colors::ink(
            &ui_theme,
            hsla(
                0.15,
                0.2 * (1.0 - brightness) as f32,
                (0.5 + 0.5 * brightness) as f32,
                1.0,
            ),
        ));
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
                .child("Star Map"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/star-map — {} stars visible",
            result.stars.len()
        )))
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.background)
                .border_1()
                .border_color(ui_theme.border)
                .child(
                    canvas(
                        move |bounds, _, _| {
                            let fills: Vec<_> = d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(p, bounds, 0.0, 0.0)
                                })
                                .collect();
                            let grat = super::path_utils::d3rs_path_to_gpui_stroke(
                                &graticule_path,
                                bounds,
                                0.6,
                            );
                            (fills, grat)
                        },
                        move |_bounds, (fills, grat), window, _| {
                            for (i, path_opt) in fills.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                            if let Some(path) = grat {
                                window.paint_path(path, graticule_color);
                            }
                        },
                    )
                    .size_full(),
                ),
        )
}
