//! HCL (Hue-Chroma-Luminance) and LAB (Lightness-a-b) color spaces
//!
//! This module provides support for perceptually uniform color spaces
//! that are superior to RGB for color interpolation.

use crate::color::D3Color;
use std::f64::consts::PI;

const REF_X: f64 = 0.95047;
const _REF_Y: f64 = 1.0;
const REF_Z: f64 = 1.08883;

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn lab_f(t: f64) -> f64 {
    if t > 0.008856 {
        t.powf(1.0 / 3.0)
    } else {
        7.787 * t + 16.0 / 116.0
    }
}

fn lab_f_inv(t: f64) -> f64 {
    let t3 = t * t * t;
    if t3 > 0.008856 {
        t3
    } else {
        (t - 16.0 / 116.0) / 7.787
    }
}

/// LAB color representation (CIELAB)
///
/// LAB is a perceptually uniform color space designed to approximate
/// human vision. It is particularly useful for color interpolation
/// because changes in LAB values correspond to perceptually uniform
/// changes in color.
///
/// # Example
///
/// ```
/// use d3rs::color::{Lab, Hcl};
///
/// let lab = Lab::new(50.0, 30.0, -20.0);
/// let hcl = Hcl::from_lab(&lab);
/// let rgb = lab.to_rgb();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    /// Lightness [0, 100]
    pub l: f64,
    /// Green-Red axis [-128, 127]
    pub a: f64,
    /// Blue-Yellow axis [-128, 127]
    pub b: f64,
    /// Alpha [0, 1]
    pub alpha: f64,
}

impl Lab {
    /// Create a new LAB color.
    ///
    /// # Arguments
    /// * `l` - Lightness from 0 (black) to 100 (white)
    /// * `a` - Green-Red axis (negative = green, positive = red)
    /// * `b` - Blue-Yellow axis (negative = blue, positive = yellow)
    pub fn new(l: f64, a: f64, b: f64) -> Self {
        Self {
            l: l.clamp(0.0, 100.0),
            a,
            b,
            alpha: 1.0,
        }
    }

    /// Create a new LAB color with alpha.
    pub fn with_alpha(l: f64, a: f64, b: f64, alpha: f64) -> Self {
        Self {
            l: l.clamp(0.0, 100.0),
            a,
            b,
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Create from a D3Color (RGB).
    pub fn from_rgb(color: &D3Color) -> Self {
        let r = srgb_to_linear(color.r as f64);
        let g = srgb_to_linear(color.g as f64);
        let b = srgb_to_linear(color.b as f64);

        let x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / REF_X;
        let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
        let z = (0.0193339 * r + 0.1191920 * g + 0.9503041 * b) / REF_Z;

        let fx = lab_f(x);
        let fy = lab_f(y);
        let fz = lab_f(z);

        Self {
            l: 116.0 * fy - 16.0,
            a: 500.0 * (fx - fy),
            b: 200.0 * (fy - fz),
            alpha: color.a as f64,
        }
    }

    /// Convert to D3Color (RGB).
    pub fn to_rgb(&self) -> D3Color {
        let fy = (self.l + 16.0) / 116.0;
        let fx = self.a / 500.0 + fy;
        let fz = fy - self.b / 200.0;

        let x = REF_X * lab_f_inv(fx);
        let y = lab_f_inv(fy);
        let z = REF_Z * lab_f_inv(fz);

        let r = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
        let g = -0.9692660 * x + 1.8760108 * y + 0.0415560 * z;
        let b = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;

        D3Color {
            r: linear_to_srgb(r).clamp(0.0, 1.0) as f32,
            g: linear_to_srgb(g).clamp(0.0, 1.0) as f32,
            b: linear_to_srgb(b).clamp(0.0, 1.0) as f32,
            a: self.alpha as f32,
        }
    }

    /// Calculate the perceived difference (Delta E) between two LAB colors.
    pub fn delta_e(&self, other: &Lab) -> f64 {
        let dl = self.l - other.l;
        let da = self.a - other.a;
        let db = self.b - other.b;
        (dl * dl + da * da + db * db).sqrt()
    }

    /// Get the chroma (saturation) of this LAB color.
    pub fn chroma(&self) -> f64 {
        (self.a * self.a + self.b * self.b).sqrt()
    }
}

/// HCL color representation (cylindrical LAB)
///
/// HCL is a cylindrical transformation of LAB color space, similar to
/// HSL but using perceptually uniform coordinates. This makes HCL ideal
/// for data visualization where you want perceptually uniform color ramps.
///
/// In HCL:
/// - H (Hue): The color angle [0, 360)
/// - C (Chroma): The color intensity/purity [0, ~100+]
/// - L (Luminance): The lightness [0, 100]
///
/// # Example
///
/// ```
/// use d3rs::color::Hcl;
///
/// let hcl = Hcl::new(180.0, 50.0, 60.0);
/// let rgb = hcl.to_rgb();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hcl {
    /// Hue in degrees [0, 360)
    pub h: f64,
    /// Chroma (color intensity) [0, ~100+]
    pub c: f64,
    /// Luminance [0, 100]
    pub l: f64,
    /// Alpha [0, 1]
    pub alpha: f64,
}

impl Hcl {
    /// Create a new HCL color.
    ///
    /// # Arguments
    /// * `h` - Hue in degrees [0, 360)
    /// * `c` - Chroma (color intensity)
    /// * `l` - Luminance [0, 100]
    pub fn new(h: f64, c: f64, l: f64) -> Self {
        Self {
            h: h.rem_euclid(360.0),
            c: c.max(0.0),
            l: l.clamp(0.0, 100.0),
            alpha: 1.0,
        }
    }

