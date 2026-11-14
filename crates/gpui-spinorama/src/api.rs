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

/// Full-access API base (requires a token).
pub const FULL_API_BASE_URL: &str = "https://api.spinorama.org/v1";
/// Demo API base (no token, 10 speakers).
pub const DEMO_API_BASE_URL: &str = "https://api.spinorama.org/demo/v1";
/// Explicit base-URL override (CI / headless / tests).
pub const ENV_API_BASE_URL: &str = "SPINORAMA_API_BASE";
/// Set to `1`/`true`/`demo` to default to the demo API.
pub const ENV_USE_DEMO_API: &str = "SPINORAMA_USE_DEMO";

/// True when the process (or wasm query string) requests the demo API.
///
/// Native: `SPINORAMA_USE_DEMO=1` (or `true`/`demo`), or a
/// `SPINORAMA_API_BASE` containing `/demo/`.
/// Wasm: `?demo=1` (or `?api=demo`) on the hosting page URL.
pub fn demo_api_requested() -> bool {
    if std::env::var(ENV_USE_DEMO_API)
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("demo"))
    {
        return true;
    }
    if let Ok(base) = std::env::var(ENV_API_BASE_URL)
        && base.contains("/demo/")
    {
        return true;
    }
    #[cfg(target_family = "wasm")]
    {
        if gpui_miniapp::web_query_param("demo")
            .as_deref()
            .is_some_and(|v| {
                v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("demo")
            })
        {
            return true;
        }
        if gpui_miniapp::web_query_param("api")
            .as_deref()
            .is_some_and(|v| v.eq_ignore_ascii_case("demo"))
        {
            return true;
        }
    }
    false
}

/// Base URL for the selected API mode.
///
/// An explicit `SPINORAMA_API_BASE` always wins (native and tests);
/// otherwise the demo flag selects between the demo (10 speakers, no
/// token) and the full token-protected API. On wasm the base is the
/// same-origin proxy, matching the production nginx locations.
// The wasm branch reads the browser origin and can fail, so the
// `Result` is only vacuous on native targets.
#[allow(clippy::unnecessary_wraps)]
pub fn api_base_url(use_demo: bool) -> Result<String, String> {
    if let Ok(custom) = std::env::var(ENV_API_BASE_URL) {
        let trimmed = custom.trim().trim_end_matches('/').to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }
    if use_demo {
        #[cfg(not(target_family = "wasm"))]
        {
            return Ok(DEMO_API_BASE_URL.to_string());
        }
        #[cfg(target_family = "wasm")]
        {
            // Same-origin proxy mirrors the upstream path; derive the
            // suffix from the public constant so both stay in sync.
            let suffix = DEMO_API_BASE_URL
                .strip_prefix("https://api.spinorama.org/")
                .unwrap_or("demo/v1");
            return Ok(format!(
                "{}/api/spinorama/{suffix}",
                web_sys::window()
                    .ok_or("Browser window unavailable")?
                    .location()
                    .origin()
                    .map_err(|_| "Browser origin unavailable")?
            ));
        }
    }
    // The wasm branch reads the browser origin and can fail, so the
    // `Result` is only vacuous on native targets.
    #[cfg(not(target_family = "wasm"))]
    let base = FULL_API_BASE_URL.to_string();
    #[cfg(target_family = "wasm")]
    let base = {
        let suffix = FULL_API_BASE_URL
            .strip_prefix("https://api.spinorama.org/")
            .unwrap_or("v1");
        format!(
            "{}/api/spinorama/{suffix}",
            web_sys::window()
                .ok_or("Browser window unavailable")?
                .location()
                .origin()
                .map_err(|_| "Browser origin unavailable")?
        )
    };
    Ok(base)
}

// The wasm branch reads the browser origin and can fail, so the
// `Result` is only vacuous on native targets.
#[allow(clippy::unnecessary_wraps)]
fn api_url(path: &str) -> Result<String, String> {
    Ok(format!("{}/{path}", api_base_url(false)?))
}

fn api_url_for(path: &str, use_demo: bool) -> Result<String, String> {
    Ok(format!("{}/{path}", api_base_url(use_demo)?))
}

