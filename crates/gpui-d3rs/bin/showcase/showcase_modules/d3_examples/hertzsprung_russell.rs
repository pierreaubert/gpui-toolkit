//! Hertzsprung-Russell Diagram — ~29000 stars colored by spectral type.
//! Source: <https://observablehq.com/@d3/hertzsprung-russell-diagram>

use crate::ShowcaseApp;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;

const CATALOG_CSV: &str = include_str!("../../data/catalog.csv");

pub fn render(_app: &ShowcaseApp, _cx: &mut Context<ShowcaseApp>) -> Div {
    let data = d3rs::examples::hertzsprung_russell::load_csv(CATALOG_CSV);
    let result = d3rs::examples::hertzsprung_russell::compute(&data);

    let width = result.width;
    let height = result.height;

    // Each star is a tiny 0.75×0.75 filled rectangle
    let dot_size = 0.75;
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Dark background
    d3_paths.push(
        D3PathBuilder::new()
            .move_to(0.0, 0.0)
            .line_to(width, 0.0)
            .line_to(width, height)
            .line_to(0.0, height)
            .close_path()
            .build(),
    );
    all_colors.push(hsla(0.0, 0.0, 0.05, 1.0));

    for star in &result.stars {
        let path = D3PathBuilder::new()
            .move_to(star.x, star.y)
            .line_to(star.x + dot_size, star.y)
            .line_to(star.x + dot_size, star.y + dot_size)
            .line_to(star.x, star.y + dot_size)
            .close_path()
            .build();
        d3_paths.push(path);
        all_colors.push(Hsla::from(Rgba {
            r: star.r as f32 / 255.0,
            g: star.g as f32 / 255.0,
            b: star.b as f32 / 255.0,
            a: 0.8,
        }));
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
                .child("Hertzsprung-Russell Diagram"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/hertzsprung-russell-diagram — {} stars",
            data.len()
        )))
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(rgb(0x111111))
                .border_1()
                .border_color(rgb(0x333333))
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
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                // Y-axis: absolute magnitude (brighter at top, -7 to 19)
                .children((-7..=19).step_by(2).map(|mag| {
                    let y = 40.0 + (mag as f64 + 7.0) / 26.0 * (height - 80.0);
                    div()
                        .absolute()
                        .left(px(2.0))
                        .top(px((y - 5.0) as f32))
                        .text_size(px(8.0))
                        .child(format!("{mag}"))
                }))
                // X-axis: B-V color index
                .children([0.0, 0.5, 1.0, 1.5, 2.0].iter().map(|&bv| {
                    let x = 40.0 + (bv + 0.39) / 2.58 * (width - 80.0);
                    div()
                        .absolute()
                        .left(px((x - 8.0) as f32))
                        .top(px((height - 15.0) as f32))
                        .text_size(px(8.0))
                        .child(format!("{bv:.1}"))
                }))
                // X-axis: temperature (top axis)
                .children(
                    [30000, 10000, 7000, 5000, 3500]
                        .iter()
                        .enumerate()
                        .map(|(i, &temp)| {
                            let bv = [0.0_f64, 0.5, 0.8, 1.2, 1.8][i];
                            let x = 40.0 + (bv + 0.39) / 2.58 * (width - 80.0);
                            div()
                                .absolute()
                                .left(px((x - 15.0) as f32))
                                .top(px(5.0))
                                .text_size(px(8.0))
                                .child(format!("{temp}K"))
                        }),
                )
                // Axis labels
                .child(
                    div()
                        .absolute()
                        .left(px(2.0))
                        .top(px((height / 2.0 - 30.0) as f32))
                        .text_size(px(9.0))
                        .child("← Brighter"),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(5.0))
                        .top(px(20.0))
                        .text_size(px(9.0))
                        .child("Temperature →"),
                )
                .child(
                    div()
                        .absolute()
                        .left(px((width / 2.0 - 30.0) as f32))
                        .bottom(px(2.0))
                        .text_size(px(9.0))
                        .child("B-V Color Index"),
                ),
        )
}
