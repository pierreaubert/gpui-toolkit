//! Batch static graph rendering from private Plotly JSON (macOS).
//!
//! Walks a private measurement tree (`dist-private/speakers`), renders
//! chart-only PNGs for the supported graph kinds (CEA2034 and SPL
//! contours) through the Metal headless renderer, and mirrors the tree
//! layout in the output directory. Titles, axis ranges and trace colors
//! are taken from the Plotly layout so output matches the public site.
//!
//! Usage: `spinorama-render-images <private_root> <out_root> [--limit N] [--filter SUBSTR]`

// Rust guideline compliant 2026-02-21

use crate::api::{
    contour_plot_data, extract_cea2034_curves_original, parse_plot, plotly_symbol,
};
use crate::app::SpinoramaApp;
use crate::headless::{draw_capture, make_context, open_app_window};
use crate::types::{
    ContourRenderMode, GRAPH_CATALOG, GraphRenderer, LoadState, PlotSection,
    graph_for_measurement,
};
use d3rs::color::D3Color;
use d3rs::zoom::ZoomState;
use gpui::{AnyWindowHandle, HeadlessAppContext, WindowHandle};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Render viewport in CSS pixels: matches the public site's 1200x800
/// graph images (captured at 2x, downscaled by the hardening step).
const WIDTH: f32 = 1200.0;
/// See [`WIDTH`].
const HEIGHT: f32 = 800.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphKind {
    Cea2034,
    Contour { plane: &'static str },
    /// Any other catalog graph, rendered through the generic views.
    Generic { graph_index: usize },
}

fn classify(stem: &str) -> Option<GraphKind> {
    match stem {
        // Preserve the complete payload, including filled validity zones.
        "CEA2034" | "CEA2034 Normalized" => Some(GraphKind::Generic {
            graph_index: GRAPH_CATALOG
                .iter()
                .position(|graph| graph.measurement == "CEA2034")?,
        }),
        "filters_spin_noeq" | "filters_spin_auto" => {
            Some(GraphKind::Cea2034)
        }
        "SPL Horizontal Contour" | "SPL Horizontal Contour Normalized" => {
            Some(GraphKind::Contour {
                plane: "Horizontal",
            })
        }
        "SPL Vertical Contour" | "SPL Vertical Contour Normalized" => {
            Some(GraphKind::Contour { plane: "Vertical" })
        }
        _ => {
            // Prefer an exact label match: the 3D entries share their
            // measurement name with the 2D contour entries
            // ("SPL Horizontal Contour") but their label is the file
            // stem ("SPL Horizontal Contour 3D"); a measurement-first
            // lookup would render every 3D/globe file as a 2D contour.
            let graph_index = GRAPH_CATALOG
                .iter()
                .position(|g| g.label == stem)
                .or_else(|| {
                    let def = graph_for_measurement(stem)?;
                    GRAPH_CATALOG
                        .iter()
                        .position(|g| g.measurement == def.measurement)
                })?;
            Some(GraphKind::Generic { graph_index })
        }
    }
}

/// True for the two directivity graphs, whose pictures are cropped.
fn is_directivity(kind: GraphKind) -> bool {
    match kind {
        GraphKind::Generic { graph_index } => matches!(
            GRAPH_CATALOG[graph_index].measurement,
            "Horizontal Directivity" | "Vertical Directivity"
        ),
        _ => false,
    }
}

/// Crops trailing rows that are uniformly the background colour.
///
/// Directivity charts only occupy the top of the capture; keeping the
/// blank band below the legend would give the published picture dead
/// height. A small padding row is kept under the last content row.
fn crop_blank_bottom(image: &image::RgbaImage) -> image::RgbaImage {
    let (width, height) = image.dimensions();
    if height == 0 || width == 0 {
        return image.clone();
    }
    let bg = *image.get_pixel(0, height - 1);
    let blank = |y: u32| -> bool {
        (0..width).all(|x| {
            let p = image.get_pixel(x, y);
            (0..3).all(|c| (i32::from(p[c]) - i32::from(bg[c])).abs() <= 2)
        })
    };
    let mut last_content = height - 1;
    while last_content > 0 && blank(last_content) {
        last_content -= 1;
    }
    let keep = (last_content + 9).min(height);
    if keep == height {
        return image.clone();
    }
    image::imageops::crop_imm(image, 0, 0, width, keep).to_image()
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("read_dir {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "traces") {
                continue;
            }
            collect_json(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            out.push(path);
        }
    }
    Ok(())
}

