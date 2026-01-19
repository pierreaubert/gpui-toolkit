//! Color Token System
//!
//! Provides semantic color tokens with automatic variant generation.
//! This system makes it easy to create consistent color palettes with
//! hover, active, muted, and subtle variants derived from a base color.
//!
//! # Usage
//!
//! ```ignore
//! use gpui_ui_kit::color_tokens::{ColorToken, SemanticColors};
//!
//! // Create a color token from a base color
//! let primary = ColorToken::from_base(rgb(0x007acc));
//!
//! // Use the variants
//! element
//!     .bg(primary.base)
//!     .hover(|s| s.bg(primary.hover))
//!     .active(|s| s.bg(primary.active));
//!
//! // Or create a full semantic color palette
//! let colors = SemanticColors::default();
//! element.bg(colors.success.base);
//! ```
//!
//! # Color Variants
//!
//! Each `ColorToken` provides:
//! - `base`: The original color
//! - `hover`: Slightly lighter/more saturated for hover states
//! - `active`: Slightly darker for pressed/active states
//! - `muted`: Low opacity version for backgrounds
//! - `subtle`: Very low opacity for subtle highlights

use gpui::{Hsla, Rgba, rgb, rgba};

/// A color token with derived variants for different states.
///
/// All variants are automatically computed from the base color,
/// ensuring consistent relationships across your color palette.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorToken {
    /// The base color
    pub base: Rgba,
    /// Hover state - slightly lighter and more saturated
    pub hover: Rgba,
    /// Active/pressed state - slightly darker
    pub active: Rgba,
    /// Muted version - 20% opacity for subtle backgrounds
    pub muted: Rgba,
    /// Subtle version - 10% opacity for very light highlights
    pub subtle: Rgba,
}

impl ColorToken {
    /// Create a new color token from a base RGBA color.
    ///
    /// Automatically computes hover, active, muted, and subtle variants.
    pub fn from_base(base: Rgba) -> Self {
        let hsla = Hsla::from(base);

        // Hover: increase lightness slightly
        let hover_hsla = Hsla {
            h: hsla.h,
            s: (hsla.s * 1.1).min(1.0),  // Slightly more saturated
            l: (hsla.l * 1.1).min(0.95), // Slightly lighter
            a: hsla.a,
        };

        // Active: decrease lightness
        let active_hsla = Hsla {
            h: hsla.h,
            s: hsla.s,
            l: hsla.l * 0.85, // Slightly darker
            a: hsla.a,
        };

        // Muted: 20% opacity
        let muted_hsla = Hsla {
            h: hsla.h,
            s: hsla.s,
            l: hsla.l,
            a: 0.2,
        };

        // Subtle: 10% opacity
        let subtle_hsla = Hsla {
            h: hsla.h,
            s: hsla.s,
            l: hsla.l,
            a: 0.1,
        };

        Self {
            base,
            hover: hover_hsla.into(),
            active: active_hsla.into(),
            muted: muted_hsla.into(),
            subtle: subtle_hsla.into(),
        }
    }

    /// Create a color token from a hex RGB value (e.g., 0x007acc)
    pub fn from_hex(hex: u32) -> Self {
        Self::from_base(rgb(hex))
    }

    /// Create a color token from a hex RGBA value (e.g., 0x007accff)
    pub fn from_hex_alpha(hex: u32) -> Self {
        Self::from_base(rgba(hex))
    }

    /// Create a color token with custom alpha for the base color
    pub fn from_base_with_alpha(base: Rgba, alpha: f32) -> Self {
        let base_with_alpha = Rgba {
            r: base.r,
            g: base.g,
            b: base.b,
            a: alpha,
        };
        Self::from_base(base_with_alpha)
    }

    /// Get a version of this token with a different base alpha
    pub fn with_alpha(self, alpha: f32) -> Self {
        let base = Rgba {
            r: self.base.r,
            g: self.base.g,
            b: self.base.b,
            a: alpha,
        };
        Self::from_base(base)
    }

    /// Get a lighter version of this token
    pub fn lighter(self, amount: f32) -> Self {
        let hsla = Hsla::from(self.base);
        let lighter = Hsla {
            h: hsla.h,
            s: hsla.s,
            l: (hsla.l + amount).min(1.0),
            a: hsla.a,
        };
        Self::from_base(lighter.into())
    }

    /// Get a darker version of this token
    pub fn darker(self, amount: f32) -> Self {
        let hsla = Hsla::from(self.base);
        let darker = Hsla {
            h: hsla.h,
            s: hsla.s,
            l: (hsla.l - amount).max(0.0),
            a: hsla.a,
        };
        Self::from_base(darker.into())
    }
}

impl Default for ColorToken {
    fn default() -> Self {
        Self::from_hex(0x007acc) // Default blue accent
    }
}

impl From<Rgba> for ColorToken {
    fn from(color: Rgba) -> Self {
        Self::from_base(color)
    }
}

impl From<u32> for ColorToken {
    fn from(hex: u32) -> Self {
        Self::from_hex(hex)
    }
}

/// Semantic color tokens for consistent UI theming.
///
/// Provides a complete palette of colors with semantic meaning,
/// each with full variant support (hover, active, muted, subtle).
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticColors {
    /// Primary brand color (typically accent/interactive elements)
    pub primary: ColorToken,
    /// Secondary color (less prominent interactive elements)
    pub secondary: ColorToken,
    /// Success state color (confirmations, positive actions)
    pub success: ColorToken,
    /// Warning state color (caution, potential issues)
    pub warning: ColorToken,
    /// Error state color (errors, destructive actions)
    pub error: ColorToken,
    /// Info state color (informational messages)
    pub info: ColorToken,
}