pub async fn fetch_versions(
    speaker: &str,
    api_key: Option<&str>,
    use_demo: bool,
) -> Result<Vec<String>, String> {
    let value = get(
        &api_url_for(
            &format!("speaker/{}/versions", urlencoding::encode(speaker)),
            use_demo,
        )?,
        api_key,
    )
    .await?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

#[cfg_attr(
    test,
    allow(dead_code, reason = "live caller is gated out in unit tests")
)]
pub async fn fetch_available_speakers(
    api_key: Option<&str>,
    use_demo: bool,
) -> Result<Vec<String>, String> {
    let value = get(&api_url_for("speakers", use_demo)?, api_key).await?;
    serde_json::from_value(value).map_err(|e| e.to_string())
}

/// Public (unauthenticated) PNG URL for a measurement.
///
/// Used in picture mode: the user sees the rendered result without any
/// access to the underlying measurement data. The demo API serves JSON
/// only, so requesting a demo PNG is an error — demo mode stays
/// interactive instead of entering picture mode.
pub fn measurement_image_url(
    speaker: &str,
    version: &str,
    measurement: &str,
    use_demo: bool,
) -> Result<String, String> {
    if use_demo {
        return Err("The demo API serves JSON measurements only (no PNG pictures)".into());
    }
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
    use_demo: bool,
) -> Result<Value, String> {
    let url = api_url_for(
        &format!(
            "speaker/{}/version/{}/measurements/{}?measurement_format=json",
            urlencoding::encode(speaker),
            urlencoding::encode(version),
            urlencoding::encode(measurement)
        ),
        use_demo,
    )?;
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

// Plotly scatter nulls are line breaks, not malformed measurement data.
fn line_numbers(value: &Value) -> Result<Vec<f64>, String> {
    if let Some(array) = value.as_array() {
        array
            .iter()
            .map(|sample| {
                if sample.is_null() {
                    Ok(f64::NAN)
                } else {
                    sample
                        .as_f64()
                        .ok_or_else(|| "Non-numeric line sample".to_owned())
                }
            })
            .collect()
    } else {
        numbers(value)
    }
}

/// Maps a Plotly `marker.symbol` name to a d3rs symbol type.
pub fn plotly_symbol(name: &str) -> Option<d3rs::shape::SymbolType> {
    match name {
        "square" => Some(d3rs::shape::SymbolType::Square),
        "diamond" => Some(d3rs::shape::SymbolType::Diamond),
        "cross" => Some(d3rs::shape::SymbolType::Cross),
        "x" => Some(d3rs::shape::SymbolType::Wye),
        "triangle-up" => Some(d3rs::shape::SymbolType::Triangle),
        "triangle-down" => Some(d3rs::shape::SymbolType::TriangleDown),
        "triangle-left" => Some(d3rs::shape::SymbolType::TriangleLeft),
        "triangle-right" => Some(d3rs::shape::SymbolType::TriangleRight),
        "star" => Some(d3rs::shape::SymbolType::Star),
        "circle" => Some(d3rs::shape::SymbolType::Circle),
        _ => None,
    }
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

/// Parses the angle curves of an SPL Horizontal/Vertical payload.
///
/// Public wrapper for the headless image renderer, which builds
/// sphere (globe) data from the two plane payloads on disk.
#[cfg(all(target_os = "macos", feature = "capture-bench"))]
pub fn directivity_curves(plot: &Value) -> Result<Vec<DirectivityCurve>, String> {
    directivity(plot)
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
    use_demo: bool,
) -> Result<DirectivityData, String> {
    let horizontal = directivity(
        &fetch_measurement_plot_data(speaker, version, "SPL Horizontal", api_key, use_demo).await?,
    )?;
    let vertical = directivity(
        &fetch_measurement_plot_data(speaker, version, "SPL Vertical", api_key, use_demo).await?,
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
    use_demo: bool,
) -> Result<ContourPlotData, String> {
    let measurement = if plane == "horizontal" {
        "SPL Horizontal Contour"
    } else {
        "SPL Vertical Contour"
    };
    fetch_contour_measurement(speaker, version, measurement, api_key, use_demo).await
}

/// Fetches a named contour measurement (incl. Normalized variants).
pub async fn fetch_contour_measurement(
    speaker: &str,
    version: &str,
    measurement: &str,
    api_key: Option<&str>,
    use_demo: bool,
) -> Result<ContourPlotData, String> {
    contour(&fetch_measurement_plot_data(speaker, version, measurement, api_key, use_demo).await?)
}

fn string_values(value: &Value) -> Vec<String> {
    value.as_array().map_or_default(|items| {
        items
            .iter()
            .map(|v| {
                v.as_str().map_or_else(
                    || v.as_f64().map_or_default(|n| n.to_string()),
                    str::to_string,
                )
            })
            .collect()
    })
}

fn axis_layout(value: &Value) -> crate::types::AxisLayout {
    let title = value
        .pointer("/title/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let range = value
        .get("range")
        .and_then(Value::as_array)
        .and_then(|r| Some((r.first()?.as_f64()?, r.get(1)?.as_f64()?)));
    let log = value.get("type").and_then(Value::as_str) == Some("log");
    let tick_values = value
        .get("tickvals")
        .and_then(Value::as_array)
        .map_or_default(|items| items.iter().filter_map(Value::as_f64).collect());
    let tick_text = value.get("ticktext").map_or_default(string_values);
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
    let mut opacity = trace.get("opacity").and_then(Value::as_f64).unwrap_or(1.0) as f32;
    // Line/marker colour and fill colour are independent in Plotly: a
    // band trace carries both (`line.color` for its outline,
    // `fillcolor` for the zone), so parse them separately instead of
    // stopping at the first colour found.
    for pointer in ["/line/color", "/marker/color"] {
        if let Some(text) = trace.pointer(pointer).and_then(Value::as_str)
            && let Some((color, alpha)) = crate::types::parse_plotly_color(text)
        {
            out.color = color;
            if alpha < 1.0 {
                opacity = alpha;
            }
            break;
        }
    }
    if let Some(text) = trace.pointer("/fillcolor").and_then(Value::as_str)
        && let Some((color, alpha)) = crate::types::parse_plotly_color(text)
    {
        out.fill_color = Some(color);
        if alpha < 1.0 {
            opacity = alpha;
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
    out.layout.hide_zero_line = layout
        .get("hide_zero_line")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    out.layout.polar_radial_range = layout
        .pointer("/polar/radialaxis/range")
        .and_then(Value::as_array)
        .and_then(|r| Some((r.first()?.as_f64()?, r.get(1)?.as_f64()?)));
    // Subplot cells: Plotly writes one xaxis/yaxis pair per cell
    // (xaxis, xaxis2, ...) with paper domains. Cells only exist when
    // more than one x axis is declared.
    let axis_domain = |axis: &Value| -> (f64, f64) {
        axis.get("domain")
            .and_then(Value::as_array)
            .and_then(|d| Some((d.first()?.as_f64()?, d.get(1)?.as_f64()?)))
            .unwrap_or((0.0, 1.0))
    };
    let mut axis_index = 1usize;
    loop {
        let suffix = if axis_index == 1 {
            String::new()
        } else {
            axis_index.to_string()
        };
        let Some(x_axis) = layout.get(format!("xaxis{suffix}")) else {
            break;
        };
        let y_axis = layout
            .get(format!("yaxis{suffix}"))
            .cloned()
            .unwrap_or(Value::Null);
        out.layout.subplots.push(crate::types::SubplotCell {
            x_domain: axis_domain(x_axis),
            y_domain: axis_domain(&y_axis),
            title: None,
            x_axis: axis_layout(x_axis),
            y_axis: axis_layout(&y_axis),
        });
        axis_index += 1;
    }
    if out.layout.subplots.len() <= 1 {
        out.layout.subplots.clear();
    }

    let mut paper_annotations: Vec<(String, f64, f64)> = Vec::new();
    if let Some(items) = layout.get("annotations").and_then(Value::as_array) {
        for item in items {
            if item.get("visible").and_then(Value::as_bool) == Some(false) { continue; }
            let text = item
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .replace("<br>", " ")
                .replace("<br/>", " ");
            let xref = item.get("xref").and_then(Value::as_str).unwrap_or("");
            let (x, y) = (
                item.get("x").and_then(Value::as_f64).unwrap_or(0.0),
                item.get("y").and_then(Value::as_f64).unwrap_or(0.0),
            );
            if xref == "paper" && !out.layout.subplots.is_empty() {
                paper_annotations.push((text, x, y));
            } else {
                let axis_x = |v: f64| if out.layout.x_axis.log && xref == "x" { 10f64.powf(v) } else { v };
                let label = item.get("ax").and_then(Value::as_f64).zip(item.get("ay").and_then(Value::as_f64))
                    .filter(|_| item.get("axref").and_then(Value::as_str) == Some("x"))
                    .map(|(x, y)| (axis_x(x), y));
                let color = item.pointer("/font/color").and_then(Value::as_str)
                    .and_then(crate::types::parse_plotly_color).map(|c| c.0);
                out.layout.annotations.push(crate::types::Annotation { text, x: axis_x(x), y,
                    label, color, secondary_axis: item.get("yref").and_then(Value::as_str) == Some("y2") });
            }
        }
    }
    // Subplot titles are paper annotations sitting just above a cell.
    for (text, x, y) in paper_annotations {
        let mut best: Option<(usize, f64)> = None;
        for (index, cell) in out.layout.subplots.iter().enumerate() {
            let center_x = cell.x_domain.0.midpoint(cell.x_domain.1);
            if y < cell.y_domain.1 - 0.02 {
                continue;
            }
            let distance = (center_x - x).abs() + (y - cell.y_domain.1).abs();
            if best.is_none_or(|(_, d)| distance < d) {
                best = Some((index, distance));
            }
        }
        match best {
            Some((index, _)) => out.layout.subplots[index].title = Some(text),
            None => out.layout.annotations.push(crate::types::Annotation { text, x, y, ..Default::default() }),
        }
    }
    const PALETTE: [u32; 10] = [
        0x1f77b4, 0xff7f0e, 0x2ca02c, 0xd62728, 0x9467bd, 0x8c564b, 0xe377c2, 0x7f7f7f, 0xbcbd22,
        0x17becf,
    ];
    for (index, trace) in traces.iter().enumerate() {
        let type_name = trace
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("scatter");
        let (mut generic, opacity) = trace_color(trace);
        generic.opacity = opacity;
        generic.name = trace
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        generic.visible = trace
            .get("visible")
            .is_none_or(|v| v.as_bool().unwrap_or(v.as_str() != Some("legendonly")));
        generic.secondary_axis = trace
            .get("yaxis")
            .and_then(Value::as_str)
            .is_some_and(|a| a == "y2");
        if !out.layout.subplots.is_empty() {
            generic.subplot = trace
                .get("xaxis")
                .and_then(Value::as_str)
                .and_then(|axis| axis.strip_prefix('x'))
                .and_then(|digits| {
                    if digits.is_empty() {
                        Some(1usize)
                    } else {
                        digits.parse().ok()
                    }
                })
                .unwrap_or(1)
                .saturating_sub(1)
                .min(out.layout.subplots.len() - 1);
        }
        generic.show_legend = trace.get("showlegend").and_then(Value::as_bool).unwrap_or(true);
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
            Some("toself") => Some(crate::types::TraceFill::ToSelf),
            _ => None,
        };
        generic.marker_symbol = trace
            .pointer("/marker/symbol")
            .and_then(Value::as_str)
            .and_then(plotly_symbol);
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
                generic.x = line_numbers(&trace["x"]).unwrap_or_default();
                generic.y = line_numbers(&trace["y"]).unwrap_or_default();
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
    fn phase_null_samples_preserve_plot_gaps() {
        let plot = serde_json::json!({"data": [{"name": "Phase (deg)", "type": "scatter",
            "x": [100.0, 200.0, 200.0], "y": [-170.0, null, 170.0], "yaxis": "y2"}],
            "layout": {"yaxis2": {"range": [-180.0, 360.0], "title": {"text": "Phase (deg)"}}}});
        let parsed = parse_plot(&plot).unwrap();
        assert_eq!(parsed.traces[0].y.len(), 3);
        assert!(parsed.traces[0].y[1].is_nan());
        assert!(parsed.traces[0].secondary_axis);
    }

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
        let listener = match std::net::TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("skipping bearer-header socket test: local bind is not permitted");
                return;
            }
            Err(error) => panic!("bind: {error}"),
        };
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
            authed
                .iter()
                .any(|h| h.eq_ignore_ascii_case("authorization: Bearer secret-key-1")),
            "missing Bearer header in {authed:?}"
        );
        let anon = rx.recv().expect("anon headers");
        assert!(
            !anon
                .iter()
                .any(|h| h.to_ascii_lowercase().starts_with("authorization:")),
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
    fn parse_plot_reads_hide_zero_line() {
        let with_flag = json!({
            "data": [{"type":"scatter","name":"main","x":[100,1000],"y":[1,2]}],
            "layout": {"hide_zero_line": true}
        });
        assert!(parse_plot(&with_flag).unwrap().layout.hide_zero_line);
        let without_flag = json!({
            "data": [{"type":"scatter","name":"main","x":[100,1000],"y":[1,2]}],
            "layout": {}
        });
        assert!(!parse_plot(&without_flag).unwrap().layout.hide_zero_line);
    }

    #[test]
    fn parse_plot_preserves_axis_bound_callouts_and_helper_legend_visibility() {
        let value = serde_json::json!({
            "data": [{"type":"scatter", "name":"Sound Power DI slope", "x":[100,12000],
                "y":[0,4], "yaxis":"y2", "showlegend":false}],
            "layout": {"xaxis":{"type":"log"}, "annotations":[
                {"text":"0.94 db/oct sm 0.74", "x":4.0, "y":8.0, "xref":"x", "yref":"y2",
                    "ax":3.5, "ay":12.0, "axref":"x", "font":{"color":"#000000"}},
                {"text":"hidden", "x":3.0, "y":1.0, "visible":false}
            ]}
        });
        let plot = parse_plot(&value).unwrap();
        assert!(!plot.traces[0].show_legend);
        assert_eq!(plot.layout.annotations.len(), 1);
        let annotation = &plot.layout.annotations[0];
        assert_eq!(annotation.x, 10000.0);
        assert!(annotation.secondary_axis);
        let (x, y) = annotation.label.unwrap();
        assert!((x - 10f64.powf(3.5)).abs() < 1e-9);
        assert_eq!(y, 12.0);
        assert_eq!(annotation.color, Some(d3rs::color::D3Color::from_hex(0)));
    }

    #[test]
    fn spinorama_surface_colors_match_contour_colors() {
        let colormap = crate::types::Colormap::Spinorama;
        let contour = colormap.color_scale();
        let surface = colormap.surface_colormap();
        let stops = [
            0x0000a8, 0x0000c8, 0x004aff, 0x0098ff, 0x4affa1, 0xa1ff4a,
            0xffff00, 0xea9f00, 0xff4a00, 0xde4a00, 0xfd0e0d,
        ];
        for (index, hex) in stops.into_iter().enumerate() {
            let expected = d3rs::color::D3Color::from_hex(hex);
            let (r, g, b) = surface.color_at(index as f32 / 10.0);
            assert!((r - expected.r).abs() < 1e-6);
            assert!((g - expected.g).abs() < 1e-6);
            assert!((b - expected.b).abs() < 1e-6);
        }
        // Check interpolation and clamping as well as the palette stops.
        for sample in -10..=110 {
            let t = f64::from(sample) / 100.0;
            let expected = contour(t);
            let (r, g, b) = surface.color_at(t as f32);
            assert!((r - expected.r).abs() < 1e-6);
            assert!((g - expected.g).abs() < 1e-6);
            assert!((b - expected.b).abs() < 1e-6);
        }
    }

    #[test]
    fn spinorama_colormap_matches_python_stops() {
        let scale = crate::types::Colormap::Spinorama.color_scale();
        let low = scale(0.0);
        assert!((low.b - 168.0 / 255.0_f32).abs() < 1e-6 && low.r == 0.0 && low.g == 0.0);
        let high = scale(1.0);
        assert!(
            (high.r - 253.0 / 255.0_f32).abs() < 1e-6
                && (high.g - 14.0 / 255.0_f32).abs() < 1e-6
                && (high.b - 13.0 / 255.0_f32).abs() < 1e-6,
            "high stop: {high:?}"
        );
        assert_eq!(
            crate::types::Colormap::default(),
            crate::types::Colormap::Spinorama
        );
    }

    #[test]
    fn parse_plot_keeps_distinct_line_and_fill_colors() {
        // Band traces carry both an outline colour and a zone colour;
        // the fill colour must survive parsing even when a line colour
        // is present (spinorama's ±dB zones).
        let plot = json!({
            "data": [
                {"type":"scatter","name":"Band ±3dB","x":[20,20000,20000,20],"y":[3,3,-3,-3],"fill":"toself","fillcolor":"#E2F705","opacity":0.25,"line":{"color":"#0000ff"}}
            ],
            "layout": {"xaxis":{"type":"log"},"yaxis":{"range":[-45,5]}}
        });
        let data = parse_plot(&plot).unwrap();
        let trace = &data.traces[0];
        assert_eq!(trace.fill, Some(crate::types::TraceFill::ToSelf));
        assert_eq!(
            trace.fill_color,
            Some(d3rs::color::D3Color::rgb(0xE2, 0xF7, 0x05))
        );
        assert_eq!(trace.color, d3rs::color::D3Color::rgb(0, 0, 0xFF));
        assert!((trace.opacity - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_subplot_cells_traces_and_titles() {
        let plot = json!({
            "data": [
                {"type": "scatter", "name": "eq", "x": [100.0, 1000.0], "y": [1.0, 2.0], "xaxis": "x", "yaxis": "y"},
                {"type": "scatter", "name": "target", "x": [100.0, 1000.0], "y": [0.0, 1.0], "xaxis": "x2", "yaxis": "y2"},
                {"type": "bar", "name": "err", "x": ["-1-0", "0-1"], "y": [3.0, 7.0], "xaxis": "x3", "yaxis": "y3"},
                {"type": "scatter", "name": "spin", "x": [100.0, 1000.0], "y": [80.0, 85.0], "xaxis": "x4", "yaxis": "y4"}
            ],
            "layout": {
                "title": {"text": "Sheet"},
                "xaxis": {"domain": [0.0, 0.48], "type": "log"},
                "yaxis": {"domain": [0.55, 1.0]},
                "xaxis2": {"domain": [0.52, 1.0], "type": "log"},
                "yaxis2": {"domain": [0.55, 1.0]},
                "xaxis3": {"domain": [0.0, 0.48]},
                "yaxis3": {"domain": [0.0, 0.45]},
                "xaxis4": {"domain": [0.52, 1.0], "type": "log"},
                "yaxis4": {"domain": [0.0, 0.45]},
                "annotations": [
                    {"text": "PEQ details", "x": 0.24, "y": 1.0, "xref": "paper", "yref": "paper", "showarrow": false},
                    {"text": "PEQ v.s. Target", "x": 0.76, "y": 1.0, "xref": "paper", "yref": "paper", "showarrow": false},
                    {"text": "Histogram", "x": 0.24, "y": 0.45, "xref": "paper", "yref": "paper", "showarrow": false},
                    {"text": "Spin", "x": 0.76, "y": 0.45, "xref": "paper", "yref": "paper", "showarrow": false}
                ]
            }
        });
        let data = parse_plot(&plot).unwrap();
        assert_eq!(data.layout.subplots.len(), 4);
        assert_eq!(
            data.layout.subplots[0].title.as_deref(),
            Some("PEQ details")
        );
        assert_eq!(
            data.layout.subplots[1].title.as_deref(),
            Some("PEQ v.s. Target")
        );
        assert_eq!(data.layout.subplots[2].title.as_deref(), Some("Histogram"));
        assert_eq!(data.layout.subplots[3].title.as_deref(), Some("Spin"));
        assert_eq!(data.traces[0].subplot, 0);
        assert_eq!(data.traces[1].subplot, 1);
        assert_eq!(data.traces[2].subplot, 2);
        assert_eq!(data.traces[2].kind, crate::types::TraceKind::Bar);
        assert_eq!(data.traces[3].subplot, 3);
        assert!(data.layout.annotations.is_empty());
        assert!(data.layout.subplots[0].x_axis.log);
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
