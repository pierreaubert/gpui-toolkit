//! Surface data structures for 3D visualization

use glam::Vec3;

/// A single vertex in the surface mesh
#[derive(Debug, Clone, Copy)]
pub struct SurfaceVertex {
    /// Position in 3D space (normalized to [-1, 1] range)
    pub position: Vec3,
    /// Normal vector for lighting calculations
    pub normal: Vec3,
    /// Original data value (for colormap lookup)
    pub value: f32,
    /// UV coordinates for texture mapping (optional)
    pub uv: [f32; 2],
}

impl SurfaceVertex {
    /// Create a new surface vertex
    pub fn new(position: Vec3, normal: Vec3, value: f32) -> Self {
        Self {
            position,
            normal,
            value,
            uv: [0.0, 0.0],
        }
    }

    /// Create vertex with UV coordinates
    pub fn with_uv(mut self, u: f32, v: f32) -> Self {
        self.uv = [u, v];
        self
    }
}

/// Surface data container for 3D visualization
#[derive(Debug, Clone)]
pub struct SurfaceData {
    /// X-axis values (domain)
    pub x_values: Vec<f64>,
    /// Y-axis values (domain)
    pub y_values: Vec<f64>,
    /// Z values as a 2D grid [y][x] (row-major order)
    pub z_values: Vec<Vec<f64>>,
    /// Minimum Z value in the dataset
    pub z_min: f64,
    /// Maximum Z value in the dataset
    pub z_max: f64,
    /// X-axis label
    pub x_label: Option<String>,
    /// Y-axis label
    pub y_label: Option<String>,
    /// Z-axis label
    pub z_label: Option<String>,
    /// Whether X-axis is logarithmic
    pub x_log: bool,
    /// Whether Y-axis is logarithmic
    pub y_log: bool,
    /// Minimum X value (domain)
    pub x_min: f64,
    /// Maximum X value (domain)
    pub x_max: f64,
    /// Minimum Y value (domain)
    pub y_min: f64,
    /// Maximum Y value (domain)
    pub y_max: f64,
    /// Whether Z-axis is logarithmic
    pub z_log: bool,
    /// Custom ticks for X-axis
    pub x_ticks: Option<Vec<f64>>,
    /// Custom ticks for Y-axis
    pub y_ticks: Option<Vec<f64>>,
    /// Custom ticks for Z-axis
    pub z_ticks: Option<Vec<f64>>,
}

impl SurfaceData {
    /// Create new surface data from grid values
    ///
    /// # Arguments
    /// * `x_values` - X-axis coordinates
    /// * `y_values` - Y-axis coordinates
    /// * `z_values` - Z values as [y][x] grid (row-major)
    pub fn from_grid(x_values: Vec<f64>, y_values: Vec<f64>, z_values: Vec<Vec<f64>>) -> Self {
        let (z_min, z_max) = Self::compute_z_range(&z_values);
        let x_min = x_values.first().copied().unwrap_or(0.0);
        let x_max = x_values.last().copied().unwrap_or(1.0);
        let y_min = y_values.first().copied().unwrap_or(0.0);
        let y_max = y_values.last().copied().unwrap_or(1.0);

        Self {
            x_values,
            y_values,
            z_values,
            z_min,
            z_max,
            x_label: None,
            y_label: None,
            z_label: None,
            x_log: false,
            y_log: false,
            x_min,
            x_max,
            y_min,
            y_max,
            z_log: false,
            x_ticks: None,
            y_ticks: None,
            z_ticks: None,
        }
    }

