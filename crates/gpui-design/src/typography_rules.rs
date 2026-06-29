use serde::Serialize;
use std::borrow::Cow;

/// Typography rules.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypographyRules {
    /// Preferred font family name.
    pub font_family: Cow<'static, str>,
    /// Whether to use dynamic type sizes (Apple) or fixed scale.
    pub dynamic_sizing: bool,
    /// Base font size in px.
    pub base_size: f32,
    /// Small text size in px (labels, captions).
    pub small_size: f32,
    /// Large text size in px (headers, titles).
    pub large_size: f32,
}

impl TypographyRules {
    pub fn new(
        font_family: impl Into<Cow<'static, str>>,
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
        }
    }
}
