//! Renderer-independent d3-axis layout helpers.

use super::{AxisConfig, AxisOrientation};
use crate::scale::Scale;
use std::error::Error;
use std::fmt;

/// Checked axis-layout error.
#[derive(Debug, Clone, PartialEq)]
pub enum AxisLayoutError {
    /// Axis size or configuration contained a non-finite value.
    NonFiniteConfig { field: &'static str },
    /// Axis size or configuration contained a negative value.
    NegativeConfig { field: &'static str },
    /// The scale range contained a non-finite endpoint.
    NonFiniteRange,
    /// A configured tick value was not finite.
    NonFiniteTick { value: f64 },
    /// A scale returned a non-finite tick position.
    NonFiniteTickPosition { value: f64 },
}

impl fmt::Display for AxisLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteConfig { field } => {
                write!(f, "axis configuration field {field} must be finite")
            }
            Self::NegativeConfig { field } => {
                write!(f, "axis configuration field {field} must be non-negative")
            }
            Self::NonFiniteRange => write!(f, "axis scale range endpoints must be finite"),
            Self::NonFiniteTick { value } => write!(f, "axis tick value {value} must be finite"),
            Self::NonFiniteTickPosition { value } => {
                write!(f, "axis tick position for {value} must be finite")
            }
        }
    }
}

impl Error for AxisLayoutError {}

/// Two-dimensional point in axis-local coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisPoint {
    pub x: f32,
    pub y: f32,
}

impl AxisPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Domain line or tick mark geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisLine {
    pub start: AxisPoint,
    pub end: AxisPoint,
}

impl AxisLine {
    pub const fn new(start: AxisPoint, end: AxisPoint) -> Self {
        Self { start, end }
    }
}

/// One major or minor axis tick.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisTick {
    pub value: f64,
    pub position: f64,
    pub line: AxisLine,
    pub label_position: Option<AxisPoint>,
    pub label: Option<String>,
    pub label_angle_degrees: f32,
    pub is_minor: bool,
}

/// Optional axis title geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisTitle {
    pub text: String,
    pub position: AxisPoint,
    pub angle_degrees: f32,
}

/// Renderer-independent axis geometry derived from a scale and [`AxisConfig`].
#[derive(Debug, Clone, PartialEq)]
pub struct AxisLayout {
    pub orientation: AxisOrientation,
    pub size: f32,
    pub domain_line: Option<AxisLine>,
    pub major_ticks: Vec<AxisTick>,
    pub minor_ticks: Vec<AxisTick>,
    pub title: Option<AxisTitle>,
}

impl AxisLayout {
    /// Build checked axis geometry from a numeric scale.
    pub fn try_from_scale<S>(
        scale: &S,
        config: &AxisConfig,
        size: f32,
    ) -> Result<Self, AxisLayoutError>
    where
        S: Scale<f64, f64>,
    {
        validate_config(config, size)?;

        let (range_min, range_max) = scale.range();
        if !range_min.is_finite() || !range_max.is_finite() {
            return Err(AxisLayoutError::NonFiniteRange);
        }

        let major_values = config
            .tick_values
            .clone()
            .unwrap_or_else(|| scale.ticks(config.tick_count));
        validate_ticks(&major_values)?;

        let minor_values = config.minor_tick_values.clone().unwrap_or_default();
        validate_ticks(&minor_values)?;

        let major_ticks = major_values
            .into_iter()
            .map(|value| axis_tick(scale, config, value, false))
            .collect::<Result<Vec<_>, _>>()?;
        let minor_ticks = minor_values
            .into_iter()
            .map(|value| axis_tick(scale, config, value, true))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            orientation: config.orientation,
            size,
            domain_line: config
                .show_domain_line
                .then(|| domain_line(config.orientation, range_min, range_max)),
            major_ticks,
            minor_ticks,
            title: axis_title(config, range_min, range_max),
        })
    }

    /// Build axis geometry and panic on invalid input, matching the crate's
    /// existing permissive scale/axis APIs.
    pub fn from_scale<S>(scale: &S, config: &AxisConfig, size: f32) -> Self
    where
        S: Scale<f64, f64>,
    {
        Self::try_from_scale(scale, config, size).expect("invalid axis layout configuration")
    }

    /// Return all ticks in major-then-minor order.
    pub fn all_ticks(&self) -> impl Iterator<Item = &AxisTick> {
        self.major_ticks.iter().chain(self.minor_ticks.iter())
    }
}

