use super::misc::cool_color;
use super::misc::grayscale_color;
use super::misc::heat_color;
use super::misc::monochrome_color;
use super::misc::spectral_color;
use super::misc::viridis_color;
use crate::color::D3Color;

/// Color scale types for surface coloring
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ColorScaleType {
    /// Viridis color scale (perceptually uniform, colorblind-friendly)
    #[default]
    Viridis,
    /// Heat color scale (blue -> white -> red)
    Heat,
    /// Cool color scale (cyan -> magenta)
    Cool,
    /// Spectral color scale (rainbow)
    Spectral,
    /// Grayscale
    Grayscale,
    /// Single color with brightness variation
    Monochrome(D3Color),
}

impl ColorScaleType {
    /// Get the color for a normalized value [0, 1]
    pub fn color(&self, t: f64) -> D3Color {
        let t = t.clamp(0.0, 1.0);
        match self {
            ColorScaleType::Viridis => viridis_color(t),
            ColorScaleType::Heat => heat_color(t),
            ColorScaleType::Cool => cool_color(t),
            ColorScaleType::Spectral => spectral_color(t),
            ColorScaleType::Grayscale => grayscale_color(t),
            ColorScaleType::Monochrome(base) => monochrome_color(*base, t),
        }
    }
}
