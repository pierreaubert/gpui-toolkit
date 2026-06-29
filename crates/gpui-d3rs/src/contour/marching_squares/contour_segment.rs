use crate::shape::path::Point;

/// A single independent contour line segment at a specific threshold value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContourSegment {
    /// The threshold value for this contour segment
    pub value: f64,
    /// Start point in output coordinates
    pub start: Point,
    /// End point in output coordinates
    pub end: Point,
}

impl ContourSegment {
    /// Create a new contour segment.
    pub fn new(value: f64, start: Point, end: Point) -> Self {
        Self { value, start, end }
    }
}
