//! Browser-safe adapters for the Spinorama measurement API.

use base64::Engine as _;
use ndarray::Array1;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Curve {
    pub freq: Array1<f64>,
    pub spl: Array1<f64>,
}

#[derive(Debug, Clone)]
pub struct DirectivityCurve {
    pub angle: f64,
    pub freq: Array1<f64>,
    pub spl: Array1<f64>,
}

#[derive(Debug, Clone)]
pub struct DirectivityData {
    pub horizontal: Vec<DirectivityCurve>,
    pub vertical: Vec<DirectivityCurve>,
}

#[derive(Debug, Clone)]
pub struct ContourPlotData {
    pub freq: Vec<f64>,
    pub angles: Vec<f64>,
    pub spl: Vec<f64>,
    pub freq_count: usize,
    pub angle_count: usize,
}

/// Run requests on Tokio natively and on the browser executor on wasm.
pub struct Runtime {
    #[cfg(not(target_family = "wasm"))]
    inner: tokio::runtime::Runtime,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_family = "wasm"))]
            inner: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("HTTP runtime initialization failed"),
        }
    }

    #[cfg(not(target_family = "wasm"))]
    pub async fn spawn<F>(&self, future: F) -> Result<F::Output, String>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn(future).await.map_err(|e| e.to_string())
    }

    #[cfg(target_family = "wasm")]
    pub async fn spawn<F: std::future::Future>(&self, future: F) -> Result<F::Output, String> {
        Ok(future.await)
    }
}

