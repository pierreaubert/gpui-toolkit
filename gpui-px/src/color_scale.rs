//! Color scales for heatmaps, contours, and isolines.

use d3rs::color::D3Color;
use std::sync::Arc;

/// Color scale for 2D visualizations (heatmaps, contours).
#[derive(Clone, Default)]
pub enum ColorScale {
    /// Viridis - perceptually uniform, colorblind-friendly (purple → yellow).
    #[default]
    Viridis,
    /// Plasma - perceptually uniform (purple → orange → yellow).
    Plasma,
    /// Inferno - perceptually uniform (black → purple → orange → yellow).
    Inferno,
    /// Magma - perceptually uniform (black → purple → orange → white).
    Magma,
    /// Heat - diverging (blue → white → red).
    Heat,
    /// Coolwarm - diverging (cool blue → neutral → warm red).
    Coolwarm,
    /// Greys - sequential grayscale (white → black).
    Greys,
    /// Custom color scale function.
    Custom(Arc<dyn Fn(f64) -> D3Color + Send + Sync>),
}

impl std::fmt::Debug for ColorScale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorScale::Viridis => write!(f, "ColorScale::Viridis"),
            ColorScale::Plasma => write!(f, "ColorScale::Plasma"),
            ColorScale::Inferno => write!(f, "ColorScale::Inferno"),
            ColorScale::Magma => write!(f, "ColorScale::Magma"),
            ColorScale::Heat => write!(f, "ColorScale::Heat"),
            ColorScale::Coolwarm => write!(f, "ColorScale::Coolwarm"),
            ColorScale::Greys => write!(f, "ColorScale::Greys"),
            ColorScale::Custom(_) => write!(f, "ColorScale::Custom(...)"),
        }
    }
}

impl ColorScale {
    /// Create a custom color scale from a function.
    ///
    /// The function should map values in [0, 1] to colors.
    pub fn custom<F>(f: F) -> Self
    where
        F: Fn(f64) -> D3Color + Send + Sync + 'static,
    {
        ColorScale::Custom(Arc::new(f))
    }

    /// Convert to a function that maps [0, 1] → D3Color.
    pub fn to_fn(&self) -> impl Fn(f64) -> D3Color + Send + Sync + Clone + 'static {
        let scale = self.clone();
        move |t: f64| scale.map(t)
    }

    /// Map a value in [0, 1] to a color.
    pub fn map(&self, t: f64) -> D3Color {
        let t = t.clamp(0.0, 1.0);

        match self {
            ColorScale::Viridis => viridis(t),
            ColorScale::Plasma => plasma(t),
            ColorScale::Inferno => inferno(t),
            ColorScale::Magma => magma(t),
            ColorScale::Heat => heat(t),
            ColorScale::Coolwarm => coolwarm(t),
            ColorScale::Greys => greys(t),
            ColorScale::Custom(f) => f(t),
        }
    }
}

// Helper function to interpolate between colors in a palette
fn interpolate_palette(t: f64, colors: &[D3Color]) -> D3Color {
    let idx = (t * (colors.len() - 1) as f64) as usize;
    let idx = idx.min(colors.len() - 2);
    let local_t = (t * (colors.len() - 1) as f64) - idx as f64;
    colors[idx].interpolate(&colors[idx + 1], local_t as f32)
}

/// Viridis colormap (matplotlib/d3)
fn viridis(t: f64) -> D3Color {
    let colors = [
        D3Color::from_hex(0x440154),
        D3Color::from_hex(0x482878),
        D3Color::from_hex(0x3e4a89),
        D3Color::from_hex(0x31688e),
        D3Color::from_hex(0x26838f),
        D3Color::from_hex(0x1f9e89),
        D3Color::from_hex(0x35b779),
        D3Color::from_hex(0x6ece58),
        D3Color::from_hex(0xb5de2b),
        D3Color::from_hex(0xfde725),
    ];
    interpolate_palette(t, &colors)
}

/// Plasma colormap (matplotlib/d3)
fn plasma(t: f64) -> D3Color {
    let colors = [
        D3Color::from_hex(0x0d0887),
        D3Color::from_hex(0x46039f),
        D3Color::from_hex(0x7201a8),
        D3Color::from_hex(0x9c179e),
        D3Color::from_hex(0xbd3786),
        D3Color::from_hex(0xd8576b),
        D3Color::from_hex(0xed7953),
        D3Color::from_hex(0xfb9f3a),
        D3Color::from_hex(0xfdca26),
        D3Color::from_hex(0xf0f921),
    ];
    interpolate_palette(t, &colors)
}

/// Inferno colormap (matplotlib/d3)
fn inferno(t: f64) -> D3Color {
    let colors = [
        D3Color::from_hex(0x000004),
        D3Color::from_hex(0x1b0c41),
        D3Color::from_hex(0x4a0c6b),
        D3Color::from_hex(0x781c6d),
        D3Color::from_hex(0xa52c60),
        D3Color::from_hex(0xcf4446),
        D3Color::from_hex(0xed6925),
        D3Color::from_hex(0xfb9b06),
        D3Color::from_hex(0xf7d13d),
        D3Color::from_hex(0xfcffa4),
    ];
    interpolate_palette(t, &colors)
}

