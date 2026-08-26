#[cfg(feature = "headless-qa")]
use super::super::config::Surface3DConfig;

#[cfg(feature = "headless-qa")]
pub(super) fn background_surface_clear_color(config: &Surface3DConfig) -> wgpu::Color {
    let bg = &config.background_color;
    wgpu::Color {
        r: bg[0].clamp(0.0, 1.0) as f64,
        g: bg[1].clamp(0.0, 1.0) as f64,
        b: bg[2].clamp(0.0, 1.0) as f64,
        a: 1.0,
    }
}
