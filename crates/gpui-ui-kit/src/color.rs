//! Serializable color representation for GPUI applications
//!
//! Provides a Color type with:
//! - RGBA components (0-255 values for readability)
//! - Hex string conversion
//! - HSL conversion
//! - GPUI Rgba conversion

use gpui::Rgba;
use serde::{Deserialize, Serialize};

/// Serializable color representation (RGBA with 0-255 values for readability)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    /// Create a new color from RGBA components (0-255)
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque color from RGB components
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create from a hex value (0xRRGGBB)
    pub fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as u8,
            g: ((hex >> 8) & 0xFF) as u8,
            b: (hex & 0xFF) as u8,
            a: 255,
        }
    }

    /// Create from a hex value with alpha (0xRRGGBBAA)
    pub fn from_hex_alpha(hex: u32) -> Self {
        Self {
            r: ((hex >> 24) & 0xFF) as u8,
            g: ((hex >> 16) & 0xFF) as u8,
            b: ((hex >> 8) & 0xFF) as u8,
            a: (hex & 0xFF) as u8,
        }
    }

    /// Convert to hex string (#RRGGBB or #RRGGBBAA)
    pub fn to_hex_string(&self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// Parse from hex string (#RGB, #RRGGBB, or #RRGGBBAA)
    pub fn from_hex_string(s: &str) -> Option<Self> {
        let s = s.trim_start_matches('#');
        match s.len() {
            3 => {
                let r = u8::from_str_radix(&s[0..1], 16).ok()?;
                let g = u8::from_str_radix(&s[1..2], 16).ok()?;
                let b = u8::from_str_radix(&s[2..3], 16).ok()?;
                Some(Self::rgb(r * 17, g * 17, b * 17))
            }
            6 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                let a = u8::from_str_radix(&s[6..8], 16).ok()?;
                Some(Self::new(r, g, b, a))
            }
            _ => None,
        }
    }

    /// Convert to GPUI Rgba
    pub fn to_rgba(&self) -> Rgba {
        Rgba {
            r: self.r as f32 / 255.0,
            g: self.g as f32 / 255.0,
            b: self.b as f32 / 255.0,
            a: self.a as f32 / 255.0,
        }
    }

    /// Convert from GPUI Rgba
    pub fn from_rgba(rgba: Rgba) -> Self {
        Self {
            r: (rgba.r * 255.0).round() as u8,
            g: (rgba.g * 255.0).round() as u8,
            b: (rgba.b * 255.0).round() as u8,
            a: (rgba.a * 255.0).round() as u8,
        }
    }

    /// Apply alpha (0.0-1.0 scale)
    pub fn with_alpha(&self, alpha: f32) -> Self {
        Self {
            r: self.r,
            g: self.g,
            b: self.b,
            a: (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
        }
    }

    /// Get HSL components
    pub fn to_hsl(&self) -> (f32, f32, f32) {
        let r = self.r as f32 / 255.0;
        let g = self.g as f32 / 255.0;
        let b = self.b as f32 / 255.0;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if (max - min).abs() < f32::EPSILON {
            return (0.0, 0.0, l);
        }

        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };

        let h = if (max - r).abs() < f32::EPSILON {
            ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
        } else if (max - g).abs() < f32::EPSILON {
            ((b - r) / d + 2.0) / 6.0
        } else {
            ((r - g) / d + 4.0) / 6.0
        };

        (h, s, l)
    }

    /// Create from HSL components (h: 0-1, s: 0-1, l: 0-1)
    pub fn from_hsl(h: f32, s: f32, l: f32) -> Self {
        let (r, g, b) = if s.abs() < f32::EPSILON {
            (l, l, l)
        } else {
            fn hue_to_rgb(p: f32, q: f32, t: f32) -> f32 {
                let t = if t < 0.0 {
                    t + 1.0
                } else if t > 1.0 {
                    t - 1.0
                } else {
                    t
                };
                if t < 1.0 / 6.0 {
                    p + (q - p) * 6.0 * t
                } else if t < 1.0 / 2.0 {
                    q
                } else if t < 2.0 / 3.0 {
                    p + (q - p) * (2.0 / 3.0 - t) * 6.0
                } else {
                    p
                }
            }

            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
            let p = 2.0 * l - q;
            (
                hue_to_rgb(p, q, h + 1.0 / 3.0),
                hue_to_rgb(p, q, h),
                hue_to_rgb(p, q, h - 1.0 / 3.0),
            )
        };

        Self::rgb(
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
        )
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::rgb(128, 128, 128)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_hex_conversion() {
        let color = Color::from_hex(0xff5500);
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 85);
        assert_eq!(color.b, 0);
        assert_eq!(color.a, 255);
        assert_eq!(color.to_hex_string(), "#ff5500");
    }

    #[test]
    fn test_color_hex_string_parsing() {
        let color = Color::from_hex_string("#ff5500").unwrap();
        assert_eq!(color.r, 255);
        assert_eq!(color.g, 85);
        assert_eq!(color.b, 0);

        let color_short = Color::from_hex_string("#f50").unwrap();
        assert_eq!(color_short.r, 255);
        assert_eq!(color_short.g, 85);
        assert_eq!(color_short.b, 0);
    }

    #[test]
    fn test_hsl_roundtrip() {
        let color = Color::rgb(255, 128, 64);
        let (h, s, l) = color.to_hsl();
        let back = Color::from_hsl(h, s, l);
        // Allow small rounding errors
        assert!((color.r as i16 - back.r as i16).abs() <= 1);
        assert!((color.g as i16 - back.g as i16).abs() <= 1);
        assert!((color.b as i16 - back.b as i16).abs() <= 1);
    }
}
