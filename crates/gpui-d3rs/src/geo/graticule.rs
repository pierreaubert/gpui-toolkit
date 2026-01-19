//! Graticule - latitude/longitude grid lines
//!
//! This module provides functionality for generating graticule lines
//! (latitude and longitude grid lines) for map displays.

use super::EPSILON;

/// Configuration for a graticule.
#[derive(Clone, Debug)]
pub struct GraticuleConfig {
    /// Major extent: [[lon0, lat0], [lon1, lat1]]
    pub extent_major: [[f64; 2]; 2],
    /// Minor extent: [[lon0, lat0], [lon1, lat1]]
    pub extent_minor: [[f64; 2]; 2],
    /// Major step: [lon_step, lat_step]
    pub step_major: [f64; 2],
    /// Minor step: [lon_step, lat_step]
    pub step_minor: [f64; 2],
    /// Precision for line generation
    pub precision: f64,
}

impl Default for GraticuleConfig {
    fn default() -> Self {
        Self {
            extent_major: [[-180.0, -90.0 + EPSILON], [180.0, 90.0 - EPSILON]],
            extent_minor: [[-180.0, -80.0 - EPSILON], [180.0, 80.0 + EPSILON]],
            step_major: [90.0, 360.0],
            step_minor: [10.0, 10.0],
            precision: 2.5,
        }
    }
}

/// A graticule generator for creating latitude/longitude grid lines.
#[derive(Clone, Debug)]
pub struct Graticule {
    config: GraticuleConfig,
}

impl Default for Graticule {
    fn default() -> Self {
        Self::new()
    }
}

impl Graticule {
    /// Create a new graticule with default configuration.
    pub fn new() -> Self {
        Self {
            config: GraticuleConfig::default(),
        }
    }

    /// Set the extent for both major and minor lines.
    pub fn extent(mut self, extent: [[f64; 2]; 2]) -> Self {
        self.config.extent_major = extent;
        self.config.extent_minor = extent;
        self
    }

    /// Set the major extent.
    pub fn extent_major(mut self, extent: [[f64; 2]; 2]) -> Self {
        self.config.extent_major = extent;
        self
    }

    /// Set the minor extent.
    pub fn extent_minor(mut self, extent: [[f64; 2]; 2]) -> Self {
        self.config.extent_minor = extent;
        self
    }

    /// Set the step for both major and minor lines.
    pub fn step(mut self, step: [f64; 2]) -> Self {
        self.config.step_major = step;
        self.config.step_minor = step;
        self
    }

    /// Set the major step.
    pub fn step_major(mut self, step: [f64; 2]) -> Self {
        self.config.step_major = step;
        self
    }

    /// Set the minor step.
    pub fn step_minor(mut self, step: [f64; 2]) -> Self {
        self.config.step_minor = step;
        self
    }

    /// Set the precision for line generation.
    pub fn precision(mut self, precision: f64) -> Self {
        self.config.precision = precision;
        self
    }

    /// Generate all graticule lines as a MultiLineString.
    ///
    /// Returns a vector of line coordinates, where each line is a vector of (lon, lat) points.
    pub fn lines(&self) -> Vec<Vec<(f64, f64)>> {
        let mut result = Vec::new();

        let [x0, y0] = self.config.extent_minor[0];
        let [x1, y1] = self.config.extent_minor[1];
        let [dx, dy] = self.config.step_minor;
        let [major_dx, major_dy] = self.config.step_major;
        let [major_x0, major_y0] = self.config.extent_major[0];
        let [major_x1, major_y1] = self.config.extent_major[1];

        // Generate major meridians (vertical lines at major longitude intervals)
        let start_x = (major_x0 / major_dx).ceil() * major_dx;
        let mut x = start_x;
        while x < major_x1 {
            result.push(self.graticule_x(x, major_y0, major_y1));
            x += major_dx;
        }

        // Generate major parallels (horizontal lines at major latitude intervals)
        let start_y = (major_y0 / major_dy).ceil() * major_dy;
        let mut y = start_y;
        while y < major_y1 {
            result.push(self.graticule_y(y, major_x0, major_x1));
            y += major_dy;
        }

        // Generate minor meridians (excluding major ones)
        let start_x = (x0 / dx).ceil() * dx;
        let mut x = start_x;
        while x < x1 {
            if (x % major_dx).abs() > EPSILON {
                result.push(self.graticule_x(x, y0, y1));
            }
            x += dx;
        }

        // Generate minor parallels (excluding major ones)
        let start_y = (y0 / dy).ceil() * dy;
        let mut y = start_y;
        while y < y1 {
            if (y % major_dy).abs() > EPSILON {
                result.push(self.graticule_y(y, x0, x1));
            }
            y += dy;
        }

        result
    }

