//! Generic Plotly trace model for every spinorama.org graph.
//!
//! [`PlotCurve`] only carries line points, so it cannot express the fills,
//! bars, polar traces, secondary axes, or annotations in the real payloads.
//! This module is the general model the generic renderers consume; the
//! specialised CEA2034/contour paths keep using their typed data.

// Rust guideline compliant 2026-02-21

use d3rs::color::D3Color;

/// Kind of a Plotly trace, covering every graph on spinorama.org.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceKind {
    /// `scatter` with a numeric (usually log-frequency) x axis.
    Scatter,
    /// `scatterpolar` with numeric angles in degrees.
    ScatterPolar,
    /// `scatterpolar` whose theta values are category labels (spider).
    ScatterPolarCategorical,
    /// `bar` with categorical x labels.
    Bar,
    /// `contour` / `heatmap` grid (z over x/y).
    Contour,
    /// `surface` grid (rendered by the existing 3D/sphere views).
    Surface,
}

/// How a scatter trace connects its points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TraceMode {
    #[default]
    Lines,
    Markers,
    LinesMarkers,
    None,
}

/// Fill rule for a scatter trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceFill {
    /// Fill down to y = 0 (`tozeroy`).
    ToZeroY,
    /// Fill to the previous trace (`tonexty`).
    ToNextY,
}

/// A single Plotly annotation (text placed at a data position).
#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub text: String,
    pub x: f64,
    pub y: f64,
}

/// Axis description taken from the Plotly layout.
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub struct AxisLayout {
    pub title: String,
    pub range: Option<(f64, f64)>,
    /// `log` when the payload declares a log axis, otherwise linear.
    pub log: bool,
    pub tick_values: Vec<f64>,
    pub tick_text: Vec<String>,
}


/// Layout of a Plotly figure: axes, title, and annotations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlotLayout {
    pub title: Option<String>,
    pub x_axis: AxisLayout,
    pub y_axis: AxisLayout,
    /// Secondary y axis (`yaxis2`), e.g. DI in CEA2034 or `filters_eq`.
    pub y_axis2: Option<AxisLayout>,
    pub annotations: Vec<Annotation>,
    /// Radial range for polar figures (`polar.radialaxis.range`).
    pub polar_radial_range: Option<(f64, f64)>,
}

/// One generic trace with everything the renderers need.
#[derive(Debug, Clone, PartialEq)]
pub struct GenericTrace {
    pub kind: TraceKind,
    pub name: String,
    /// Numeric x values (frequency, angle index, …).
    pub x: Vec<f64>,
    /// Numeric y values.
    pub y: Vec<f64>,
    /// Grid values for contour/surface traces, row-major.
    pub z: Vec<f64>,
    /// Category labels for bar / categorical-polar traces.
    pub categories: Vec<String>,
    /// Polar angles in degrees (scatterpolar traces).
    pub theta: Vec<f64>,
    /// Polar radii (scatterpolar traces).
    pub r: Vec<f64>,
    pub color: D3Color,
    pub stroke_width: f32,
    /// Plotly dash name (`solid`, `dash`, `dot`, `dashdot`, …).
    pub dash: Option<String>,
    pub mode: TraceMode,
    pub fill: Option<TraceFill>,
    pub fill_color: Option<D3Color>,
    pub opacity: f32,
    /// True when the trace is bound to `yaxis2`.
    pub secondary_axis: bool,
    pub visible: bool,
    /// Marker size for marker modes.
    pub marker_size: f32,
}

