//! Surface data structures

/// A single point on a 3D surface with a color-mapping value
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePoint3D {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Z coordinate (height)
    pub z: f64,
    /// Value for color mapping (can be different from z)
    pub t: f64,
}

impl SurfacePoint3D {
    /// Create a new surface point
    pub fn new(x: f64, y: f64, z: f64, t: f64) -> Self {
        Self { x, y, z, t }
    }

    /// Create a point where t equals z
    pub fn from_xyz(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, t: z }
    }
}

/// Container for surface data stored as a 2D grid of points
#[derive(Clone, Debug)]
pub struct SurfaceData {
    /// Grid of points [row][col]
    points: Vec<Vec<SurfacePoint3D>>,
    /// Computed range of x values
    pub x_range: (f64, f64),
    /// Computed range of y values
    pub y_range: (f64, f64),
    /// Computed range of z values
    pub z_range: (f64, f64),
    /// Computed range of t values (for color mapping)
    pub t_range: (f64, f64),
}

impl SurfaceData {
    /// Create a new empty surface data container
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            x_range: (0.0, 1.0),
            y_range: (0.0, 1.0),
            z_range: (0.0, 1.0),
            t_range: (0.0, 1.0),
        }
    }

    /// Create surface data from a pre-built grid
    ///
    /// Automatically computes ranges from the data.
    pub fn from_grid(points: Vec<Vec<SurfacePoint3D>>) -> Self {
        let mut data = Self {
            points,
            x_range: (f64::INFINITY, f64::NEG_INFINITY),
            y_range: (f64::INFINITY, f64::NEG_INFINITY),
            z_range: (f64::INFINITY, f64::NEG_INFINITY),
            t_range: (f64::INFINITY, f64::NEG_INFINITY),
        };
        data.compute_ranges();
        data
    }

    /// Create surface data from a mathematical function
    ///
    /// # Arguments
    /// * `x_range` - (min, max) range for x values
    /// * `y_range` - (min, max) range for y values
    /// * `resolution` - Number of points in each dimension
    /// * `f` - Function that takes (x, y) and returns (z, t)
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::surface::SurfaceData;
    ///
    /// let data = SurfaceData::from_function(
    ///     (-2.0, 2.0),
    ///     (-2.0, 2.0),
    ///     50,
    ///     |x, y| {
    ///         let z = (x*x + y*y).sin();
    ///         (z, z)  // Color by z value
    ///     },
    /// );
    /// ```
    pub fn from_function<F>(
        x_range: (f64, f64),
        y_range: (f64, f64),
        resolution: usize,
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> (f64, f64),
    {
        let resolution = resolution.max(2);
        let x_step = (x_range.1 - x_range.0) / (resolution - 1) as f64;
        let y_step = (y_range.1 - y_range.0) / (resolution - 1) as f64;

        let mut points = Vec::with_capacity(resolution);

        for j in 0..resolution {
            let y = y_range.0 + j as f64 * y_step;
            let mut row = Vec::with_capacity(resolution);

            for i in 0..resolution {
                let x = x_range.0 + i as f64 * x_step;
                let (z, t) = f(x, y);
                row.push(SurfacePoint3D::new(x, y, z, t));
            }

            points.push(row);
        }

        Self::from_grid(points)
    }

    /// Create surface data where z = f(x, y) and t = z
    pub fn from_z_function<F>(
        x_range: (f64, f64),
        y_range: (f64, f64),
        resolution: usize,
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> f64,
    {
        Self::from_function(x_range, y_range, resolution, |x, y| {
            let z = f(x, y);
            (z, z)
        })
    }

    /// Create surface data from a function with logarithmic X-axis sampling
    ///
    /// # Arguments
    /// * `x_range` - (min, max) range for x values (must be positive)
    /// * `y_range` - (min, max) range for y values (linear)
    /// * `resolution` - Number of points in each dimension
    /// * `f` - Function that takes (x, y) and returns (z, t)
    ///
    /// # Panics
    /// Panics in debug mode if x_range contains non-positive values.
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::surface::SurfaceData;
    ///
    /// // Frequency response plot: x is frequency in Hz (log scale)
    /// let data = SurfaceData::from_function_logx(
    ///     (20.0, 20000.0),  // 20 Hz to 20 kHz
    ///     (0.0, 1.0),       // Linear Y
    ///     100,
    ///     |freq, y| {
    ///         let z = (freq / 1000.0).ln();
    ///         (z, z)
    ///     },
    /// );
    /// ```
    pub fn from_function_logx<F>(
        x_range: (f64, f64),
        y_range: (f64, f64),
        resolution: usize,
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> (f64, f64),
    {
        debug_assert!(
            x_range.0 > 0.0 && x_range.1 > 0.0,
            "Logarithmic X range must contain only positive values, got ({}, {})",
            x_range.0,
            x_range.1
        );

        let resolution = resolution.max(2);
        let log_x_min = x_range.0.ln();
        let log_x_max = x_range.1.ln();
        let log_x_step = (log_x_max - log_x_min) / (resolution - 1) as f64;
        let y_step = (y_range.1 - y_range.0) / (resolution - 1) as f64;

        let mut points = Vec::with_capacity(resolution);

        for j in 0..resolution {
            let y = y_range.0 + j as f64 * y_step;
            let mut row = Vec::with_capacity(resolution);

            for i in 0..resolution {
                let log_x = log_x_min + i as f64 * log_x_step;
                let x = log_x.exp();
                let (z, t) = f(x, y);
                row.push(SurfacePoint3D::new(x, y, z, t));
            }

            points.push(row);
        }

        Self::from_grid(points)
    }

    /// Create surface data from a function with logarithmic Y-axis sampling
    ///
    /// # Arguments
    /// * `x_range` - (min, max) range for x values (linear)
    /// * `y_range` - (min, max) range for y values (must be positive)
    /// * `resolution` - Number of points in each dimension
    /// * `f` - Function that takes (x, y) and returns (z, t)
    ///
    /// # Panics
    /// Panics in debug mode if y_range contains non-positive values.
    pub fn from_function_logy<F>(
        x_range: (f64, f64),
        y_range: (f64, f64),
        resolution: usize,
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> (f64, f64),
    {
        debug_assert!(
            y_range.0 > 0.0 && y_range.1 > 0.0,
            "Logarithmic Y range must contain only positive values, got ({}, {})",
            y_range.0,
            y_range.1
        );

        let resolution = resolution.max(2);
        let x_step = (x_range.1 - x_range.0) / (resolution - 1) as f64;
        let log_y_min = y_range.0.ln();
        let log_y_max = y_range.1.ln();
        let log_y_step = (log_y_max - log_y_min) / (resolution - 1) as f64;

        let mut points = Vec::with_capacity(resolution);

        for j in 0..resolution {
            let log_y = log_y_min + j as f64 * log_y_step;
            let y = log_y.exp();
            let mut row = Vec::with_capacity(resolution);

            for i in 0..resolution {
                let x = x_range.0 + i as f64 * x_step;
                let (z, t) = f(x, y);
                row.push(SurfacePoint3D::new(x, y, z, t));
            }

            points.push(row);
        }

        Self::from_grid(points)
    }

    /// Create surface data from a function with logarithmic X and Y axis sampling
    ///
    /// # Arguments
    /// * `x_range` - (min, max) range for x values (must be positive)
    /// * `y_range` - (min, max) range for y values (must be positive)
    /// * `resolution` - Number of points in each dimension
    /// * `f` - Function that takes (x, y) and returns (z, t)
    ///
    /// # Panics
    /// Panics in debug mode if either range contains non-positive values.
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::surface::SurfaceData;
    ///
    /// // 2D frequency domain plot
    /// let data = SurfaceData::from_function_logxy(
    ///     (20.0, 20000.0),   // X: 20 Hz to 20 kHz
    ///     (20.0, 20000.0),   // Y: 20 Hz to 20 kHz
    ///     50,
    ///     |fx, fy| {
    ///         let z = ((fx * fy).ln() / 1000.0).sin();
    ///         (z, z)
    ///     },
    /// );
    /// ```
    pub fn from_function_logxy<F>(
        x_range: (f64, f64),
        y_range: (f64, f64),
        resolution: usize,
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> (f64, f64),
    {
        debug_assert!(
            x_range.0 > 0.0 && x_range.1 > 0.0,
            "Logarithmic X range must contain only positive values, got ({}, {})",
            x_range.0,
            x_range.1
        );
        debug_assert!(
            y_range.0 > 0.0 && y_range.1 > 0.0,
            "Logarithmic Y range must contain only positive values, got ({}, {})",
            y_range.0,
            y_range.1
        );

        let resolution = resolution.max(2);
        let log_x_min = x_range.0.ln();
        let log_x_max = x_range.1.ln();
        let log_x_step = (log_x_max - log_x_min) / (resolution - 1) as f64;
        let log_y_min = y_range.0.ln();
        let log_y_max = y_range.1.ln();
        let log_y_step = (log_y_max - log_y_min) / (resolution - 1) as f64;

        let mut points = Vec::with_capacity(resolution);

        for j in 0..resolution {
            let log_y = log_y_min + j as f64 * log_y_step;
            let y = log_y.exp();
            let mut row = Vec::with_capacity(resolution);

            for i in 0..resolution {
                let log_x = log_x_min + i as f64 * log_x_step;
                let x = log_x.exp();
                let (z, t) = f(x, y);
                row.push(SurfacePoint3D::new(x, y, z, t));
            }

            points.push(row);
        }

        Self::from_grid(points)
    }

    /// Create surface data with logarithmic X-axis sampling where z = f(x, y) and t = z
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::surface::SurfaceData;
    ///
    /// // Audio frequency response visualization
    /// let data = SurfaceData::from_z_function_logx(
    ///     (20.0, 20000.0),  // Frequency in Hz
    ///     (0.0, 1.0),
    ///     100,
    ///     |freq, y| (freq / 1000.0).ln(),
    /// );
    /// ```
    pub fn from_z_function_logx<F>(
        x_range: (f64, f64),
        y_range: (f64, f64),
        resolution: usize,
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> f64,
    {
        Self::from_function_logx(x_range, y_range, resolution, |x, y| {
            let z = f(x, y);
            (z, z)
        })
    }

    /// Create surface data with logarithmic Y-axis sampling where z = f(x, y) and t = z
    pub fn from_z_function_logy<F>(
        x_range: (f64, f64),
        y_range: (f64, f64),
        resolution: usize,
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> f64,
    {
        Self::from_function_logy(x_range, y_range, resolution, |x, y| {
            let z = f(x, y);
            (z, z)
        })
    }

    /// Create surface data with logarithmic X and Y axis sampling where z = f(x, y) and t = z
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::surface::SurfaceData;
    ///
    /// // 2D frequency domain
    /// let data = SurfaceData::from_z_function_logxy(
    ///     (20.0, 20000.0),
    ///     (20.0, 20000.0),
    ///     50,
    ///     |fx, fy| ((fx * fy).sqrt() / 1000.0).ln(),
    /// );
    /// ```
    pub fn from_z_function_logxy<F>(
        x_range: (f64, f64),
        y_range: (f64, f64),
        resolution: usize,
        f: F,
    ) -> Self
    where
        F: Fn(f64, f64) -> f64,
    {
        Self::from_function_logxy(x_range, y_range, resolution, |x, y| {
            let z = f(x, y);
            (z, z)
        })
    }

    /// Compute ranges from the data
    fn compute_ranges(&mut self) {
        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;
        let mut z_min = f64::INFINITY;
        let mut z_max = f64::NEG_INFINITY;
        let mut t_min = f64::INFINITY;
        let mut t_max = f64::NEG_INFINITY;

        for row in &self.points {
            for p in row {
                x_min = x_min.min(p.x);
                x_max = x_max.max(p.x);
                y_min = y_min.min(p.y);
                y_max = y_max.max(p.y);
                z_min = z_min.min(p.z);
                z_max = z_max.max(p.z);
                t_min = t_min.min(p.t);
                t_max = t_max.max(p.t);
            }
        }

        // Avoid zero-range issues
        if x_min == x_max {
            x_max = x_min + 1.0;
        }
        if y_min == y_max {
            y_max = y_min + 1.0;
        }
        if z_min == z_max {
            z_max = z_min + 1.0;
        }
        if t_min == t_max {
            t_max = t_min + 1.0;
        }

        self.x_range = (x_min, x_max);
        self.y_range = (y_min, y_max);
        self.z_range = (z_min, z_max);
        self.t_range = (t_min, t_max);
    }

    /// Get the grid of points
    pub fn points(&self) -> &Vec<Vec<SurfacePoint3D>> {
        &self.points
    }

    /// Get the number of rows in the grid
    pub fn rows(&self) -> usize {
        self.points.len()
    }

    /// Get the number of columns in the grid
    pub fn cols(&self) -> usize {
        self.points.first().map(|r| r.len()).unwrap_or(0)
    }

    /// Get a specific point by row and column index
    pub fn get(&self, row: usize, col: usize) -> Option<&SurfacePoint3D> {
        self.points.get(row).and_then(|r| r.get(col))
    }

    /// Normalize the data to fit within a unit cube [0,1]^3
    ///
    /// The t values are normalized to [0,1] as well.
    pub fn normalize(&self) -> Self {
        let x_scale = self.x_range.1 - self.x_range.0;
        let y_scale = self.y_range.1 - self.y_range.0;
        let z_scale = self.z_range.1 - self.z_range.0;
        let t_scale = self.t_range.1 - self.t_range.0;

        let points: Vec<Vec<SurfacePoint3D>> = self
            .points
            .iter()
            .map(|row| {
                row.iter()
                    .map(|p| SurfacePoint3D {
                        x: (p.x - self.x_range.0) / x_scale,
                        y: (p.y - self.y_range.0) / y_scale,
                        z: (p.z - self.z_range.0) / z_scale,
                        t: (p.t - self.t_range.0) / t_scale,
                    })
                    .collect()
            })
            .collect();

        Self {
            points,
            x_range: (0.0, 1.0),
            y_range: (0.0, 1.0),
            z_range: (0.0, 1.0),
            t_range: (0.0, 1.0),
        }
    }

    /// Normalize t values only to [0, 1]
    pub fn normalize_t(&self) -> f64 {
        self.t_range.1 - self.t_range.0
    }

    /// Get normalized t value for a point
    pub fn normalized_t(&self, t: f64) -> f64 {
        (t - self.t_range.0) / (self.t_range.1 - self.t_range.0)
    }
}

impl Default for SurfaceData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_point_creation() {
        let p = SurfacePoint3D::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(p.x, 1.0);
        assert_eq!(p.y, 2.0);
        assert_eq!(p.z, 3.0);
        assert_eq!(p.t, 4.0);
    }

    #[test]
    fn test_surface_point_from_xyz() {
        let p = SurfacePoint3D::from_xyz(1.0, 2.0, 3.0);
        assert_eq!(p.t, 3.0);
    }

    #[test]
    fn test_from_function() {
        let data = SurfaceData::from_function((-1.0, 1.0), (-1.0, 1.0), 5, |x, y| (x + y, x * y));

        assert_eq!(data.rows(), 5);
        assert_eq!(data.cols(), 5);
        assert_eq!(data.x_range, (-1.0, 1.0));
        assert_eq!(data.y_range, (-1.0, 1.0));
    }

    #[test]
    fn test_from_z_function() {
        let data = SurfaceData::from_z_function((-1.0, 1.0), (-1.0, 1.0), 3, |x, y| x * y);

        let p = data.get(1, 1).unwrap();
        assert_eq!(p.z, p.t);
    }

    #[test]
    fn test_normalize() {
        let data = SurfaceData::from_function((0.0, 10.0), (0.0, 10.0), 3, |x, y| (x + y, x - y));

        let normalized = data.normalize();

        assert_eq!(normalized.x_range, (0.0, 1.0));
        assert_eq!(normalized.y_range, (0.0, 1.0));
        assert_eq!(normalized.z_range, (0.0, 1.0));
        assert_eq!(normalized.t_range, (0.0, 1.0));
    }

    #[test]
    fn test_normalized_t() {
        let data = SurfaceData::from_function((0.0, 1.0), (0.0, 1.0), 3, |_, _| (0.0, 50.0));
        // t_range should be (50.0, 50.0) -> adjusted to (50.0, 51.0)

        let data2 = SurfaceData::from_function((0.0, 1.0), (0.0, 1.0), 3, |x, y| (0.0, x + y));
        // t should range from 0 to 2

        assert!((data2.normalized_t(0.0) - 0.0).abs() < 1e-10);
        assert!((data2.normalized_t(2.0) - 1.0).abs() < 1e-10);
        assert!((data2.normalized_t(1.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_from_function_logx() {
        // Create surface with log X-axis (frequency-like)
        let data =
            SurfaceData::from_function_logx((10.0, 1000.0), (0.0, 1.0), 5, |x, y| (x.ln(), x + y));

        assert_eq!(data.rows(), 5);
        assert_eq!(data.cols(), 5);

        // Check that X values are logarithmically spaced
        let x_values: Vec<f64> = (0..5).filter_map(|i| data.get(0, i).map(|p| p.x)).collect();

        // First and last should match range
        assert!((x_values[0] - 10.0).abs() < 1e-6);
        assert!((x_values[4] - 1000.0).abs() < 1e-6);

        // Check logarithmic spacing: log(x[i+1]) - log(x[i]) should be constant
        let log_diffs: Vec<f64> = x_values.windows(2).map(|w| w[1].ln() - w[0].ln()).collect();

        let expected_log_diff = log_diffs[0];
        for diff in &log_diffs {
            assert!((diff - expected_log_diff).abs() < 1e-6);
        }
    }

    #[test]
    fn test_from_function_logy() {
        // Create surface with log Y-axis
        let data =
            SurfaceData::from_function_logy((0.0, 1.0), (10.0, 1000.0), 5, |x, y| (y.ln(), x + y));

        assert_eq!(data.rows(), 5);
        assert_eq!(data.cols(), 5);

        // Check that Y values are logarithmically spaced
        let y_values: Vec<f64> = (0..5).filter_map(|j| data.get(j, 0).map(|p| p.y)).collect();

        assert!((y_values[0] - 10.0).abs() < 1e-6);
        assert!((y_values[4] - 1000.0).abs() < 1e-6);

        // Check logarithmic spacing
        let log_diffs: Vec<f64> = y_values.windows(2).map(|w| w[1].ln() - w[0].ln()).collect();

        let expected_log_diff = log_diffs[0];
        for diff in &log_diffs {
            assert!((diff - expected_log_diff).abs() < 1e-6);
        }
    }

    #[test]
    fn test_from_function_logxy() {
        // Create surface with both axes logarithmic
        let data = SurfaceData::from_function_logxy((20.0, 20000.0), (10.0, 1000.0), 5, |x, y| {
            ((x * y).ln(), x + y)
        });

        assert_eq!(data.rows(), 5);
        assert_eq!(data.cols(), 5);

        // Check X range
        let first_row = 0;
        let x_min = data.get(first_row, 0).unwrap().x;
        let x_max = data.get(first_row, 4).unwrap().x;
        assert!((x_min - 20.0).abs() < 1e-6);
        assert!((x_max - 20000.0).abs() < 1e-6);

        // Check Y range
        let first_col = 0;
        let y_min = data.get(0, first_col).unwrap().y;
        let y_max = data.get(4, first_col).unwrap().y;
        assert!((y_min - 10.0).abs() < 1e-6);
        assert!((y_max - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn test_from_z_function_logx() {
        // Simple case where z = t
        let data = SurfaceData::from_z_function_logx((10.0, 100.0), (0.0, 1.0), 5, |x, _y| x.ln());

        // Check that z equals t for all points
        for row in 0..data.rows() {
            for col in 0..data.cols() {
                let p = data.get(row, col).unwrap();
                assert!((p.z - p.t).abs() < 1e-10);
            }
        }

        // Check X is logarithmic
        let x_values: Vec<f64> = (0..5).filter_map(|i| data.get(0, i).map(|p| p.x)).collect();
        assert!((x_values[0] - 10.0).abs() < 1e-6);
        assert!((x_values[4] - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_from_z_function_logy() {
        let data = SurfaceData::from_z_function_logy((0.0, 1.0), (10.0, 100.0), 5, |_x, y| y.ln());

        // Check that z equals t
        for row in 0..data.rows() {
            for col in 0..data.cols() {
                let p = data.get(row, col).unwrap();
                assert!((p.z - p.t).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn test_from_z_function_logxy() {
        let data = SurfaceData::from_z_function_logxy((10.0, 100.0), (10.0, 100.0), 5, |x, y| {
            (x * y).ln()
        });

        // Check that z equals t
        for row in 0..data.rows() {
            for col in 0..data.cols() {
                let p = data.get(row, col).unwrap();
                assert!((p.z - p.t).abs() < 1e-10);
            }
        }

        // Check ranges are correct
        assert!((data.x_range.0 - 10.0).abs() < 1e-6);
        assert!((data.x_range.1 - 100.0).abs() < 1e-6);
        assert!((data.y_range.0 - 10.0).abs() < 1e-6);
        assert!((data.y_range.1 - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_logx_frequency_response() {
        // Simulate a typical audio frequency response plot
        let data = SurfaceData::from_z_function_logx(
            (20.0, 20000.0), // 20 Hz to 20 kHz
            (0.0, 1.0),
            100,
            |freq, _| {
                // Simulated frequency response
                if freq < 100.0 {
                    -6.0 // Low frequency rolloff
                } else if freq > 10000.0 {
                    -3.0 // High frequency rolloff
                } else {
                    0.0 // Flat response
                }
            },
        );

        assert_eq!(data.rows(), 100);
        assert_eq!(data.cols(), 100);
        assert!((data.x_range.0 - 20.0).abs() < 0.1);
        assert!((data.x_range.1 - 20000.0).abs() < 1.0);
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn test_logx_negative_range_panics() {
        SurfaceData::from_function_logx((-10.0, 10.0), (0.0, 1.0), 5, |x, y| (x, y));
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn test_logy_negative_range_panics() {
        SurfaceData::from_function_logy((0.0, 1.0), (-10.0, 10.0), 5, |x, y| (x, y));
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn test_logxy_negative_x_range_panics() {
        SurfaceData::from_function_logxy((-10.0, 10.0), (10.0, 100.0), 5, |x, y| (x, y));
    }

    #[test]
    #[should_panic]
    #[cfg(debug_assertions)]
    fn test_logxy_negative_y_range_panics() {
        SurfaceData::from_function_logxy((10.0, 100.0), (-10.0, 10.0), 5, |x, y| (x, y));
    }
}
