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
