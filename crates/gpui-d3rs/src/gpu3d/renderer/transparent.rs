pub(super) fn transparent_surface_clear_color() -> wgpu::Color {
    wgpu::Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    }
}

#[doc(hidden)]
pub fn transparent_surface_clear_color_for_testing() -> [f64; 4] {
    let color = transparent_surface_clear_color();
    [color.r, color.g, color.b, color.a]
}
