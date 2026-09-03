use super::types::CornerRadiusStyle;
use serde::{Deserialize, Serialize};

/// Corner radius values for different element sizes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CornerRadii {
    /// Small elements (badges, chips): px.
    pub sm: f32,
    /// Medium elements (buttons, inputs, controls): px.
    pub md: f32,
    /// Large elements (cards, panels): px.
    pub lg: f32,
    /// Extra-large / pill shape: px.
    pub xl: f32,
    /// Corner rendering style.
    pub style: CornerRadiusStyle,
    /// Whether asymmetric corners flip in RTL layouts.
    pub flip_in_rtl: bool,
}

impl CornerRadii {
    pub fn new(sm: f32, md: f32, lg: f32, xl: f32, style: CornerRadiusStyle) -> Self {
        assert!(sm >= 0.0, "sm must be >= 0");
        assert!(md >= 0.0, "md must be >= 0");
        assert!(lg >= 0.0, "lg must be >= 0");
        assert!(xl >= 0.0, "xl must be >= 0");
        Self {
            sm,
            md,
            lg,
            xl,
            style,
            flip_in_rtl: true,
        }
    }
}
