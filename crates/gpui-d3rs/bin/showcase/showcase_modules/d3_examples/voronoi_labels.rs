//! Voronoi Labels — scatter with Voronoi-based label placement.
//! Source: <https://observablehq.com/@d3/voronoi-labels>

use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const VORONOI_CSV: &str = include_str!("../../data/voronoi.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let coords = d3rs::examples::voronoi_labels::load_csv(VORONOI_CSV);
    let result = d3rs::examples::voronoi_labels::compute(&coords);

    let width = result.width;
    let height = result.height;

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // Voronoi mesh as a true stroke (not ribbon fills).
    let mesh_path = result.voronoi_mesh.clone();
    let mesh_color: Hsla = chart_colors::ink(&ui_theme, hsla(0.0, 0.0, 0.85, 0.5));

    // Points as circles
    let n_sides = 10;
    for pt in &result.points {
        let r = 3.0;
        let mut builder = D3PathBuilder::new();
        for v in 0..n_sides {
            let angle = std::f64::consts::TAU * v as f64 / n_sides as f64;
            if v == 0 {
                builder = builder.move_to(pt.x + r, pt.y);
            } else {
                builder = builder.line_to(pt.x + r * angle.cos(), pt.y + r * angle.sin());
            }
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
        all_colors.push(chart_colors::axis_line(&ui_theme));
    }

    // Labels for points with large Voronoi cells
    let labels: Vec<(
        String,
        f64,
        f64,
        d3rs::examples::voronoi_labels::LabelAnchor,
    )> = result
        .points
        .iter()
        .filter(|p| p.show_label)
        .map(|p| (format!("{}", p.index), p.x, p.y, p.label_anchor))
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
                .child("Voronoi Labels"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/voronoi-labels — {} points, {} labeled",
            result.point_count, result.label_count
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
                            let mesh = super::path_utils::d3rs_path_to_gpui_stroke(
                                &mesh_path,
                                bounds,
                                0.6,
                            );
                            (fills, mesh)
                        },
                        move |_bounds, (fills, mesh), window, _| {
                            for (i, path_opt) in fills.into_iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path, all_colors[i]);
                                }
                            }
                            if let Some(path) = mesh {
                                window.paint_path(path, mesh_color);
                            }
                        },
                    )
                    .size_full(),
                )
                .children(labels.into_iter().map(|(label, x, y, anchor)| {
                    use d3rs::examples::voronoi_labels::LabelAnchor;
                    let (dx, dy) = match anchor {
                        LabelAnchor::Right => (5.0, -4.0),
                        LabelAnchor::Left => (-20.0, -4.0),
                        LabelAnchor::Top => (-5.0, -14.0),
                        LabelAnchor::Bottom => (-5.0, 6.0),
                    };
                    div()
                        .absolute()
                        .left(px((x + dx) as f32))
                        .top(px((y + dy) as f32))
                        .text_size(px(9.0))
                        .child(label)
                })),
        )
}
