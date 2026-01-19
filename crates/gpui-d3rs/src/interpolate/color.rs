//! Color interpolation functions
//!
//! Provides interpolation in various color spaces including RGB, HSL, LAB, HCL,
//! and Cubehelix.

use crate::color::D3Color;
use std::f64::consts::PI;

/// Interpolate between two colors in RGB space.
///
/// # Example
///
/// ```
/// use d3rs::color::D3Color;
/// use d3rs::interpolate::interpolate_rgb;
///
/// let red = D3Color::rgb(255, 0, 0);
/// let blue = D3Color::rgb(0, 0, 255);
/// let interp = interpolate_rgb(red, blue);
///
/// let purple = interp(0.5);
/// assert!((purple.r - 0.5).abs() < 0.01);
/// assert!((purple.b - 0.5).abs() < 0.01);
/// ```
pub fn interpolate_rgb(a: D3Color, b: D3Color) -> impl Fn(f64) -> D3Color {
    move |t| {
        let t = t as f32;
        D3Color {
            r: a.r + (b.r - a.r) * t,
            g: a.g + (b.g - a.g) * t,
            b: a.b + (b.b - a.b) * t,
            a: a.a + (b.a - a.a) * t,
        }
    }
}

/// HSL color representation
#[derive(Debug, Clone, Copy)]
pub struct Hsl {
    pub h: f64, // Hue in degrees [0, 360)
    pub s: f64, // Saturation [0, 1]
    pub l: f64, // Lightness [0, 1]
    pub a: f64, // Alpha [0, 1]
}

impl Hsl {
    /// Create a new HSL color.
    pub fn new(h: f64, s: f64, l: f64) -> Self {
        Self { h, s, l, a: 1.0 }
    }

    /// Create from a D3Color (RGB).
    pub fn from_rgb(color: &D3Color) -> Self {
        let r = color.r as f64;
        let g = color.g as f64;
        let b = color.b as f64;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;

        if (max - min).abs() < f64::EPSILON {
            return Self {
                h: 0.0,
                s: 0.0,
                l,
                a: color.a as f64,
            };
        }

        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };

        let h = if (max - r).abs() < f64::EPSILON {
            (g - b) / d + if g < b { 6.0 } else { 0.0 }
        } else if (max - g).abs() < f64::EPSILON {
            (b - r) / d + 2.0
        } else {
            (r - g) / d + 4.0
        };

        Self {
            h: h * 60.0,
            s,
            l,
            a: color.a as f64,
        }
    }

    /// Convert to D3Color (RGB).
    pub fn to_rgb(&self) -> D3Color {
        let h = self.h / 360.0;
        let s = self.s;
        let l = self.l;

        if s.abs() < f64::EPSILON {
            let v = l as f32;
            return D3Color {
                r: v,
                g: v,
                b: v,
                a: self.a as f32,
            };
        }

        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;

        fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
            if t < 0.0 {
                t += 1.0;
            }
            if t > 1.0 {
                t -= 1.0;
            }
            if t < 1.0 / 6.0 {
                return p + (q - p) * 6.0 * t;
            }
            if t < 1.0 / 2.0 {
                return q;
            }
            if t < 2.0 / 3.0 {
                return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
            }
            p
        }

        D3Color {
            r: hue_to_rgb(p, q, h + 1.0 / 3.0) as f32,
            g: hue_to_rgb(p, q, h) as f32,
            b: hue_to_rgb(p, q, h - 1.0 / 3.0) as f32,
            a: self.a as f32,
        }
    }
}

