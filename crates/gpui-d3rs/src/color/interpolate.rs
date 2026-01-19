//! Color interpolation utilities

use super::D3Color;

/// Interpolate between multiple colors using a normalized value (0.0 - 1.0)
///
/// This creates a gradient across multiple color stops.
///
/// # Example
///
/// ```
/// use d3rs::color::{D3Color, interpolate_colors};
///
/// let colors = vec![
///     D3Color::rgb(255, 0, 0),    // Red
///     D3Color::rgb(255, 255, 0),  // Yellow
///     D3Color::rgb(0, 255, 0),    // Green
/// ];
///
/// let color = interpolate_colors(&colors, 0.5); // Yellow
/// ```
pub fn interpolate_colors(colors: &[D3Color], t: f32) -> D3Color {
    if colors.is_empty() {
        return D3Color::rgb(0, 0, 0);
    }

    if colors.len() == 1 {
        return colors[0];
    }

    let t = t.clamp(0.0, 1.0);
    let segment_count = colors.len() - 1;
    let scaled_t = t * segment_count as f32;
    let segment_index = scaled_t.floor() as usize;

    if segment_index >= segment_count {
        return colors[colors.len() - 1];
    }

    let local_t = scaled_t - segment_index as f32;
    colors[segment_index].interpolate(&colors[segment_index + 1], local_t)
}

/// Create a sequential color scale from a value in [0.0, 1.0]
///
/// Uses a common blue → white → red gradient for diverging data.
pub fn sequential_color(t: f32) -> D3Color {
    let colors = vec![
        D3Color::from_hex(0x0571b0), // Dark blue
        D3Color::from_hex(0x92c5de), // Light blue
        D3Color::from_hex(0xf7f7f7), // White
        D3Color::from_hex(0xf4a582), // Light red
        D3Color::from_hex(0xca0020), // Dark red
    ];
    interpolate_colors(&colors, t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_interpolate_colors_two() {
        let colors = vec![D3Color::rgb(255, 0, 0), D3Color::rgb(0, 0, 255)];

        let start = interpolate_colors(&colors, 0.0);
        assert_relative_eq!(start.r, 1.0);

        let mid = interpolate_colors(&colors, 0.5);
        assert_relative_eq!(mid.r, 0.5, epsilon = 1e-6);
        assert_relative_eq!(mid.b, 0.5, epsilon = 1e-6);

        let end = interpolate_colors(&colors, 1.0);
        assert_relative_eq!(end.b, 1.0);
    }

    #[test]
    fn test_interpolate_colors_three() {
        let colors = vec![
            D3Color::rgb(255, 0, 0), // Red
            D3Color::rgb(0, 255, 0), // Green
            D3Color::rgb(0, 0, 255), // Blue
        ];

        let quarter = interpolate_colors(&colors, 0.25);
        // Should be halfway between red and green
        assert_relative_eq!(quarter.r, 0.5, epsilon = 1e-6);
        assert_relative_eq!(quarter.g, 0.5, epsilon = 1e-6);

        let three_quarters = interpolate_colors(&colors, 0.75);
        // Should be halfway between green and blue
        assert_relative_eq!(three_quarters.g, 0.5, epsilon = 1e-6);
        assert_relative_eq!(three_quarters.b, 0.5, epsilon = 1e-6);
    }

    #[test]
    fn test_interpolate_colors_edge_cases() {
        let colors = vec![D3Color::rgb(255, 0, 0)];
        let color = interpolate_colors(&colors, 0.5);
        assert_relative_eq!(color.r, 1.0);

        let empty: Vec<D3Color> = vec![];
        let black = interpolate_colors(&empty, 0.5);
        assert_relative_eq!(black.r, 0.0);
        assert_relative_eq!(black.g, 0.0);
        assert_relative_eq!(black.b, 0.0);
    }

    #[test]
    fn test_sequential_color() {
        let blue_end = sequential_color(0.0);
        let red_end = sequential_color(1.0);
        let middle = sequential_color(0.5);

        // Blue end should be bluish
        assert!(blue_end.b > blue_end.r);
        // Red end should be reddish
        assert!(red_end.r > red_end.b);
        // Middle should be near white
        assert!(middle.r > 0.8 && middle.g > 0.8 && middle.b > 0.8);
    }
}
