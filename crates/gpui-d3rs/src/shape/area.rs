//! Area shape generator
//!
//! Generates area shapes for area charts and stacked area charts.

use std::fmt;

use super::curve::Curve;
use super::path::{Path, PathBuilder, Point};

/// Recoverable errors for checked area path input validation.
#[derive(Debug, Clone, PartialEq)]
pub enum AreaGenerationError {
    /// Checked area coordinates must be finite before path commands are emitted.
    NonFiniteCoordinate {
        index: usize,
        coordinate: &'static str,
        value: f64,
    },
    /// Checked simple areas require equal coordinate array lengths.
    CoordinateLengthMismatch {
        x_len: usize,
        y0_len: usize,
        y1_len: usize,
    },
}

impl fmt::Display for AreaGenerationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteCoordinate {
                index,
                coordinate,
                value,
            } => write!(
                f,
                "area coordinate {coordinate} at index {index} is not finite: {value}"
            ),
            Self::CoordinateLengthMismatch {
                x_len,
                y0_len,
                y1_len,
            } => write!(
                f,
                "area coordinate arrays have mismatched lengths: x={x_len}, y0={y0_len}, y1={y1_len}"
            ),
        }
    }
}

impl std::error::Error for AreaGenerationError {}

#[derive(Debug, Clone, Default)]
struct AreaSegmentCoordinates {
    top_points: Vec<Point>,
    bottom_points: Vec<Point>,
}

/// An area generator for creating filled area shapes.
///
/// # Example
///
/// ```
/// use d3rs::shape::area::Area;
///
/// let data = vec![(0.0, 10.0), (1.0, 20.0), (2.0, 15.0), (3.0, 25.0)];
/// let area = Area::new()
///     .x(|d: &(f64, f64)| d.0)
///     .y0(|_| 0.0)
///     .y1(|d: &(f64, f64)| d.1);
///
/// let path = area.generate(&data);
/// assert!(!path.is_empty());
/// ```
#[allow(clippy::type_complexity)]
pub struct Area<T> {
    x: Box<dyn Fn(&T) -> f64>,
    x0: Option<Box<dyn Fn(&T) -> f64>>,
    x1: Option<Box<dyn Fn(&T) -> f64>>,
    y: Box<dyn Fn(&T) -> f64>,
    y0: Box<dyn Fn(&T) -> f64>,
    y1: Option<Box<dyn Fn(&T) -> f64>>,
    defined: Box<dyn Fn(&T) -> bool>,
    curve: Curve,
}

impl<T> Default for Area<T> {
    fn default() -> Self {
        Self {
            x: Box::new(|_| 0.0),
            x0: None,
            x1: None,
            y: Box::new(|_| 0.0),
            y0: Box::new(|_| 0.0),
            y1: None,
            defined: Box::new(|_| true),
            curve: Curve::Linear,
        }
    }
}

