//! Music Industry Revenue — Stacked Area Chart
//!
//! Uses real RIAA revenue data from music.csv, parsed with `d3rs::fetch::parse_csv`,
//! pivoted with `d3rs::examples::stacked_area::load_csv`, stacked with `d3rs::shape::stack::Stack`,
//! and rendered with `d3rs::shape::area::Area` + `d3rs_path_to_gpui_simple`.

use crate::ShowcaseApp;
use d3rs::legend::{LegendConfig, LegendItem, render_legend};
use d3rs::scale::{LinearScale, Scale};
use d3rs::shape::area::Area;
use d3rs::shape::curve::Curve;
use d3rs::shape::stack::{Stack, StackOffset, StackOrder};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;

const MUSIC_CSV: &str = include_str!("../../data/music.csv");

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let width = app.content_width as f64;
    let height = (width * 0.56).min(app.content_height as f64 * 0.6);
    let margin_left = 70.0;
    let margin_right = 20.0;
    let margin_top = 20.0;
    let margin_bottom = 50.0;
    let chart_width = width - margin_left - margin_right;
    let chart_height = height - margin_top - margin_bottom;

    // Load real music revenue data via d3rs CSV parser + pivot
    let (categories, rows) =
        d3rs::examples::stacked_area::load_csv(MUSIC_CSV, "year", "name", "value");

    let n = rows.len();
    if n == 0 {
        return div().child("No data loaded");
    }

    // Extract matrix, clamping negative values to 0 (some formats have negative "returns")
    let matrix: Vec<Vec<f64>> = rows
        .iter()
        .map(|r| r.values.iter().map(|v| v.max(0.0)).collect())
        .collect();
    // Years: the dates are epoch seconds from year values. Convert back to years for display.
    let first_year = 1973.0_f64;
    let last_year = first_year + (n - 1) as f64;

    // Stack with appearance order (formats appear chronologically)
    let stack = Stack::new()
        .keys(categories.clone())
        .offset(StackOffset::None)
        .order(StackOrder::Appearance);
    let series = stack.generate(&matrix);

    // Scales
    let x_scale = LinearScale::new()
        .domain(first_year, last_year)
        .range(0.0, chart_width);

    let max_y = series
        .iter()
        .flat_map(|s| s.values.iter())
        .flat_map(|p| [p[0], p[1]])
        .fold(0.0f64, f64::max);
    let y_scale = LinearScale::new()
        .domain(0.0, max_y)
        .range(chart_height, 0.0);

    // Build area paths using d3rs Area generator
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    for s in &series {
        let data: Vec<(usize, [f64; 2])> = (0..n)
            .map(|i| (i, s.get(i).unwrap_or([0.0, 0.0])))
            .collect();

        let area = Area::new()
            .x(move |d: &(usize, [f64; 2])| x_scale.scale(first_year + d.0 as f64))
            .y0(move |d: &(usize, [f64; 2])| y_scale.scale(d.1[0]))
            .y1(move |d: &(usize, [f64; 2])| y_scale.scale(d.1[1]))
            .curve(Curve::linear());

        d3_paths.push(area.generate(&data));
    }

    // Observable color map for music formats
    let color_map: Vec<(&str, u32)> = vec![
        ("LP/EP", 0x2A5784),
        ("Vinyl Single", 0x43719F),
        ("8 - Track", 0x5B8DB8),
        ("Cassette", 0x7AAAD0),
        ("Cassette Single", 0x9BC7E4),
        ("Other Tapes", 0xBADDF1),
        ("Kiosk", 0xE1575A),
        ("CD", 0xEE7423),
        ("CD Single", 0xF59D3D),
        ("SACD", 0xFFC686),
        ("DVD Audio", 0x9D7760),
        ("Music Video (Physical)", 0xF1CF63),
        ("Download Album", 0x7C4D79),
        ("Download Single", 0x9B6A97),
        ("Ringtones & Ringbacks", 0xBE89AC),
        ("Download Music Video", 0xD5A5C4),
        ("Other Digital", 0xEFC9E6),
        ("Synchronization", 0xBBB1AC),
        ("Paid Subscription", 0x24693D),
        ("On-Demand Streaming (Ad-Supported)", 0x398949),
        ("Other Ad-Supported Streaming", 0x61AA57),
        ("SoundExchange Distributions", 0x7DC470),
        ("Limited Tier Paid Subscription", 0xB4E0A7),
    ];

    // Map each category to its color (fallback to grey)
    let colors: Vec<Rgba> = categories
        .iter()
        .map(|cat| {
            color_map
                .iter()
                .find(|(name, _)| *name == cat.as_str())
                .map(|(_, hex)| rgb(*hex))
                .unwrap_or(rgb(0x999999))
        })
        .collect();

    // Legend: show categories sorted by first appearance year (chronological)
    let mut cat_first_year: Vec<(usize, &str, usize)> = categories
        .iter()
        .enumerate()
        .filter_map(|(ci, name)| {
            // Find first year with nonzero value
            let first = matrix
                .iter()
                .position(|row| row.get(ci).copied().unwrap_or(0.0) > 0.0);
            first.map(|yr| (ci, name.as_str(), yr))
        })
        .collect();
    cat_first_year.sort_by_key(|&(_, _, yr)| yr);

    let legend_config = LegendConfig::new()
        .font_size(11.0)
        .symbol_size(10.0)
        .item_spacing(4.0)
        .padding(6.0)
        .items(
            cat_first_year
                .iter()
                .map(|&(ci, name, _)| {
                    let c = colors[ci];
                    LegendItem::color(
                        name,
                        d3rs::color::D3Color {
                            r: c.r,
                            g: c.g,
                            b: c.b,
                            a: c.a,
                        },
                    )
                })
                .collect(),
        );

    // X-axis ticks (every 5 years)
    let x_ticks: Vec<i32> = (1975..=2015).step_by(5).collect();

    // Y-axis ticks (revenue in billions)
    let y_tick_step = (max_y / 5.0).ceil();
    let y_tick_step = if y_tick_step > 1e9 {
        (y_tick_step / 1e9).ceil() * 1e9
    } else {
        y_tick_step
    };
    let y_ticks: Vec<f64> = (0..=8)
        .map(|i| i as f64 * y_tick_step)
        .filter(|v| *v <= max_y * 1.05)
        .collect();

    let theme = cx.theme();
    let chart_w = width as f32;

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
                .child("Revenue by Music Format 1973-2018"),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme.text_secondary)
                .mb_2()
                .child(format!(
                    "RIAA data — {} formats, {} years",
                    categories.len(),
                    n
                )),
        )
        .child(
            render_legend(
                &legend_config,
                chart_w,
                theme.text_primary,
                Some(theme.muted),
            )
            .mb_2(),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(theme.surface)
                .border_1()
                .border_color(theme.border)
                .relative()
                // Y-axis label
                .child(
                    div()
                        .absolute()
                        .left(px(2.0))
                        .top(px((margin_top + chart_height / 2.0 - 30.0) as f32))
                        .child(render_glyph_text(
                            "Revenue ($)",
                            &GlyphTextConfig::vertical_bottom_to_top(10.0, theme.text_secondary),
                        )),
                )
                // Y-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(1.0))
                        .h(px(chart_height as f32))
                        .bg(theme.border),
                )
                // X-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + chart_height) as f32))
                        .w(px(chart_width as f32))
                        .h(px(1.0))
                        .bg(theme.border),
                )
                // Y-axis ticks and labels
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
                    let label = if val >= 1e9 {
                        format!("{:.0}B", val / 1e9)
                    } else if val >= 1e6 {
                        format!("{:.0}M", val / 1e6)
                    } else {
                        format!("{:.0}", val)
                    };
                    let label_config = GlyphTextConfig::horizontal(9.0, theme.text_secondary);
                    div()
                        .absolute()
                        .left(px((margin_left - 40.0) as f32))
                        .top(px((margin_top + y - 6.0) as f32))
                        .w(px(35.0))
                        .flex()
                        .justify_end()
                        .child(render_glyph_text(&label, &label_config))
                }))
                // Y grid lines
                .children(y_ticks.iter().map(|&val| {
                    let y = y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + y) as f32))
                        .w(px(chart_width as f32))
                        .h(px(1.0))
                        .bg(theme.surface)
                }))
                // Vertical year-tick lines (every year, thin)
                .children((1973..=2018).map(|year| {
                    let x = x_scale.scale(year as f64);
                    div()
                        .absolute()
                        .left(px((margin_left + x) as f32))
                        .top(px(margin_top as f32))
                        .w(px(0.5))
                        .h(px(chart_height as f32))
                        .bg(theme.surface)
                }))
                // X-axis ticks and labels
                .children(x_ticks.iter().map(|&year| {
                    let x = x_scale.scale(year as f64);
                    let label_config = GlyphTextConfig::horizontal(9.0, theme.text_primary);
                    div()
                        .absolute()
                        .left(px((margin_left + x - 15.0) as f32))
                        .top(px((margin_top + chart_height) as f32))
                        .w(px(30.0))
                        .flex()
                        .flex_col()
                        .items_center()
                        .child(div().w(px(1.0)).h(px(5.0)).bg(theme.border))
                        .child(render_glyph_text(&format!("{}", year), &label_config))
                }))
                // X-axis label
                .child(
                    div()
                        .absolute()
                        .left(px((margin_left + chart_width / 2.0 - 10.0) as f32))
                        .top(px((height - 16.0) as f32))
                        .child(render_glyph_text(
                            "Year",
                            &GlyphTextConfig::horizontal(10.0, theme.text_secondary),
                        )),
                )
                // Chart area with stacked areas
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(chart_width as f32))
                        .h(px(chart_height as f32))
                        .overflow_hidden()
                        .child(
                            canvas(
                                move |bounds, _, _| {
                                    d3_paths
                                        .iter()
                                        .map(|p| {
                                            super::path_utils::d3rs_path_to_gpui_simple(
                                                p, bounds, 0.0, 0.0,
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                },
                                move |_bounds, paths, window, _| {
                                    for (i, path_opt) in paths.into_iter().enumerate() {
                                        if let Some(path) = path_opt {
                                            window.paint_path(path, colors[i % colors.len()]);
                                        }
                                    }
                                },
                            )
                            .size_full(),
                        ),
                ),
        )
}