/// Magma colormap (matplotlib/d3)
fn magma(t: f64) -> D3Color {
    let colors = [
        D3Color::from_hex(0x000004),
        D3Color::from_hex(0x180f3d),
        D3Color::from_hex(0x440f76),
        D3Color::from_hex(0x721f81),
        D3Color::from_hex(0x9e2f7f),
        D3Color::from_hex(0xcd4071),
        D3Color::from_hex(0xf1605d),
        D3Color::from_hex(0xfd9668),
        D3Color::from_hex(0xfeca8d),
        D3Color::from_hex(0xfcfdbf),
    ];
    interpolate_palette(t, &colors)
}

/// Heat colormap (blue → white → red)
fn heat(t: f64) -> D3Color {
    if t < 0.5 {
        let local_t = t * 2.0;
        D3Color::from_hex(0x0571b0).interpolate(&D3Color::from_hex(0xf7f7f7), local_t as f32)
    } else {
        let local_t = (t - 0.5) * 2.0;
        D3Color::from_hex(0xf7f7f7).interpolate(&D3Color::from_hex(0xca0020), local_t as f32)
    }
}

/// Coolwarm colormap (diverging)
fn coolwarm(t: f64) -> D3Color {
    let colors = [
        D3Color::from_hex(0x3b4cc0),
        D3Color::from_hex(0x6788ee),
        D3Color::from_hex(0x9abbff),
        D3Color::from_hex(0xc9d7f0),
        D3Color::from_hex(0xedd1c2),
        D3Color::from_hex(0xf7a789),
        D3Color::from_hex(0xe36a53),
        D3Color::from_hex(0xb40426),
    ];
    interpolate_palette(t, &colors)
}

/// Greys colormap (white → black)
fn greys(t: f64) -> D3Color {
    D3Color::from_hex(0xffffff).interpolate(&D3Color::from_hex(0x000000), t as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viridis_endpoints() {
        let start = ColorScale::Viridis.map(0.0);
        let end = ColorScale::Viridis.map(1.0);
        // Viridis starts dark purple, ends yellow
        assert!(start.r < 0.3);
        assert!(end.g > 0.8);
    }

    #[test]
    fn test_plasma_endpoints() {
        let start = ColorScale::Plasma.map(0.0);
        let end = ColorScale::Plasma.map(1.0);
        // Plasma starts dark blue, ends light yellow
        assert!(start.b > 0.3);
        assert!(end.r > 0.8);
    }

    #[test]
    fn test_heat_diverging() {
        let low = ColorScale::Heat.map(0.0);
        let mid = ColorScale::Heat.map(0.5);
        let high = ColorScale::Heat.map(1.0);
        // Low is blue, mid is white-ish, high is red
        assert!(low.b > low.r);
        assert!(mid.r > 0.9 && mid.g > 0.9 && mid.b > 0.9);
        assert!(high.r > high.b);
    }

    #[test]
    fn test_greys_endpoints() {
        let start = ColorScale::Greys.map(0.0);
        let end = ColorScale::Greys.map(1.0);
        // Start is white, end is black
        assert!(start.r > 0.99 && start.g > 0.99 && start.b > 0.99);
        assert!(end.r < 0.01 && end.g < 0.01 && end.b < 0.01);
    }

    #[test]
    fn test_custom_scale() {
        let scale = ColorScale::custom(|t| {
            D3Color::from_hex(0xff0000).interpolate(&D3Color::from_hex(0x00ff00), t as f32)
        });
        let mid = scale.map(0.5);
        // Mid should be yellow-ish (red + green)
        assert!(mid.r > 0.4 && mid.g > 0.4);
    }

    #[test]
    fn test_clamp_out_of_bounds() {
        // Values outside [0, 1] should be clamped
        let below = ColorScale::Viridis.map(-0.5);
        let above = ColorScale::Viridis.map(1.5);
        let at_zero = ColorScale::Viridis.map(0.0);
        let at_one = ColorScale::Viridis.map(1.0);

        // Should be clamped to endpoints
        assert_eq!(below.r, at_zero.r);
        assert_eq!(above.r, at_one.r);
    }

    #[test]
    fn test_to_fn() {
        let f = ColorScale::Viridis.to_fn();
        let direct = ColorScale::Viridis.map(0.5);
        let via_fn = f(0.5);
        assert_eq!(direct.r, via_fn.r);
        assert_eq!(direct.g, via_fn.g);
        assert_eq!(direct.b, via_fn.b);
    }

    #[test]
    fn test_default() {
        let scale = ColorScale::default();
        assert!(matches!(scale, ColorScale::Viridis));
    }

    #[test]
    fn test_debug() {
        assert_eq!(format!("{:?}", ColorScale::Viridis), "ColorScale::Viridis");
        assert_eq!(format!("{:?}", ColorScale::Heat), "ColorScale::Heat");
        let custom = ColorScale::custom(|_| D3Color::from_hex(0x000000));
        assert_eq!(format!("{:?}", custom), "ColorScale::Custom(...)");
    }
}