impl<T> Area<T> {
    /// Create a new area generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the x accessor function.
    pub fn x<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.x = Box::new(f);
        self
    }

    /// Set the x0 (left baseline) accessor function.
    pub fn x0<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.x0 = Some(Box::new(f));
        self
    }

    /// Set the x1 (right edge) accessor function.
    pub fn x1<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.x1 = Some(Box::new(f));
        self
    }

    /// Set the y accessor function.
    pub fn y<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.y = Box::new(f);
        self
    }

    /// Set the y0 (bottom baseline) accessor function.
    pub fn y0<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.y0 = Box::new(f);
        self
    }

    /// Set the y1 (top edge) accessor function.
    pub fn y1<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> f64 + 'static,
    {
        self.y1 = Some(Box::new(f));
        self
    }

    /// Set the defined accessor function.
    ///
    /// Points where this returns false will be treated as gaps.
    pub fn defined<F>(mut self, f: F) -> Self
    where
        F: Fn(&T) -> bool + 'static,
    {
        self.defined = Box::new(f);
        self
    }

    /// Set the curve type.
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Generate the area path from data.
    pub fn generate(&self, data: &[T]) -> Path {
        let mut builder = PathBuilder::new();
        self.generate_into(data, &mut builder);
        builder.build()
    }

    /// Generate the area path from data after validating rendered coordinates.
    ///
    /// This checked path rejects non-finite coordinates returned by `x`, `x0`,
    /// `x1`, `y`, `y0`, or `y1` for defined data points. Undefined points are
    /// still treated as gaps and are not inspected, matching [`Self::generate`].
    pub fn try_generate(&self, data: &[T]) -> Result<Path, AreaGenerationError> {
        let mut builder = PathBuilder::new();
        self.try_generate_into(data, &mut builder)?;
        Ok(builder.build())
    }

    /// Generate the area path from data, appending commands into `builder`.
    ///
    /// `builder` is *not* cleared; callers should call `builder.commands.clear()`
    /// (or create a fresh builder) if they want to replace a previous path.
    pub fn generate_into(&self, data: &[T], builder: &mut PathBuilder) {
        if data.is_empty() {
            return;
        }

        let segments = self.collect_area_segments(data, false).unwrap_or_default();
        self.append_area_segments(segments, builder);
    }

    /// Generate the checked area path from data, appending commands into `builder`.
    ///
    /// The destination builder is not mutated when validation fails.
    pub fn try_generate_into(
        &self,
        data: &[T],
        builder: &mut PathBuilder,
    ) -> Result<(), AreaGenerationError> {
        if data.is_empty() {
            return Ok(());
        }

        let segments = self.collect_area_segments(data, true)?;
        self.append_area_segments(segments, builder);
        Ok(())
    }

    fn append_area_segments(
        &self,
        segments: Vec<AreaSegmentCoordinates>,
        builder: &mut PathBuilder,
    ) {
        let mut seg_builder = PathBuilder::new();
        for segment in segments {
            if segment.top_points.is_empty() {
                continue;
            }

            let top_points = segment.top_points;
            let bottom_points: Vec<Point> = segment.bottom_points.iter().rev().copied().collect();

            seg_builder.commands.clear();
            seg_builder.current_point = Point::default();
            seg_builder.start_point = Point::default();

            // Generate curved path for top line
            if !top_points.is_empty() {
                let first = top_points[0];
                seg_builder = seg_builder.move_to(first.x, first.y);

                // Apply curve interpolation
                match self.curve {
                    Curve::Linear => {
                        for p in top_points.iter().skip(1) {
                            seg_builder = seg_builder.line_to(p.x, p.y);
                        }
                    }
                    _ => {
                        // For other curves, use the curve's interpolation
                        let curved = self.curve.interpolate(&top_points);
                        for p in curved.iter().skip(1) {
                            seg_builder = seg_builder.line_to(p.x, p.y);
                        }
                    }
                }

                // Connect to bottom line and draw it
                match self.curve {
                    Curve::Linear => {
                        for p in &bottom_points {
                            seg_builder = seg_builder.line_to(p.x, p.y);
                        }
                    }
                    _ => {
                        let curved = self.curve.interpolate(&bottom_points);
                        for p in &curved {
                            seg_builder = seg_builder.line_to(p.x, p.y);
                        }
                    }
                }

                seg_builder = seg_builder.close_path();
            }

            builder
                .commands
                .extend(std::mem::take(&mut seg_builder.commands));
        }
    }

    fn collect_area_segments(
        &self,
        data: &[T],
        checked: bool,
    ) -> Result<Vec<AreaSegmentCoordinates>, AreaGenerationError> {
        let mut segments = Vec::new();
        let mut current = AreaSegmentCoordinates::default();

        for (index, d) in data.iter().enumerate() {
            if (self.defined)(d) {
                let top_x = self
                    .x1
                    .as_ref()
                    .map(|f| f(d))
                    .unwrap_or_else(|| (self.x)(d));
                let top_y = self
                    .y1
                    .as_ref()
                    .map(|f| f(d))
                    .unwrap_or_else(|| (self.y)(d));
                let bottom_x = self
                    .x0
                    .as_ref()
                    .map(|f| f(d))
                    .unwrap_or_else(|| (self.x)(d));
                let bottom_y = (self.y0)(d);

                if checked {
                    validate_area_coordinate(index, "x1", top_x)?;
                    validate_area_coordinate(index, "y1", top_y)?;
                    validate_area_coordinate(index, "x0", bottom_x)?;
                    validate_area_coordinate(index, "y0", bottom_y)?;
                }

                current.top_points.push(Point::new(top_x, top_y));
                current.bottom_points.push(Point::new(bottom_x, bottom_y));
            } else if !current.top_points.is_empty() {
                segments.push(current);
                current = AreaSegmentCoordinates::default();
            }
        }

        if !current.top_points.is_empty() {
            segments.push(current);
        }

        Ok(segments)
    }
}