    /// Create a new HCL color with alpha.
    pub fn with_alpha(h: f64, c: f64, l: f64, alpha: f64) -> Self {
        Self {
            h: h.rem_euclid(360.0),
            c: c.max(0.0),
            l: l.clamp(0.0, 100.0),
            alpha: alpha.clamp(0.0, 1.0),
        }
    }

    /// Create from LAB color.
    pub fn from_lab(lab: &Lab) -> Self {
        let c = (lab.a * lab.a + lab.b * lab.b).sqrt();
        let h = if c.abs() < f64::EPSILON {
            0.0
        } else {
            lab.b.atan2(lab.a) * 180.0 / PI
        };
        Self {
            h: if h < 0.0 { h + 360.0 } else { h },
            c,
            l: lab.l,
            alpha: lab.alpha,
        }
    }

    /// Create from RGB color.
    pub fn from_rgb(color: &D3Color) -> Self {
        Self::from_lab(&Lab::from_rgb(color))
    }

    /// Convert to LAB.
    pub fn to_lab(&self) -> Lab {
        let h_rad = self.h * PI / 180.0;
        Lab {
            l: self.l,
            a: self.c * h_rad.cos(),
            b: self.c * h_rad.sin(),
            alpha: self.alpha,
        }
    }

    /// Convert to RGB.
    pub fn to_rgb(&self) -> D3Color {
        self.to_lab().to_rgb()
    }

    /// Interpolate to another HCL color along the shorter hue arc.
    pub fn interpolate(&self, other: &Hcl, t: f64) -> Hcl {
        let t = t.clamp(0.0, 1.0);

        let mut h_diff = other.h - self.h;
        if h_diff > 180.0 {
            h_diff -= 360.0;
        } else if h_diff < -180.0 {
            h_diff += 360.0;
        }

        Hcl {
            h: (self.h + h_diff * t).rem_euclid(360.0),
            c: self.c + (other.c - self.c) * t,
            l: self.l + (other.l - self.l) * t,
            alpha: self.alpha + (other.alpha - self.alpha) * t,
        }
    }

    /// Interpolate to another HCL color along the longer hue arc.
    pub fn interpolate_long(&self, other: &Hcl, t: f64) -> Hcl {
        let t = t.clamp(0.0, 1.0);

        let mut h_diff = other.h - self.h;
        if h_diff.abs() < 180.0 {
            if h_diff > 0.0 {
                h_diff -= 360.0;
            } else {
                h_diff += 360.0;
            }
        }

        Hcl {
            h: (self.h + h_diff * t).rem_euclid(360.0),
            c: self.c + (other.c - self.c) * t,
            l: self.l + (other.l - self.l) * t,
            alpha: self.alpha + (other.alpha - self.alpha) * t,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_lab_creation() {
        let lab = Lab::new(50.0, 30.0, -20.0);
        assert_relative_eq!(lab.l, 50.0);
        assert_relative_eq!(lab.a, 30.0);
        assert_relative_eq!(lab.b, -20.0);
    }

    #[test]
    fn test_lab_roundtrip() {
        let original = D3Color::rgb(128, 64, 192);
        let lab = Lab::from_rgb(&original);
        let result = lab.to_rgb();

        assert_relative_eq!(original.r, result.r, epsilon = 0.02);
        assert_relative_eq!(original.g, result.g, epsilon = 0.02);
        assert_relative_eq!(original.b, result.b, epsilon = 0.02);
    }

    #[test]
    fn test_hcl_creation() {
        let hcl = Hcl::new(180.0, 50.0, 60.0);
        assert_relative_eq!(hcl.h, 180.0);
        assert_relative_eq!(hcl.c, 50.0);
        assert_relative_eq!(hcl.l, 60.0);
    }

    #[test]
    fn test_hcl_roundtrip() {
        let original = D3Color::rgb(128, 64, 192);
        let hcl = Hcl::from_rgb(&original);
        let result = hcl.to_rgb();

        assert_relative_eq!(original.r, result.r, epsilon = 0.02);
        assert_relative_eq!(original.g, result.g, epsilon = 0.02);
        assert_relative_eq!(original.b, result.b, epsilon = 0.02);
    }

    #[test]
    fn test_hcl_lab_conversion() {
        let hcl = Hcl::new(180.0, 50.0, 60.0);
        let lab = hcl.to_lab();
        let hcl_back = Hcl::from_lab(&lab);

        assert_relative_eq!(hcl.h, hcl_back.h, epsilon = 0.001);
        assert_relative_eq!(hcl.c, hcl_back.c, epsilon = 0.001);
        assert_relative_eq!(hcl.l, hcl_back.l, epsilon = 0.001);
    }

    #[test]
    fn test_hcl_interpolation() {
        let red = D3Color::rgb(255, 0, 0);
        let blue = D3Color::rgb(0, 0, 255);

        let red_hcl = Hcl::from_rgb(&red);
        let blue_hcl = Hcl::from_rgb(&blue);

        let mid_hcl = red_hcl.interpolate(&blue_hcl, 0.5);
        let mid_rgb = mid_hcl.to_rgb();

        assert!(mid_rgb.r > 0.3);
        assert!(mid_rgb.b > 0.3);
    }

    #[test]
    fn test_lab_delta_e() {
        let lab1 = Lab::new(50.0, 0.0, 0.0);
        let lab2 = Lab::new(55.0, 0.0, 0.0);

        let delta = lab1.delta_e(&lab2);
        assert_relative_eq!(delta, 5.0, epsilon = 0.01);
    }

    #[test]
    fn test_hue_wrapping() {
        let hcl1 = Hcl::new(350.0, 50.0, 50.0);
        let hcl2 = Hcl::new(10.0, 50.0, 50.0);

        let mid = hcl1.interpolate(&hcl2, 0.5);
        assert!(mid.h >= 0.0 && mid.h <= 360.0);
    }
}
