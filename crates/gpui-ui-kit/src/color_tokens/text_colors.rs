use super::color_token::ColorToken;

/// Text color tokens for consistent typography
#[derive(Debug, Clone, PartialEq)]
pub struct TextColors {
    /// Primary text (headings, important content)
    pub primary: ColorToken,
    /// Secondary text (body, less important)
    pub secondary: ColorToken,
    /// Muted text (placeholders, disabled)
    pub muted: ColorToken,
    /// Inverted text (for use on dark/accent backgrounds)
    pub inverted: ColorToken,
}

impl TextColors {
    /// Create dark theme text colors
    pub fn dark() -> Self {
        Self {
            primary: ColorToken::from_hex(0xffffff),
            secondary: ColorToken::from_hex(0xcccccc),
            muted: ColorToken::from_hex(0x888888),
            inverted: ColorToken::from_hex(0x1a1a1a),
        }
    }

    /// Create light theme text colors
    pub fn light() -> Self {
        Self {
            primary: ColorToken::from_hex(0x1a1a1a),
            secondary: ColorToken::from_hex(0x4a4a4a),
            muted: ColorToken::from_hex(0x888888),
            inverted: ColorToken::from_hex(0xffffff),
        }
    }
}

impl Default for TextColors {
    fn default() -> Self {
        Self::dark()
    }
}
