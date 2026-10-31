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

use crate::api::{contour_plot_data, extract_cea2034_curves_original};
use crate::headless::{draw_capture, make_context, open_app_window};
use crate::types::{ContourRenderMode, LoadState, PlotSection};
use d3rs::color::D3Color;
use d3rs::zoom::ZoomState;
use gpui::{AnyWindowHandle, HeadlessAppContext, WindowHandle};
use crate::app::SpinoramaApp;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Render viewport in CSS pixels: matches the public site's 1200x800
/// graph images (captured at 2x, downscaled by the hardening step).
const WIDTH: f32 = 1200.0;
/// See [`WIDTH`].
const HEIGHT: f32 = 800.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum GraphKind {
    Cea2034,
    Contour { plane: &'static str },
}

fn classify(stem: &str) -> Option<GraphKind> {
    match stem {
        "CEA2034" | "CEA2034 Normalized" => Some(GraphKind::Cea2034),
        "SPL Horizontal Contour" | "SPL Horizontal Contour Normalized" => {
            Some(GraphKind::Contour { plane: "Horizontal" })
        }
        "SPL Vertical Contour" | "SPL Vertical Contour Normalized" => {
            Some(GraphKind::Contour { plane: "Vertical" })
        }
        _ => None,
    }
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
) -> Result<image::RgbaImage, String> {
    let title = title_text(plot);
    match kind {
        GraphKind::Cea2034 => {
            let curves = extract_cea2034_curves_original(plot, "CEA2034")?;
            let colors = trace_colors(plot);
            let (x0, x1) = axis_range(plot, "xaxis")
                .map_or((20.0, 20_000.0), |(lo, hi)| (10f64.powf(lo), 10f64.powf(hi)));
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
                    state.di_domain = di_domain;
                    state.freq_spl_zoom = ZoomState::new(x0, x1, y0, y1).with_log_x(true);
                    state.cea2034_curves = curves;
                })
                .map_err(|e| format!("inject CEA2034 data: {e}"))?;
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
                    state.contour_mode_spl = ContourRenderMode::Surface;
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
        args.first().ok_or("--render-images requires <private_root> <out_root>")?,
    );
    let out_root = PathBuf::from(
        args.get(1).ok_or("--render-images requires <private_root> <out_root>")?,
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

    // SAFETY: set once at startup, before any other thread reads the env.
    unsafe { std::env::set_var("SPINORAMA_OFFLINE", "1") };

    let mut files = Vec::new();
    collect_json(&private_root, &mut files)?;
    let jobs: Vec<(PathBuf, GraphKind)> = files
        .into_iter()
        .filter(|path| filter.as_deref().is_none_or(|needle| path.to_string_lossy().contains(needle)))
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
            let image = render_one(&mut cx, &handle, any_handle, *kind, &plot, speaker, version)?;
            let out_path = out_root.join(relative).with_extension("png");
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
            }
            image.save(&out_path).map_err(|e| format!("save {}: {e}", out_path.display()))?;
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