fn title_text(plot: &Value) -> Option<String> {
    let raw = plot.pointer("/layout/title/text")?.as_str()?;
    Some(raw.replace("<br>", " ").replace("<br/>", " "))
}

fn axis_range(plot: &Value, axis: &str) -> Option<(f64, f64)> {
    let range = plot.pointer(&format!("/layout/{axis}/range"))?.as_array()?;
    Some((range.first()?.as_f64()?, range.get(1)?.as_f64()?))
}

fn parse_hex_color(text: &str) -> Option<D3Color> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(D3Color::rgb(r, g, b))
}

fn tickvals(plot: &Value, axis: &str) -> Vec<f64> {
    plot.pointer(&format!("/layout/{axis}/tickvals"))
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default()
}

fn trace_markers(
    plot: &Value,
) -> (
    HashMap<String, d3rs::shape::SymbolType>,
    HashMap<String, f32>,
) {
    let mut symbols = HashMap::new();
    let mut sizes = HashMap::new();
    if let Some(traces) = plot.get("data").and_then(Value::as_array) {
        for trace in traces {
            let Some(name) = trace.get("name").and_then(Value::as_str) else {
                continue;
            };
            if let Some(symbol) = trace
                .pointer("/marker/symbol")
                .and_then(Value::as_str)
                .and_then(plotly_symbol)
            {
                symbols.insert(name.to_string(), symbol);
                sizes.insert(
                    name.to_string(),
                    trace
                        .pointer("/marker/size")
                        .and_then(Value::as_f64)
                        .unwrap_or(7.0) as f32,
                );
            }
        }
    }
    (symbols, sizes)
}

fn trace_colors(plot: &Value) -> HashMap<String, D3Color> {
    let mut colors = HashMap::new();
    if let Some(traces) = plot.get("data").and_then(Value::as_array) {
        for trace in traces {
            let Some(name) = trace.get("name").and_then(Value::as_str) else {
                continue;
            };
            let color_text = trace
                .pointer("/line/color")
                .or_else(|| trace.pointer("/marker/color"))
                .and_then(Value::as_str);
            if let Some(color) = color_text.and_then(parse_hex_color) {
                colors.insert(name.to_string(), color);
            }
        }
    }
    colors
}