impl GenericTrace {
    /// Creates an empty scatter trace with default styling.
    pub fn new(kind: TraceKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            x: Vec::new(),
            y: Vec::new(),
            z: Vec::new(),
            categories: Vec::new(),
            theta: Vec::new(),
            r: Vec::new(),
            color: D3Color::rgb(128, 128, 128),
            stroke_width: 2.0,
            dash: None,
            mode: TraceMode::default(),
            fill: None,
            fill_color: None,
            opacity: 1.0,
            secondary_axis: false,
            visible: true,
            marker_size: 5.0,
        }
    }

    /// Line points as `(x, y)` pairs, dropping non-finite samples.
    pub fn points(&self) -> Vec<(f64, f64)> {
        self.x
            .iter()
            .zip(&self.y)
            .filter(|(a, b)| a.is_finite() && b.is_finite())
            .map(|(&a, &b)| (a, b))
            .collect()
    }

    /// Polar points as `(theta_deg, r)` pairs.
    pub fn polar_points(&self) -> Vec<(f64, f64)> {
        self.theta
            .iter()
            .zip(&self.r)
            .filter(|(a, b)| a.is_finite() && b.is_finite())
            .map(|(&a, &b)| (a, b))
            .collect()
    }
}

/// A parsed Plotly figure: traces plus layout.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PlotData {
    pub traces: Vec<GenericTrace>,
    pub layout: PlotLayout,
}

impl PlotData {
    /// Visible traces of one kind, in payload (legend) order.
    #[allow(dead_code, reason = "public query helper for generic consumers")]
    pub fn visible(&self, kind: TraceKind) -> Vec<&GenericTrace> {
        self.traces
            .iter()
            .filter(|t| t.visible && t.kind == kind)
            .collect()
    }

    /// Data extent over visible scatter/bar traces for one axis.
    pub fn extent(&self, secondary: bool, y_axis: bool) -> Option<(f64, f64)> {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for trace in &self.traces {
            if !trace.visible || trace.secondary_axis != secondary {
                continue;
            }
            let values: &[f64] = if y_axis { &trace.y } else { &trace.x };
            for &v in values {
                if v.is_finite() {
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
        }
        (lo.is_finite() && hi.is_finite() && hi > lo).then_some((lo, hi))
    }
}

/// Parses a Plotly colour (`#rrggbb`, `rgb(r,g,b)`, `rgba(r,g,b,a)`).
///
/// Returns the colour and its alpha (1.0 when unspecified).
pub fn parse_plotly_color(text: &str) -> Option<(D3Color, f32)> {
    let text = text.trim();
    if let Some(hex) = text.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some((D3Color::rgb(r, g, b), 1.0));
        }
        return None;
    }
    let inner = text
        .strip_prefix("rgba(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|s| (s, true))
        .or_else(|| {
            text.strip_prefix("rgb(")
                .and_then(|s| s.strip_suffix(')'))
                .map(|s| (s, false))
        })?;
    let parts: Vec<f32> = inner
        .0
        .split(',')
        .map(|p| p.trim().parse::<f32>())
        .collect::<Result<_, _>>()
        .ok()?;
    if parts.len() < 3 {
        return None;
    }
    let alpha = if inner.1 && parts.len() >= 4 {
        parts[3].clamp(0.0, 1.0)
    } else {
        1.0
    };
    Some((
        D3Color::rgb(
            parts[0].clamp(0.0, 255.0) as u8,
            parts[1].clamp(0.0, 255.0) as u8,
            parts[2].clamp(0.0, 255.0) as u8,
        ),
        alpha,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_and_rgba_colors() {
        assert_eq!(
            parse_plotly_color("#ff8000"),
            Some((D3Color::rgb(255, 128, 0), 1.0))
        );
        assert_eq!(
            parse_plotly_color("rgba(10, 20, 30, 0.5)"),
            Some((D3Color::rgb(10, 20, 30), 0.5))
        );
        assert_eq!(parse_plotly_color("not-a-color"), None);
    }

    #[test]
    fn points_drop_non_finite_samples() {
        let mut trace = GenericTrace::new(TraceKind::Scatter, "t");
        trace.x = vec![100.0, f64::NAN, 1000.0];
        trace.y = vec![1.0, 2.0, 3.0];
        assert_eq!(trace.points(), vec![(100.0, 1.0), (1000.0, 3.0)]);
    }
}