/// Convenience helper mirroring d3-axis layout generation for numeric scales.
pub fn axis_layout<S>(
    scale: &S,
    config: &AxisConfig,
    size: f32,
) -> Result<AxisLayout, AxisLayoutError>
where
    S: Scale<f64, f64>,
{
    AxisLayout::try_from_scale(scale, config, size)
}

fn validate_config(config: &AxisConfig, size: f32) -> Result<(), AxisLayoutError> {
    let fields = [
        ("size", size),
        ("tick_size", config.tick_size),
        ("minor_tick_size", config.minor_tick_size),
        ("tick_padding", config.tick_padding),
        ("label_font_size", config.label_font_size),
        ("domain_line_width", config.domain_line_width),
        ("title_font_size", config.title_font_size),
        ("title_padding", config.title_padding),
        ("label_angle", config.label_angle),
    ];

    for (field, value) in fields {
        if !value.is_finite() {
            return Err(AxisLayoutError::NonFiniteConfig { field });
        }
    }

    for (field, value) in fields
        .into_iter()
        .filter(|(field, _)| *field != "label_angle")
    {
        if value < 0.0 {
            return Err(AxisLayoutError::NegativeConfig { field });
        }
    }

    Ok(())
}

fn validate_ticks(values: &[f64]) -> Result<(), AxisLayoutError> {
    for &value in values {
        if !value.is_finite() {
            return Err(AxisLayoutError::NonFiniteTick { value });
        }
    }
    Ok(())
}

fn axis_tick<S>(
    scale: &S,
    config: &AxisConfig,
    value: f64,
    is_minor: bool,
) -> Result<AxisTick, AxisLayoutError>
where
    S: Scale<f64, f64>,
{
    let position = scale.scale(value);
    if !position.is_finite() {
        return Err(AxisLayoutError::NonFiniteTickPosition { value });
    }

    let tick_size = if is_minor {
        config.minor_tick_size
    } else {
        config.tick_size
    };
    let position = position as f32;
    let line = tick_line(config.orientation, position, tick_size);
    let label_position = (!is_minor).then(|| label_position(config, position));
    let label = (!is_minor).then(|| format_tick(value, &config.tick_format));

    Ok(AxisTick {
        value,
        position: f64::from(position),
        line,
        label_position,
        label,
        label_angle_degrees: config.label_angle,
        is_minor,
    })
}

fn domain_line(orientation: AxisOrientation, range_min: f64, range_max: f64) -> AxisLine {
    let start = range_min as f32;
    let end = range_max as f32;
    match orientation {
        AxisOrientation::Top | AxisOrientation::Bottom => {
            AxisLine::new(AxisPoint::new(start, 0.0), AxisPoint::new(end, 0.0))
        }
        AxisOrientation::Left | AxisOrientation::Right => {
            AxisLine::new(AxisPoint::new(0.0, start), AxisPoint::new(0.0, end))
        }
    }
}

fn tick_line(orientation: AxisOrientation, position: f32, tick_size: f32) -> AxisLine {
    match orientation {
        AxisOrientation::Top => AxisLine::new(
            AxisPoint::new(position, 0.0),
            AxisPoint::new(position, -tick_size),
        ),
        AxisOrientation::Bottom => AxisLine::new(
            AxisPoint::new(position, 0.0),
            AxisPoint::new(position, tick_size),
        ),
        AxisOrientation::Left => AxisLine::new(
            AxisPoint::new(0.0, position),
            AxisPoint::new(-tick_size, position),
        ),
        AxisOrientation::Right => AxisLine::new(
            AxisPoint::new(0.0, position),
            AxisPoint::new(tick_size, position),
        ),
    }
}

fn label_position(config: &AxisConfig, position: f32) -> AxisPoint {
    let offset = config.tick_size + config.tick_padding;
    match config.orientation {
        AxisOrientation::Top => AxisPoint::new(position, -offset),
        AxisOrientation::Bottom => AxisPoint::new(position, offset + config.label_font_size),
        AxisOrientation::Left => AxisPoint::new(-offset, position),
        AxisOrientation::Right => AxisPoint::new(offset, position),
    }
}

