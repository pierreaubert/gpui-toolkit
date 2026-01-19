//! Axis orientation types

/// Axis orientation determines where ticks and labels appear
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisOrientation {
    /// Ticks above the axis line, labels above ticks
    Top,
    /// Ticks to the right of the axis line, labels to the right
    Right,
    /// Ticks below the axis line, labels below ticks
    Bottom,
    /// Ticks to the left of the axis line, labels to the left
    Left,
}

impl AxisOrientation {
    /// Check if the orientation is horizontal
    pub fn is_horizontal(&self) -> bool {
        matches!(self, AxisOrientation::Top | AxisOrientation::Bottom)
    }

    /// Check if the orientation is vertical
    pub fn is_vertical(&self) -> bool {
        !self.is_horizontal()
    }
}
