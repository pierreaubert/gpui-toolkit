use crate::ShowcaseApp;
use d3rs::chord::{ChordLayout, RibbonGenerator};
use d3rs::text::{
    GlyphTextConfig, HorizontalTextAnchor, VerticalTextAnchor, measure_glyph_text_width,
    render_glyph_text_anchored,
};
use gpui::*;
use crate::showcase_modules::chart_colors;
use gpui_ui_kit::theme::ThemeExt;
use std::f64::consts::PI;

pub fn render(_app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let matrix = vec![
        vec![11975.0, 5871.0, 8916.0, 2868.0],
        vec![1951.0, 10048.0, 2060.0, 6171.0],
        vec![8010.0, 16145.0, 8090.0, 8045.0],
        vec![1013.0, 990.0, 940.0, 6907.0],
    ];
    let names = ["Black", "Blond", "Brown", "Red"];

    let layout = ChordLayout::new()
        .pad_angle(0.05)
        .sort_subgroups(|a, b| b.partial_cmp(&a).unwrap_or(std::cmp::Ordering::Equal));
    let chords = layout.compute(&matrix);

    let outer_radius = 180.0;
    let inner_radius = 160.0;
    let tick_radius = outer_radius + 4.0;
    let label_radius = outer_radius + 18.0;

    let width = 600.0;
    let height = 600.0;

    let ribbon = RibbonGenerator::new(inner_radius);

    let tick_color = chart_colors::ink_hex(&ui_theme, 0x444444);
    let colors = [
        chart_colors::ink_hex(&ui_theme, 0x000000),
        chart_colors::ink_hex(&ui_theme, 0xffdd89),
        chart_colors::ink_hex(&ui_theme, 0x957244),
        chart_colors::ink_hex(&ui_theme, 0xf26223),
    ];

    use d3rs::shape::arc::{Arc, ArcDatum};
    let arc_gen = Arc::new();

    // Pre-compute arc-following label glyphs: each character of a group name
    // is placed on the label circle (outside the ticks) at the group's
    // mid-angle, rotated tangent to the arc. Labels centered in the bottom
    // half flip tops-inward so the text never reads upside-down.
    let label_font_size = 10.0;
    let label_glyphs: Vec<(String, f64, f64, f32)> = chords
        .groups
        .iter()
        .flat_map(|g| {
            let name = names[g.index % names.len()];
            let d3_mid = (g.start_angle + g.end_angle) / 2.0;
            let std_mid = d3_mid - PI / 2.0;
            let flip = std_mid.sin() > 0.0;
            let advances: Vec<f32> = name
                .chars()
                .map(|ch| {
                    let mut buf = [0u8; 4];
                    measure_glyph_text_width(ch.encode_utf8(&mut buf), label_font_size)
                })
                .collect();
            let total: f32 = advances.iter().sum();
            if total <= 0.0 {
                return Vec::new();
            }
            let span = f64::from(total) / label_radius;
            let mut acc = 0.0f32;
            name.chars()
                .zip(advances.iter())
                .map(|(ch, advance)| {
                    let center_frac =
                        (acc + *advance / 2.0) / total;
                    acc += *advance;
                    let angle = if flip {
                        std_mid + span / 2.0 - f64::from(center_frac) * span
                    } else {
                        std_mid - span / 2.0 + f64::from(center_frac) * span
                    };
                    let rotation = (if flip {
                        angle - PI / 2.0
                    } else {
                        angle + PI / 2.0
                    }) as f32;
                    let gx = width / 2.0 + label_radius * angle.cos();
                    let gy = height / 2.0 + label_radius * angle.sin();
                    (ch.to_string(), gx, gy, rotation)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .child("Chord Diagram"),
        )
        .child(
            div()
                .text_xs()
                .child("Hair color preferences — with ticks and group labels"),
        )
        .child(
            // Legend
            div()
                .flex()
                .gap_4()
                .children(names.iter().enumerate().map(|(i, name)| {
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size_3().bg(colors[i % colors.len()]))
                        .child(div().text_xs().child(*name))
                })),
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
                        |_bounds, _window, _cx| {},
                        move |bounds, _state, window, _cx| {
                            let center = bounds.center();
                            let paint_d3_path =
                                |d3_path: d3rs::shape::path::Path,
                                 color: Rgba,
                                 opacity: f32,
                                 window: &mut gpui::Window| {
                                    let points = d3_path.flatten(0.1);
                                    if points.is_empty() {
                                        return;
                                    }

                                    let mut builder = gpui::PathBuilder::fill();
                                    let start =
                                        point(px(points[0].x as f32), px(points[0].y as f32))
                                            + center;
                                    builder.move_to(start);
                                    for pt in &points[1..] {
                                        let p = point(px(pt.x as f32), px(pt.y as f32)) + center;
                                        builder.line_to(p);
                                    }
                                    builder.close();

                                    if let Ok(path) = builder.build() {
                                        let final_color = gpui::Rgba {
                                            r: color.r,
                                            g: color.g,
                                            b: color.b,
                                            a: opacity,
                                        };
                                        window.paint_path(path, final_color);
                                    }
                                };

                            // Draw group arcs
                            for group in &chords.groups {
                                let datum = ArcDatum::new()
                                    .inner_radius(inner_radius)
                                    .outer_radius(outer_radius)
                                    .start_angle(group.start_angle - PI / 2.0)
                                    .end_angle(group.end_angle - PI / 2.0);

                                let d3_path = arc_gen.generate(&datum);
                                let color = colors[group.index % colors.len()];
                                paint_d3_path(d3_path, Rgba::from(color), 1.0, window);
                            }

                            // Draw tick marks along each group arc
                            for group in &chords.groups {
                                let arc_span = group.end_angle - group.start_angle;
                                let n_ticks = ((arc_span * 30.0) as usize).max(2);
                                for t in 0..=n_ticks {
                                    let frac = t as f64 / n_ticks as f64;
                                    let angle = group.start_angle + arc_span * frac - PI / 2.0;
                                    let x1 = outer_radius * angle.cos();
                                    let y1 = outer_radius * angle.sin();
                                    let x2 = tick_radius * angle.cos();
                                    let y2 = tick_radius * angle.sin();

                                    // Tick as a thin line
                                    let nx = -angle.sin() * 0.5;
                                    let ny = angle.cos() * 0.5;
                                    let mut builder = gpui::PathBuilder::fill();
                                    builder.move_to(
                                        center + point(px((x1 + nx) as f32), px((y1 + ny) as f32)),
                                    );
                                    builder.line_to(
                                        center + point(px((x2 + nx) as f32), px((y2 + ny) as f32)),
                                    );
                                    builder.line_to(
                                        center + point(px((x2 - nx) as f32), px((y2 - ny) as f32)),
                                    );
                                    builder.line_to(
                                        center + point(px((x1 - nx) as f32), px((y1 - ny) as f32)),
                                    );
                                    builder.close();
                                    if let Ok(path) = builder.build() {
                                        window.paint_path(path, tick_color);
                                    }
                                }
                            }

                            // Draw chord ribbons
                            for chord in &chords.chords {
                                let d3_path = ribbon.generate_path(chord);
                                let color = colors[chord.target.index % colors.len()];
                                paint_d3_path(d3_path, Rgba::from(color), 0.67, window);
                            }
                        },
                    )
                    .size_full(),
                )
                // Group name labels as arc-following glyphs outside the ticks
                .children(
                    label_glyphs
                        .iter()
                        .map(|(glyph, gx, gy, rotation)| {
                            let config = GlyphTextConfig::rotated(
                                label_font_size,
                                ui_theme.text_primary,
                                *rotation,
                            );
                            div()
                                .absolute()
                                .left(px(*gx as f32))
                                .top(px(*gy as f32))
                                .child(render_glyph_text_anchored(
                                    glyph,
                                    &config,
                                    HorizontalTextAnchor::Middle,
                                    VerticalTextAnchor::Middle,
                                ))
                        }),
                ),
        )
}
