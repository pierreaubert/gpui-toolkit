//! County Unemployment Choropleth — <https://observablehq.com/@d3/choropleth>
//!
//! Joins `unemployment-x.csv` rates (by FIPS id) to the `counties-albers-10m`
//! geometry (pre-projected Albers USA, like the official example's
//! `d3.geoPath()` with no projection), colors with
//! `scaleQuantize([1, 10], schemeBlues[9])`, and overlays white county and
//! state borders plus a legend ramp.
use crate::ShowcaseApp;
use crate::showcase_modules::chart_colors;
use d3rs::color::SequentialScheme;
use d3rs::geo::GeoJsonGeometry;
use d3rs::scale::{QuantizeScale, Scale};
use d3rs::shape::path::PathBuilder as D3PathBuilder;
use gpui::prelude::*;
use gpui::*;
use gpui_ui_kit::theme::ThemeExt;
use std::collections::HashMap;
use std::sync::OnceLock;

const COUNTIES_JSON: &str = include_str!("../../data/counties-albers-10m.json");
const UNEMPLOYMENT_CSV: &str = include_str!("../../data/unemployment-x.csv");

/// Decoded county polygons with FIPS ids, state polygons, and data bounds.
struct CountyCache {
    /// (FIPS id, rings) per county polygon part.
    features: Vec<(String, Vec<Vec<(f64, f64)>>)>,
    /// State polygons for the border overlay.
    states: Vec<Vec<Vec<(f64, f64)>>>,
    /// (min_x, min_y, max_x, max_y) over all county rings.
    bbox: (f64, f64, f64, f64),
    /// FIPS id -> unemployment rate.
    rates: HashMap<String, f64>,
}

fn county_cache() -> &'static CountyCache {
    static CACHE: OnceLock<CountyCache> = OnceLock::new();
    CACHE.get_or_init(|| {
        let mut features = Vec::new();
        let mut bbox = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        if let Ok(counties) = d3rs::geo::parse_counties(COUNTIES_JSON) {
            for county in &counties {
                if let GeoJsonGeometry::Polygon(rings) = &county.geometry {
                    for (x, y) in rings.iter().flat_map(|r| r.iter()) {
                        bbox.0 = bbox.0.min(*x);
                        bbox.1 = bbox.1.min(*y);
                        bbox.2 = bbox.2.max(*x);
                        bbox.3 = bbox.3.max(*y);
                    }
                    features.push((county.id.clone(), rings.clone()));
                }
            }
        }
        let mut states = Vec::new();
        if let Ok(Some(GeoJsonGeometry::MultiPolygon(polys))) =
            d3rs::geo::parse_county_states(COUNTIES_JSON)
        {
            states = polys;
        }
        let mut rates = HashMap::new();
        if let Ok(rows) = d3rs::fetch::parse_csv(UNEMPLOYMENT_CSV) {
            for row in &rows {
                if let (Some(id), Some(rate)) = (row.get("id"), row.get("rate")) {
                    // CSV ids are bare numbers ("1001"); geometry ids are
                    // zero-padded FIPS ("01001").
                    let digits: String = id.chars().filter(|c| c.is_ascii_digit()).collect();
                    if let Ok(rate) = rate.parse::<f64>() {
                        rates.insert(format!("{digits:0>5}"), rate);
                    }
                }
            }
        }
        CountyCache {
            features,
            states,
            bbox,
            rates,
        }
    })
}

/// Compact number formatting (12.5 -> "12.5").
fn format_rate(v: f64) -> String {
    format!("{v:.1}")
}

