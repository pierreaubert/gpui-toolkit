//! Area shape generator
//!
//! Generates area shapes for area charts and stacked area charts.

use super::curve::Curve;
use super::path::{Path, PathBuilder, Point};

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
        if data.is_empty() {
            return Path::new();
        }

        let mut builder = PathBuilder::new();

        // Collect defined points
        let defined_segments = self.collect_defined_segments(data);

        for segment in defined_segments {
            if segment.is_empty() {
                continue;
            }

            // Build top line points
            let top_points: Vec<Point> = segment
                .iter()
                .map(|d| {
                    let x = self
                        .x1
                        .as_ref()
                        .map(|f| f(d))
                        .unwrap_or_else(|| (self.x)(d));
                    let y = self
                        .y1
                        .as_ref()
                        .map(|f| f(d))
                        .unwrap_or_else(|| (self.y)(d));
                    Point::new(x, y)
                })
                .collect();

            // Build bottom line points (reversed)
            let bottom_points: Vec<Point> = segment
                .iter()
                .rev()
                .map(|d| {
                    let x = self
                        .x0
                        .as_ref()
                        .map(|f| f(d))
                        .unwrap_or_else(|| (self.x)(d));
                    let y = (self.y0)(d);
                    Point::new(x, y)
                })
                .collect();

            // Generate curved path for top line
            if !top_points.is_empty() {
                let first = top_points[0];
                builder = builder.move_to(first.x, first.y);

                // Apply curve interpolation
                match self.curve {
                    Curve::Linear => {
                        for p in top_points.iter().skip(1) {
                            builder = builder.line_to(p.x, p.y);
                        }
                    }
                    _ => {
                        // For other curves, use the curve's interpolation
                        let curved = self.curve.interpolate(&top_points);
                        for p in curved.iter().skip(1) {
                            builder = builder.line_to(p.x, p.y);
                        }
                    }
                }

                // Connect to bottom line and draw it
                match self.curve {
                    Curve::Linear => {
                        for p in &bottom_points {
                            builder = builder.line_to(p.x, p.y);
                        }
                    }
                    _ => {
                        let curved = self.curve.interpolate(&bottom_points);
                        for p in &curved {
                            builder = builder.line_to(p.x, p.y);
                        }
                    }
                }

                builder = builder.close_path();
            }
        }

        builder.build()
    }

    /// Collect data into segments of defined points.
    fn collect_defined_segments<'a>(&self, data: &'a [T]) -> Vec<Vec<&'a T>> {
        let mut segments = Vec::new();
        let mut current = Vec::new();

        for d in data {
            if (self.defined)(d) {
                current.push(d);
            } else if !current.is_empty() {
                segments.push(current);
                current = Vec::new();
            }
        }

        if !current.is_empty() {
            segments.push(current);
        }

        segments
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
    fn test_area_empty() {
        let data: Vec<(f64, f64)> = vec![];
        let area = Area::new()
            .x(|d: &(f64, f64)| d.0)
            .y0(|_| 0.0)
            .y1(|d: &(f64, f64)| d.1);

        let path = area.generate(&data);
        assert!(path.is_empty());
    }
}