impl SemanticColors {
    /// Create semantic colors with custom base colors
    pub fn new(
        primary: impl Into<ColorToken>,
        secondary: impl Into<ColorToken>,
        success: impl Into<ColorToken>,
        warning: impl Into<ColorToken>,
        error: impl Into<ColorToken>,
        info: impl Into<ColorToken>,
    ) -> Self {
        Self {
            primary: primary.into(),
            secondary: secondary.into(),
            success: success.into(),
            warning: warning.into(),
            error: error.into(),
            info: info.into(),
        }
    }

    /// Create a dark theme semantic color palette
    pub fn dark() -> Self {
        Self {
            primary: ColorToken::from_hex(0x007acc),
            secondary: ColorToken::from_hex(0x6c757d),
            success: ColorToken::from_hex(0x22c55e),
            warning: ColorToken::from_hex(0xf59e0b),
            error: ColorToken::from_hex(0xef4444),
            info: ColorToken::from_hex(0x3b82f6),
        }
    }

    /// Create a light theme semantic color palette
    pub fn light() -> Self {
        Self {
            primary: ColorToken::from_hex(0x0066cc),
            secondary: ColorToken::from_hex(0x6c757d),
            success: ColorToken::from_hex(0x16a34a),
            warning: ColorToken::from_hex(0xd97706),
            error: ColorToken::from_hex(0xdc2626),
            info: ColorToken::from_hex(0x2563eb),
        }
    }
}

impl Default for SemanticColors {
    fn default() -> Self {
        Self::dark()
    }
}

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

/// Helper function to create a muted version of a color
pub fn with_alpha(color: Rgba, alpha: f32) -> Rgba {
    Rgba {
        r: color.r,
        g: color.g,
        b: color.b,
        a: alpha,
    }
}

/// Helper function to lighten a color
pub fn lighten(color: Rgba, amount: f32) -> Rgba {
    let hsla = Hsla::from(color);
    Hsla {
        h: hsla.h,
        s: hsla.s,
        l: (hsla.l + amount).min(1.0),
        a: hsla.a,
    }
    .into()
}

/// Helper function to darken a color
pub fn darken(color: Rgba, amount: f32) -> Rgba {
    let hsla = Hsla::from(color);
    Hsla {
        h: hsla.h,
        s: hsla.s,
        l: (hsla.l - amount).max(0.0),
        a: hsla.a,
    }
    .into()
}

/// Helper function to adjust saturation
pub fn saturate(color: Rgba, amount: f32) -> Rgba {
    let hsla = Hsla::from(color);
    Hsla {
        h: hsla.h,
        s: (hsla.s + amount).clamp(0.0, 1.0),
        l: hsla.l,
        a: hsla.a,
    }
    .into()
}

/// Helper function to desaturate a color
pub fn desaturate(color: Rgba, amount: f32) -> Rgba {
    saturate(color, -amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_token_from_hex() {
        let token = ColorToken::from_hex(0x007acc);
        assert_eq!(token.base.r, 0.0);
        assert!(token.base.g > 0.4 && token.base.g < 0.5); // ~0.478
        assert!(token.base.b > 0.75 && token.base.b < 0.85); // ~0.8
    }

    #[test]
    fn test_color_token_variants_different() {
        let token = ColorToken::from_hex(0x007acc);
        // Variants should be different from base
        assert_ne!(token.base, token.hover);
        assert_ne!(token.base, token.active);
        assert_ne!(token.base, token.muted);
        assert_ne!(token.base, token.subtle);
    }

    #[test]
    fn test_muted_has_low_alpha() {
        let token = ColorToken::from_hex(0x007acc);
        assert!(token.muted.a < 0.3); // Should be ~0.2
        assert!(token.subtle.a < 0.15); // Should be ~0.1
    }

    #[test]
    fn test_semantic_colors_default() {
        let colors = SemanticColors::default();
        // Primary should be blue-ish
        assert!(colors.primary.base.b > colors.primary.base.r);
        // Success should be green-ish
        assert!(colors.success.base.g > colors.success.base.r);
        // Error should be red-ish
        assert!(colors.error.base.r > colors.error.base.g);
    }

    #[test]
    fn test_color_palette_dark_and_light_differ() {
        let dark = ColorPalette::dark();
        let light = ColorPalette::light();
        // Text primary should be inverted
        assert_ne!(dark.text.primary.base, light.text.primary.base);
        // Backgrounds should differ
        assert_ne!(dark.backgrounds.page.base, light.backgrounds.page.base);
    }

    #[test]
    fn test_lighter_darker() {
        let token = ColorToken::from_hex(0x808080); // Gray
        let lighter = token.lighter(0.2);
        let darker = token.darker(0.2);

        let base_hsla = Hsla::from(token.base);
        let lighter_hsla = Hsla::from(lighter.base);
        let darker_hsla = Hsla::from(darker.base);

        assert!(lighter_hsla.l > base_hsla.l);
        assert!(darker_hsla.l < base_hsla.l);
    }

    #[test]
    fn test_helper_functions() {
        let color = rgb(0x808080);

        let alpha_color = with_alpha(color, 0.5);
        assert!((alpha_color.a - 0.5).abs() < 0.01);

        let lighter_color = lighten(color, 0.1);
        let hsla = Hsla::from(lighter_color);
        assert!(hsla.l > 0.5);

        let darker_color = darken(color, 0.1);
        let hsla = Hsla::from(darker_color);
        assert!(hsla.l < 0.5);
    }
}