fn validate_area_coordinate(
    index: usize,
    coordinate: &'static str,
    value: f64,
) -> Result<(), AreaGenerationError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(AreaGenerationError::NonFiniteCoordinate {
            index,
            coordinate,
            value,
        })
    }
}

/// Generate area points for rendering.
///
/// Returns the outline points of an area shape.
///
/// # Arguments
///
/// * `data` - The data points
/// * `x` - X accessor
/// * `y0` - Baseline Y accessor
/// * `y1` - Top line Y accessor
pub fn area_points<T, FX, FY0, FY1>(data: &[T], x: FX, y0: FY0, y1: FY1) -> Vec<Point>
where
    FX: Fn(&T) -> f64,
    FY0: Fn(&T) -> f64,
    FY1: Fn(&T) -> f64,
{
    if data.is_empty() {
        return Vec::new();
    }

    let mut points = Vec::with_capacity(data.len() * 2 + 1);

    // Top line (left to right)
    for d in data {
        points.push(Point::new(x(d), y1(d)));
    }

    // Bottom line (right to left)
    for d in data.iter().rev() {
        points.push(Point::new(x(d), y0(d)));
    }

    // Close the shape
    if !points.is_empty() {
        points.push(points[0]);
    }

    points
}

/// Checked area outline point generation.
pub fn try_area_points<T, FX, FY0, FY1>(
    data: &[T],
    x: FX,
    y0: FY0,
    y1: FY1,
) -> Result<Vec<Point>, AreaGenerationError>
where
    FX: Fn(&T) -> f64,
    FY0: Fn(&T) -> f64,
    FY1: Fn(&T) -> f64,
{
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let mut points = Vec::with_capacity(data.len() * 2 + 1);

    for (index, d) in data.iter().enumerate() {
        let x = x(d);
        let y1 = y1(d);
        validate_area_coordinate(index, "x", x)?;
        validate_area_coordinate(index, "y1", y1)?;
        points.push(Point::new(x, y1));
    }

    for (index, d) in data.iter().enumerate().rev() {
        let x = x(d);
        let y0 = y0(d);
        validate_area_coordinate(index, "x", x)?;
        validate_area_coordinate(index, "y0", y0)?;
        points.push(Point::new(x, y0));
    }

    if !points.is_empty() {
        points.push(points[0]);
    }

    Ok(points)
}

/// A simple area defined by x, y0, and y1 arrays.
#[derive(Debug, Clone)]
pub struct SimpleArea {
    /// X coordinates
    pub x: Vec<f64>,
    /// Baseline Y coordinates
    pub y0: Vec<f64>,
    /// Top line Y coordinates
    pub y1: Vec<f64>,
}

impl SimpleArea {
    /// Create a new simple area from coordinate arrays.
    ///
    /// All arrays should have the same length.
    pub fn new(x: Vec<f64>, y0: Vec<f64>, y1: Vec<f64>) -> Self {
        Self { x, y0, y1 }
    }

    /// Generate points for rendering.
    pub fn points(&self) -> Vec<Point> {
        let n = self.x.len().min(self.y0.len()).min(self.y1.len());
        let mut points = Vec::with_capacity(n * 2 + 1);

        // Top line
        for i in 0..n {
            points.push(Point::new(self.x[i], self.y1[i]));
        }

        // Bottom line (reversed)
        for i in (0..n).rev() {
            points.push(Point::new(self.x[i], self.y0[i]));
        }

        // Close
        if !points.is_empty() {
            points.push(points[0]);
        }

        points
    }

    /// Generate points for rendering after validating coordinates and lengths.
    pub fn try_points(&self) -> Result<Vec<Point>, AreaGenerationError> {
        validate_simple_area_lengths(self)?;
        try_area_points(
            &(0..self.x.len()).collect::<Vec<_>>(),
            |&i| self.x[i],
            |&i| self.y0[i],
            |&i| self.y1[i],
        )
    }

    /// Generate path for rendering.
    pub fn path(&self) -> Path {
        let points = self.points();
        if points.is_empty() {
            return Path::new();
        }

        let mut builder = PathBuilder::new();
        let first = points[0];
        builder = builder.move_to(first.x, first.y);

        for p in points.iter().skip(1) {
            builder = builder.line_to(p.x, p.y);
        }

        builder.build()
    }

