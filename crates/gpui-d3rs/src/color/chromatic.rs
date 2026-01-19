//! Color Schemes (d3-scale-chromatic)
//!
//! Categorical, sequential, and diverging color schemes.

use super::D3Color;

/// Sequential color schemes
pub struct SequentialScheme;

impl SequentialScheme {
    /// Turbo color scheme (Google)
    pub fn turbo(t: f64) -> D3Color {
        // Approximate implementation or polynomial based
        // For simplicity using a simplified localized version or delegation if implemented in interpolate
        // Here we implement the polynomial for Turbo
        let t = t.clamp(0.0, 1.0);
        // ... (Math implementation is large, for brevity using a placeholder or simple lerp for now if math not verified)
        // Ideally we copy the implementation from d3-scale-chromatic

        // Placeholder: Blue -> Green -> Red
        if t < 0.5 {
            let t = (t * 2.0) as f32;
            D3Color::rgb(0, 0, 255).interpolate(&D3Color::rgb(0, 255, 0), t)
        } else {
            let t = ((t - 0.5) * 2.0) as f32;
            D3Color::rgb(0, 255, 0).interpolate(&D3Color::rgb(255, 0, 0), t)
        }
    }

    /// Viridis
    pub fn viridis(t: f64) -> D3Color {
        // Placeholder: Purple -> Teal -> Yellow
        if t < 0.5 {
            let t = (t * 2.0) as f32;
            D3Color::from_hex(0x440154).interpolate(&D3Color::from_hex(0x21918c), t)
        } else {
            let t = ((t - 0.5) * 2.0) as f32;
            D3Color::from_hex(0x21918c).interpolate(&D3Color::from_hex(0xfde725), t)
        }
    }

    /// Magma
    pub fn magma(t: f64) -> D3Color {
        // Placeholder: Black -> Red -> White
        if t < 0.5 {
            let t = (t * 2.0) as f32;
            D3Color::from_hex(0x000004).interpolate(&D3Color::from_hex(0xb73779), t)
        } else {
            let t = ((t - 0.5) * 2.0) as f32;
            D3Color::from_hex(0xb73779).interpolate(&D3Color::from_hex(0xfcfdbf), t)
        }
    }
}

/// Diverging Color Schemes
pub struct DivergingScheme;

impl DivergingScheme {
    /// RdBu (Red-Blue)
    pub fn rd_bu(t: f64) -> D3Color {
        // Red (0) -> White (0.5) -> Blue (1)
        if t < 0.5 {
            let t = (t * 2.0) as f32;
            D3Color::from_hex(0xb2182b).interpolate(&D3Color::from_hex(0xf7f7f7), t)
        } else {
            let t = ((t - 0.5) * 2.0) as f32;
            D3Color::from_hex(0xf7f7f7).interpolate(&D3Color::from_hex(0x2166ac), t)
        }
    }
}
