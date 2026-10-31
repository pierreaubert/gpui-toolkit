#[cfg(feature = "headless-qa")]
use super::super::config::Surface3DConfig;

#[cfg(feature = "headless-qa")]
pub(super) fn background_surface_clear_color(config: &Surface3DConfig) -> wgpu::Color {
    let bg = &config.background_color;
    wgpu::Color {
        r: f64::from(bg[0].clamp(0.0, 1.0)),
        g: f64::from(bg[1].clamp(0.0, 1.0)),
        b: f64::from(bg[2].clamp(0.0, 1.0)),
        a: 1.0,
    }
}