pub fn render(app: &ShowcaseApp, cx: &mut Context<ShowcaseApp>) -> Div {
    let ui_theme = cx.theme();
    let cache = county_cache();
    let width = app.content_width as f64;
    let height = (width * 0.625).min(app.content_height as f64 * 0.8);

    // Official color scale: quantize [1, 10] into 9 Blues.
    let blues = SequentialScheme::blues().sample(9);
    let fill_colors: Vec<Rgba> = blues.iter().map(|c| c.to_rgba()).collect();
    let quantize: QuantizeScale<usize> = QuantizeScale::new()
        .domain(1.0, 10.0)
        .range((0..9).collect());
    let missing_color = Rgba::from(chart_colors::missing(&ui_theme));

    // Fit the pre-projected geometry into the panel, preserving aspect.
    let (min_x, min_y, max_x, max_y) = cache.bbox;
    let data_w = (max_x - min_x).max(1.0);
    let data_h = (max_y - min_y).max(1.0);
    let fit = (width / data_w).min(height / data_h);
    let ox = (width - data_w * fit) / 2.0 - min_x * fit;
    let oy = (height - data_h * fit) / 2.0 - min_y * fit;
    let project = |x: f64, y: f64| (ox + x * fit, oy + y * fit);

    // Build one SVG string per county polygon part, in screen coordinates.
    let mut feature_paths: Vec<String> = Vec::with_capacity(cache.features.len());
    let mut feature_colors: Vec<Rgba> = Vec::with_capacity(cache.features.len());
    let mut missing = 0usize;
    for (id, rings) in &cache.features {
        let mut builder = D3PathBuilder::new();
        for ring in rings {
            for (i, &(x, y)) in ring.iter().enumerate() {
                let (sx, sy) = project(x, y);
                if i == 0 {
                    builder = builder.move_to(sx, sy);
                } else {
                    builder = builder.line_to(sx, sy);
                }
            }
            builder = builder.close_path();
        }
        feature_paths.push(builder.build().to_svg_string());
        match cache.rates.get(id) {
            Some(&rate) => {
                let ci = quantize.scale(rate).min(fill_colors.len() - 1);
                feature_colors.push(fill_colors[ci]);
            }
            None => {
                missing += 1;
                feature_colors.push(missing_color);
            }
        }
    }

    // State borders as SVG strings for the overlay.
    let mut state_paths: Vec<String> = Vec::with_capacity(cache.states.len());
    for polygon in &cache.states {
        let mut builder = D3PathBuilder::new();
        for ring in polygon {
            for (i, &(x, y)) in ring.iter().enumerate() {
                let (sx, sy) = project(x, y);
                if i == 0 {
                    builder = builder.move_to(sx, sy);
                } else {
                    builder = builder.line_to(sx, sy);
                }
            }
            builder = builder.close_path();
        }
        state_paths.push(builder.build().to_svg_string());
    }

    let county_border: Hsla = chart_colors::ink(&ui_theme, hsla(0.0, 0.0, 1.0, 0.9));
    let state_border: Hsla = chart_colors::ink(&ui_theme, hsla(0.0, 0.0, 1.0, 1.0));

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
                .child("Choropleth — County Unemployment"),
        )
        .child(div().text_xs().mb_2().child(format!(
            "Source: observablehq.com/@d3/choropleth — {} counties, {} without data",
            cache.features.len(),
            missing
        )))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .mb_2()
                .child(div().text_xs().child("Unemployment rate (%)"))
                .child(div().text_xs().child(format_rate(1.0)))
                .child(
                    div()
                        .flex()
                        .h(px(10.0))
                        .w(px(200.0))
                        .rounded_sm()
                        .overflow_hidden()
                        .children(fill_colors.iter().map(|c| div().flex_1().h_full().bg(*c))),
                )
                .child(div().text_xs().child(format!("{}+", format_rate(10.0)))),
        )
        .child(
            div()
                .w(px(width as f32))
                .h(px(height as f32))
                .bg(ui_theme.surface)
                .border_1()
                .border_color(ui_theme.border)
                .overflow_hidden()
                .relative()
                .child(canvas(
                    move |bounds, _, _| {
                        let fills: Vec<_> = feature_paths
                            .iter()
                            .map(|d| super::path_utils::parse_svg_path(d, bounds))
                            .collect();
                        let county_strokes: Vec<_> = feature_paths
                            .iter()
                            .map(|d| super::path_utils::parse_svg_path_stroke(d, bounds, 0.5))
                            .collect();
                        let state_strokes: Vec<_> = state_paths
                            .iter()
                            .map(|d| super::path_utils::parse_svg_path_stroke(d, bounds, 1.0))
                            .collect();
                        (fills, county_strokes, state_strokes)
                    },
                    move |_bounds, (fills, county_strokes, state_strokes), window, _| {
                        for (i, path_opt) in fills.into_iter().enumerate() {
                            if let Some(path) = path_opt {
                                window.paint_path(path, feature_colors[i]);
                            }
                        }
                        for path_opt in county_strokes.into_iter().flatten() {
                            window.paint_path(path_opt, county_border);
                        }
                        for path_opt in state_strokes.into_iter().flatten() {
                            window.paint_path(path_opt, state_border);
                        }
                    },
                )),
        )
}
