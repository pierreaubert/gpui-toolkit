//! Path building utilities
//!
//! Provides an SVG-like path builder for creating complex shapes.

use std::f64::consts::PI;

/// A path command representing a single drawing operation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    /// Move to a point without drawing
    MoveTo { x: f64, y: f64 },
    /// Draw a line to a point
    LineTo { x: f64, y: f64 },
    /// Draw a horizontal line
    HorizontalLineTo { x: f64 },
    /// Draw a vertical line
    VerticalLineTo { y: f64 },
    /// Close the current path
    ClosePath,
    /// Quadratic Bezier curve
    QuadraticCurveTo { x1: f64, y1: f64, x: f64, y: f64 },
    /// Cubic Bezier curve
    CubicCurveTo {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    /// Arc (elliptical arc)
    Arc {
        x: f64,
        y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        anticlockwise: bool,
    },
    /// Elliptical arc (SVG-style)
    EllipticalArc {
        rx: f64,
        ry: f64,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        x: f64,
        y: f64,
    },
    /// Rectangle
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
}

/// A 2D point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    /// Create a new point.
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Distance to another point.
    pub fn distance(&self, other: &Point) -> f64 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Linear interpolation between two points.
    pub fn lerp(&self, other: &Point, t: f64) -> Point {
        Point {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }
}

/// Path builder for creating SVG-like paths.
///
/// # Example
///
/// ```
/// use d3rs::shape::path::PathBuilder;
///
/// let path = PathBuilder::new()
///     .move_to(0.0, 0.0)
///     .line_to(100.0, 0.0)
///     .line_to(100.0, 100.0)
///     .close_path()
///     .build();
///
/// assert_eq!(path.commands().len(), 4);
/// ```
#[derive(Debug, Clone, Default)]
pub struct PathBuilder {
    commands: Vec<PathCommand>,
    current_point: Point,
    start_point: Point,
}

impl PathBuilder {
    /// Create a new path builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Move to a point without drawing.
    pub fn move_to(mut self, x: f64, y: f64) -> Self {
        self.commands.push(PathCommand::MoveTo { x, y });
        self.current_point = Point::new(x, y);
        self.start_point = self.current_point;
        self
    }

    /// Draw a line to a point.
    pub fn line_to(mut self, x: f64, y: f64) -> Self {
        self.commands.push(PathCommand::LineTo { x, y });
        self.current_point = Point::new(x, y);
        self
    }

    /// Draw a horizontal line.
    pub fn horizontal_line_to(mut self, x: f64) -> Self {
        self.commands.push(PathCommand::HorizontalLineTo { x });
        self.current_point.x = x;
        self
    }

    /// Draw a vertical line.
    pub fn vertical_line_to(mut self, y: f64) -> Self {
        self.commands.push(PathCommand::VerticalLineTo { y });
        self.current_point.y = y;
        self
    }

    /// Close the current path.
    pub fn close_path(mut self) -> Self {
        self.commands.push(PathCommand::ClosePath);
        self.current_point = self.start_point;
        self
    }

    /// Draw a quadratic Bezier curve.
    pub fn quadratic_curve_to(mut self, x1: f64, y1: f64, x: f64, y: f64) -> Self {
        self.commands
            .push(PathCommand::QuadraticCurveTo { x1, y1, x, y });
        self.current_point = Point::new(x, y);
        self
    }

