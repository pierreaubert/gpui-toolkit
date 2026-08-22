use gpui::SharedString;
use gpui_px::ScaleType;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone)]
pub(super) struct LineStoryData {
    pub(super) x: Arc<[f64]>,
    pub(super) y: Arc<[f64]>,
    pub(super) comparison_y: Option<Arc<[f64]>>,
    pub(super) title: &'static str,
    pub(super) x_label: &'static str,
    pub(super) y_label: &'static str,
    pub(super) primary_label: &'static str,
    pub(super) comparison_label: &'static str,
    pub(super) x_scale: ScaleType,
    pub(super) y_range: Option<(f64, f64)>,
}

#[derive(Clone)]
pub(super) struct AreaStoryData {
    pub(super) x: Arc<[f64]>,
    pub(super) y: Arc<[f64]>,
    pub(super) y0: Option<Arc<[f64]>>,
    pub(super) title: &'static str,
}

fn line_story_data_inner(series: &str) -> LineStoryData {
    match series {
        "sweep" => {
            let x: Vec<f64> = (0..72)
                .map(|index| 20.0 * 1000.0_f64.powf(index as f64 / 71.0))
                .collect();
            let y: Vec<f64> = x
                .iter()
                .map(|frequency| {
                    let octave = (frequency / 1000.0).log2();
                    (octave * 1.7).sin() * 2.4 - (frequency / 18_000.0).sqrt() * 1.6
                })
                .collect();
            let comparison_y: Vec<f64> = x
                .iter()
                .map(|frequency| -0.8 * (frequency / 20_000.0).sqrt())
                .collect();
            LineStoryData {
                x: x.into(),
                y: y.into(),
                comparison_y: Some(comparison_y.into()),
                title: "Frequency Sweep",
                x_label: "Hz",
                y_label: "dB",
                primary_label: "Measured",
                comparison_label: "Target",
                x_scale: ScaleType::Log,
                y_range: Some((-7.0, 5.0)),
            }
        }
        "flat" => {
            let x: Vec<f64> = (0..40).map(|index| index as f64).collect();
            let y: Vec<f64> = x
                .iter()
                .map(|value| (value * 0.41).sin() * 0.18 + (value * 0.09).cos() * 0.08)
                .collect();
            LineStoryData {
                x: x.into(),
                y: y.into(),
                comparison_y: None,
                title: "Flat Reference",
                x_label: "Step",
                y_label: "Delta",
                primary_label: "Reference",
                comparison_label: "Target",
                x_scale: ScaleType::Linear,
                y_range: Some((-1.0, 1.0)),
            }
        }
        _ => {
            let x: Vec<f64> = (0..64).map(|index| index as f64 / 6.0).collect();
            let y: Vec<f64> = x.iter().map(|value| value.sin()).collect();
            let comparison_y: Vec<f64> =
                x.iter().map(|value| (value * 0.72).cos() * 0.62).collect();
            LineStoryData {
                x: x.into(),
                y: y.into(),
                comparison_y: Some(comparison_y.into()),
                title: "Sine Envelope",
                x_label: "Time",
                y_label: "Value",
                primary_label: "Sine",
                comparison_label: "Cosine",
                x_scale: ScaleType::Linear,
                y_range: Some((-1.2, 1.2)),
            }
        }
    }
}

pub(super) fn line_story_data(series: &str) -> LineStoryData {
    static CACHE: OnceLock<std::collections::HashMap<String, LineStoryData>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            ["sweep", "flat", "sine"]
                .into_iter()
                .map(|series| (series.to_string(), line_story_data_inner(series)))
                .collect()
        })
        .get(series)
        .cloned()
        .unwrap_or_else(|| line_story_data_inner(series))
}

fn area_story_data_inner(series: &str) -> AreaStoryData {
    match series {
        "decay" => {
            let x: Vec<f64> = (0..64).map(|index| index as f64 / 8.0).collect();
            let y: Vec<f64> = x
                .iter()
                .map(|value| (value * 1.2).sin().abs() * (-value / 8.0).exp() + 0.04)
                .collect();
            AreaStoryData {
                x: x.into(),
                y: y.into(),
                y0: None,
                title: "Decay Envelope",
            }
        }
        "baseline" => {
            let x: Vec<f64> = (0..72).map(|index| index as f64 / 9.0).collect();
            let y0: Vec<f64> = x.iter().map(|value| value.sin() * 0.12 - 0.25).collect();
            let y: Vec<f64> = x
                .iter()
                .zip(y0.iter())
                .map(|(value, base)| base + 0.42 + (value * 1.4).cos().abs() * 0.28)
                .collect();
            AreaStoryData {
                x: x.into(),
                y: y.into(),
                y0: Some(y0.into()),
                title: "Baseline Band",
            }
        }
        _ => {
            let x: Vec<f64> = (0..72).map(|index| index as f64 / 9.0).collect();
            let y: Vec<f64> = x
                .iter()
                .map(|value| (value * 1.1).sin().abs() * 0.72 + (value * 0.45).cos() * 0.08)
                .collect();
            AreaStoryData {
                x: x.into(),
                y: y.into(),
                y0: None,
                title: "Signal Envelope",
            }
        }
    }
}

pub(super) fn area_story_data(series: &str) -> AreaStoryData {
    static CACHE: OnceLock<std::collections::HashMap<String, AreaStoryData>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            ["decay", "baseline", "envelope"]
                .into_iter()
                .map(|series| (series.to_string(), area_story_data_inner(series)))
                .collect()
        })
        .get(series)
        .cloned()
        .unwrap_or_else(|| area_story_data_inner(series))
}

#[derive(Clone)]
pub(super) struct BarStoryData {
    pub(super) categories: Arc<[SharedString]>,
    pub(super) values: Arc<[f64]>,
    pub(super) comparison_values: Arc<[f64]>,
}

fn bar_story_data_inner(count: usize) -> BarStoryData {
    let categories = (0..count)
        .map(|index| SharedString::new(format!("B{}", index + 1)))
        .collect::<Vec<_>>();
    let values = (0..count)
        .map(|index| {
            let t = index as f64 / count.max(1) as f64;
            34.0 + (t * std::f64::consts::TAU).sin().abs() * 42.0 + index as f64 * 1.8
        })
        .collect::<Vec<_>>();
    let comparison_values: Vec<f64> = (0..count)
        .map(|index| {
            let t = index as f64 / count.max(1) as f64;
            38.0 + (t * std::f64::consts::TAU + 0.8).cos().abs() * 34.0
        })
        .collect();

    BarStoryData {
        categories: categories.into(),
        values: values.into(),
        comparison_values: comparison_values.into(),
    }
}

pub(super) fn bar_story_data(count: usize) -> BarStoryData {
    static CACHE: OnceLock<Mutex<HashMap<usize, BarStoryData>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().unwrap();
    guard
        .entry(count)
        .or_insert_with(|| bar_story_data_inner(count))
        .clone()
}
