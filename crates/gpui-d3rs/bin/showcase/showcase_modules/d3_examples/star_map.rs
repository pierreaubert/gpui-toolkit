//! Star Map — stereographic star chart.
//! Source: <https://observablehq.com/@d3/star-map>

use crate::ShowcaseApp;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;

const STARS_CSV: &str = include_str!("../../data/stars.csv");

pub fn render(_app: &ShowcaseApp, _cx: &mut Context<ShowcaseApp>) -> Div {
    let stars_data = d3rs::examples::star_map::load_csv(STARS_CSV);
    let result = d3rs::examples::star_map::compute(&stars_data);

    let width = result.width;
    let height = result.height;

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Dark background
    d3_paths.push(result.outline_path.clone());
    all_colors.push(hsla(0.63, 0.2, 0.08, 1.0)); // dark navy

    // Graticule ribbons
    {
        use d3rs::shape::path::PathCommand;
        let mut prev: Option<(f64, f64)> = None;
        for cmd in result.graticule_path.commands() {
            match cmd {
                PathCommand::MoveTo { x, y } => {
                    prev = Some((*x, *y));
                }
                PathCommand::LineTo { x, y } => {
                    if let Some((px, py)) = prev {
                        let dx = x - px;
                        let dy = y - py;
                        let len = (dx * dx + dy * dy).sqrt();
                        if len > 0.5 {
                            let nx = -dy / len * 0.3;
                            let ny = dx / len * 0.3;
                            d3_paths.push(
                                D3PathBuilder::new()
                                    .move_to(px + nx, py + ny)
                                    .line_to(x + nx, y + ny)
                                    .line_to(x - nx, y - ny)
                                    .line_to(px - nx, py - ny)
                                    .close_path()
                                    .build(),
                            );
                            all_colors.push(hsla(0.63, 0.1, 0.25, 0.3));
                        }
                    }
                    prev = Some((*x, *y));
                }
                _ => {
                    prev = None;
                }
            }
        }
    }

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
        all_colors.push(hsla(
            0.15,
            0.2 * (1.0 - brightness) as f32,
            (0.5 + 0.5 * brightness) as f32,
            1.0,
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
                .bg(rgb(0x0a0a1a))
                .border_1()
                .border_color(rgb(0x333333))
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
                ),
        )
}
