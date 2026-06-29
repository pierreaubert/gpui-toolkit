use super::lab::Lab;
use crate::color::D3Color;
use std::f64::consts::PI;

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