/// Interpolate between two colors in HSL space.
///
/// This provides smoother color transitions through the color wheel.
///
/// # Example
///
/// ```
/// use d3rs::color::D3Color;
/// use d3rs::interpolate::interpolate_hsl;
///
/// let red = D3Color::rgb(255, 0, 0);
/// let blue = D3Color::rgb(0, 0, 255);
/// let interp = interpolate_hsl(red, blue);
///
/// let mid = interp(0.5);
/// // Should be a purple/magenta color
/// ```
pub fn interpolate_hsl(a: D3Color, b: D3Color) -> impl Fn(f64) -> D3Color {
    let a_hsl = Hsl::from_rgb(&a);
    let b_hsl = Hsl::from_rgb(&b);

    move |t| {
        // Interpolate hue along the shorter arc
        let mut h_diff = b_hsl.h - a_hsl.h;
        if h_diff > 180.0 {
            h_diff -= 360.0;
        } else if h_diff < -180.0 {
            h_diff += 360.0;
        }

        let h = (a_hsl.h + h_diff * t).rem_euclid(360.0);
        let s = a_hsl.s + (b_hsl.s - a_hsl.s) * t;
        let l = a_hsl.l + (b_hsl.l - a_hsl.l) * t;
        let alpha = a_hsl.a + (b_hsl.a - a_hsl.a) * t;

        Hsl { h, s, l, a: alpha }.to_rgb()
    }
}

/// Interpolate between two colors in HSL space (long arc).
///
/// Unlike `interpolate_hsl`, this always goes the long way around the color wheel.
pub fn interpolate_hsl_long(a: D3Color, b: D3Color) -> impl Fn(f64) -> D3Color {
    let a_hsl = Hsl::from_rgb(&a);
    let b_hsl = Hsl::from_rgb(&b);

    move |t| {
        // Interpolate hue along the longer arc
        let mut h_diff = b_hsl.h - a_hsl.h;
        if h_diff.abs() < 180.0 {
            if h_diff > 0.0 {
                h_diff -= 360.0;
            } else {
                h_diff += 360.0;
            }
        }

        let h = (a_hsl.h + h_diff * t).rem_euclid(360.0);
        let s = a_hsl.s + (b_hsl.s - a_hsl.s) * t;
        let l = a_hsl.l + (b_hsl.l - a_hsl.l) * t;
        let alpha = a_hsl.a + (b_hsl.a - a_hsl.a) * t;

        Hsl { h, s, l, a: alpha }.to_rgb()
    }
}

/// LAB color representation (CIELAB)
#[derive(Debug, Clone, Copy)]
pub struct Lab {
    pub l: f64, // Lightness [0, 100]
    pub a: f64, // Green-Red axis
    pub b: f64, // Blue-Yellow axis
    pub alpha: f64,
}

impl Lab {
    /// Create a new LAB color.
    pub fn new(l: f64, a: f64, b: f64) -> Self {
        Self {
            l,
            a,
            b,
            alpha: 1.0,
        }
    }

    /// Create from a D3Color (RGB).
    pub fn from_rgb(color: &D3Color) -> Self {
        // Convert sRGB to linear RGB
        fn to_linear(c: f64) -> f64 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }

        let r = to_linear(color.r as f64);
        let g = to_linear(color.g as f64);
        let b = to_linear(color.b as f64);

        // Convert to XYZ (D65 illuminant)
        let x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047;
        let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
        let z = (0.0193339 * r + 0.1191920 * g + 0.9503041 * b) / 1.08883;

        // Convert to LAB
        fn lab_f(t: f64) -> f64 {
            if t > 0.008856 {
                t.powf(1.0 / 3.0)
            } else {
                7.787 * t + 16.0 / 116.0
            }
        }

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

        fn lab_f_inv(t: f64) -> f64 {
            let t3 = t * t * t;
            if t3 > 0.008856 {
                t3
            } else {
                (t - 16.0 / 116.0) / 7.787
            }
        }

        let x = 0.95047 * lab_f_inv(fx);
        let y = lab_f_inv(fy);
        let z = 1.08883 * lab_f_inv(fz);

        // Convert XYZ to linear RGB
        let r = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
        let g = -0.9692660 * x + 1.8760108 * y + 0.0415560 * z;
        let b = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;

        // Convert linear RGB to sRGB
        fn from_linear(c: f64) -> f64 {
            if c <= 0.0031308 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        }

