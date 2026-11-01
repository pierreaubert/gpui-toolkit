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

/// True when an API error string is an HTTP 401/403 (missing/invalid key).
pub fn is_auth_error(error: &str) -> bool {
    error.contains("401") || error.contains("403")
}

async fn get(url: &str, api_key: Option<&str>) -> Result<Value, String> {
    let client = reqwest::Client::new();
    let mut request = client.get(url);
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        request = request.bearer_auth(key.trim());
    }
    request
        .send()
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

pub async fn fetch_versions(speaker: &str, api_key: Option<&str>) -> Result<Vec<String>, String> {
    let value = get(
        &api_url(&format!(
            "speaker/{}/versions",
            urlencoding::encode(speaker)
        ))?,
        api_key,
    )
    .await?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

#[cfg_attr(test, allow(dead_code, reason = "live caller is gated out in unit tests"))]
pub async fn fetch_available_speakers(api_key: Option<&str>) -> Result<Vec<String>, String> {
    let value = get(&api_url("speakers")?, api_key).await?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

/// Public (unauthenticated) PNG URL for a measurement.
///
/// Used in picture mode: the user sees the rendered result without any
/// access to the underlying measurement data.
pub fn measurement_image_url(
    speaker: &str,
    version: &str,
    measurement: &str,
) -> Result<String, String> {
    api_url(&format!(
        "speaker/{}/version/{}/measurements/{}?measurement_format=png",
        urlencoding::encode(speaker),
        urlencoding::encode(version),
        urlencoding::encode(measurement)
    ))
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
    api_key: Option<&str>,
) -> Result<Value, String> {
    let url = api_url(&format!(
        "speaker/{}/version/{}/measurements/{}?measurement_format=json",
        urlencoding::encode(speaker),
        urlencoding::encode(version),
        urlencoding::encode(measurement)
    ))?;
    normalize(get(&url, api_key).await?)
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
    api_key: Option<&str>,
) -> Result<DirectivityData, String> {
    let horizontal = directivity(
        &fetch_measurement_plot_data(speaker, version, "SPL Horizontal", api_key).await?,
    )?;
    let vertical = directivity(
        &fetch_measurement_plot_data(speaker, version, "SPL Vertical", api_key).await?,
    )?;
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
#[cfg(all(target_os = "macos", feature = "capture-bench"))]
pub fn contour_plot_data(plot: &Value) -> Result<ContourPlotData, String> {
    contour(plot)
}

pub async fn fetch_contour_data(
    speaker: &str,
    version: &str,
    plane: &str,
    api_key: Option<&str>,
) -> Result<ContourPlotData, String> {
    let measurement = if plane == "horizontal" {
        "SPL Horizontal Contour"
    } else {
        "SPL Vertical Contour"
    };
    fetch_contour_measurement(speaker, version, measurement, api_key).await
}

/// Fetches a named contour measurement (incl. Normalized variants).
pub async fn fetch_contour_measurement(
    speaker: &str,
    version: &str,
    measurement: &str,
    api_key: Option<&str>,
) -> Result<ContourPlotData, String> {
    contour(&fetch_measurement_plot_data(speaker, version, measurement, api_key).await?)
}

fn string_values(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|v| {
                    v.as_str().map_or_else(|| {
                        v.as_f64()
                            .map(|n| n.to_string())
                            .unwrap_or_default()
                    }, str::to_string)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn axis_layout(value: &Value) -> crate::types::AxisLayout {
    let title = value
        .pointer("/title/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let range = value.get("range").and_then(Value::as_array).and_then(|r| {
        Some((r.first()?.as_f64()?, r.get(1)?.as_f64()?))
    });
    let log = value.get("type").and_then(Value::as_str) == Some("log");
    let tick_values = value
        .get("tickvals")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    let tick_text = value
        .get("ticktext")
        .map(string_values)
        .unwrap_or_default();
    crate::types::AxisLayout {
        title,
        range,
        log,
        tick_values,
        tick_text,
    }
}

fn trace_color(trace: &Value) -> (crate::types::GenericTrace, f32) {
    let mut out = crate::types::GenericTrace::new(crate::types::TraceKind::Scatter, "");
    let mut opacity = trace
        .get("opacity")
        .and_then(Value::as_f64)
        .unwrap_or(1.0) as f32;
    for pointer in ["/line/color", "/marker/color", "/fillcolor"] {
        if let Some(text) = trace.pointer(pointer).and_then(Value::as_str)
            && let Some((color, alpha)) = crate::types::parse_plotly_color(text) {
                if pointer == "/fillcolor" {
                    out.fill_color = Some(color);
                    if alpha < 1.0 {
                        opacity = alpha;
                    }
                } else {
                    out.color = color;
                    if alpha < 1.0 {
                        opacity = alpha;
                    }
                }
                if pointer != "/fillcolor" {
                    break;
                }
            }
    }
    (out, opacity)
}

/// Parses any Plotly payload into the generic trace model.
///
/// Handles `scatter`, `scatterpolar` (numeric and categorical), `bar`,
/// `contour`/`heatmap`, and `surface` traces, plus layout axes, a
/// secondary y axis, polar radial range, and annotations. All traces are
/// kept in payload order, including fills and slope/zone traces that the
/// specialised CEA2034 extractor drops.
///
/// # Errors
///
/// Returns an error when the payload has no trace array or a trace
/// contains malformed numeric data.
pub fn parse_plot(plot: &Value) -> Result<crate::types::PlotData, String> {
    let plot = normalize(plot.clone())?;
    let traces = plot["data"]
        .as_array()
        .ok_or("Missing measurement traces")?;
    let mut out = crate::types::PlotData::default();
    let layout = plot.get("layout").cloned().unwrap_or(Value::Null);
    out.layout.title = layout
        .pointer("/title/text")
        .and_then(Value::as_str)
        .map(str::to_string);
    if let Some(x) = layout.get("xaxis") {
        out.layout.x_axis = axis_layout(x);
    }
    if let Some(y) = layout.get("yaxis") {
        out.layout.y_axis = axis_layout(y);
    }
    if let Some(y2) = layout.get("yaxis2") {
        out.layout.y_axis2 = Some(axis_layout(y2));
    }
    out.layout.polar_radial_range = layout
        .pointer("/polar/radialaxis/range")
        .and_then(Value::as_array)
        .and_then(|r| Some((r.first()?.as_f64()?, r.get(1)?.as_f64()?)));
    if let Some(items) = layout.get("annotations").and_then(Value::as_array) {
        for item in items {
            let text = item
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .replace("<br>", " ")
                .replace("<br/>", " ");
            out.layout.annotations.push(crate::types::Annotation {
                text,
                x: item.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                y: item.get("y").and_then(Value::as_f64).unwrap_or(0.0),
            });
        }
    }
    const PALETTE: [u32; 10] = [
        0x1f77b4, 0xff7f0e, 0x2ca02c, 0xd62728, 0x9467bd, 0x8c564b, 0xe377c2, 0x7f7f7f,
        0xbcbd22, 0x17becf,
    ];
    for (index, trace) in traces.iter().enumerate() {
        let type_name = trace.get("type").and_then(Value::as_str).unwrap_or("scatter");
        let (mut generic, opacity) = trace_color(trace);
        generic.opacity = opacity;
        generic.name = trace
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        generic.visible = trace.get("visible").is_none_or(|v| {
            v.as_bool().unwrap_or(v.as_str() != Some("legendonly"))
        });
        generic.secondary_axis = trace
            .get("yaxis")
            .and_then(Value::as_str)
            .is_some_and(|a| a == "y2");
        generic.stroke_width = trace
            .pointer("/line/width")
            .and_then(Value::as_f64)
            .unwrap_or(2.0) as f32;
        generic.dash = trace
            .pointer("/line/dash")
            .and_then(Value::as_str)
            .map(str::to_string);
        generic.marker_size = trace
            .pointer("/marker/size")
            .and_then(Value::as_f64)
            .unwrap_or(5.0) as f32;
        generic.mode = match trace.get("mode").and_then(Value::as_str) {
            Some("markers") => crate::types::TraceMode::Markers,
            Some("lines+markers") => crate::types::TraceMode::LinesMarkers,
            Some("none") => crate::types::TraceMode::None,
            _ => crate::types::TraceMode::Lines,
        };
        generic.fill = match trace.get("fill").and_then(Value::as_str) {
            Some("tozeroy") => Some(crate::types::TraceFill::ToZeroY),
            Some("tonexty") => Some(crate::types::TraceFill::ToNextY),
            _ => None,
        };
        if trace.pointer("/line/color").is_none() && trace.pointer("/marker/color").is_none() {
            generic.color = d3rs_color_from_hex(PALETTE[index % PALETTE.len()]);
        }
        match type_name {
            "bar" => {
                generic.kind = crate::types::TraceKind::Bar;
                generic.categories = string_values(&trace["x"]);
                generic.y = numbers(&trace["y"]).unwrap_or_default();
                if generic.categories.is_empty() {
                    // Horizontal bars: categories on y, values on x.
                    generic.categories = string_values(&trace["y"]);
                    generic.y = numbers(&trace["x"]).unwrap_or_default();
                }
            }
            "scatterpolar" => {
                let theta_strings = string_values(&trace["theta"]);
                let numeric_theta = numbers(&trace["theta"]).unwrap_or_default();
                generic.r = numbers(&trace["r"]).unwrap_or_default();
                if numeric_theta.len() == generic.r.len() && !generic.r.is_empty() {
                    generic.kind = crate::types::TraceKind::ScatterPolar;
                    generic.theta = numeric_theta;
                } else {
                    generic.kind = crate::types::TraceKind::ScatterPolarCategorical;
                    generic.categories = theta_strings;
                }
            }
            "contour" | "heatmap" => {
                generic.kind = crate::types::TraceKind::Contour;
                generic.x = numbers(&trace["x"]).unwrap_or_default();
                generic.y = numbers(&trace["y"]).unwrap_or_default();
                if let Ok(parsed) = contour(&plot) {
                    generic.z = parsed.spl;
                }
            }
            "surface" => {
                generic.kind = crate::types::TraceKind::Surface;
                generic.x = numbers(&trace["x"]).unwrap_or_default();
                generic.y = numbers(&trace["y"]).unwrap_or_default();
            }
            _ => {
                generic.kind = crate::types::TraceKind::Scatter;
                generic.x = numbers(&trace["x"]).unwrap_or_default();
                generic.y = numbers(&trace["y"]).unwrap_or_default();
            }
        }
        out.traces.push(generic);
    }
    Ok(out)
}

fn d3rs_color_from_hex(hex: u32) -> d3rs::color::D3Color {
    d3rs::color::D3Color::from_hex(hex)
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

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn bearer_header_is_sent_only_with_a_key() {
        use std::io::{BufRead, BufReader, Write};
        use std::sync::mpsc;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let (tx, rx) = mpsc::channel::<Vec<String>>();
        std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("accept");
                let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
                let mut headers = Vec::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).expect("read line");
                    if line == "\r\n" || line.is_empty() {
                        break;
                    }
                    headers.push(line.trim().to_string());
                }
                tx.send(headers).expect("send headers");
                let body = "{}";
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write response");
            }
        });
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        let url = format!("http://{addr}/v1/speakers");
        runtime.block_on(async {
            get(&url, Some("secret-key-1")).await.expect("authed get");
            get(&url, None).await.expect("anon get");
        });
        let authed = rx.recv().expect("authed headers");
        assert!(
            authed.iter().any(|h| h.eq_ignore_ascii_case("authorization: Bearer secret-key-1")),
            "missing Bearer header in {authed:?}"
        );
        let anon = rx.recv().expect("anon headers");
        assert!(
            !anon.iter().any(|h| h.to_ascii_lowercase().starts_with("authorization:")),
            "unexpected Authorization header in {anon:?}"
        );
    }

    #[test]
    fn parse_plot_keeps_fills_secondary_annotations_and_order() {
        let plot = json!({
            "data": [
                {"type":"scatter","name":"zone","x":[100,1000],"y":[5,5],"fill":"tozeroy","fillcolor":"rgba(255,0,0,0.2)","line":{"color":"#ff0000"}},
                {"type":"scatter","name":"main","x":[100,1000],"y":[1,2],"yaxis":"y2","line":{"color":"rgb(0,128,0)","dash":"dash","width":3}},
                {"type":"bar","name":"bars","x":["a","b"],"y":[3,4]},
                {"type":"scatterpolar","name":"radar","theta":[0,90],"r":[-5,-10]},
                {"type":"scatterpolar","name":"spider","theta":["bass","treble"],"r":[1,2]}
            ],
            "layout": {
                "title":{"text":"T"},
                "xaxis":{"title":{"text":"Frequency (Hz)"},"type":"log"},
                "yaxis":{"title":{"text":"SPL (dB)"},"range":[-40,10]},
                "yaxis2":{"title":{"text":"DI (dB)"}},
                "annotations":[{"text":"note","x":1000,"y":2}]
            }
        });
        let data = parse_plot(&plot).unwrap();
        assert_eq!(data.traces.len(), 5);
        assert_eq!(data.traces[0].fill, Some(crate::types::TraceFill::ToZeroY));
        assert!(data.traces[1].secondary_axis);
        assert_eq!(data.traces[2].kind, crate::types::TraceKind::Bar);
        assert_eq!(data.traces[2].categories, ["a", "b"]);
        assert_eq!(data.traces[3].kind, crate::types::TraceKind::ScatterPolar);
        assert_eq!(
            data.traces[4].kind,
            crate::types::TraceKind::ScatterPolarCategorical
        );
        assert_eq!(data.layout.annotations[0].text, "note");
        assert!(data.layout.x_axis.log);
        assert!(is_auth_error("HTTP status client error (401 Unauthorized)"));
        assert!(!is_auth_error("HTTP status client error (404 Not Found)"));
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
