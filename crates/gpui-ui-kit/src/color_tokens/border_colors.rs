use super::color_token::ColorToken;

/// Border color tokens
#[derive(Debug, Clone, PartialEq)]
pub struct BorderColors {
    /// Default border
    pub default: ColorToken,
    /// Focus/active border
    pub focus: ColorToken,
    /// Error state border
    pub error: ColorToken,
}

impl BorderColors {
    /// Create dark theme border colors
    pub fn dark() -> Self {
        Self {
            default: ColorToken::from_hex(0x3a3a3a),
            focus: ColorToken::from_hex(0x007acc),
            error: ColorToken::from_hex(0xef4444),
        }
    }

    /// Create light theme border colors
    pub fn light() -> Self {
        Self {
            default: ColorToken::from_hex(0xd4d4d4),
            focus: ColorToken::from_hex(0x0066cc),
            error: ColorToken::from_hex(0xdc2626),
        }
    }
}

impl Default for BorderColors {
    fn default() -> Self {
        Self::dark()
    }
}
