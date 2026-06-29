use super::background_colors::BackgroundColors;
use super::border_colors::BorderColors;
use super::semantic_colors::SemanticColors;
use super::text_colors::TextColors;

/// Complete color palette combining all token categories
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ColorPalette {
    /// Semantic colors (primary, success, warning, error, info)
    pub semantic: SemanticColors,
    /// Background colors
    pub backgrounds: BackgroundColors,
    /// Text colors
    pub text: TextColors,
    /// Border colors
    pub borders: BorderColors,
}

impl ColorPalette {
    /// Create a dark theme color palette
    pub fn dark() -> Self {
        Self {
            semantic: SemanticColors::dark(),
            backgrounds: BackgroundColors::dark(),
            text: TextColors::dark(),
            borders: BorderColors::dark(),
        }
    }

    /// Create a light theme color palette
    pub fn light() -> Self {
        Self {
            semantic: SemanticColors::light(),
            backgrounds: BackgroundColors::light(),
            text: TextColors::light(),
            borders: BorderColors::light(),
        }
    }
}