    /// Generate the outline polygon of the graticule extent.
    ///
    /// Returns the coordinates of a closed polygon outlining the graticule.
    pub fn outline(&self) -> Vec<(f64, f64)> {
        let [x0, y0] = self.config.extent_major[0];
        let [x1, y1] = self.config.extent_major[1];

        let mut coords = Vec::new();

        // Top edge (west to east at north)
        let top = self.graticule_x(x0, y0, y1);
        coords.extend(top);

        // Right edge (north to south at east)
        let right = self.graticule_y(y1, x0, x1);
        for (i, coord) in right.into_iter().enumerate() {
            if i > 0 {
                coords.push(coord);
            }
        }

        // Bottom edge (east to west at south)
        let mut bottom = self.graticule_x(x1, y0, y1);
        bottom.reverse();
        for (i, coord) in bottom.into_iter().enumerate() {
            if i > 0 {
                coords.push(coord);
            }
        }

        // Left edge (south to north at west)
        let mut left = self.graticule_y(y0, x0, x1);
        left.reverse();
        for (i, coord) in left.into_iter().enumerate() {
            if i > 0 {
                coords.push(coord);
            }
        }

        // Close the polygon
        if let Some(first) = coords.first().copied() {
            coords.push(first);
        }

        coords
    }

    /// Generate a meridian line (constant longitude).
    fn graticule_x(&self, x: f64, y0: f64, y1: f64) -> Vec<(f64, f64)> {
        let step = 90.0; // Always use 90 degree steps for meridians
        let mut coords = Vec::new();

        let mut y = y0;
        while y < y1 - EPSILON {
            coords.push((x, y));
            y += step;
        }
        coords.push((x, y1));

        coords
    }

    /// Generate a parallel line (constant latitude).
    fn graticule_y(&self, y: f64, x0: f64, x1: f64) -> Vec<(f64, f64)> {
        let step = self.config.precision;
        let mut coords = Vec::new();

        let mut x = x0;
        while x < x1 - EPSILON {
            coords.push((x, y));
            x += step;
        }
        coords.push((x1, y));

        coords
    }
}

/// Create a default graticule with 10-degree spacing.
#[allow(dead_code)]
pub fn graticule10() -> Vec<Vec<(f64, f64)>> {
    Graticule::new().lines()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graticule_default() {
        let g = Graticule::new();
        let lines = g.lines();
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_graticule_lines_count() {
        let g = Graticule::new().step([30.0, 30.0]);
        let lines = g.lines();
        // Should have meridians and parallels
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_graticule_outline() {
        let g = Graticule::new();
        let outline = g.outline();
        assert!(!outline.is_empty());
        // Should be a closed polygon
        assert_eq!(outline.first(), outline.last());
    }

    #[test]
    fn test_graticule_custom_extent() {
        let g = Graticule::new().extent([[-90.0, -45.0], [90.0, 45.0]]);
        let lines = g.lines();
        // All coordinates should be within extent
        for line in &lines {
            for &(lon, lat) in line {
                assert!(lon >= -90.0 - EPSILON && lon <= 90.0 + EPSILON);
                assert!(lat >= -45.0 - EPSILON && lat <= 45.0 + EPSILON);
            }
        }
    }

    #[test]
    fn test_graticule10() {
        let lines = graticule10();
        assert!(!lines.is_empty());
    }
}
