//! Single-source Cartesian tick values and labels.
//!
//! Spinorama views always set explicit `x/y/z_ticks`, so the audio-domain
//! formatting below (kHz, degrees, dB) only ever applies to explicit ticks
//! or log-frequency data. Generic linear data with no explicit ticks gets
//! d3-style nice ticks with plain numeric labels instead of the old
//! fallbacks (log-frequency ticks that landed off-range, one 30° step,
//! one dB step).

use super::super::data::SurfaceData;
use super::linear::{format_plain_tick, linear_nice_ticks};
use super::misc::default_frequency_ticks;
use super::spl::spl_major_ticks;

/// Tick values plus their display labels for the three Cartesian axes.
/// Positions (paint loops) and grid majors consume the same vectors the
/// label cache formats, so the three can never drift apart.
pub struct CartesianTickPlan {
    pub x_ticks: Vec<f64>,
    pub x_labels: Vec<String>,
    pub y_ticks: Vec<f64>,
    pub y_labels: Vec<String>,
    pub z_ticks: Vec<f64>,
    pub z_labels: Vec<String>,
}

/// Audio frequency label: `100` stays bare, `1500` becomes `1.5k`.
fn format_audio_freq(freq: f64) -> String {
    if freq >= 1000.0 {
        format!("{}k", freq / 1000.0)
    } else {
        format!("{}", freq)
    }
}

pub(super) fn cartesian_tick_plan(data: &SurfaceData) -> CartesianTickPlan {
    // X: explicit ticks and log-frequency data keep the audio path; linear
    // data without explicit ticks gets nice ticks with plain labels.
    let x_audio = data.x_ticks.is_some() || data.x_log;
    let x_ticks = match &data.x_ticks {
        Some(ticks) => ticks.clone(),
        None if data.x_log => default_frequency_ticks(),
        None => linear_nice_ticks(data.x_min, data.x_max, 8),
    };
    let x_labels = x_ticks
        .iter()
        .map(|&value| {
            if x_audio {
                format_audio_freq(value)
            } else {
                format_plain_tick(value)
            }
        })
        .collect();

    // Y: explicit ticks keep the degree suffix; otherwise plain nice ticks.
    let y_audio = data.y_ticks.is_some();
    let y_ticks = match &data.y_ticks {
        Some(ticks) => ticks.clone(),
        None => linear_nice_ticks(data.y_min, data.y_max, 6),
    };
    let y_labels = y_ticks
        .iter()
        .map(|&value| {
            if y_audio {
                format!("{}°", format_plain_tick(value))
            } else {
                format_plain_tick(value)
            }
        })
        .collect();

    // Z: explicit ticks keep the dB suffix; otherwise plain nice ticks.
    let z_audio = data.z_ticks.is_some();
    let (z_ticks, _) = spl_major_ticks(data);
    let z_labels = z_ticks
        .iter()
        .map(|&value| {
            if z_audio {
                format!("{}dB", format_plain_tick(value))
            } else {
                format_plain_tick(value)
            }
        })
        .collect();

    CartesianTickPlan {
        x_ticks,
        x_labels,
        y_ticks,
        y_labels,
        z_ticks,
        z_labels,
    }
}

/// Test entry point: the exact tick values and labels the Cartesian
/// painters and grid majors use for `data`.
pub fn cartesian_tick_plan_for_testing(data: &SurfaceData) -> CartesianTickPlan {
    cartesian_tick_plan(data)
}
