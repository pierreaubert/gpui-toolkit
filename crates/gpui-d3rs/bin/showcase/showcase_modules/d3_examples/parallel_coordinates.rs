//! Parallel Coordinates - D3.js Example Port
//!
//! Loads the Auto MPG dataset from cars.csv via `d3rs::fetch::parse_csv`.
//! Ported from: <https://observablehq.com/@d3/parallel-coordinates>

use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::color::ColorScheme;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const CARS_CSV: &str = include_str!("../../data/cars.csv");

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let width = app.content_width as f64;
    let height = (width * 0.525).min(app.content_height as f64 * 0.6);
    let margin_top = 30.0;
    let margin_bottom = 30.0;
    let margin_left = 40.0;
    let margin_right = 40.0;
    let plot_h = height - margin_top - margin_bottom;

    // Parse CSV with d3rs
    let rows = d3rs::fetch::parse_csv(CARS_CSV).expect("valid cars CSV");

    // Axes: column names (skip "name" and "year")
    let axis_keys = [
        "economy (mpg)",
        "cylinders",
        "displacement (cc)",
        "power (hp)",
        "weight (lb)",
        "0-60 mph (s)",
    ];
    let axis_labels = [
        "MPG",
        "Cylinders",
        "Displacement",
        "Power (HP)",
        "Weight (lb)",
        "0-60 mph",
    ];
    let num_axes = axis_keys.len();
    let axis_spacing = (width - margin_left - margin_right) / (num_axes - 1) as f64;

    // Extract numeric values per axis, compute extents
    let mut columns: Vec<Vec<f64>> = vec![Vec::new(); num_axes];
    let mut car_names: Vec<String> = Vec::new();
    let mut car_years: Vec<String> = Vec::new();

    for row in &rows {
        let mut valid = true;
        let mut vals = Vec::with_capacity(num_axes);
        for key in &axis_keys {
            match row
                .get(&key.to_string())
                .and_then(|s| s.parse::<f64>().ok())
            {
                Some(v) => vals.push(v),
                None => {
                    valid = false;
                    break;
                }
            }
        }
        if valid {
            for (i, v) in vals.iter().enumerate() {
                columns[i].push(*v);
            }
            car_names.push(row.get("name").cloned().unwrap_or_default());
            car_years.push(row.get("year").cloned().unwrap_or_default());
        }
    }

    let n_cars = columns[0].len();

    // Compute extent per axis and create LinearScales
    let scales: Vec<LinearScale> = columns
        .iter()
        .map(|col| {
            let min = col.iter().copied().fold(f64::MAX, f64::min);
            let max = col.iter().copied().fold(f64::MIN, f64::max);
            LinearScale::new().domain(min, max).range(plot_h, 0.0)
        })
        .collect();

    let extents: Vec<(f64, f64)> = columns
        .iter()
        .map(|col| {
            let min = col.iter().copied().fold(f64::MAX, f64::min);
            let max = col.iter().copied().fold(f64::MIN, f64::max);
            (min, max)
        })
        .collect();

    // Color by cylinder count: map to scheme
    let scheme = ColorScheme::tableau10();
    let cyl_col_idx = 1; // "cylinders"

    // Build paths
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // 1. Axis lines (vertical)
    for i in 0..num_axes {
        let x = margin_left + i as f64 * axis_spacing;
        let path = D3PathBuilder::new()
            .move_to(x, margin_top)
            .line_to(x, margin_top + plot_h)
            .line_to(x + 1.0, margin_top + plot_h)
            .line_to(x + 1.0, margin_top)
            .close_path()
            .build();
        d3_paths.push(path);
        all_colors.push(chart_colors::axis_line(&ui_theme));
    }

    // 2. Tick marks on each axis (5 ticks per axis)
    for i in 0..num_axes {
        let x = margin_left + i as f64 * axis_spacing;
        let (min_val, max_val) = extents[i];
        let step = (max_val - min_val) / 4.0;
        for t in 0..=4 {
            let val = min_val + t as f64 * step;
            let y = margin_top + scales[i].scale(val);
            let path = D3PathBuilder::new()
                .move_to(x - 4.0, y)
                .line_to(x, y)
                .line_to(x, y + 0.5)
                .line_to(x - 4.0, y + 0.5)
                .close_path()
                .build();
            d3_paths.push(path);
            all_colors.push(chart_colors::axis_line(&ui_theme));
        }
    }
    let _num_structural = d3_paths.len();

    // 3. Data polylines — built as GPUI stroke paths (not d3rs paths)
    // These are rendered separately using PathBuilder::stroke for true line rendering
    struct CarLine {
        points: Vec<(f64, f64)>,
        color: Hsla,
    }
    // Transpose columns to rows for iteration
    let car_rows: Vec<Vec<f64>> = (0..n_cars)
        .map(|ci| columns.iter().map(|col| col[ci]).collect())
        .collect();

    let mut car_lines: Vec<CarLine> = Vec::with_capacity(n_cars);
    for row in &car_rows {
        let mut points = Vec::with_capacity(num_axes);
        for (ai, scale) in scales.iter().enumerate() {
            let x = margin_left + ai as f64 * axis_spacing;
            let y = margin_top + scale.scale(row[ai]);
            points.push((x, y));
        }

        let cyl = row[cyl_col_idx] as usize;
        let color_idx = match cyl {
            3 => 0,
            4 => 1,
            5 => 2,
            6 => 3,
            8 => 4,
            _ => 5,
        };
        let mut c: Hsla = scheme.color(color_idx).to_rgba().into();
        c.a = 0.15;
        car_lines.push(CarLine { points, color: c });
    }

    // Axis tick value labels
    let mut tick_labels: Vec<(f64, f64, String)> = Vec::new();
    for i in 0..num_axes {
        let x = margin_left + i as f64 * axis_spacing;
        let (min_val, max_val) = extents[i];
        let step = (max_val - min_val) / 4.0;
        for t in 0..=4 {
            let val = min_val + t as f64 * step;
            let y = margin_top + scales[i].scale(val);
            let label = if val.fract() == 0.0 || val.abs() >= 1000.0 {
                format!("{:.0}", val)
            } else {
                format!("{:.1}", val)
            };
            tick_labels.push((x, y, label));
        }
    }

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Parallel Coordinates"),
        )
        .child(div().text_sm().child(format!(
            "Source: observablehq.com/@d3/parallel-coordinates — {} cars from cars.csv",
            n_cars
        )))
        .child(
            // Legend by cylinder count
            div().flex().gap_4().mb_2().children(
                [
                    (3, "3 cyl"),
                    (4, "4 cyl"),
                    (5, "5 cyl"),
                    (6, "6 cyl"),
                    (8, "8 cyl"),
                ]
                .iter()
                .enumerate()
                .map(|(ci, (_cyl, label))| {
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size_3().bg(scheme.color(ci).to_rgba()))
                        .child(div().text_xs().child(*label))
                }),
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
                            // Prepare structural paths (axes, ticks)
                            d3_paths
                                .iter()
                                .map(|p| {
                                    super::path_utils::d3rs_path_to_gpui_simple(p, bounds, 0.0, 0.0)
                                })
                                .collect::<Vec<_>>()
                        },
                        move |bounds, structural, window, _| {
                            let origin = bounds.origin;

                            // Draw car lines first (behind axes) using PathBuilder::stroke
                            for car in &car_lines {
                                let mut builder = gpui::PathBuilder::stroke(px(1.0));
                                for (j, &(x, y)) in car.points.iter().enumerate() {
                                    let pt = origin + point(px(x as f32), px(y as f32));
                                    if j == 0 {
                                        builder.move_to(pt);
                                    } else {
                                        builder.line_to(pt);
                                    }
                                }
                                if let Ok(path) = builder.build() {
                                    window.paint_path(path, car.color);
                                }
                            }

                            // Draw structural elements on top (axes, ticks)
                            for (i, path_opt) in structural.iter().enumerate() {
                                if let Some(path) = path_opt {
                                    window.paint_path(path.clone(), all_colors[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                // Axis labels at the top
                .children((0..num_axes).map(|i| {
                    let x = margin_left + i as f64 * axis_spacing;
                    div()
                        .absolute()
                        .left(px((x - 30.0) as f32))
                        .top(px(4.0))
                        .w(px(60.0))
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(axis_labels[i]),
                        )
                }))
                // Tick value labels (left side of each axis)
                .children(tick_labels.iter().map(|(x, y, label)| {
                    div()
                        .absolute()
                        .left(px((*x - 32.0) as f32))
                        .top(px((*y - 5.0) as f32))
                        .w(px(28.0))
                        .flex()
                        .justify_end()
                        .child(div().text_xs().child(label.clone()))
                })),
        )
}