async fn get(url: &str) -> Result<Value, String> {
    reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

// The wasm branch reads the browser origin and can fail, so the
// `Result` is only vacuous on native targets.
#[allow(clippy::unnecessary_wraps)]
fn api_url(path: &str) -> Result<String, String> {
    #[cfg(not(target_family = "wasm"))]
    let base = "https://api.spinorama.org/v1".to_string();
    #[cfg(target_family = "wasm")]
    let base = format!(
        "{}/api/spinorama/v1",
        web_sys::window()
            .ok_or("Browser window unavailable")?
            .location()
            .origin()
            .map_err(|_| "Browser origin unavailable")?
    );
    Ok(format!("{base}/{path}"))
}

pub async fn fetch_versions(speaker: &str) -> Result<Vec<String>, String> {
    let value = get(&api_url(&format!(
        "speaker/{}/versions",
        urlencoding::encode(speaker)
    ))?)
    .await?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

pub async fn fetch_available_speakers() -> Result<Vec<String>, String> {
    let value = get(&api_url("speakers")?).await?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

fn normalize(mut value: Value) -> Result<Value, String> {
    for _ in 0..4 {
        if value.get("data").and_then(Value::as_array).is_some() {
            return Ok(value);
        }
        value = if let Some(text) = value.as_str() {
            serde_json::from_str(text).map_err(|e| e.to_string())?
        } else if let Some(first) = value.as_array().and_then(|a| a.first()) {
            first.clone()
        } else {
            return Err(format!("No Plotly measurement data: {value}"));
        };
    }
    Err("Measurement JSON nesting is too deep".into())
}

pub async fn fetch_measurement_plot_data(
    speaker: &str,
    version: &str,
    measurement: &str,
) -> Result<Value, String> {
    let url = api_url(&format!(
        "speaker/{}/version/{}/measurements/{}?measurement_format=json",
        urlencoding::encode(speaker),
        urlencoding::encode(version),
        urlencoding::encode(measurement)
    ))?;
    normalize(get(&url).await?)
}

fn numbers(value: &Value) -> Result<Vec<f64>, String> {
    let values = if let Some(array) = value.as_array() {
        array
            .iter()
            .map(|v| {
                v.as_f64()
                    .ok_or_else(|| "Non-numeric measurement sample".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let dtype = value["dtype"].as_str().ok_or("Missing typed-array dtype")?;
        let encoded = value["bdata"].as_str().ok_or("Missing typed-array bytes")?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| e.to_string())?;
        let size = match dtype {
            "f8" => 8,
            "f4" | "i4" | "u4" => 4,
            "i2" | "u2" => 2,
            "i1" | "u1" => 1,
            _ => return Err(format!("Unsupported dtype: {dtype}")),
        };
        if bytes.len() % size != 0 {
            return Err("Truncated typed-array sample".into());
        }
        bytes
            .chunks_exact(size)
            .map(|chunk| match dtype {
                "f8" => f64::from_le_bytes(chunk.try_into().expect("eight-byte chunk")),
                "f4" => f64::from(f32::from_le_bytes(
                    chunk.try_into().expect("four-byte chunk"),
                )),
                "i4" => f64::from(i32::from_le_bytes(
                    chunk.try_into().expect("four-byte chunk"),
                )),
                "u4" => f64::from(u32::from_le_bytes(
                    chunk.try_into().expect("four-byte chunk"),
                )),
                "i2" => f64::from(i16::from_le_bytes(
                    chunk.try_into().expect("two-byte chunk"),
                )),
                "u2" => f64::from(u16::from_le_bytes(
                    chunk.try_into().expect("two-byte chunk"),
                )),
                "i1" => f64::from(chunk[0] as i8),
                "u1" => f64::from(chunk[0]),
                _ => unreachable!("validated dtype"),
            })
            .collect()
    };
    if values.iter().any(|v| !v.is_finite()) {
        return Err("Non-finite measurement sample".into());
    }
    Ok(values)
}

fn curve(trace: &Value) -> Result<Curve, String> {
    let freq = numbers(&trace["x"])?;
    let spl = numbers(&trace["y"])?;
    if freq.len() < 2
        || freq.len() != spl.len()
        || freq.iter().any(|f| *f <= 0.0)
        || freq.windows(2).any(|w| w[0] >= w[1])
    {
        return Err("Invalid frequency response grid".into());
    }
    Ok(Curve {
        freq: freq.into(),
        spl: spl.into(),
    })
}

pub fn extract_cea2034_curves_original(
    plot: &Value,
    _measurement: &str,
) -> Result<HashMap<String, Curve>, String> {
    let traces = plot["data"]
        .as_array()
        .ok_or("Missing measurement traces")?;
    let mut curves = HashMap::new();
    for name in [
        "On Axis",
        "Listening Window",
        "Early Reflections",
        "Sound Power",
        "Early Reflections DI",
        "Sound Power DI",
    ] {
        let trace = traces
            .iter()
            .find(|t| t["name"].as_str() == Some(name))
            .ok_or_else(|| format!("Missing {name} curve"))?;
        curves.insert(name.to_string(), curve(trace)?);
    }
    let lw = &curves["Listening Window"];
    let er = &curves["Early Reflections"];
    let sp = &curves["Sound Power"];
    if lw.freq != er.freq || lw.freq != sp.freq {
        return Err("CEA2034 frequency grids differ".into());
    }
    // Same energy-weighted PIR computation as the full native viewer.
    let pir = lw
        .spl
        .iter()
        .zip(&er.spl)
        .zip(&sp.spl)
        .map(|((&l, &e), &s)| {
            10.0 * (0.12 * 10f64.powf(l / 10.0)
                + 0.44 * 10f64.powf(e / 10.0)
                + 0.44 * 10f64.powf(s / 10.0))
            .log10()
        })
        .collect::<Vec<_>>();
    curves.insert(
        "Estimated In-Room Response".into(),
        Curve {
            freq: lw.freq.clone(),
            spl: pir.into(),
        },
    );
    Ok(curves)
}

fn directivity(plot: &Value) -> Result<Vec<DirectivityCurve>, String> {
    let traces = plot["data"]
        .as_array()
        .ok_or("Missing directivity traces")?;
    let mut curves = Vec::new();
    for trace in traces {
        let name = trace["name"]
            .as_str()
            .unwrap_or("")
            .replace('\u{2212}', "-")
            .replace('°', " ");
        let angle = if name.trim().eq_ignore_ascii_case("ON") {
            Some(0.0)
        } else {
            name.split(|c: char| c.is_whitespace() || "(),:".contains(c))
                .find_map(|s| s.parse::<f64>().ok())
        };
        if let Some(angle) = angle {
            let response = curve(trace)?;
            curves.push(DirectivityCurve {
                angle,
                freq: response.freq,
                spl: response.spl,
            });
        }
    }
    curves.sort_by(|a, b| a.angle.total_cmp(&b.angle));
    if curves.is_empty() {
        return Err("No angular measurements available".into());
    }
    Ok(curves)
}

pub async fn fetch_directivity_data(
    speaker: &str,
    version: &str,
) -> Result<DirectivityData, String> {
    let horizontal =
        directivity(&fetch_measurement_plot_data(speaker, version, "SPL Horizontal").await?)?;
    let vertical =
        directivity(&fetch_measurement_plot_data(speaker, version, "SPL Vertical").await?)?;
    Ok(DirectivityData {
        horizontal,
        vertical,
    })
}

fn contour(plot: &Value) -> Result<ContourPlotData, String> {
    let trace = plot["data"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t.get("z").is_some()))
        .ok_or("No contour measurement")?;
    let freq = numbers(&trace["x"])?;
    let angles = numbers(&trace["y"])?;
    let spl = if let Some(rows) = trace["z"].as_array() {
        rows.iter()
            .map(numbers)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect()
    } else {
        numbers(&trace["z"])?
    };
    if freq.len() < 2 || angles.len() < 2 || spl.len() != freq.len() * angles.len() {
        return Err("Contour grid dimensions do not match".into());
    }
    Ok(ContourPlotData {
        freq_count: freq.len(),
        angle_count: angles.len(),
        freq,
        angles,
        spl,
    })
}

/// Parse contour data from an already-fetched Plotly measurement payload.
pub fn contour_plot_data(plot: &Value) -> Result<ContourPlotData, String> {
    contour(plot)
}

pub async fn fetch_contour_data(
    speaker: &str,
    version: &str,
    plane: &str,
) -> Result<ContourPlotData, String> {
    let measurement = if plane == "horizontal" {
        "SPL Horizontal Contour"
    } else {
        "SPL Vertical Contour"
    };
    contour(&fetch_measurement_plot_data(speaker, version, measurement).await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn real_api_fixture_preserves_samples_and_energy_weighted_pir() {
        let plot: Value =
            serde_json::from_str(include_str!("../tests/fixtures/kef-r3-asr-cea2034.json"))
                .unwrap();
        let curves = extract_cea2034_curves_original(&plot, "CEA2034").unwrap();
        assert_eq!(curves.len(), 7);
        let lw = &curves["Listening Window"];
        let er = &curves["Early Reflections"];
        let sp = &curves["Sound Power"];
        assert!(lw.freq.len() > 100);
        let expected = 10.0
            * (0.12 * 10f64.powf(lw.spl[20] / 10.0)
                + 0.44 * 10f64.powf(er.spl[20] / 10.0)
                + 0.44 * 10f64.powf(sp.spl[20] / 10.0))
            .log10();
        assert!((curves["Estimated In-Room Response"].spl[20] - expected).abs() < 1e-10);
        assert_eq!(curves["On Axis"].freq, lw.freq);
    }

    #[test]
    fn api_wrappers_and_plain_angular_arrays_are_supported() {
        let plot = json!({"data": [
            {"name": "10°", "x": [100, 1000], "y": [1, 2]},
            {"name": "−10°", "x": [100, 1000], "y": [3, 4]},
            {"name": "ON", "x": [100, 1000], "y": [5, 6]}
        ]});
        for wrapped in [
            plot.clone(),
            json!(plot.to_string()),
            json!([plot.to_string()]),
        ] {
            let curves = directivity(&normalize(wrapped).unwrap()).unwrap();
            assert_eq!(
                curves.iter().map(|c| c.angle).collect::<Vec<_>>(),
                [-10.0, 0.0, 10.0]
            );
            assert_eq!(curves[1].spl[1], 6.0);
        }
    }

    #[test]
    fn malformed_grids_and_truncated_typed_arrays_are_rejected() {
        numbers(&json!({"dtype": "f8", "bdata": "AA=="})).unwrap_err();
        curve(&json!({"x": [100, 10], "y": [1, 2]})).unwrap_err();
        curve(&json!({"x": [100, 1000], "y": [1]})).unwrap_err();
        contour(&json!({"data": [{"x": [100, 1000], "y": [-10, 10], "z": [[1, 2]]}]})).unwrap_err();
    }

    #[test]
    fn contour_grid_keeps_angle_major_order() {
        let data =
            contour(&json!({"data": [{"x": [100, 1000], "y": [-10, 10], "z": [[1, 2], [3, 4]]}]}))
                .unwrap();
        assert_eq!(data.spl, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!((data.angle_count, data.freq_count), (2, 2));
    }

    #[test]
    fn real_contour_fixture_decodes_signed_integer_angles() {
        let plot: Value =
            serde_json::from_str(include_str!("../tests/fixtures/kef-r3-asr-contour.json"))
                .unwrap();
        let data = contour(&plot).unwrap();
        assert_eq!(data.angles.first(), Some(&-180.0));
        assert_eq!(data.angles.last(), Some(&180.0));
        assert_eq!(data.spl.len(), data.angle_count * data.freq_count);
        assert!(data.freq_count > 100);
    }
}
