//! Volcano elevation data
//!
//! This is a synthetic volcano terrain dataset inspired by the classic R/D3.js
//! Maungawhau (Mt. Eden) volcano dataset.
//!
//! Original data source: R's built-in `volcano` dataset
//! Reference: https://observablehq.com/@d3/volcano-contours
//!
//! Dimensions: 87 rows x 61 columns (5307 total values)
//! Elevation range: ~94m to ~195m

/// Volcano grid width (number of columns)
pub const VOLCANO_WIDTH: usize = 61;

/// Volcano grid height (number of rows)
pub const VOLCANO_HEIGHT: usize = 87;

/// Generate synthetic volcano elevation data
///
/// Creates a realistic volcanic cone with:
/// - Central peak with slight offset
/// - Secondary smaller peak
/// - Crater depression at the top
/// - Gradual slopes with some noise
pub fn generate_volcano_data() -> Vec<f64> {
    let mut values = vec![0.0; VOLCANO_WIDTH * VOLCANO_HEIGHT];

    // Parameters for the volcanic shape
    let center_x = 0.52; // Slightly off-center (normalized 0-1)
    let center_y = 0.48;
    let base_elevation = 94.0;
    let peak_elevation = 195.0;

    for row in 0..VOLCANO_HEIGHT {
        for col in 0..VOLCANO_WIDTH {
            let x = col as f64 / (VOLCANO_WIDTH - 1) as f64;
            let y = row as f64 / (VOLCANO_HEIGHT - 1) as f64;

            // Distance from center (normalized)
            let dx = x - center_x;
            let dy = y - center_y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Main volcanic cone shape (Gaussian-like)
            let cone_radius = 0.35;
            let cone_height = peak_elevation - base_elevation;
            let cone = if dist < cone_radius * 1.5 {
                cone_height * (-2.5 * (dist / cone_radius).powi(2)).exp()
            } else {
                0.0
            };

            // Crater depression at the very top
            let crater_radius = 0.08;
            let crater_depth = 15.0;
            let crater = if dist < crater_radius {
                let t = dist / crater_radius;
                -crater_depth * (1.0 - t * t)
            } else {
                0.0
            };

            // Secondary bump (ridge)
            let bump_x = 0.35;
            let bump_y = 0.55;
            let bump_dx = x - bump_x;
            let bump_dy = y - bump_y;
            let bump_dist = (bump_dx * bump_dx + bump_dy * bump_dy).sqrt();
            let bump = 20.0 * (-8.0 * bump_dist * bump_dist).exp();

            // Add some terrain variation using simple pseudo-random noise
            // Using a deterministic function of position for reproducibility
            let noise = terrain_noise(x, y) * 8.0;

            // Combine all components
            let elevation = base_elevation + cone + crater + bump + noise;

            // Clamp to realistic range
            let elevation = elevation.clamp(base_elevation, peak_elevation);

            values[row * VOLCANO_WIDTH + col] = elevation;
        }
    }

    values
}

/// Simple deterministic noise function for terrain variation
fn terrain_noise(x: f64, y: f64) -> f64 {
    // Multi-octave noise approximation
    let mut value = 0.0;
    let mut amplitude = 1.0;

    for octave in 0..3 {
        let freq = (1 << octave) as f64;
        let nx = x * freq * 5.0;
        let ny = y * freq * 5.0;

        // Simple hash-based pseudo-noise
        let hash = ((nx.sin() * 12.9898 + ny.cos() * 78.233) * 43758.5453).fract();
        value += hash * amplitude;
        amplitude *= 0.5;
    }

    // Normalize to -1..1 range, then scale by distance from center
    let center_dist = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();
    let falloff = (1.0 - center_dist * 2.0).max(0.0);

    (value * 2.0 - 1.5) * falloff
}

/// Get the elevation value range
#[allow(dead_code)]
pub fn volcano_extent() -> (f64, f64) {
    let data = generate_volcano_data();
    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dimensions() {
        let data = generate_volcano_data();
        assert_eq!(data.len(), VOLCANO_WIDTH * VOLCANO_HEIGHT);
    }

    #[test]
    fn test_elevation_range() {
        let (min, max) = volcano_extent();
        assert!(min >= 90.0, "Min elevation should be >= 90, got {}", min);
        assert!(max <= 200.0, "Max elevation should be <= 200, got {}", max);
        assert!(
            max - min > 50.0,
            "Should have significant elevation range, got {}",
            max - min
        );
    }
}
