//! Chord Diagram -- Observable example using d3rs::examples::chord
//!
//! Demonstrates idiomatic d3rs usage: `ChordLayout` for computing chords,
//! `Arc` for group arcs, `RibbonGenerator` for chord ribbons,
//! `d3rs_path_to_gpui_simple` for rendering, with outer tick marks and labels.
use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::chord::{ChordLayout, RibbonGenerator};
use d3rs::color::ColorScheme;
use d3rs::shape::arc::{Arc as D3Arc, ArcDatum};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use d3rs::text::{
    GlyphTextConfig, HorizontalTextAnchor, VerticalTextAnchor, render_glyph_text_anchored,
};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

/// Nice data step giving roughly `target` ticks across `value` (1/2/5 x 10^n).
fn nice_tick_step(value: f64, target: usize) -> f64 {
    let raw = (value / target.max(1) as f64).max(f64::EPSILON);
    let mag = 10f64.powf(raw.log10().floor());
    for m in [1.0, 2.0, 5.0, 10.0] {
        if m * mag >= raw {
            return m * mag;
        }
    }
    10.0 * mag
}

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let (names, matrix) = d3rs::examples::chord::default_matrix();

    let scheme = ColorScheme::tableau10();

    let width = 700.0_f64;
    let height = 500.0_f64;
    let cx_center = width / 2.0;
    let cy_center = height / 2.0;
    let outer_radius = height.min(width) / 2.0 - 50.0; // extra margin for labels
    let inner_radius = outer_radius - 20.0;
    let tick_radius = outer_radius + 3.0;
    let tick_label_radius = outer_radius + 12.0;
    let label_radius = outer_radius + 34.0;

    // Use d3rs ChordLayout to compute groups and chords
    let chord_layout = ChordLayout::new()
        .pad_angle(0.05)
        .sort_subgroups(|a, b| b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal));
    let chord_result = chord_layout.compute(&matrix);

    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut all_colors: Vec<Hsla> = Vec::new();

    // 1. Draw group arcs using d3rs Arc generator
    let arc_gen = D3Arc::new().center(cx_center, cy_center);
    for group in &chord_result.groups {
        let datum = ArcDatum::new()
            .inner_radius(inner_radius)
            .outer_radius(outer_radius)
            .start_angle(group.start_angle)
            .end_angle(group.end_angle);
        let path = arc_gen.generate(&datum);
        d3_paths.push(path);
        all_colors.push(scheme.color(group.index).to_rgba().into());
    }

    // 2. Group ticks at data intervals with value labels, like the official
    // example (`groupTicks`: angle = value * span / group.value).
    let half_pi = std::f64::consts::FRAC_PI_2;
    // (tick value label, x, y) in chart coordinates.
    let mut tick_labels: Vec<(String, f64, f64)> = Vec::new();
    for group in &chord_result.groups {
        if group.value <= 0.0 {
            continue;
        }
        let arc_span = group.end_angle - group.start_angle;
        let step = nice_tick_step(group.value, 4);
        let mut v = 0.0;
        while v < group.value {
            let angle = group.start_angle + arc_span * (v / group.value) - half_pi;
            let x1 = cx_center + outer_radius * angle.cos();
            let y1 = cy_center + outer_radius * angle.sin();
            let x2 = cx_center + tick_radius * angle.cos();
            let y2 = cy_center + tick_radius * angle.sin();
            // Tick as a thin line (1px wide rectangle)
            let nx = -angle.sin() * 0.5;
            let ny = angle.cos() * 0.5;
            let tick_path = D3PathBuilder::new()
                .move_to(x1 + nx, y1 + ny)
                .line_to(x2 + nx, y2 + ny)
                .line_to(x2 - nx, y2 - ny)
                .line_to(x1 - nx, y1 - ny)
                .close_path()
                .build();
            d3_paths.push(tick_path);
            all_colors.push(chart_colors::ink(&ui_theme, hsla(0.0, 0.0, 0.3, 1.0)));
            let lx = cx_center + tick_label_radius * angle.cos();
            let ly = cy_center + tick_label_radius * angle.sin();
            let text = if v.abs() >= 1000.0 {
                format!("{:.0}k", v / 1000.0)
            } else {
                format!("{v:.0}")
            };
            tick_labels.push((text, lx, ly));
            v += step;
        }
    }

    // 3. Draw chord ribbons using d3rs RibbonGenerator (solid source colors
    // like the official example).
    let ribbon_gen = RibbonGenerator::new(inner_radius).center(cx_center, cy_center);
    for chord in &chord_result.chords {
        let path = ribbon_gen.generate_path(chord);
        d3_paths.push(path);
        all_colors.push(scheme.color(chord.source.index).to_rgba().into());
    }

    // Group name labels — positioned at the midpoint angle of each arc and
    // rotated tangent to the circle so they read outward.
    let label_items: Vec<Div> = chord_result
        .groups
        .iter()
        .map(|g| {
            let d3_mid = (g.start_angle + g.end_angle) / 2.0;
            let std_mid = d3_mid - half_pi;
            let lx = cx_center + label_radius * std_mid.cos();
            let ly = cy_center + label_radius * std_mid.sin();

            let on_left = d3_mid > std::f64::consts::PI;
            let rotation = std_mid as f32 + if on_left { std::f32::consts::PI } else { 0.0 };
            let h_anchor = if on_left {
                HorizontalTextAnchor::Start
            } else {
                HorizontalTextAnchor::End
            };

            let config = GlyphTextConfig::rotated(10.0, ui_theme.text_primary, rotation);

            div()
                .absolute()
                .left(px(lx as f32))
                .top(px(ly as f32))
                .child(render_glyph_text_anchored(
                    &names[g.index],
                    &config,
                    h_anchor,
                    VerticalTextAnchor::Middle,
                ))
        })
        .collect();

    // The official example has no legend: groups are named around the circle.
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
                .child("Chord Diagram"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child("Source: observablehq.com/@d3/chord-diagram"),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .relative()
                // Canvas for arcs, ticks, and ribbons
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
                // Group name labels as rotated glyph text
                .children(label_items)
                // Tick value labels
                .children(tick_labels.iter().map(|(text, x, y)| {
                    div()
                        .absolute()
                        .left(px((*x - 20.0) as f32))
                        .top(px((*y - 6.0) as f32))
                        .w(px(40.0))
                        .flex()
                        .justify_center()
                        .text_size(px(8.0))
                        .child(text.clone())
                })),
        )
}
