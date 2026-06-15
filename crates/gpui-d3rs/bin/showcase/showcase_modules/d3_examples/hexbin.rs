//! Hexbin Chart -- Observable example using d3rs::examples::hexbin
//!
//! Demonstrates idiomatic d3rs usage: `LogScale` for axes, `Hexbin` for binning,
//! `PathBuilder` for hex polygons, `d3rs_path_to_gpui_simple` for rendering.
use crate::ShowcaseApp;
use d3rs::color::SequentialScheme;
use d3rs::hexbin::Hexbin;
use d3rs::scale::{LogScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use std::rc::Rc;

const DIAMONDS_CSV: &str = include_str!("../../data/diamonds.csv");

/// Cached hexbin data so the expensive CSV parse, binning, and path generation
/// happen only once.
pub struct HexbinCache {
    pub data_count: usize,
    pub bin_count: usize,
    pub d3_paths: Rc<[d3rs::shape::path::Path]>,
    pub hex_colors: Rc<[Hsla]>,
    pub x_scale: LogScale,
    pub y_scale: LogScale,
    pub plot_w: f64,
    pub plot_h: f64,
}

fn build_cache() -> Rc<HexbinCache> {
    // Load real diamonds dataset (53,940 rows) via d3rs CSV parser
    let rows = d3rs::fetch::parse_csv(DIAMONDS_CSV).expect("valid diamonds CSV");
    let data: Vec<[f64; 2]> = rows
        .iter()
        .filter_map(|row| {
            let carat: f64 = row.get("carat")?.parse().ok()?;
            let price: f64 = row.get("price")?.parse().ok()?;
            if carat > 0.0 && price > 0.0 {
                Some([carat, price])
            } else {
                None
            }
        })
        .collect();

    let width = 700.0_f64;
    let height = 700.0_f64;
    let margin_left = 60.0_f64;
    let margin_top = 20.0_f64;
    let margin_right = 20.0_f64;
    let margin_bottom = 40.0_f64;
    let plot_w = width - margin_left - margin_right;
    let plot_h = height - margin_top - margin_bottom;

    // Log scales mapping data domain to plot area
    let x_min = data.iter().map(|d| d[0]).fold(f64::MAX, f64::min).max(0.1);
    let x_max = data.iter().map(|d| d[0]).fold(f64::MIN, f64::max);
    let y_min = data
        .iter()
        .map(|d| d[1])
        .fold(f64::MAX, f64::min)
        .max(100.0);
    let y_max = data.iter().map(|d| d[1]).fold(f64::MIN, f64::max);

    let x_scale = LogScale::new().domain(x_min, x_max).range(0.0, plot_w);
    let y_scale = LogScale::new().domain(y_min, y_max).range(plot_h, 0.0);

    // Map data points into plot coordinates and use d3rs Hexbin for binning
    let hex_radius = (8.0 * plot_w / 928.0).max(4.0);
    let mapped_data: Vec<[f64; 2]> = data
        .iter()
        .map(|d| [x_scale.scale(d[0]), y_scale.scale(d[1])])
        .collect();

    let hexbin: Hexbin<[f64; 2]> = Hexbin::new()
        .radius(hex_radius)
        .extent(0.0, 0.0, plot_w, plot_h);
    let bins = hexbin.bin(mapped_data);

    let max_count = bins.iter().map(|b| b.len()).max().unwrap_or(1);
    let data_count = data.len();
    let bin_count = bins.len();

    // Build a hexagon d3rs Path for each bin using D3PathBuilder (pointy-top like D3)
    let bu_pu = SequentialScheme::bu_pu();
    let mut d3_paths: Vec<d3rs::shape::path::Path> = Vec::new();
    let mut hex_colors: Vec<Hsla> = Vec::new();
    for bin in &bins {
        let cx = bin.x;
        let cy = bin.y;
        let mut builder = D3PathBuilder::new();
        for v in 0..6 {
            let angle = std::f64::consts::PI / 3.0 * v as f64 - std::f64::consts::FRAC_PI_2;
            let px_val = cx + hex_radius * angle.cos();
            let py_val = cy + hex_radius * angle.sin();
            if v == 0 {
                builder = builder.move_to(px_val, py_val);
            } else {
                builder = builder.line_to(px_val, py_val);
            }
        }
        builder = builder.close_path();
        d3_paths.push(builder.build());

        // Color: interpolateBuPu from d3rs sequential scheme
        let t = bin.len() as f64 / max_count as f64;
        hex_colors.push(bu_pu.get(t).to_rgba().into());
    }

    Rc::new(HexbinCache {
        data_count,
        bin_count,
        d3_paths: d3_paths.into(),
        hex_colors: hex_colors.into(),
        x_scale,
        y_scale,
        plot_w,
        plot_h,
    })
}

fn ensure_cache(app: &mut ShowcaseApp) -> Rc<HexbinCache> {
    if let Some(cache) = app.hexbin_cache.clone() {
        return cache;
    }
    let cache = build_cache();
    app.hexbin_cache = Some(cache.clone());
    cache
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let cache = ensure_cache(app);

    let width = 700.0_f64;
    let height = 700.0_f64;
    let margin_left = 60.0_f64;
    let margin_top = 20.0_f64;

    // Log-scale friendly ticks
    let x_ticks: Vec<f64> = vec![0.2, 0.5, 1.0, 2.0, 5.0];
    let y_ticks: Vec<f64> = vec![500.0, 1000.0, 2000.0, 5000.0, 10000.0];

    let data_count = cache.data_count;
    let bin_count = cache.bin_count;
    let bu_pu = SequentialScheme::bu_pu();

    // The canvas closures need their own cheap clone of the shared cache.
    let cache_for_paths = cache.clone();
    let cache_for_paint = cache.clone();

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
                .child("Hexbin Chart"),
        )
        .child(
            div()
                .text_xs()
                .mb_2()
                .child("Source: observablehq.com/@d3/hexbin"),
        )
        .child(
            div()
                .flex()
                .gap_4()
                .mb_2()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size_3().bg(bu_pu.get(0.1).to_rgba()))
                        .child(div().text_xs().child("Few points")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(div().size_3().bg(bu_pu.get(0.9).to_rgba()))
                        .child(div().text_xs().child("Many points")),
                )
                .child(
                    div()
                        .text_xs()
                        .child(format!("{} points -> {} bins", data_count, bin_count)),
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
                // Y-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(1.0))
                        .h(px(cache.plot_h as f32))
                        .bg(ui_theme.border),
                )
                // X-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + cache.plot_h) as f32))
                        .w(px(cache.plot_w as f32))
                        .h(px(1.0))
                        .bg(ui_theme.border),
                )
                // Y-axis tick labels
                .children(y_ticks.iter().map(|&val| {
                    let y = cache.y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px((margin_top + y - 6.0) as f32))
                        .w(px(margin_left as f32))
                        .flex()
                        .justify_end()
                        .pr_1()
                        .child(div().text_xs().child(format!("{:.0}", val)))
                }))
                // Y grid lines
                .children(y_ticks.iter().map(|&val| {
                    let y = cache.y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + y) as f32))
                        .w(px(cache.plot_w as f32))
                        .h(px(1.0))
                        .bg(ui_theme.surface)
                }))
                // X-axis tick labels
                .children(x_ticks.iter().map(|&val| {
                    let x = cache.x_scale.scale(val);
                    div()
                        .absolute()
                        .left(px((margin_left + x - 15.0) as f32))
                        .top(px((margin_top + cache.plot_h + 4.0) as f32))
                        .w(px(30.0))
                        .flex()
                        .justify_center()
                        .child(div().text_xs().child(if val < 1.0 {
                            format!("{:.1}", val)
                        } else {
                            format!("{:.0}", val)
                        }))
                }))
                // X grid lines
                .children(x_ticks.iter().map(|&val| {
                    let x = cache.x_scale.scale(val);
                    div()
                        .absolute()
                        .left(px((margin_left + x) as f32))
                        .top(px(margin_top as f32))
                        .w(px(1.0))
                        .h(px(cache.plot_h as f32))
                        .bg(ui_theme.surface)
                }))
                // Plot area with hexbin
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px(margin_top as f32))
                        .w(px(cache.plot_w as f32))
                        .h(px(cache.plot_h as f32))
                        .child(
                            canvas(
                                move |bounds, _, _| {
                                    cache_for_paths
                                        .d3_paths
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
                                            window.paint_path(path, cache_for_paint.hex_colors[i]);
                                        }
                                    }
                                },
                            )
                            .size_full(),
                        ),
                ),
        )
}