        D3Color {
            r: from_linear(r).clamp(0.0, 1.0) as f32,
            g: from_linear(g).clamp(0.0, 1.0) as f32,
            b: from_linear(b).clamp(0.0, 1.0) as f32,
            a: self.alpha as f32,
        }
    }
}

/// Interpolate between two colors in LAB space.
///
/// LAB interpolation is perceptually more uniform than RGB.
///
/// # Example
///
/// ```
/// use d3rs::color::D3Color;
/// use d3rs::interpolate::interpolate_lab;
///
/// let red = D3Color::rgb(255, 0, 0);
/// let blue = D3Color::rgb(0, 0, 255);
/// let interp = interpolate_lab(red, blue);
///
/// let mid = interp(0.5);
/// // LAB interpolation produces different results than RGB
/// ```
pub fn interpolate_lab(a: D3Color, b: D3Color) -> impl Fn(f64) -> D3Color {
    let a_lab = Lab::from_rgb(&a);
    let b_lab = Lab::from_rgb(&b);

    move |t| {
        Lab {
            l: a_lab.l + (b_lab.l - a_lab.l) * t,
            a: a_lab.a + (b_lab.a - a_lab.a) * t,
            b: a_lab.b + (b_lab.b - a_lab.b) * t,
            alpha: a_lab.alpha + (b_lab.alpha - a_lab.alpha) * t,
        }
        .to_rgb()
    }
}

/// HCL color representation (cylindrical LAB)
#[derive(Debug, Clone, Copy)]
pub struct Hcl {
    pub h: f64, // Hue in degrees
    pub c: f64, // Chroma
    pub l: f64, // Luminance
    pub alpha: f64,
}