    /// Draw a cubic Bezier curve.
    pub fn cubic_curve_to(mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> Self {
        self.commands.push(PathCommand::CubicCurveTo {
            x1,
            y1,
            x2,
            y2,
            x,
            y,
        });
        self.current_point = Point::new(x, y);
        self
    }

    /// Draw an arc (Canvas-style).
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate of the arc center
    /// * `y` - Y coordinate of the arc center
    /// * `radius` - Arc radius
    /// * `start_angle` - Start angle in radians
    /// * `end_angle` - End angle in radians
    /// * `anticlockwise` - Draw anticlockwise if true
    pub fn arc(
        mut self,
        x: f64,
        y: f64,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        anticlockwise: bool,
    ) -> Self {
        self.commands.push(PathCommand::Arc {
            x,
            y,
            radius,
            start_angle,
            end_angle,
            anticlockwise,
        });

        // Update current point to end of arc
        self.current_point = Point::new(x + radius * end_angle.cos(), y + radius * end_angle.sin());
        self
    }

    /// Draw an elliptical arc (SVG-style).
    pub fn elliptical_arc(
        mut self,
        rx: f64,
        ry: f64,
        x_axis_rotation: f64,
        large_arc: bool,
        sweep: bool,
        x: f64,
        y: f64,
    ) -> Self {
        self.commands.push(PathCommand::EllipticalArc {
            rx,
            ry,
            x_axis_rotation,
            large_arc,
            sweep,
            x,
            y,
        });
        self.current_point = Point::new(x, y);
        self
    }

    /// Draw a rectangle.
    pub fn rect(mut self, x: f64, y: f64, width: f64, height: f64) -> Self {
        self.commands.push(PathCommand::Rect {
            x,
            y,
            width,
            height,
        });
        self.current_point = Point::new(x, y);
        self
    }

    /// Build the path.
    pub fn build(self) -> Path {
        Path {
            commands: self.commands,
        }
    }

    /// Get current point.
    pub fn current_point(&self) -> Point {
        self.current_point
    }
}

/// A path consisting of drawing commands.
#[derive(Debug, Clone, Default)]
pub struct Path {
    commands: Vec<PathCommand>,
}

impl Path {
    /// Create a new empty path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the commands in this path.
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// Check if the path is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get the bounding box of this path.
    ///
    /// Returns (min_x, min_y, max_x, max_y).
    pub fn bounds(&self) -> Option<(f64, f64, f64, f64)> {
        if self.commands.is_empty() {
            return None;
        }

        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        for cmd in &self.commands {
            match *cmd {
                PathCommand::MoveTo { x, y } | PathCommand::LineTo { x, y } => {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
                PathCommand::HorizontalLineTo { x } => {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                }
                PathCommand::VerticalLineTo { y } => {
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
                PathCommand::ClosePath => {}
                PathCommand::QuadraticCurveTo { x1, y1, x, y } => {
                    // Approximate bounds with control point and endpoint
                    min_x = min_x.min(x1).min(x);
                    min_y = min_y.min(y1).min(y);
                    max_x = max_x.max(x1).max(x);
                    max_y = max_y.max(y1).max(y);
                }
                PathCommand::CubicCurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    // Approximate bounds with control points and endpoint
                    min_x = min_x.min(x1).min(x2).min(x);
                    min_y = min_y.min(y1).min(y2).min(y);
                    max_x = max_x.max(x1).max(x2).max(x);
                    max_y = max_y.max(y1).max(y2).max(y);
                }
                PathCommand::Arc {
                    x,
                    y,
                    radius,
                    start_angle,
                    ..
                } => {
                    // Approximate with bounding circle
                    min_x = min_x.min(x - radius);
                    min_y = min_y.min(y - radius);
                    max_x = max_x.max(x + radius);
                    max_y = max_y.max(y + radius);
                    // Update with start point too
                    let start_x = x + radius * start_angle.cos();
                    let start_y = y + radius * start_angle.sin();
                    min_x = min_x.min(start_x);
                    min_y = min_y.min(start_y);
                    max_x = max_x.max(start_x);
                    max_y = max_y.max(start_y);
                }
                PathCommand::EllipticalArc { x, y, rx, ry, .. } => {
                    // Conservative bounds
                    min_x = min_x.min(x - rx);
                    min_y = min_y.min(y - ry);
                    max_x = max_x.max(x + rx);
                    max_y = max_y.max(y + ry);
                }
                PathCommand::Rect {
                    x,
                    y,
                    width,
                    height,
                } => {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x + width);
                    max_y = max_y.max(y + height);
                }
            }
        }

        Some((min_x, min_y, max_x, max_y))
    }

