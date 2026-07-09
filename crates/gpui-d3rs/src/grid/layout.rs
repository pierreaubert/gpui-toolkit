//! Renderer-independent grid geometry.

use super::GridConfig;
use crate::scale::Scale;
use std::borrow::Cow;
use std::error::Error;
use std::fmt;

const DEFAULT_TICK_COUNT: usize = 10;

/// Checked grid-layout error.
#[derive(Debug, Clone, PartialEq)]
pub enum GridLayoutError {
    /// Chart size contained a non-finite value.
    NonFiniteSize { field: &'static str },
    /// Chart size contained a negative value.
    NegativeSize { field: &'static str },
    /// Grid visual configuration contained a non-finite value.
    NonFiniteConfig { field: &'static str },
    /// Grid visual configuration contained a negative value.
    NegativeConfig { field: &'static str },
    /// Grid opacity was outside the inclusive 0..=1 range.
    InvalidOpacity { field: &'static str, value: f32 },
    /// A scale range contained a non-finite endpoint.
    NonFiniteRange { axis: &'static str },
    /// A scale range collapsed to a single point.
    DegenerateRange { axis: &'static str },
    /// A configured tick value was not finite.
    NonFiniteTick { axis: &'static str, value: f64 },
    /// A scale returned a non-finite tick position.
    NonFiniteTickPosition { axis: &'static str, value: f64 },
}

impl fmt::Display for GridLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteSize { field } => write!(f, "grid size field {field} must be finite"),
            Self::NegativeSize { field } => {
                write!(f, "grid size field {field} must be non-negative")
            }
            Self::NonFiniteConfig { field } => {
                write!(f, "grid configuration field {field} must be finite")
            }
            Self::NegativeConfig { field } => {
                write!(f, "grid configuration field {field} must be non-negative")
            }
            Self::InvalidOpacity { field, value } => {
                write!(
                    f,
                    "grid opacity field {field} must be within 0..=1, got {value}"
                )
            }
            Self::NonFiniteRange { axis } => {
                write!(f, "grid {axis} scale range endpoints must be finite")
            }
            Self::DegenerateRange { axis } => {
                write!(f, "grid {axis} scale range must span a non-zero distance")
            }
            Self::NonFiniteTick { axis, value } => {
                write!(f, "grid {axis} tick value {value} must be finite")
            }
            Self::NonFiniteTickPosition { axis, value } => {
                write!(f, "grid {axis} tick position for {value} must be finite")
            }
        }
    }
}

impl Error for GridLayoutError {}

/// Two-dimensional point in chart-local pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridPoint {
    pub x: f32,
    pub y: f32,
}

impl GridPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// One vertical or horizontal grid line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridLine {
    pub value: f64,
    pub start: GridPoint,
    pub end: GridPoint,
}

impl GridLine {
    pub const fn new(value: f64, start: GridPoint, end: GridPoint) -> Self {
        Self { value, start, end }
    }
}

/// One dot at an x/y tick intersection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridDot {
    pub x_value: f64,
    pub y_value: f64,
    pub center: GridPoint,
}

impl GridDot {
    pub const fn new(x_value: f64, y_value: f64, center: GridPoint) -> Self {
        Self {
            x_value,
            y_value,
            center,
        }
    }
}

/// Renderer-independent grid geometry derived from x/y scales and [`GridConfig`].
#[derive(Debug, Clone, PartialEq)]
pub struct GridLayout {
    pub width: f32,
    pub height: f32,
    pub vertical_lines: Vec<GridLine>,
    pub horizontal_lines: Vec<GridLine>,
    pub dots: Vec<GridDot>,
}

impl GridLayout {
    /// Build checked grid geometry from numeric x/y scales.
    pub fn try_from_scales<XS, YS>(
        x_scale: &XS,
        y_scale: &YS,
        config: &GridConfig,
        width: f32,
        height: f32,
    ) -> Result<Self, GridLayoutError>
    where
        XS: Scale<f64, f64>,
        YS: Scale<f64, f64>,
    {
        validate_config(config, width, height)?;

        let x_ticks: Cow<[f64]> = config
            .vertical_line_values
            .as_deref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(x_scale.ticks(DEFAULT_TICK_COUNT)));
        let y_ticks: Cow<[f64]> = config
            .horizontal_line_values
            .as_deref()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(y_scale.ticks(DEFAULT_TICK_COUNT)));

        validate_ticks("x", &x_ticks)?;
        validate_ticks("y", &y_ticks)?;

        let x_range = scale_range("x", x_scale)?;
        let y_range = scale_range("y", y_scale)?;

        let x_positions = x_ticks
            .iter()
            .map(|&value| tick_position("x", x_scale, value, x_range, width, false))
            .collect::<Result<Vec<_>, _>>()?;
        let y_positions = y_ticks
            .iter()
            .map(|&value| tick_position("y", y_scale, value, y_range, height, true))
            .collect::<Result<Vec<_>, _>>()?;

        let vertical_lines = if config.show_vertical_lines {
            x_positions
                .iter()
                .map(|&(value, x)| {
                    GridLine::new(value, GridPoint::new(x, 0.0), GridPoint::new(x, height))
                })
                .collect()
        } else {
            Vec::new()
        };

        let horizontal_lines = if config.show_horizontal_lines {
            y_positions
                .iter()
                .map(|&(value, y)| {
                    GridLine::new(value, GridPoint::new(0.0, y), GridPoint::new(width, y))
                })
                .collect()
        } else {
            Vec::new()
        };

        let dots = if config.show_dots {
            y_positions
                .iter()
                .flat_map(|&(y_value, y)| {
                    x_positions.iter().map(move |&(x_value, x)| {
                        GridDot::new(x_value, y_value, GridPoint::new(x, y))
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(Self {
            width,
            height,
            vertical_lines,
            horizontal_lines,
            dots,
        })
    }

    /// Build grid geometry and panic on invalid input, matching the crate's
    /// existing permissive rendering helpers.
    pub fn from_scales<XS, YS>(
        x_scale: &XS,
        y_scale: &YS,
        config: &GridConfig,
        width: f32,
        height: f32,
    ) -> Self
    where
        XS: Scale<f64, f64>,
        YS: Scale<f64, f64>,
    {
        Self::try_from_scales(x_scale, y_scale, config, width, height)
            .expect("invalid grid layout configuration")
    }

    /// Return true when no grid primitives will be drawn.
    pub fn is_empty(&self) -> bool {
        self.vertical_lines.is_empty() && self.horizontal_lines.is_empty() && self.dots.is_empty()
    }
}

/// Convenience helper mirroring d3-style grid layout generation for numeric scales.
pub fn grid_layout<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    config: &GridConfig,
    width: f32,
    height: f32,
) -> Result<GridLayout, GridLayoutError>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    GridLayout::try_from_scales(x_scale, y_scale, config, width, height)
}

fn validate_config(config: &GridConfig, width: f32, height: f32) -> Result<(), GridLayoutError> {
    for (field, value) in [("width", width), ("height", height)] {
        if !value.is_finite() {
            return Err(GridLayoutError::NonFiniteSize { field });
        }
        if value < 0.0 {
            return Err(GridLayoutError::NegativeSize { field });
        }
    }

    for (field, value) in [
        ("line_width", config.line_width),
        ("dot_radius", config.dot_radius),
    ] {
        if !value.is_finite() {
            return Err(GridLayoutError::NonFiniteConfig { field });
        }
        if value < 0.0 {
            return Err(GridLayoutError::NegativeConfig { field });
        }
    }

    for (field, value) in [
        ("line_opacity", config.line_opacity),
        ("dot_opacity", config.dot_opacity),
    ] {
        if !value.is_finite() {
            return Err(GridLayoutError::NonFiniteConfig { field });
        }
        if !(0.0..=1.0).contains(&value) {
            return Err(GridLayoutError::InvalidOpacity { field, value });
        }
    }

    Ok(())
}

fn validate_ticks(axis: &'static str, values: &[f64]) -> Result<(), GridLayoutError> {
    for &value in values {
        if !value.is_finite() {
            return Err(GridLayoutError::NonFiniteTick { axis, value });
        }
    }
    Ok(())
}

fn scale_range<S>(axis: &'static str, scale: &S) -> Result<(f64, f64), GridLayoutError>
where
    S: Scale<f64, f64>,
{
    let (min, max) = scale.range();
    if !min.is_finite() || !max.is_finite() {
        return Err(GridLayoutError::NonFiniteRange { axis });
    }
    if min == max {
        return Err(GridLayoutError::DegenerateRange { axis });
    }
    Ok((min, max))
}

fn tick_position<S>(
    axis: &'static str,
    scale: &S,
    value: f64,
    range: (f64, f64),
    size: f32,
    invert: bool,
) -> Result<(f64, f32), GridLayoutError>
where
    S: Scale<f64, f64>,
{
    let scaled = scale.scale(value);
    if !scaled.is_finite() {
        return Err(GridLayoutError::NonFiniteTickPosition { axis, value });
    }

    let fraction = (scaled - range.0) / (range.1 - range.0);
    let fraction = if invert { 1.0 - fraction } else { fraction };
    Ok((value, (fraction as f32) * size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::LinearScale;

    #[test]
    fn grid_layout_builds_lines_and_dots_from_explicit_ticks() {
        let x_scale = LinearScale::new().domain(0.0, 10.0).range(0.0, 100.0);
        let y_scale = LinearScale::new().domain(0.0, 10.0).range(100.0, 0.0);
        let config = GridConfig::with_lines()
            .with_vertical_values(vec![0.0, 5.0, 10.0])
            .with_horizontal_values(vec![0.0, 10.0]);

        let layout = GridLayout::try_from_scales(&x_scale, &y_scale, &config, 200.0, 120.0)
            .expect("valid grid layout");

        assert_eq!(layout.vertical_lines.len(), 3);
        assert_eq!(layout.horizontal_lines.len(), 2);
        assert_eq!(layout.dots.len(), 6);
        assert_eq!(layout.vertical_lines[1].start, GridPoint::new(100.0, 0.0));
        assert_eq!(layout.vertical_lines[1].end, GridPoint::new(100.0, 120.0));
        assert_eq!(
            layout.horizontal_lines[0],
            GridLine::new(
                0.0,
                GridPoint::new(0.0, 120.0),
                GridPoint::new(200.0, 120.0)
            )
        );
        assert_eq!(layout.dots[5].center, GridPoint::new(200.0, 0.0));
    }

    #[test]
    fn grid_layout_respects_disabled_primitives() {
        let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 10.0);
        let y_scale = LinearScale::new().domain(0.0, 1.0).range(10.0, 0.0);
        let config = GridConfig::new()
            .with_dots(false)
            .with_vertical_lines(false)
            .with_horizontal_lines(false);

        let layout = grid_layout(&x_scale, &y_scale, &config, 20.0, 20.0).unwrap();

        assert!(layout.is_empty());
    }

    #[test]
    fn grid_layout_rejects_invalid_config() {
        let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 10.0);
        let y_scale = LinearScale::new().domain(0.0, 1.0).range(10.0, 0.0);
        let mut config = GridConfig::with_lines();
        config.line_width = f32::NAN;

        assert_eq!(
            GridLayout::try_from_scales(&x_scale, &y_scale, &config, 20.0, 20.0),
            Err(GridLayoutError::NonFiniteConfig {
                field: "line_width"
            })
        );

        config.line_width = 1.0;
        config.dot_opacity = 1.1;
        assert_eq!(
            GridLayout::try_from_scales(&x_scale, &y_scale, &config, 20.0, 20.0),
            Err(GridLayoutError::InvalidOpacity {
                field: "dot_opacity",
                value: 1.1
            })
        );
    }

    #[test]
    fn grid_layout_rejects_invalid_ticks_and_ranges() {
        let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 10.0);
        let y_scale = LinearScale::new().domain(0.0, 1.0).range(10.0, 0.0);
        let config = GridConfig::with_lines().with_vertical_values(vec![f64::INFINITY]);

        assert_eq!(
            GridLayout::try_from_scales(&x_scale, &y_scale, &config, 20.0, 20.0),
            Err(GridLayoutError::NonFiniteTick {
                axis: "x",
                value: f64::INFINITY
            })
        );

        let degenerate_x = LinearScale::new().domain(0.0, 1.0).range(10.0, 10.0);
        assert_eq!(
            GridLayout::try_from_scales(
                &degenerate_x,
                &y_scale,
                &GridConfig::with_lines(),
                20.0,
                20.0
            ),
            Err(GridLayoutError::DegenerateRange { axis: "x" })
        );
    }
}
