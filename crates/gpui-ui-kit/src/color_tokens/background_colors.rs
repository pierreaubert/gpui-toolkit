use super::color_token::ColorToken;
use gpui::rgb;

/// Background color tokens for consistent surface styling
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundColors {
    /// Main page background
    pub page: ColorToken,
    /// Card/elevated surface background
    pub surface: ColorToken,
    /// Overlay/modal backdrop
    pub overlay: ColorToken,
}

impl BackgroundColors {
    /// Create dark theme background colors
    pub fn dark() -> Self {
        Self {
            page: ColorToken::from_hex(0x1e1e1e),
            surface: ColorToken::from_hex(0x2a2a2a),
            overlay: ColorToken::from_base_with_alpha(rgb(0x000000), 0.5),
        }
    }

    /// Create light theme background colors
    pub fn light() -> Self {
        Self {
            page: ColorToken::from_hex(0xf5f5f5),
            surface: ColorToken::from_hex(0xffffff),
            overlay: ColorToken::from_base_with_alpha(rgb(0x000000), 0.5),
        }
    }
}

impl Default for BackgroundColors {
    fn default() -> Self {
        Self::dark()
    }
}