    /// Flatten the path into line segments.
    ///
    /// Converts curves and arcs into sequences of line segments
    /// for rendering or hit testing.
    ///
    /// # Arguments
    ///
    /// * `tolerance` - Maximum distance between curve and approximation
    pub fn flatten(&self, tolerance: f64) -> Vec<Point> {
        let mut points = Vec::new();
        let mut current = Point::default();
        let mut start = Point::default();

        for cmd in &self.commands {
            match *cmd {
                PathCommand::MoveTo { x, y } => {
                    current = Point::new(x, y);
                    start = current;
                    points.push(current);
                }
                PathCommand::LineTo { x, y } => {
                    current = Point::new(x, y);
                    points.push(current);
                }
                PathCommand::HorizontalLineTo { x } => {
                    current.x = x;
                    points.push(current);
                }
                PathCommand::VerticalLineTo { y } => {
                    current.y = y;
                    points.push(current);
                }
                PathCommand::ClosePath => {
                    if current.distance(&start) > tolerance {
                        points.push(start);
                    }
                    current = start;
                }
                PathCommand::QuadraticCurveTo { x1, y1, x, y } => {
                    let control = Point::new(x1, y1);
                    let end = Point::new(x, y);
                    flatten_quadratic(&current, &control, &end, tolerance, &mut points);
                    current = end;
                }
                PathCommand::CubicCurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    let c1 = Point::new(x1, y1);
                    let c2 = Point::new(x2, y2);
                    let end = Point::new(x, y);
                    flatten_cubic(&current, &c1, &c2, &end, tolerance, &mut points);
                    current = end;
                }
                PathCommand::Arc {
                    x,
                    y,
                    radius,
                    start_angle,
                    end_angle,
                    anticlockwise,
                } => {
                    flatten_arc(
                        x,
                        y,
                        radius,
                        start_angle,
                        end_angle,
                        anticlockwise,
                        tolerance,
                        &mut points,
                    );
                    current =
                        Point::new(x + radius * end_angle.cos(), y + radius * end_angle.sin());
                }
                PathCommand::EllipticalArc { x, y, .. } => {
                    // For now, just add the endpoint
                    // Full implementation would convert to arc segments
                    current = Point::new(x, y);
                    points.push(current);
                }
                PathCommand::Rect {
                    x,
                    y,
                    width,
                    height,
                } => {
                    points.push(Point::new(x, y));
                    points.push(Point::new(x + width, y));
                    points.push(Point::new(x + width, y + height));
                    points.push(Point::new(x, y + height));
                    points.push(Point::new(x, y));
                    current = Point::new(x, y);
                }
            }
        }

        points
    }

    /// Convert to SVG path string.
    pub fn to_svg_string(&self) -> String {
        let mut s = String::new();

        for cmd in &self.commands {
            if !s.is_empty() {
                s.push(' ');
            }

            match *cmd {
                PathCommand::MoveTo { x, y } => {
                    s.push_str(&format!("M{},{}", x, y));
                }
                PathCommand::LineTo { x, y } => {
                    s.push_str(&format!("L{},{}", x, y));
                }
                PathCommand::HorizontalLineTo { x } => {
                    s.push_str(&format!("H{}", x));
                }
                PathCommand::VerticalLineTo { y } => {
                    s.push_str(&format!("V{}", y));
                }
                PathCommand::ClosePath => {
                    s.push('Z');
                }
                PathCommand::QuadraticCurveTo { x1, y1, x, y } => {
                    s.push_str(&format!("Q{},{},{},{}", x1, y1, x, y));
                }
                PathCommand::CubicCurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    s.push_str(&format!("C{},{},{},{},{},{}", x1, y1, x2, y2, x, y));
                }
                PathCommand::Arc {
                    x,
                    y,
                    radius,
                    start_angle,
                    end_angle,
                    ..
                } => {
                    // Convert to SVG arc format
                    let x1 = x + radius * start_angle.cos();
                    let y1 = y + radius * start_angle.sin();
                    let x2 = x + radius * end_angle.cos();
                    let y2 = y + radius * end_angle.sin();
                    let large_arc = (end_angle - start_angle).abs() > PI;
                    let sweep = end_angle > start_angle;
                    s.push_str(&format!(
                        "M{},{} A{},{},0,{},{},{},{}",
                        x1,
                        y1,
                        radius,
                        radius,
                        if large_arc { 1 } else { 0 },
                        if sweep { 1 } else { 0 },
                        x2,
                        y2
                    ));
                }
                PathCommand::EllipticalArc {
                    rx,
                    ry,
                    x_axis_rotation,
                    large_arc,
                    sweep,
                    x,
                    y,
                } => {
                    s.push_str(&format!(
                        "A{},{},{},{},{},{},{}",
                        rx,
                        ry,
                        x_axis_rotation,
                        if large_arc { 1 } else { 0 },
                        if sweep { 1 } else { 0 },
                        x,
                        y
                    ));
                }
                PathCommand::Rect {
                    x,
                    y,
                    width,
                    height,
                } => {
                    s.push_str(&format!(
                        "M{},{} L{},{} L{},{} L{},{} Z",
                        x,
                        y,
                        x + width,
                        y,
                        x + width,
                        y + height,
                        x,
                        y + height
                    ));
                }
            }
        }

        s
    }
}

