//! Hexbin Chart -- Observable example using d3rs::examples::hexbin
//!
//! Demonstrates idiomatic d3rs usage: `LogScale` for axes, `Hexbin` for binning,
//! `PathBuilder` for hex polygons, `d3rs_path_to_gpui_simple` for rendering.
use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::color::SequentialScheme;
use d3rs::hexbin::Hexbin;
use d3rs::scale::{LogScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::Slider;
use gpui_ui_kit::theme::ThemeExt;
use std::rc::Rc;

const DIAMONDS_CSV: &str = include_str!("../../data/diamonds.csv");

/// Log-decade ticks for a `[min, max]` domain: labeled majors at 1/2/5 × 10^k
/// and faint minors at 3/4/6/7/8/9 × 10^k, all clipped to the domain so no
/// tick silently falls outside a logarithmic axis.
fn log_axis_ticks(min: f64, max: f64) -> (Vec<f64>, Vec<f64>) {
    let mut majors = Vec::new();
    let mut minors = Vec::new();
    if min <= 0.0 || max <= min {
        return (majors, minors);
    }
    let lo = min.log10().floor() as i32;
    let hi = max.log10().ceil() as i32;
    for k in lo..=hi {
        let decade = 10_f64.powi(k);
        for m in 1..=9 {
            let value = m as f64 * decade;
            if value >= min && value <= max {
                if m == 1 || m == 2 || m == 5 {
                    majors.push(value);
                } else {
                    minors.push(value);
                }
            }
        }
    }
    (majors, minors)
}

/// Cached hexbin data: the CSV parse happens once per session (see
/// [`load_points`]); binning and path generation re-run only when the hexagon
/// radius changes.
pub struct HexbinCache {
    pub hex_radius: f32,
    pub data_count: usize,
    pub bin_count: usize,
    pub d3_paths: Rc<[d3rs::shape::path::Path]>,
    pub hex_colors: Rc<[Hsla]>,
    pub x_scale: LogScale,
    pub y_scale: LogScale,
    /// Data domains (for domain-derived log ticks).
    pub x_domain: (f64, f64),
    pub y_domain: (f64, f64),
    pub plot_w: f64,
    pub plot_h: f64,
}

/// Load real diamonds dataset (53,940 rows) via d3rs CSV parser.
fn load_points() -> Rc<[[f64; 2]]> {
    let rows = d3rs::fetch::parse_csv(DIAMONDS_CSV).expect("valid diamonds CSV");
    rows.iter()
        .filter_map(|row| {
            let carat: f64 = row.get("carat")?.parse().ok()?;
            let price: f64 = row.get("price")?.parse().ok()?;
            if carat > 0.0 && price > 0.0 {
                Some([carat, price])
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .into()
}

fn build_cache(data: &[[f64; 2]], hex_radius: f32) -> Rc<HexbinCache> {
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
    let hex_radius = f64::from(hex_radius);
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
        hex_radius: hex_radius as f32,
        data_count,
        bin_count,
        d3_paths: d3_paths.into(),
        hex_colors: hex_colors.into(),
        x_scale,
        y_scale,
        x_domain: (x_min, x_max),
        y_domain: (y_min, y_max),
        plot_w,
        plot_h,
    })
}

#[cfg(test)]
mod tests {
    use super::log_axis_ticks;

    #[test]
    fn log_ticks_cover_decades_without_duplicates_or_gaps() {
        // Diamonds price domain: labeled 1/2/5 set, minors fill the rest.
        let (majors, minors) = log_axis_ticks(326.0, 18823.0);
        assert_eq!(majors, vec![500.0, 1000.0, 2000.0, 5000.0, 10000.0]);
        for tick in majors.iter().chain(minors.iter()) {
            assert!(
                (326.0..=18823.0).contains(tick),
                "tick {tick} escapes the domain"
            );
        }
        assert!(minors.contains(&400.0));
        assert!(minors.contains(&9000.0));
        // No major duplicated as a minor.
        for major in &majors {
            assert!(!minors.contains(major), "major {major} duplicated");
        }
        // Carat domain keeps its classic labeled set.
        let (majors, _) = log_axis_ticks(0.2, 5.01);
        assert_eq!(majors, vec![0.2, 0.5, 1.0, 2.0, 5.0]);
        // Degenerate input yields no ticks instead of NaNs.
        assert_eq!(log_axis_ticks(0.0, -1.0), (Vec::new(), Vec::new()));
    }
}

fn ensure_cache(app: &mut ShowcaseApp) -> Rc<HexbinCache> {
    if app.hexbin_points.is_none() {
        app.hexbin_points = Some(load_points());
    }
    let radius = app.hexbin_radius;
    if let Some(cache) = app.hexbin_cache.clone()
        && cache.hex_radius == radius
    {
        return cache;
    }
    let points = app.hexbin_points.clone().expect("points loaded above");
    let cache = build_cache(&points, radius);
    app.hexbin_cache = Some(cache.clone());
    cache
}

pub fn render(app: &mut ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let entity = cx.entity().clone();
    let cache = ensure_cache(app);

    let width = 700.0_f64;
    let height = 700.0_f64;
    let margin_left = 60.0_f64;
    let margin_top = 20.0_f64;

    // Log-decade ticks derived from the data domains so every tick lands
    // inside its logarithmic axis.
    let (x_ticks, x_minor_ticks) = log_axis_ticks(cache.x_domain.0, cache.x_domain.1);
    let (y_ticks, y_minor_ticks) = log_axis_ticks(cache.y_domain.0, cache.y_domain.1);

    let data_count = cache.data_count;
    let bin_count = cache.bin_count;
    let bu_pu = SequentialScheme::bu_pu();

    // The canvas closures need their own cheap clone of the shared cache.
    let cache_for_paths = cache.clone();
    let cache_for_paint = cache.clone();
    let hex_colors: Rc<[Hsla]> = cache_for_paint
        .hex_colors
        .iter()
        .map(|c| chart_colors::ink(&ui_theme, *c))
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
                )
                .child({
                    let entity = entity.clone();
                    Slider::new("hexbin-radius")
                        .label("Hexagon size")
                        .value(app.hexbin_radius)
                        .min(2.0)
                        .max(20.0)
                        .step(0.5)
                        .show_value(true)
                        .width(200.0)
                        .on_change(move |value, _window, cx| {
                            entity.update(cx, |this, cx| {
                                this.hexbin_radius = value;
                                cx.notify();
                            });
                        })
                }),
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
                        .w(px(2.0))
                        .h(px(cache.plot_h as f32))
                        .bg(ui_theme.text_muted),
                )
                // X-axis line
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + cache.plot_h) as f32))
                        .w(px(cache.plot_w as f32))
                        .h(px(1.0))
                        .bg(ui_theme.text_muted),
                )
                // Axis titles (official: "Carats" / "$ Price", bold)
                .child(
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + cache.plot_h - 18.0) as f32))
                        .w(px(cache.plot_w as f32))
                        .flex()
                        .justify_end()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .child("Carats"),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .left(px((margin_left + 4.0) as f32))
                        .top(px(2.0))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::BOLD)
                                .child("$ Price"),
                        ),
                )
                // Y-axis tick marks (official: 6px outward ticks)
                .children(y_ticks.iter().map(|&val| {
                    let y = cache.y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px((margin_left - 6.0) as f32))
                        .top(px((margin_top + y) as f32))
                        .w(px(6.0))
                        .h(px(1.0))
                        .bg(ui_theme.text_muted)
                }))
                // Y-axis tick labels (cleared past the 6px marks)
                .children(y_ticks.iter().map(|&val| {
                    let y = cache.y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(0.0))
                        .top(px((margin_top + y - 6.0) as f32))
                        .w(px(margin_left as f32))
                        .flex()
                        .justify_end()
                        .pr_2()
                        .child(div().text_xs().child(format!("{:.0}", val)))
                }))
                // Fine Y grid lines
                .children(y_minor_ticks.iter().map(|&val| {
                    let y = cache.y_scale.scale(val);
                    div()
                        .absolute()
                        .left(px(margin_left as f32))
                        .top(px((margin_top + y) as f32))
                        .w(px(cache.plot_w as f32))
                        .h(px(1.0))
                        .bg(Hsla::from(ui_theme.text_muted).opacity(0.12))
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
                        .bg(Hsla::from(ui_theme.text_muted).opacity(0.25))
                }))
                // X-axis tick marks (official: 6px outward ticks)
                .children(x_ticks.iter().map(|&val| {
                    let x = cache.x_scale.scale(val);
                    div()
                        .absolute()
                        .left(px((margin_left + x) as f32))
                        .top(px((margin_top + cache.plot_h + 1.0) as f32))
                        .w(px(1.0))
                        .h(px(6.0))
                        .bg(ui_theme.text_muted)
                }))
                // X-axis tick labels (below the 6px marks plus a 3px gap)
                .children(x_ticks.iter().map(|&val| {
                    let x = cache.x_scale.scale(val);
                    div()
                        .absolute()
                        .left(px((margin_left + x - 15.0) as f32))
                        .top(px((margin_top + cache.plot_h + 10.0) as f32))
                        .w(px(30.0))
                        .flex()
                        .justify_center()
                        .child(div().text_xs().child(if val < 1.0 {
                            format!("{:.1}", val)
                        } else {
                            format!("{:.0}", val)
                        }))
                }))
                // Fine X grid lines
                .children(x_minor_ticks.iter().map(|&val| {
                    let x = cache.x_scale.scale(val);
                    div()
                        .absolute()
                        .left(px((margin_left + x) as f32))
                        .top(px(margin_top as f32))
                        .w(px(1.0))
                        .h(px(cache.plot_h as f32))
                        .bg(Hsla::from(ui_theme.text_muted).opacity(0.12))
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
                        .bg(Hsla::from(ui_theme.text_muted).opacity(0.25))
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
                                            window.paint_path(path, hex_colors[i]);
                                        }
                                    }
                                },
                            )
                            .size_full(),
                        ),
                ),
        )
}
