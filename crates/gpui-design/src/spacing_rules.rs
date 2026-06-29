use serde::Serialize;

/// Spacing and density rules.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SpacingRules {
    /// Base grid unit in px. All spacing should be multiples of this.
    pub grid_unit: f32,
    /// Inline (horizontal) padding for controls: px.
    pub control_padding_x: f32,
    /// Block (vertical) padding for controls: px.
    pub control_padding_y: f32,
    /// Gap between controls in a group: px.
    pub control_gap: f32,
    /// Gap between groups/sections: px.
    pub section_gap: f32,
    /// Padding inside a card/panel: px.
    pub card_padding: f32,
}

impl SpacingRules {
    pub fn new(
        grid_unit: f32,
        control_padding_x: f32,
        control_padding_y: f32,
        control_gap: f32,
        section_gap: f32,
        card_padding: f32,
    ) -> Self {
        assert!(grid_unit >= 0.0, "grid_unit must be >= 0");
        assert!(control_padding_x >= 0.0, "control_padding_x must be >= 0");
        assert!(control_padding_y >= 0.0, "control_padding_y must be >= 0");
        assert!(control_gap >= 0.0, "control_gap must be >= 0");
        assert!(section_gap >= 0.0, "section_gap must be >= 0");
        assert!(card_padding >= 0.0, "card_padding must be >= 0");
        Self {
            grid_unit,
            control_padding_x,
            control_padding_y,
            control_gap,
            section_gap,
            card_padding,
        }
    }
}