fn render_one(
    cx: &mut HeadlessAppContext,
    handle: &WindowHandle<SpinoramaApp>,
    any_handle: AnyWindowHandle,
    kind: GraphKind,
    plot: &Value,
    speaker: &str,
    version: &str,
    _plot_dir: &Path,
) -> Result<image::RgbaImage, String> {
    let title = title_text(plot);
    match kind {
        GraphKind::Cea2034 => {
            let curves = extract_cea2034_curves_original(plot, "CEA2034")?;
            let colors = trace_colors(plot);
            let (marker_symbols, marker_sizes) = trace_markers(plot);
            let grid_tickvals = {
                let x = tickvals(plot, "xaxis");
                let y = tickvals(plot, "yaxis");
                if x.is_empty() && y.is_empty() {
                    None
                } else {
                    Some((x, y))
                }
            };
            let (x0, x1) = axis_range(plot, "xaxis").map_or((20.0, 20_000.0), |(lo, hi)| {
                (10f64.powf(lo), 10f64.powf(hi))
            });
            let (y0, y1) = axis_range(plot, "yaxis").unwrap_or((-40.0, 10.0));
            let di_domain = axis_range(plot, "yaxis2").unwrap_or((-5.0, 45.0));
            handle
                .update(cx, |state, _window, _cx| {
                    state.chart_only = true;
                    state.selected_speaker = Some(speaker.to_string());
                    state.selected_version = Some(version.to_string());
                    state.speakers_load_state = LoadState::Loaded;
                    state.versions_load_state = LoadState::Loaded;
                    state.data_load_state = LoadState::Loaded;
                    state.current_section = PlotSection::CEA2034;
                    state.title_override = title;
                    state.trace_colors = colors;
                    state.cea_marker_symbols = marker_symbols;
                    state.cea_marker_sizes = marker_sizes;
                    state.cea_grid_tickvals = grid_tickvals;
                    state.di_domain = di_domain;
                    state.freq_spl_zoom = ZoomState::new(x0, x1, y0, y1).with_log_x(true);
                    state.cea2034_curves = curves;
                })
                .map_err(|e| format!("inject CEA2034 data: {e}"))?;
        }
        GraphKind::Generic { graph_index } => {
            let def = &GRAPH_CATALOG[graph_index];
            let generic = parse_plot(plot)?;
            let section = match def.renderer {
                GraphRenderer::PolarChart => PlotSection::PolarChart,
                GraphRenderer::BarChart => PlotSection::BarChart,
                GraphRenderer::Contour => PlotSection::Contour,
                GraphRenderer::Surface3d => PlotSection::Surface3D,
                // Published Globe pictures are the polar disc
                // (angle around, log-frequency as radius): one static
                // image covering every frequency, unlike the
                // per-frequency 3D sphere used interactively.
                GraphRenderer::Sphere => PlotSection::PolarContour,
                _ => PlotSection::FrequencyChart,
            };
            handle
                .update(cx, |state, _window, _cx| {
                    state.chart_only = true;
                    state.selected_speaker = Some(speaker.to_string());
                    state.selected_version = Some(version.to_string());
                    state.speakers_load_state = LoadState::Loaded;
                    state.versions_load_state = LoadState::Loaded;
                    state.data_load_state = LoadState::Loaded;
                    state.current_section = section;
                    state.selected_graph_index = graph_index;
                    state.title_override = title;
                    state.generic_plot = Some(generic);
                    state.generic_load_state = LoadState::Loaded;
                    if matches!(
                        def.renderer,
                        GraphRenderer::Contour | GraphRenderer::Surface3d | GraphRenderer::Sphere
                    ) && let Ok(data) = contour_plot_data(plot)
                    {
                        state.set_contour_data(data);
                    }
                })
                .map_err(|e| format!("inject generic data: {e}"))?;
        }
        GraphKind::Contour { plane } => {
            let data = contour_plot_data(plot)?;
            handle
                .update(cx, |state, _window, _cx| {
                    state.chart_only = true;
                    state.selected_speaker = Some(speaker.to_string());
                    state.selected_version = Some(version.to_string());
                    state.speakers_load_state = LoadState::Loaded;
                    state.versions_load_state = LoadState::Loaded;
                    state.data_load_state = LoadState::Loaded;
                    state.current_section = PlotSection::Contour;
                    state.title_override = title;
                    state.trace_colors = HashMap::new();
                    state.contour_plane_label = plane;
                    // Published contour pictures are the smooth
                    // top-down raster of the SPL field (Heatmap mode
                    // resamples it bilinearly), not marching-squares
                    // band polygons.
                    state.contour_mode_spl = ContourRenderMode::Heatmap;
                    state.set_contour_data(data);
                })
                .map_err(|e| format!("inject contour data: {e}"))?;
        }
    }
    draw_capture(cx, any_handle)
}