/// Flatten a quadratic Bezier curve into line segments.
fn flatten_quadratic(p0: &Point, p1: &Point, p2: &Point, tolerance: f64, points: &mut Vec<Point>) {
    // Check if the curve is flat enough
    let mid = p0.lerp(p2, 0.5);
    let control_dist = mid.distance(p1);

    if control_dist < tolerance {
        points.push(*p2);
    } else {
        // Subdivide
        let p01 = p0.lerp(p1, 0.5);
        let p12 = p1.lerp(p2, 0.5);
        let p012 = p01.lerp(&p12, 0.5);

        flatten_quadratic(p0, &p01, &p012, tolerance, points);
        flatten_quadratic(&p012, &p12, p2, tolerance, points);
    }
}

/// Flatten a cubic Bezier curve into line segments.
fn flatten_cubic(
    p0: &Point,
    p1: &Point,
    p2: &Point,
    p3: &Point,
    tolerance: f64,
    points: &mut Vec<Point>,
) {
    // Check if the curve is flat enough using de Casteljau subdivision
    let d1 = distance_to_line(p1, p0, p3);
    let d2 = distance_to_line(p2, p0, p3);

    if d1 + d2 < tolerance {
        points.push(*p3);
    } else {
        // Subdivide
        let p01 = p0.lerp(p1, 0.5);
        let p12 = p1.lerp(p2, 0.5);
        let p23 = p2.lerp(p3, 0.5);
        let p012 = p01.lerp(&p12, 0.5);
        let p123 = p12.lerp(&p23, 0.5);
        let p0123 = p012.lerp(&p123, 0.5);

        flatten_cubic(p0, &p01, &p012, &p0123, tolerance, points);
        flatten_cubic(&p0123, &p123, &p23, p3, tolerance, points);
    }
}

/// Calculate distance from point to line.
fn distance_to_line(point: &Point, line_start: &Point, line_end: &Point) -> f64 {
    let dx = line_end.x - line_start.x;
    let dy = line_end.y - line_start.y;
    let length_sq = dx * dx + dy * dy;

    if length_sq < 1e-10 {
        return point.distance(line_start);
    }

    let cross = (point.x - line_start.x) * dy - (point.y - line_start.y) * dx;
    cross.abs() / length_sq.sqrt()
}

/// Flatten an arc into line segments.
fn flatten_arc(
    cx: f64,
    cy: f64,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    anticlockwise: bool,
    tolerance: f64,
    points: &mut Vec<Point>,
) {
    // Calculate number of segments needed
    let mut delta = end_angle - start_angle;

    if anticlockwise {
        if delta > 0.0 {
            delta -= 2.0 * PI;
        }
    } else if delta < 0.0 {
        delta += 2.0 * PI;
    }

    let n = ((delta.abs() * radius / tolerance).sqrt().ceil() as usize).max(1);

    for i in 1..=n {
        let t = i as f64 / n as f64;
        let angle = start_angle + delta * t;
        points.push(Point::new(
            cx + radius * angle.cos(),
            cy + radius * angle.sin(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_builder() {
        let path = PathBuilder::new()
            .move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .line_to(100.0, 100.0)
            .line_to(0.0, 100.0)
            .close_path()
            .build();

        assert_eq!(path.commands().len(), 5);
    }

    #[test]
    fn test_path_bounds() {
        let path = PathBuilder::new()
            .move_to(10.0, 20.0)
            .line_to(50.0, 20.0)
            .line_to(50.0, 80.0)
            .line_to(10.0, 80.0)
            .close_path()
            .build();

        let bounds = path.bounds().unwrap();
        assert_eq!(bounds, (10.0, 20.0, 50.0, 80.0));
    }

    #[test]
    fn test_path_flatten() {
        let path = PathBuilder::new()
            .move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .line_to(100.0, 100.0)
            .build();

        let points = path.flatten(1.0);
        assert_eq!(points.len(), 3);
    }

    #[test]
    fn test_path_to_svg() {
        let path = PathBuilder::new()
            .move_to(0.0, 0.0)
            .line_to(100.0, 0.0)
            .close_path()
            .build();

        let svg = path.to_svg_string();
        assert!(svg.contains("M0,0"));
        assert!(svg.contains("L100,0"));
        assert!(svg.contains('Z'));
    }

    #[test]
    fn test_point_distance() {
        let p1 = Point::new(0.0, 0.0);
        let p2 = Point::new(3.0, 4.0);
        assert!((p1.distance(&p2) - 5.0).abs() < 1e-10);
    }
}