impl Hcl {
    /// Create a new HCL color.
    pub fn new(h: f64, c: f64, l: f64) -> Self {
        Self {
            h,
            c,
            l,
            alpha: 1.0,
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
}

/// Interpolate between two colors in HCL space.
///
/// HCL provides perceptually uniform interpolation through the color wheel.
///
/// # Example
///
/// ```
/// use d3rs::color::D3Color;
/// use d3rs::interpolate::interpolate_hcl;
///
/// let red = D3Color::rgb(255, 0, 0);
/// let blue = D3Color::rgb(0, 0, 255);
/// let interp = interpolate_hcl(red, blue);
///
/// let mid = interp(0.5);
/// ```
pub fn interpolate_hcl(a: D3Color, b: D3Color) -> impl Fn(f64) -> D3Color {
    let a_hcl = Hcl::from_rgb(&a);
    let b_hcl = Hcl::from_rgb(&b);

    move |t| {
        // Interpolate hue along the shorter arc
        let mut h_diff = b_hcl.h - a_hcl.h;
        if h_diff > 180.0 {
            h_diff -= 360.0;
        } else if h_diff < -180.0 {
            h_diff += 360.0;
        }

        let h = (a_hcl.h + h_diff * t).rem_euclid(360.0);
        let c = a_hcl.c + (b_hcl.c - a_hcl.c) * t;
        let l = a_hcl.l + (b_hcl.l - a_hcl.l) * t;
        let alpha = a_hcl.alpha + (b_hcl.alpha - a_hcl.alpha) * t;

        Hcl { h, c, l, alpha }.to_rgb()
    }
}

/// Interpolate between two colors in HCL space (long arc).
pub fn interpolate_hcl_long(a: D3Color, b: D3Color) -> impl Fn(f64) -> D3Color {
    let a_hcl = Hcl::from_rgb(&a);
    let b_hcl = Hcl::from_rgb(&b);

    move |t| {
        let mut h_diff = b_hcl.h - a_hcl.h;
        if h_diff.abs() < 180.0 {
            if h_diff > 0.0 {
                h_diff -= 360.0;
            } else {
                h_diff += 360.0;
            }
        }

        let h = (a_hcl.h + h_diff * t).rem_euclid(360.0);
        let c = a_hcl.c + (b_hcl.c - a_hcl.c) * t;
        let l = a_hcl.l + (b_hcl.l - a_hcl.l) * t;
        let alpha = a_hcl.alpha + (b_hcl.alpha - a_hcl.alpha) * t;

        Hcl { h, c, l, alpha }.to_rgb()
    }
}

/// Cubehelix color representation.
///
/// Cubehelix is designed for perceptually uniform color maps.
#[derive(Debug, Clone, Copy)]
pub struct Cubehelix {
    pub h: f64, // Hue in degrees
    pub s: f64, // Saturation
    pub l: f64, // Lightness
    pub alpha: f64,
}

impl Cubehelix {
    /// Create a new Cubehelix color.
    pub fn new(h: f64, s: f64, l: f64) -> Self {
        Self {
            h,
            s,
            l,
            alpha: 1.0,
        }
    }

    /// Create from RGB.
    pub fn from_rgb(color: &D3Color) -> Self {
        let r = color.r as f64;
        let g = color.g as f64;
        let b = color.b as f64;

        let l = (0.299 * r + 0.587 * g + 0.114 * b + 0.00001).clamp(0.0, 1.0);
        let amp = ((-0.14861 * r + 1.78277 * g - 0.29227 * b).powi(2)
            + (-0.29227 * r - 0.90649 * g + 1.97294 * b).powi(2))
        .sqrt();
        let s = amp / (l * (1.0 - l)).sqrt().max(0.00001);
        let h = (-0.14861 * r + 1.78277 * g - 0.29227 * b)
            .atan2(-0.29227 * r - 0.90649 * g + 1.97294 * b)
            * 180.0
            / PI
            - 120.0;

        Self {
            h: if h < 0.0 { h + 360.0 } else { h },
            s: if s.is_nan() { 0.0 } else { s },
            l,
            alpha: color.a as f64,
        }
    }

    /// Convert to RGB.
    pub fn to_rgb(&self) -> D3Color {
        let h = (self.h + 120.0) * PI / 180.0;
        let l = self.l;
        let a = self.s * l * (1.0 - l);

        let cos_h = h.cos();
        let sin_h = h.sin();

        D3Color {
            r: (l + a * (-0.14861 * cos_h + 1.78277 * sin_h)).clamp(0.0, 1.0) as f32,
            g: (l + a * (-0.29227 * cos_h - 0.90649 * sin_h)).clamp(0.0, 1.0) as f32,
            b: (l + a * (1.97294 * cos_h)).clamp(0.0, 1.0) as f32,
            a: self.alpha as f32,
        }
    }
}

/// Interpolate between two colors in Cubehelix space.
///
/// Cubehelix is designed for data visualization with smooth,
/// perceptually uniform color transitions.
///
/// # Example
///
/// ```
/// use d3rs::color::D3Color;
/// use d3rs::interpolate::interpolate_cubehelix;
///
/// let black = D3Color::rgb(0, 0, 0);
/// let white = D3Color::rgb(255, 255, 255);
/// let interp = interpolate_cubehelix(black, white);
///
/// let mid = interp(0.5);
/// ```
pub fn interpolate_cubehelix(a: D3Color, b: D3Color) -> impl Fn(f64) -> D3Color {
    let a_ch = Cubehelix::from_rgb(&a);
    let b_ch = Cubehelix::from_rgb(&b);

    move |t| {
        // Interpolate hue along the shorter arc
        let mut h_diff = b_ch.h - a_ch.h;
        if h_diff > 180.0 {
            h_diff -= 360.0;
        } else if h_diff < -180.0 {
            h_diff += 360.0;
        }

        let h = (a_ch.h + h_diff * t).rem_euclid(360.0);
        let s = a_ch.s + (b_ch.s - a_ch.s) * t;
        let l = a_ch.l + (b_ch.l - a_ch.l) * t;
        let alpha = a_ch.alpha + (b_ch.alpha - a_ch.alpha) * t;

        Cubehelix { h, s, l, alpha }.to_rgb()
    }
}

/// Interpolate between two colors in Cubehelix space (long arc).
pub fn interpolate_cubehelix_long(a: D3Color, b: D3Color) -> impl Fn(f64) -> D3Color {
    let a_ch = Cubehelix::from_rgb(&a);
    let b_ch = Cubehelix::from_rgb(&b);

    move |t| {
        // Direct interpolation (may go through more colors)
        let h = a_ch.h + (b_ch.h - a_ch.h) * t;
        let s = a_ch.s + (b_ch.s - a_ch.s) * t;
        let l = a_ch.l + (b_ch.l - a_ch.l) * t;
        let alpha = a_ch.alpha + (b_ch.alpha - a_ch.alpha) * t;

        Cubehelix {
            h: h.rem_euclid(360.0),
            s,
            l,
            alpha,
        }
        .to_rgb()
    }
}

/// Create a cubehelix color ramp with customizable parameters.
///
/// # Arguments
/// * `start` - Starting hue (0-360)
/// * `rotations` - Number of rotations through the color wheel
/// * `hue` - Hue intensity (saturation multiplier)
/// * `gamma` - Gamma correction
///
/// # Example
///
/// ```
/// use d3rs::interpolate::cubehelix_default;
///
/// let interp = cubehelix_default();
/// let color = interp(0.5);
/// ```
pub fn cubehelix_default() -> impl Fn(f64) -> D3Color {
    cubehelix_custom(300.0, -1.5, 1.0, 1.0)
}

/// Create a custom cubehelix interpolator.
pub fn cubehelix_custom(
    start: f64,
    rotations: f64,
    hue: f64,
    gamma: f64,
) -> impl Fn(f64) -> D3Color {
    move |t| {
        let t = t.clamp(0.0, 1.0);
        let l = t.powf(gamma);
        let h = start + 360.0 * rotations * t;
        let s = hue * l * (1.0 - l);

        Cubehelix {
            h: h.rem_euclid(360.0),
            s,
            l,
            alpha: 1.0,
        }
        .to_rgb()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_hsl_roundtrip() {
        let original = D3Color::rgb(128, 64, 192);
        let hsl = Hsl::from_rgb(&original);
        let result = hsl.to_rgb();

        assert_relative_eq!(original.r, result.r, epsilon = 0.01);
        assert_relative_eq!(original.g, result.g, epsilon = 0.01);
        assert_relative_eq!(original.b, result.b, epsilon = 0.01);
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
    fn test_interpolate_rgb() {
        let red = D3Color::rgb(255, 0, 0);
        let blue = D3Color::rgb(0, 0, 255);
        let interp = interpolate_rgb(red, blue);

        let start = interp(0.0);
        assert_relative_eq!(start.r, 1.0);
        assert_relative_eq!(start.b, 0.0);

        let end = interp(1.0);
        assert_relative_eq!(end.r, 0.0);
        assert_relative_eq!(end.b, 1.0);
    }

    #[test]
    fn test_interpolate_hsl() {
        let red = D3Color::rgb(255, 0, 0);
        let blue = D3Color::rgb(0, 0, 255);
        let interp = interpolate_hsl(red, blue);

        let mid = interp(0.5);
        // Should be a purple/magenta color (around 270 degrees on HSL)
        assert!(mid.r > 0.3);
        assert!(mid.b > 0.3);
    }

    #[test]
    fn test_cubehelix_default() {
        let interp = cubehelix_default();

        let start = interp(0.0);
        let mid = interp(0.5);
        let end = interp(1.0);

        // Start should be dark, end should be light
        let start_lum = 0.299 * start.r as f64 + 0.587 * start.g as f64 + 0.114 * start.b as f64;
        let end_lum = 0.299 * end.r as f64 + 0.587 * end.g as f64 + 0.114 * end.b as f64;

        assert!(start_lum < mid.g as f64);
        assert!(end_lum > mid.g as f64);
    }
}
