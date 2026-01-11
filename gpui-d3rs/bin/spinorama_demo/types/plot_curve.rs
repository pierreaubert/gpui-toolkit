use d3rs::color::D3Color;
use d3rs::shape::LinePoint;

/// A single curve to be rendered on the frequency/SPL plot
pub struct PlotCurve {
    /// Data points as (frequency, value) pairs
    pub points: Vec<LinePoint>,
    /// Curve color
    pub color: D3Color,
    /// Stroke width
    pub stroke_width: f32,
    /// Whether this curve uses the secondary (right) Y-axis
    pub use_secondary_axis: bool,
}

impl PlotCurve {
    pub fn new(points: Vec<LinePoint>, color: D3Color) -> Self {
        Self {
            points,
            color,
            stroke_width: 2.0,
            use_secondary_axis: false,
        }
    }

    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    pub fn secondary_axis(mut self) -> Self {
        self.use_secondary_axis = true;
        self
    }
}
