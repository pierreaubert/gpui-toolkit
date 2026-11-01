use d3rs::color::D3Color;
use d3rs::shape::{LinePoint, StrokeDashArray};

/// A single curve to be rendered on the frequency/SPL plot
pub struct PlotCurve {
    /// Data points as (frequency, value) pairs
    pub points: Vec<LinePoint>,
    /// Curve color
    pub color: D3Color,
    /// Stroke width
    pub stroke_width: f32,
    /// Optional dash pattern for this curve
    pub dash_array: Option<StrokeDashArray>,
    /// Whether this curve uses the secondary (right) Y-axis
    pub use_secondary_axis: bool,
    /// Optional fill under the curve: (colour, opacity).
    pub fill: Option<(D3Color, f32)>,
    /// Trace name for legend ordering / toggling.
    pub name: String,
}

impl PlotCurve {
    pub fn new(points: Vec<LinePoint>, color: D3Color) -> Self {
        Self {
            points,
            color,
            stroke_width: 2.0,
            dash_array: None,
            use_secondary_axis: false,
            fill: None,
            name: String::new(),
        }
    }

    #[allow(dead_code, reason = "builder API for filled zone curves")]
    pub fn fill(mut self, color: D3Color, opacity: f32) -> Self {
        self.fill = Some((color, opacity.clamp(0.0, 1.0)));
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    pub fn dash_array(mut self, pattern: StrokeDashArray) -> Self {
        self.dash_array = Some(pattern);
        self
    }

    pub fn secondary_axis(mut self) -> Self {
        self.use_secondary_axis = true;
        self
    }
}