    /// Create surface data from a function z = f(x, y)
    pub fn from_function<F>(
        x_range: (f64, f64),
        y_range: (f64, f64),
        x_steps: usize,
        y_steps: usize,
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> f64,
    {
        let x_values: Vec<f64> = (0..x_steps)
            .map(|i| x_range.0 + (x_range.1 - x_range.0) * (i as f64) / (x_steps - 1) as f64)
            .collect();

        let y_values: Vec<f64> = (0..y_steps)
            .map(|i| y_range.0 + (y_range.1 - y_range.0) * (i as f64) / (y_steps - 1) as f64)
            .collect();

        let z_values: Vec<Vec<f64>> = y_values
            .iter()
            .map(|&y| x_values.iter().map(|&x| f(x, y)).collect())
            .collect();

        Self::from_grid(x_values, y_values, z_values)
    }

    /// Set X-axis label
    pub fn with_x_label(mut self, label: impl Into<String>) -> Self {
        self.x_label = Some(label.into());
        self
    }

    /// Set Y-axis label
    pub fn with_y_label(mut self, label: impl Into<String>) -> Self {
        self.y_label = Some(label.into());
        self
    }

    /// Set Z-axis label
    pub fn with_z_label(mut self, label: impl Into<String>) -> Self {
        self.z_label = Some(label.into());
        self
    }

    /// Set logarithmic X-axis
    pub fn with_log_x(mut self, log: bool) -> Self {
        self.x_log = log;
        self
    }

    /// Set logarithmic Y-axis
    pub fn with_log_y(mut self, log: bool) -> Self {
        self.y_log = log;
        self
    }

    /// Set logarithmic Z-axis
    pub fn with_log_z(mut self, log: bool) -> Self {
        self.z_log = log;
        self
    }

    /// Set custom X-axis ticks
    pub fn with_x_ticks(mut self, ticks: Vec<f64>) -> Self {
        self.x_ticks = Some(ticks);
        self
    }

    /// Set custom Y-axis ticks
    pub fn with_y_ticks(mut self, ticks: Vec<f64>) -> Self {
        self.y_ticks = Some(ticks);
        self
    }

    /// Set custom Z-axis ticks
    pub fn with_z_ticks(mut self, ticks: Vec<f64>) -> Self {
        self.z_ticks = Some(ticks);
        self
    }

    /// Override X range
    pub fn with_x_range(mut self, min: f64, max: f64) -> Self {
        self.x_min = min;
        self.x_max = max;
        self
    }

    /// Override Y range
    pub fn with_y_range(mut self, min: f64, max: f64) -> Self {
        self.y_min = min;
        self.y_max = max;
        self
    }

    /// Override Z range (useful for consistent colormap scaling)
    pub fn with_z_range(mut self, min: f64, max: f64) -> Self {
        self.z_min = min;
        self.z_max = max;
        self
    }

    /// Get number of X points
    pub fn x_count(&self) -> usize {
        self.x_values.len()
    }

    /// Get number of Y points
    pub fn y_count(&self) -> usize {
        self.y_values.len()
    }

    /// Get Z value at grid position, returns None if out of bounds
    pub fn z_at(&self, xi: usize, yi: usize) -> Option<f64> {
        self.z_values.get(yi).and_then(|row| row.get(xi).copied())
    }

    /// Normalize a Z value to [0, 1] range
    pub fn normalize_z(&self, z: f64) -> f32 {
        if (self.z_max - self.z_min).abs() < 1e-10 {
            0.5
        } else if self.z_log {
            // Log scale for Z
            // Avoid log of <= 0 if data allows. If data has <= 0, map to small epsilon?
            // Assuming positive data for now if log is requested, or caller handles it.
            // Using similar logic to X/Y using normalized log range.
            // Wait, normalize_z returns [0, 1], X/Y returned [-1, 1].
            let z_val = if z <= 0.0 { 1e-10 } else { z };
            let min_val = if self.z_min <= 0.0 { 1e-10 } else { self.z_min };
            let max_val = if self.z_max <= 0.0 { 1e-10 } else { self.z_max };

            let log_min = min_val.ln();
            let log_max = max_val.ln();
            let log_z = z_val.ln();

            if (log_max - log_min).abs() < 1e-10 {
                0.5
            } else {
                ((log_z - log_min) / (log_max - log_min)) as f32
            }
        } else {
            ((z - self.z_min) / (self.z_max - self.z_min)) as f32
        }
    }

    /// Normalize X value to [-1, 1] range (accounting for log scale)
    pub fn normalize_x(&self, x: f64) -> f32 {
        if (self.x_max - self.x_min).abs() < 1e-10 {
            return 0.0;
        }

        if self.x_log {
            let log_min = self.x_min.ln();
            let log_max = self.x_max.ln();
            let log_x = x.ln();
            (2.0 * (log_x - log_min) / (log_max - log_min) - 1.0) as f32
        } else {
            (2.0 * (x - self.x_min) / (self.x_max - self.x_min) - 1.0) as f32
        }
    }

    /// Normalize Y value to [-1, 1] range (accounting for log scale)
    pub fn normalize_y(&self, y: f64) -> f32 {
        if (self.y_max - self.y_min).abs() < 1e-10 {
            return 0.0;
        }

        if self.y_log {
            let log_min = self.y_min.ln();
            let log_max = self.y_max.ln();
            let log_y = y.ln();
            (2.0 * (log_y - log_min) / (log_max - log_min) - 1.0) as f32
        } else {
            (2.0 * (y - self.y_min) / (self.y_max - self.y_min) - 1.0) as f32
        }
    }

    fn compute_z_range(z_values: &[Vec<f64>]) -> (f64, f64) {
        let mut z_min = f64::INFINITY;
        let mut z_max = f64::NEG_INFINITY;

        for row in z_values {
            for &z in row {
                if z.is_finite() {
                    z_min = z_min.min(z);
                    z_max = z_max.max(z);
                }
            }
        }

        if z_min.is_infinite() {
            z_min = 0.0;
        }
        if z_max.is_infinite() {
            z_max = 1.0;
        }

        (z_min, z_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_data_from_function() {
        let data =
            SurfaceData::from_function((-1.0, 1.0), (-1.0, 1.0), 10, 10, |x, y| x * x + y * y);

        assert_eq!(data.x_count(), 10);
        assert_eq!(data.y_count(), 10);
        assert!(data.z_min >= 0.0);
        assert!(data.z_max <= 2.0);
    }

    #[test]
    fn test_normalize_z() {
        let data = SurfaceData::from_function((-1.0, 1.0), (-1.0, 1.0), 5, 5, |x, y| x + y);

        // z ranges from -2 to 2
        assert!((data.normalize_z(-2.0) - 0.0).abs() < 0.01);
        assert!((data.normalize_z(2.0) - 1.0).abs() < 0.01);
        assert!((data.normalize_z(0.0) - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_normalize_x_linear() {
        let data = SurfaceData::from_function((0.0, 100.0), (0.0, 1.0), 11, 2, |_, _| 0.0);

        assert!((data.normalize_x(0.0) - (-1.0)).abs() < 0.01);
        assert!((data.normalize_x(100.0) - 1.0).abs() < 0.01);
        assert!((data.normalize_x(50.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_x_log() {
        let data = SurfaceData::from_function((10.0, 1000.0), (0.0, 1.0), 3, 2, |_, _| 0.0)
            .with_log_x(true);

        // For log scale: 10 -> -1, 1000 -> 1, 100 (geometric mean) -> 0
        assert!((data.normalize_x(10.0) - (-1.0)).abs() < 0.01);
        assert!((data.normalize_x(1000.0) - 1.0).abs() < 0.01);
        assert!((data.normalize_x(100.0) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_normalize_z_log() {
        let data = SurfaceData::from_function((-1.0, 1.0), (-1.0, 1.0), 2, 2, |_, _| 0.0)
            .with_z_range(10.0, 1000.0)
            .with_log_z(true);

        // normalize_z maps to [0, 1]
        // 10 -> 0.0
        // 1000 -> 1.0
        // 100 (geometric mean) -> 0.5
        assert!((data.normalize_z(10.0) - 0.0).abs() < 0.01);
        assert!((data.normalize_z(1000.0) - 1.0).abs() < 0.01);
        assert!((data.normalize_z(100.0) - 0.5).abs() < 0.01);
    }
}
