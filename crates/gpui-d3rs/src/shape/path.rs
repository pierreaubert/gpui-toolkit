//! Path building utilities
//!
//! Provides an SVG-like path builder for creating complex shapes.

use std::f64::consts::PI;

mod flatten;
mod path_builder;
mod point;
#[cfg(test)]
mod tests;
mod types;

pub use path_builder::*;
pub use point::*;
pub use types::*;

use flatten::elliptical_arc_center;
use flatten::flatten_arc;
use flatten::flatten_cubic;
use flatten::flatten_elliptical_arc;
use flatten::flatten_quadratic;

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
        let mut current = Point::default();
        let mut subpath_start = Point::default();

        for cmd in &self.commands {
            match *cmd {
                PathCommand::MoveTo { x, y } => {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    current = Point::new(x, y);
                    subpath_start = current;
                }
                PathCommand::LineTo { x, y } => {
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    current = Point::new(x, y);
                }
                PathCommand::HorizontalLineTo { x } => {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(current.y);
                    max_y = max_y.max(current.y);
                    current.x = x;
                }
                PathCommand::VerticalLineTo { y } => {
                    min_x = min_x.min(current.x);
                    max_x = max_x.max(current.x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                    current.y = y;
                }
                PathCommand::ClosePath => current = subpath_start,
                PathCommand::QuadraticCurveTo { x1, y1, x, y } => {
                    // Approximate bounds with control point and endpoint
                    min_x = min_x.min(x1).min(x);
                    min_y = min_y.min(y1).min(y);
                    max_x = max_x.max(x1).max(x);
                    max_y = max_y.max(y1).max(y);
                    current = Point::new(x, y);
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
                    current = Point::new(x, y);
                }
                PathCommand::Arc {
                    x,
                    y,
                    radius,
                    start_angle,
                    end_angle,
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
                    current =
                        Point::new(x + radius * end_angle.cos(), y + radius * end_angle.sin());
                }
                PathCommand::EllipticalArc {
                    x,
                    y,
                    rx,
                    ry,
                    x_axis_rotation,
                    large_arc,
                    sweep,
                } => {
                    let end = Point::new(x, y);
                    let mut candidates = vec![current, end];
                    if let Some(arc) = elliptical_arc_center(
                        current,
                        end,
                        rx,
                        ry,
                        x_axis_rotation,
                        large_arc,
                        sweep,
                    ) {
                        let (sin_phi, cos_phi) = arc.phi.sin_cos();
                        let x_extreme = (-arc.ry * sin_phi).atan2(arc.rx * cos_phi);
                        let y_extreme = (arc.ry * cos_phi).atan2(arc.rx * sin_phi);
                        for angle in [x_extreme, x_extreme + PI, y_extreme, y_extreme + PI] {
                            if arc.contains_angle(angle) {
                                candidates.push(arc.point_at(angle));
                            }
                        }
                    }
                    for point in candidates {
                        min_x = min_x.min(point.x);
                        min_y = min_y.min(point.y);
                        max_x = max_x.max(point.x);
                        max_y = max_y.max(point.y);
                    }
                    current = end;
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
                PathCommand::EllipticalArc {
                    rx,
                    ry,
                    x_axis_rotation,
                    large_arc,
                    sweep,
                    x,
                    y,
                } => {
                    let end = Point::new(x, y);
                    flatten_elliptical_arc(
                        current,
                        end,
                        rx,
                        ry,
                        x_axis_rotation,
                        large_arc,
                        sweep,
                        tolerance,
                        &mut points,
                    );
                    current = end;
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

    /// Write the SVG path string representation into `buf`.
    pub fn write_svg_string(&self, buf: &mut String) {
        use std::fmt::Write;

        for cmd in &self.commands {
            if !buf.is_empty() {
                buf.push(' ');
            }

            match *cmd {
                PathCommand::MoveTo { x, y } => {
                    write!(buf, "M{},{}", x, y).unwrap();
                }
                PathCommand::LineTo { x, y } => {
                    write!(buf, "L{},{}", x, y).unwrap();
                }
                PathCommand::HorizontalLineTo { x } => {
                    write!(buf, "H{}", x).unwrap();
                }
                PathCommand::VerticalLineTo { y } => {
                    write!(buf, "V{}", y).unwrap();
                }
                PathCommand::ClosePath => {
                    buf.push('Z');
                }
                PathCommand::QuadraticCurveTo { x1, y1, x, y } => {
                    write!(buf, "Q{},{},{},{}", x1, y1, x, y).unwrap();
                }
                PathCommand::CubicCurveTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    write!(buf, "C{},{},{},{},{},{}", x1, y1, x2, y2, x, y).unwrap();
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
                    write!(
                        buf,
                        "M{},{} A{},{},0,{},{},{},{}",
                        x1,
                        y1,
                        radius,
                        radius,
                        if large_arc { 1 } else { 0 },
                        if sweep { 1 } else { 0 },
                        x2,
                        y2
                    )
                    .unwrap();
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
                    write!(
                        buf,
                        "A{},{},{},{},{},{},{}",
                        rx,
                        ry,
                        x_axis_rotation,
                        if large_arc { 1 } else { 0 },
                        if sweep { 1 } else { 0 },
                        x,
                        y
                    )
                    .unwrap();
                }
                PathCommand::Rect {
                    x,
                    y,
                    width,
                    height,
                } => {
                    write!(
                        buf,
                        "M{},{} L{},{} L{},{} L{},{} Z",
                        x,
                        y,
                        x + width,
                        y,
                        x + width,
                        y + height,
                        x,
                        y + height
                    )
                    .unwrap();
                }
            }
        }
    }

    /// Convert to SVG path string.
    pub fn to_svg_string(&self) -> String {
        let mut s = String::with_capacity(self.commands.len() * 24);
        self.write_svg_string(&mut s);
        s
    }
}