fn axis_title(config: &AxisConfig, range_min: f64, range_max: f64) -> Option<AxisTitle> {
    let text = config.title.clone()?;
    let center = ((range_min + range_max) / 2.0) as f32;
    let title_offset =
        config.tick_size + config.tick_padding + config.label_font_size + config.title_padding;
    let (position, angle_degrees) = match config.orientation {
        AxisOrientation::Top => (AxisPoint::new(center, -title_offset), 0.0),
        AxisOrientation::Bottom => (AxisPoint::new(center, title_offset), 0.0),
        AxisOrientation::Left => (AxisPoint::new(-title_offset, center), -90.0),
        AxisOrientation::Right => (AxisPoint::new(title_offset, center), 90.0),
    };

    Some(AxisTitle {
        text,
        position,
        angle_degrees,
    })
}

fn format_tick(value: f64, formatter: &Option<fn(f64) -> String>) -> String {
    match formatter {
        Some(formatter) => formatter(value),
        None if value.abs() < 1e-10 => "0".to_string(),
        None if value.abs() >= 1000.0 || value.abs() < 0.01 => format!("{value:.1e}"),
        None if value.fract().abs() < 1e-10 => format!("{value:.0}"),
        None => format!("{value:.1}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::LinearScale;

    #[test]
    fn bottom_axis_layout_generates_major_ticks_labels_and_title() {
        let scale = LinearScale::new().domain(0.0, 100.0).range(0.0, 500.0);
        let config = AxisConfig::bottom()
            .with_tick_values(vec![0.0, 50.0, 100.0])
            .with_minor_tick_values(vec![25.0, 75.0])
            .with_title("Frequency");

        let layout = AxisLayout::try_from_scale(&scale, &config, 500.0).unwrap();

        assert_eq!(layout.orientation, AxisOrientation::Bottom);
        assert_eq!(
            layout.domain_line,
            Some(AxisLine::new(
                AxisPoint::new(0.0, 0.0),
                AxisPoint::new(500.0, 0.0)
            ))
        );
        assert_eq!(layout.major_ticks.len(), 3);
        assert_eq!(layout.major_ticks[1].position, 250.0);
        assert_eq!(layout.major_ticks[1].label.as_deref(), Some("50"));
        assert_eq!(
            layout.major_ticks[1].line,
            AxisLine::new(AxisPoint::new(250.0, 0.0), AxisPoint::new(250.0, 6.0))
        );
        assert_eq!(layout.minor_ticks.len(), 2);
        assert!(layout.minor_ticks.iter().all(|tick| tick.label.is_none()));
        assert_eq!(
            layout.title.as_ref().map(|title| title.text.as_str()),
            Some("Frequency")
        );
    }

    #[test]
    fn left_axis_layout_positions_ticks_outward() {
        let scale = LinearScale::new().domain(0.0, 10.0).range(100.0, 0.0);
        let config = AxisConfig::left().with_tick_values(vec![0.0, 5.0, 10.0]);

        let layout = axis_layout(&scale, &config, 100.0).unwrap();

        assert_eq!(
            layout.domain_line,
            Some(AxisLine::new(
                AxisPoint::new(0.0, 100.0),
                AxisPoint::new(0.0, 0.0)
            ))
        );
        assert_eq!(
            layout.major_ticks[0].line,
            AxisLine::new(AxisPoint::new(0.0, 100.0), AxisPoint::new(-6.0, 100.0))
        );
        assert_eq!(layout.major_ticks[2].position, 0.0);
    }

    #[test]
    fn axis_layout_supports_custom_formatters_and_hidden_domain() {
        let scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 100.0);
        let config = AxisConfig::top()
            .with_tick_values(vec![0.5])
            .with_formatter(|value| format!("{:.0}%", value * 100.0))
            .hide_domain_line();

        let layout = axis_layout(&scale, &config, 100.0).unwrap();

        assert_eq!(layout.domain_line, None);
        assert_eq!(layout.major_ticks[0].label.as_deref(), Some("50%"));
        assert_eq!(
            layout.major_ticks[0].line,
            AxisLine::new(AxisPoint::new(50.0, 0.0), AxisPoint::new(50.0, -6.0))
        );
    }

    #[test]
    fn axis_layout_reports_invalid_inputs() {
        let scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 100.0);

        assert_eq!(
            axis_layout(
                &scale,
                &AxisConfig::bottom().with_tick_size(f32::NAN),
                100.0
            )
            .unwrap_err(),
            AxisLayoutError::NonFiniteConfig { field: "tick_size" }
        );
        assert_eq!(
            axis_layout(
                &scale,
                &AxisConfig::bottom().with_tick_values(vec![f64::INFINITY]),
                100.0
            )
            .unwrap_err(),
            AxisLayoutError::NonFiniteTick {
                value: f64::INFINITY,
            }
        );
    }
}
