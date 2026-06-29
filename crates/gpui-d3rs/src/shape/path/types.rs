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
