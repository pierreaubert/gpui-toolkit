use d3rs::color::D3Color;
use std::collections::HashMap;

/// CEA2034 measurement curve names in standard order
pub const CEA2034_CURVES: &[&str] = &[
    "On Axis",
    "Listening Window",
    "Early Reflections",
    "Sound Power",
    "Early Reflections DI",
    "Sound Power DI",
];

/// Colors for CEA2034 curves
pub fn cea2034_colors() -> HashMap<&'static str, D3Color> {
    let mut colors = HashMap::new();
    colors.insert("On Axis", D3Color::rgb(31, 119, 180)); // Blue
    colors.insert("Listening Window", D3Color::rgb(255, 127, 14)); // Orange
    colors.insert("Early Reflections", D3Color::rgb(44, 160, 44)); // Green
    colors.insert("Sound Power", D3Color::rgb(214, 39, 40)); // Red
    colors.insert("Early Reflections DI", D3Color::rgb(148, 103, 189)); // Purple
    colors.insert("Sound Power DI", D3Color::rgb(140, 86, 75)); // Brown
    colors
}

/// Helper function to interpolate between colors in a palette
pub fn interpolate_colors(colors: &[D3Color], t: f64) -> D3Color {
    let idx = (t * (colors.len() - 1) as f64) as usize;
    let idx = idx.min(colors.len() - 2);
    let local_t = (t * (colors.len() - 1) as f64) - idx as f64;
    colors[idx].interpolate(&colors[idx + 1], local_t as f32)
}
