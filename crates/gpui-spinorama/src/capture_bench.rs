//! Headless capture benchmark (macOS spike).
//!
//! Renders one `SpinoramaApp` section from a local Plotly-shaped JSON
//! fixture through the Metal headless renderer, saves a PNG, and reports
//! per-frame draw/capture timings.
//!
//! Usage: `spinorama-showcase --capture-bench <cea2034|contour> <fixture.json> <out.png> [iterations]`

// Rust guideline compliant 2026-02-21

use crate::api::{contour_plot_data, extract_cea2034_curves_original};
use crate::headless::{draw_capture, make_context, open_app_window};
use crate::types::{LoadState, PlotSection};
use gpui::AnyWindowHandle;
use std::path::Path;
use std::time::Instant;

/// Benchmark viewport in CSS pixels: matches the public site's 1200x800
/// graph images and the showcase default window size.
const WIDTH: f32 = 1200.0;
/// See [`WIDTH`].
const HEIGHT: f32 = 800.0;

/// Runs the headless capture benchmark.
///
/// # Errors
///
/// Returns an error when arguments are missing, the fixture cannot be
/// read or parsed, no Metal adapter is available, or rendering fails.
pub fn run(args: &[String]) -> Result<(), String> {
    let section_arg = args.first().map_or("cea2034", String::as_str);
    let fixture = args
        .get(1)
        .ok_or("--capture-bench requires a fixture JSON path")?;
    let out = args
        .get(2)
        .ok_or("--capture-bench requires an output PNG path")?;
    let iterations: usize = args
        .get(3)
        .map_or(Ok(20), |s| s.parse())
        .map_err(|_| "iterations must be a number")?;

    let raw = std::fs::read_to_string(fixture).map_err(|e| format!("read {fixture}: {e}"))?;
    let parse_started = Instant::now();
    let plot: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("parse {fixture}: {e}"))?;
    let section = match section_arg {
        "cea2034" => PlotSection::CEA2034,
        "contour" => PlotSection::Contour,
        other => return Err(format!("unknown section {other:?} (cea2034|contour)")),
    };
    let curves = if section == PlotSection::CEA2034 {
        Some(extract_cea2034_curves_original(&plot, "CEA2034")?)
    } else {
        None
    };
    let contour_data = if section == PlotSection::Contour {
        Some(contour_plot_data(&plot)?)
    } else {
        None
    };
    let parse_ms = parse_started.elapsed().as_secs_f64() * 1000.0;

    let mut cx = make_context()?;
    let handle = open_app_window(&mut cx, WIDTH, HEIGHT)?;
    handle
        .update(&mut cx, |app_state, _window, _cx| {
            app_state.selected_speaker = Some("KEF R3".to_string());
            app_state.selected_version = Some("asr".to_string());
            app_state.speakers_load_state = LoadState::Loaded;
            app_state.versions_load_state = LoadState::Loaded;
            app_state.data_load_state = LoadState::Loaded;
            app_state.current_section = section;
            if let Some(curves) = curves.clone() {
                app_state.cea2034_curves = curves;
            }
            if let Some(contour_data) = contour_data.clone() {
                app_state.set_contour_data(contour_data);
            }
        })
        .map_err(|e| format!("inject fixture data: {e}"))?;
    let any_handle: AnyWindowHandle = handle.into();

    // Warm up: first frame pays font, pipeline and geometry-cache costs.
    let warmup_started = Instant::now();
    let mut image = draw_capture(&mut cx, any_handle)?;
    let warmup_ms = warmup_started.elapsed().as_secs_f64() * 1000.0;

    let mut samples_ms = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let started = Instant::now();
        image = draw_capture(&mut cx, any_handle)?;
        samples_ms.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    samples_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_ms = samples_ms.get(samples_ms.len() / 2).copied().unwrap_or(0.0);
    let min_ms = samples_ms.first().copied().unwrap_or(0.0);

    let save_started = Instant::now();
    image
        .save(Path::new(out))
        .map_err(|e| format!("save {out}: {e}"))?;
    let save_ms = save_started.elapsed().as_secs_f64() * 1000.0;

    println!(
        "capture-bench section={section_arg} fixture={fixture} size={}x{} parse_ms={parse_ms:.2} warmup_ms={warmup_ms:.2} median_frame_ms={median_ms:.2} min_frame_ms={min_ms:.2} png_save_ms={save_ms:.2} iterations={iterations} out={out}",
        image.width(),
        image.height()
    );
    Ok(())
}
