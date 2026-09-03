use serde::{Deserialize, Serialize};

/// Typography rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypographyRules {
    /// Preferred font family name.
    pub font_family: String,
    /// Whether to use dynamic type sizes (Apple) or fixed scale.
    pub dynamic_sizing: bool,
    /// Base font size in px.
    pub base_size: f32,
    /// Small text size in px (labels, captions).
    pub small_size: f32,
    /// Large text size in px (headers, titles).
    pub large_size: f32,
    /// Fluid-type lower bound in px (`clamp()` min).
    pub fluid_min_size: f32,
    /// Fluid-type upper bound in px (`clamp()` max).
    pub fluid_max_size: f32,
}

impl TypographyRules {
    pub fn new(
        font_family: impl Into<String>,
        dynamic_sizing: bool,
        base_size: f32,
        small_size: f32,
        large_size: f32,
    ) -> Self {
        assert!(base_size > 0.0, "base_size must be > 0");
        assert!(small_size > 0.0, "small_size must be > 0");
        assert!(large_size > 0.0, "large_size must be > 0");
        Self {
            font_family: font_family.into(),
            dynamic_sizing,
            base_size,
            small_size,
            large_size,
            fluid_min_size: small_size.min(base_size),
            fluid_max_size: large_size.max(base_size),
        }
    }

    /// CSS `clamp()` expression for fluid type between the fluid bounds.
    pub fn fluid_clamp_css(&self) -> String {
        format!(
            "clamp({}px, {}px + 1vw, {}px)",
            self.fluid_min_size, self.base_size, self.fluid_max_size
        )
    }
}