    /// Generate path for rendering after validating coordinates and lengths.
    pub fn try_path(&self) -> Result<Path, AreaGenerationError> {
        simple_area_path_from_points(&self.try_points()?)
    }
}

fn validate_simple_area_lengths(area: &SimpleArea) -> Result<(), AreaGenerationError> {
    if area.x.len() == area.y0.len() && area.x.len() == area.y1.len() {
        Ok(())
    } else {
        Err(AreaGenerationError::CoordinateLengthMismatch {
            x_len: area.x.len(),
            y0_len: area.y0.len(),
            y1_len: area.y1.len(),
        })
    }
}

fn simple_area_path_from_points(points: &[Point]) -> Result<Path, AreaGenerationError> {
    if points.is_empty() {
        return Ok(Path::new());
    }

    let mut builder = PathBuilder::new();
    let first = points[0];
    builder = builder.move_to(first.x, first.y);

    for point in points.iter().skip(1) {
        builder = builder.line_to(point.x, point.y);
    }

    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_area_basic() {
        let data = vec![(0.0, 10.0), (1.0, 20.0), (2.0, 15.0)];
        let area = Area::new()
            .x(|d: &(f64, f64)| d.0)
            .y0(|_| 0.0)
            .y1(|d: &(f64, f64)| d.1);

        let path = area.generate(&data);
        assert!(!path.is_empty());
    }

    #[test]
    fn test_area_points() {
        let data = vec![(0.0, 10.0), (1.0, 20.0), (2.0, 15.0)];
        let points = area_points(&data, |d| d.0, |_| 0.0, |d| d.1);

        // 3 top points + 3 bottom points + 1 closing point
        assert_eq!(points.len(), 7);
    }

    #[test]
    fn try_area_points_matches_area_points_for_finite_coordinates() {
        let data = vec![(0.0, 10.0), (1.0, 20.0), (2.0, 15.0)];
        let expected = area_points(&data, |d| d.0, |_| 0.0, |d| d.1);
        let checked = try_area_points(&data, |d| d.0, |_| 0.0, |d| d.1).unwrap();

        assert_eq!(expected, checked);
    }

    #[test]
    fn try_area_points_rejects_non_finite_coordinates() {
        let data = vec![(0.0, 10.0), (f64::INFINITY, 20.0)];
        assert_eq!(
            try_area_points(&data, |d| d.0, |_| 0.0, |d| d.1).unwrap_err(),
            AreaGenerationError::NonFiniteCoordinate {
                index: 1,
                coordinate: "x",
                value: f64::INFINITY,
            }
        );

        let data = vec![(0.0, 10.0), (1.0, f64::NAN)];
        let error = try_area_points(&data, |d| d.0, |_| 0.0, |d| d.1).unwrap_err();
        match error {
            AreaGenerationError::NonFiniteCoordinate {
                index,
                coordinate,
                value,
            } => {
                assert_eq!(index, 1);
                assert_eq!(coordinate, "y1");
                assert!(value.is_nan());
            }
            error => panic!("unexpected error: {error:?}"),
        }

        assert_eq!(
            try_area_points(&[(0.0, 10.0)], |d| d.0, |_| f64::INFINITY, |d| d.1).unwrap_err(),
            AreaGenerationError::NonFiniteCoordinate {
                index: 0,
                coordinate: "y0",
                value: f64::INFINITY,
            }
        );
    }

    #[test]
    fn test_simple_area() {
        let area = SimpleArea::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 0.0, 0.0],
            vec![10.0, 20.0, 15.0],
        );

        let points = area.points();
        assert_eq!(points.len(), 7);

        let path = area.path();
        assert!(!path.is_empty());
    }

    #[test]
    fn simple_area_checked_methods_match_permissive_for_finite_equal_arrays() {
        let area = SimpleArea::new(
            vec![0.0, 1.0, 2.0],
            vec![0.0, 0.0, 0.0],
            vec![10.0, 20.0, 15.0],
        );

        assert_eq!(area.points(), area.try_points().unwrap());
        assert_eq!(area.path().commands(), area.try_path().unwrap().commands());
    }

    #[test]
    fn simple_area_checked_methods_reject_length_mismatch() {
        let area = SimpleArea::new(vec![0.0, 1.0], vec![0.0], vec![10.0, 20.0]);

        assert_eq!(
            area.try_points().unwrap_err(),
            AreaGenerationError::CoordinateLengthMismatch {
                x_len: 2,
                y0_len: 1,
                y1_len: 2,
            }
        );
        assert!(matches!(
            area.try_path(),
            Err(AreaGenerationError::CoordinateLengthMismatch { .. })
        ));
    }

    #[test]
    fn simple_area_checked_methods_reject_non_finite_coordinates() {
        let area = SimpleArea::new(vec![0.0, 1.0], vec![0.0, 0.0], vec![10.0, f64::NAN]);

        let error = area.try_points().unwrap_err();
        match error {
            AreaGenerationError::NonFiniteCoordinate {
                index,
                coordinate,
                value,
            } => {
                assert_eq!(index, 1);
                assert_eq!(coordinate, "y1");
                assert!(value.is_nan());
            }
            error => panic!("unexpected error: {error:?}"),
        }
    }

    #[test]
    fn test_area_empty() {
        let data: Vec<(f64, f64)> = vec![];
        let area = Area::new()
            .x(|d: &(f64, f64)| d.0)
            .y0(|_| 0.0)
            .y1(|d: &(f64, f64)| d.1);

        let path = area.generate(&data);
        assert!(path.is_empty());
    }

    #[test]
    fn test_generate_into_matches_generate() {
        let data = vec![(0.0, 10.0), (1.0, 20.0), (2.0, 15.0)];
        let area = Area::new()
            .x(|d: &(f64, f64)| d.0)
            .y0(|_| 0.0)
            .y1(|d: &(f64, f64)| d.1);

        let expected = area.generate(&data);
        let mut builder = PathBuilder::new();
        area.generate_into(&data, &mut builder);
        let generated = builder.build();
        assert_eq!(expected.commands(), generated.commands());
    }

    #[test]
    fn try_generate_matches_generate_for_finite_defined_points() {
        let data = vec![(0.0, 10.0), (1.0, 20.0), (2.0, 15.0)];
        let area = Area::new()
            .x(|d: &(f64, f64)| d.0)
            .y0(|_| 0.0)
            .y1(|d: &(f64, f64)| d.1);

        let expected = area.generate(&data);
        let checked = area.try_generate(&data).unwrap();

        assert_eq!(expected.commands(), checked.commands());
    }

    #[test]
    fn try_generate_rejects_non_finite_defined_coordinates() {
        let data = vec![(0.0, 10.0), (1.0, f64::NAN)];
        let area = Area::new()
            .x(|d: &(f64, f64)| d.0)
            .y0(|_| 0.0)
            .y1(|d: &(f64, f64)| d.1);

        let error = area.try_generate(&data).unwrap_err();

        match error {
            AreaGenerationError::NonFiniteCoordinate {
                index,
                coordinate,
                value,
            } => {
                assert_eq!(index, 1);
                assert_eq!(coordinate, "y1");
                assert!(value.is_nan());
            }
            error => panic!("unexpected error: {error:?}"),
        }
    }

    #[test]
    fn try_generate_skips_undefined_non_finite_coordinates() {
        let data = vec![(0.0, 10.0), (1.0, f64::NAN), (2.0, 15.0)];
        let area = Area::new()
            .x(|d: &(f64, f64)| d.0)
            .y0(|_| 0.0)
            .y1(|d: &(f64, f64)| d.1)
            .defined(|d: &(f64, f64)| d.1.is_finite());

        let checked = area.try_generate(&data).unwrap();

        assert!(!checked.is_empty());
    }

    #[test]
    fn try_generate_into_does_not_mutate_builder_on_error() {
        let data = vec![(0.0, 10.0), (1.0, f64::INFINITY)];
        let area = Area::new()
            .x(|d: &(f64, f64)| d.0)
            .y0(|_| 0.0)
            .y1(|d: &(f64, f64)| d.1);
        let mut builder = PathBuilder::new().move_to(42.0, 42.0);
        let before = builder.clone().build();

        let error = area.try_generate_into(&data, &mut builder).unwrap_err();
        let after = builder.build();

        assert_eq!(before.commands(), after.commands());
        match error {
            AreaGenerationError::NonFiniteCoordinate {
                index,
                coordinate,
                value,
            } => {
                assert_eq!(index, 1);
                assert_eq!(coordinate, "y1");
                assert!(value.is_infinite());
            }
            error => panic!("unexpected error: {error:?}"),
        }
    }
}
