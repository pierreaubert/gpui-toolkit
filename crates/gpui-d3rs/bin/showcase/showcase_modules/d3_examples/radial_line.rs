//! Radial Area Chart — SFO Temperature
//!
//! Ported from: <https://observablehq.com/@d3/radial-area-chart/2>
//! Uses sfo-temperature.csv (366 days), showing daily temperature bands
//! (extreme range, mean range, average line) in polar coordinates.
//! Angle = day of year, radius = temperature (°F).

use crate::ShowcaseApp;
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use std::f64::consts::PI;

const SFO_CSV: &str = include_str!("../../data/sfo-temperature.csv");

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    // Parse CSV with d3rs
    let rows = d3rs::fetch::parse_csv(SFO_CSV).expect("valid SFO temperature CSV");

    struct DayData {
        _day_of_year: usize,
        avg: f64,
        min: f64,
        max: f64,
        minmin: f64,
        maxmax: f64,
    }

    let data: Vec<DayData> = rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            Some(DayData {
                _day_of_year: i,
                avg: row.get("avg")?.parse().ok()?,
                min: row.get("min")?.parse().ok()?,
                max: row.get("max")?.parse().ok()?,
                minmin: row.get("minmin")?.parse().ok()?,
                maxmax: row.get("maxmax")?.parse().ok()?,
            })
        })
        .collect();

    let n = data.len();
    if n == 0 {
        return div().child("No data loaded");
    }

    let canvas_size = 560.0_f64;
    let center = canvas_size / 2.0;
    let margin = 10.0;
    let inner_radius = canvas_size / 5.0;
    let outer_radius = canvas_size / 2.0 - margin;

    // Temperature extent
    let temp_min = data.iter().map(|d| d.minmin).fold(f64::MAX, f64::min);
    let temp_max = data.iter().map(|d| d.maxmax).fold(f64::MIN, f64::max);

    // Radial scale: temperature → radius
    let y_scale = LinearScale::new()
        .domain(temp_min, temp_max)
        .range(inner_radius, outer_radius);

    // Angle scale: day of year → angle [0, 2π)
    // Day 0 = Jan 1 at 12 o'clock (top), going clockwise
    let angle_for_day = |day: usize| -> f64 { (day as f64 / n as f64) * 2.0 * PI - PI / 2.0 };

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();

    // --- Temperature grid circles ---
    let temp_step = 10.0;
    let first_tick = ((temp_min / temp_step).ceil() * temp_step) as i32;
    let last_tick = ((temp_max / temp_step).floor() * temp_step) as i32;
    let temp_ticks: Vec<f64> = (first_tick..=last_tick)
        .step_by(temp_step as usize)
        .map(|t| t as f64)
        .collect();

    for &temp in &temp_ticks {
        let r = y_scale.scale(temp);
        let mut builder = D3PathBuilder::new();
        let steps = 72;
        for j in 0..=steps {
            let angle = (j as f64 / steps as f64) * 2.0 * PI;
            let x = center + r * angle.cos();
            let y = center + r * angle.sin();
            if j == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
    }
    let num_grid = temp_ticks.len();

    // --- Month spoke lines (12 months) ---
    let month_days = [0, 31, 60, 91, 121, 152, 182, 213, 244, 274, 305, 335];
    for &day in &month_days {
        let angle = angle_for_day(day);
        let x1 = center + inner_radius * angle.cos();
        let y1 = center + inner_radius * angle.sin();
        let x2 = center + outer_radius * angle.cos();
        let y2 = center + outer_radius * angle.sin();
        let nx = -angle.sin() * 0.4;
        let ny = angle.cos() * 0.4;
        let path = D3PathBuilder::new()
            .move_to(x1 + nx, y1 + ny)
            .line_to(x2 + nx, y2 + ny)
            .line_to(x2 - nx, y2 - ny)
            .line_to(x1 - nx, y1 - ny)
            .close_path()
            .build();
        d3_paths.push(path);
    }
    let num_spokes = 12;

    // --- Extreme range area (minmin to maxmax) — light fill ---
    {
        let mut builder = D3PathBuilder::new();
        // Outer edge (maxmax)
        for (i, d) in data.iter().enumerate() {
            let angle = angle_for_day(i);
            let r = y_scale.scale(d.maxmax);
            let x = center + r * angle.cos();
            let y = center + r * angle.sin();
            if i == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }
        // Inner edge (minmin), reversed
        for (i, d) in data.iter().enumerate().rev() {
            let angle = angle_for_day(i);
            let r = y_scale.scale(d.minmin);
            let x = center + r * angle.cos();
            let y = center + r * angle.sin();
            builder = builder.line_to(x, y);
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
    }

    // --- Mean range area (min to max) — darker fill ---
    {
        let mut builder = D3PathBuilder::new();
        for (i, d) in data.iter().enumerate() {
            let angle = angle_for_day(i);
            let r = y_scale.scale(d.max);
            let x = center + r * angle.cos();
            let y = center + r * angle.sin();
            if i == 0 {
                builder = builder.move_to(x, y);
            } else {
                builder = builder.line_to(x, y);
            }
        }
        for (i, d) in data.iter().enumerate().rev() {
            let angle = angle_for_day(i);
            let r = y_scale.scale(d.min);
            let x = center + r * angle.cos();
            let y = center + r * angle.sin();
            builder = builder.line_to(x, y);
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
    }

    // --- Average line (as ribbon) ---
    {
        let thickness = 1.5;
        let half = thickness / 2.0;
        let mut points: Vec<(f64, f64)> = data
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let angle = angle_for_day(i);
                let r = y_scale.scale(d.avg);
                (center + r * angle.cos(), center + r * angle.sin())
            })
            .collect();
        points.push(points[0]); // close loop

        let np = points.len();
        let mut builder = D3PathBuilder::new();
        for i in 0..np {
            let (dx, dy) = if i == 0 {
                (
                    points[1].0 - points[np - 2].0,
                    points[1].1 - points[np - 2].1,
                )
            } else if i == np - 1 {
                (points[1].0 - points[i - 1].0, points[1].1 - points[i - 1].1)
            } else {
                (
                    points[i + 1].0 - points[i - 1].0,
                    points[i + 1].1 - points[i - 1].1,
                )
            };
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let nx = -dy / len * half;
            let ny = dx / len * half;
            if i == 0 {
                builder = builder.move_to(points[i].0 + nx, points[i].1 + ny);
            } else {
                builder = builder.line_to(points[i].0 + nx, points[i].1 + ny);
            }
        }
        for i in (0..np).rev() {
            let (dx, dy) = if i == 0 {
                (
                    points[1].0 - points[np - 2].0,
                    points[1].1 - points[np - 2].1,
                )
            } else if i == np - 1 {
                (points[1].0 - points[i - 1].0, points[1].1 - points[i - 1].1)
            } else {
                (
                    points[i + 1].0 - points[i - 1].0,
                    points[i + 1].1 - points[i - 1].1,
                )
            };
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let nx = -dy / len * half;
            let ny = dx / len * half;
            builder = builder.line_to(points[i].0 - nx, points[i].1 - ny);
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());
    }

    // Colors for layers
    let grid_color: Hsla = rgb(0xdddddd).into();
    let spoke_color: Hsla = rgb(0xcccccc).into();
    let extreme_color: Hsla = rgb(0xb0c4de).into(); // lightsteelblue
    let mean_color: Hsla = rgb(0x4682b4).into(); // steelblue
    let avg_line_color: Hsla = rgb(0x2c5f8a).into(); // darker steelblue

    let num_layers = d3_paths.len();
    let layer_colors: Vec<Hsla> = (0..num_layers)
        .map(|i| {
            if i < num_grid {
                grid_color
            } else if i < num_grid + num_spokes {
                spoke_color
            } else if i == num_grid + num_spokes {
                extreme_color
            } else if i == num_grid + num_spokes + 1 {
                mean_color
            } else {
                avg_line_color
            }
        })
        .collect();

    // Temperature tick labels (along 12 o'clock spoke)
    let temp_labels: Vec<(f64, f64, String)> = temp_ticks
        .iter()
        .map(|&temp| {
            let r = y_scale.scale(temp);
            (center + 4.0, center - r, format!("{}°F", temp as i32))
        })
        .collect();

    // Month labels around the perimeter
    let month_names = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month_labels: Vec<(f64, f64, &str)> = month_days
        .iter()
        .enumerate()
        .map(|(mi, &day)| {
            let angle = angle_for_day(day);
            let r = outer_radius + 16.0;
            (
                center + r * angle.cos(),
                center + r * angle.sin(),
                month_names[mi],
            )
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .text_2xl()
                .font_weight(FontWeight::BOLD)
                .child("Radial Area Chart — SFO Temperature"),
        )
        .child(div().text_sm().child(format!(
            "Source: observablehq.com/@d3/radial-area-chart — {} days, {:.0}°F to {:.0}°F",
            n, temp_min, temp_max
        )))
        .child(
            div()
                .flex()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().w_4().h_4().bg(rgb(0xb0c4de)).rounded_sm())
                        .child(div().text_xs().child("Extreme range")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().w_4().h_4().bg(rgb(0x4682b4)).rounded_sm())
                        .child(div().text_xs().child("Mean range")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().w_4().h_2().bg(rgb(0x2c5f8a)).rounded_sm())
                        .child(div().text_xs().child("Average")),
                ),
        )
        .child(
            div()
                .w(px(canvas_size as f32))
                .h(px(canvas_size as f32))
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
                        move |_bounds, shapes, window, _| {
                            for (i, shape_opt) in shapes.iter().enumerate() {
                                if let Some(shape) = shape_opt {
                                    window.paint_path(shape.clone(), layer_colors[i]);
                                }
                            }
                        },
                    )
                    .size_full(),
                )
                // Temperature tick labels
                .children(temp_labels.iter().map(|(x, y, label)| {
                    div()
                        .absolute()
                        .left(px(*x as f32))
                        .top(px((*y - 5.0) as f32))
                        .child(div().text_xs().child(label.clone()))
                }))
                // Month labels around the perimeter
                .children(month_labels.iter().map(|(x, y, label)| {
                    div()
                        .absolute()
                        .left(px((*x - 12.0) as f32))
                        .top(px((*y - 6.0) as f32))
                        .w(px(24.0))
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .child(*label),
                        )
                })),
        )
}