/// Runs the batch renderer.
///
/// # Errors
///
/// Returns an error when arguments are missing, the tree cannot be
/// walked, no Metal adapter is available, or any file failed to render.
pub fn run(args: &[String]) -> Result<(), String> {
    let private_root = PathBuf::from(
        args.first()
            .ok_or("--render-images requires <private_root> <out_root>")?,
    );
    let out_root = PathBuf::from(
        args.get(1)
            .ok_or("--render-images requires <private_root> <out_root>")?,
    );
    let mut limit: Option<usize> = None;
    let mut filter: Option<String> = None;
    let mut iter = args.iter().skip(2);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--limit" => {
                limit = Some(
                    iter.next()
                        .ok_or("--limit requires a value")?
                        .parse()
                        .map_err(|_| "--limit must be a number")?,
                );
            }
            "--filter" => filter = Some(iter.next().ok_or("--filter requires a value")?.clone()),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }

    let mut files = Vec::new();
    collect_json(&private_root, &mut files)?;
    let jobs: Vec<(PathBuf, GraphKind)> = files
        .into_iter()
        .filter(|path| {
            filter
                .as_deref()
                .is_none_or(|needle| path.to_string_lossy().contains(needle))
        })
        .filter_map(|path| {
            let kind = classify(path.file_stem()?.to_str()?)?;
            Some((path, kind))
        })
        .take(limit.unwrap_or(usize::MAX))
        .collect();

    let mut cx = make_context()?;
    let handle = open_app_window(&mut cx, WIDTH, HEIGHT)?;
    let any_handle: AnyWindowHandle = handle.into();

    let started = Instant::now();
    let mut ok_count = 0usize;
    let mut failed_count = 0usize;
    for (path, kind) in &jobs {
        let relative = path
            .strip_prefix(&private_root)
            .map_err(|e| format!("relative path {}: {e}", path.display()))?;
        let parts: Vec<String> = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        let speaker = parts.first().map_or("Unknown", String::as_str);
        let version = parts.get(2).map_or("", String::as_str);
        let result = (|| -> Result<(), String> {
            let raw = std::fs::read_to_string(path).map_err(|e| format!("read: {e}"))?;
            let plot: Value = serde_json::from_str(&raw).map_err(|e| format!("parse: {e}"))?;
            let plot_dir = path.parent().ok_or("plot has no parent directory")?;
            let image = render_one(
                &mut cx,
                &handle,
                any_handle,
                *kind,
                &plot,
                speaker,
                version,
                plot_dir,
            )?;
            let image = if is_directivity(*kind) {
                crop_blank_bottom(&image)
            } else {
                image
            };
            let out_path = out_root.join(relative).with_extension("png");
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
            }
            image
                .save(&out_path)
                .map_err(|e| format!("save {}: {e}", out_path.display()))?;
            Ok(())
        })();
        match result {
            Ok(()) => ok_count += 1,
            Err(error) => {
                failed_count += 1;
                eprintln!("render-images FAILED {}: {error}", path.display());
            }
        }
    }

    let elapsed = started.elapsed().as_secs_f64();
    let rendered = ok_count + failed_count;
    let mean_ms = if rendered > 0 {
        elapsed * 1000.0 / rendered as f64
    } else {
        0.0
    };
    println!(
        "render-images total={} ok={ok_count} failed={failed_count} seconds={elapsed:.2} mean_ms_per_image={mean_ms:.2} out={}",
        jobs.len(),
        out_root.display()
    );
    if failed_count > 0 {
        return Err(format!("{failed_count} file(s) failed to render"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_removes_blank_bottom_band() {
        // 4x10 white image with a black pixel at row 2.
        let mut image = image::RgbaImage::from_pixel(4, 10, image::Rgba([255, 255, 255, 255]));
        image.put_pixel(1, 2, image::Rgba([0, 0, 0, 255]));
        let cropped = crop_blank_bottom(&image);
        // Last content row (2) plus 8 padding rows, capped by height.
        assert_eq!(cropped.dimensions(), (4, 10.min(2 + 9)));
        let cropped = crop_blank_bottom(&image::RgbaImage::from_pixel(
            4,
            30,
            image::Rgba([255, 255, 255, 255]),
        ));
        assert_eq!(cropped.dimensions(), (4, 9));
    }

    #[test]
    fn directivity_stems_classify_as_directivity() {
        let index = GRAPH_CATALOG
            .iter()
            .position(|g| g.measurement == "Horizontal Directivity")
            .expect("catalog has directivity");
        assert!(is_directivity(GraphKind::Generic {
            graph_index: index
        }));
        assert!(!is_directivity(GraphKind::Cea2034));
    }

    #[test]
    fn spin_filters_use_cea_renderer() {
        assert_eq!(classify("filters_spin_auto"), Some(GraphKind::Cea2034));
        assert_eq!(classify("filters_spin_noeq"), Some(GraphKind::Cea2034));
    }
}
