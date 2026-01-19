//! Kernel density estimation for 2D point clouds
//!
//! Provides density estimation using kernel functions.

use std::f64::consts::PI;

/// Gaussian kernel function.
///
/// # Example
///
/// ```
/// use d3rs::contour::gaussian_kernel;
///
/// let k = gaussian_kernel(0.0, 1.0);
/// assert!((k - 0.3989422804014327).abs() < 0.0001);
/// ```
pub fn gaussian_kernel(x: f64, bandwidth: f64) -> f64 {
    let t = x / bandwidth;
    (-(t * t) / 2.0).exp() / (bandwidth * (2.0 * PI).sqrt())
}

/// Epanechnikov kernel function.
pub fn epanechnikov_kernel(x: f64, bandwidth: f64) -> f64 {
    let t = x / bandwidth;
    if t.abs() <= 1.0 {
        0.75 * (1.0 - t * t) / bandwidth
    } else {
        0.0
    }
}

/// 2D density estimator using kernel density estimation.
///
/// # Example
///
/// ```
/// use d3rs::contour::DensityEstimator;
///
/// let points = vec![
///     (0.5, 0.5),
///     (0.6, 0.4),
///     (0.4, 0.6),
/// ];
///
/// let estimator = DensityEstimator::new()
///     .bandwidth(0.2)
///     .size(10, 10);
///
/// let grid = estimator.estimate(&points);
/// assert_eq!(grid.len(), 100);
/// ```
#[derive(Debug, Clone)]
pub struct DensityEstimator {
    /// Grid width
    width: usize,
    /// Grid height
    height: usize,
    /// X domain
    x0: f64,
    x1: f64,
    /// Y domain
    y0: f64,
    y1: f64,
    /// Bandwidth (sigma for Gaussian)
    bandwidth: f64,
    /// Kernel type
    kernel: KernelType,
}

/// Kernel type for density estimation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelType {
    /// Gaussian kernel
    Gaussian,
    /// Epanechnikov kernel
    Epanechnikov,
}

impl Default for DensityEstimator {
    fn default() -> Self {
        Self {
            width: 100,
            height: 100,
            x0: 0.0,
            x1: 1.0,
            y0: 0.0,
            y1: 1.0,
            bandwidth: 0.1,
            kernel: KernelType::Gaussian,
        }
    }
}

impl DensityEstimator {
    /// Create a new density estimator with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the grid size.
    pub fn size(mut self, width: usize, height: usize) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set the x domain.
    pub fn x(mut self, x0: f64, x1: f64) -> Self {
        self.x0 = x0;
        self.x1 = x1;
        self
    }

    /// Set the y domain.
    pub fn y(mut self, y0: f64, y1: f64) -> Self {
        self.y0 = y0;
        self.y1 = y1;
        self
    }

    /// Set the bandwidth (smoothing parameter).
    pub fn bandwidth(mut self, bandwidth: f64) -> Self {
        self.bandwidth = bandwidth;
        self
    }

    /// Set the kernel type.
    pub fn kernel(mut self, kernel: KernelType) -> Self {
        self.kernel = kernel;
        self
    }

    /// Estimate density from a list of (x, y) points.
    pub fn estimate(&self, points: &[(f64, f64)]) -> Vec<f64> {
        let mut grid = vec![0.0; self.width * self.height];

        if points.is_empty() {
            return grid;
        }

        let dx = (self.x1 - self.x0) / (self.width - 1) as f64;
        let dy = (self.y1 - self.y0) / (self.height - 1) as f64;

        // For each point, add its contribution to nearby grid cells
        for &(px, py) in points {
            // Convert to grid coordinates
            let gx = ((px - self.x0) / dx) as isize;
            let gy = ((py - self.y0) / dy) as isize;

            // Determine the radius of influence (3 sigma for Gaussian)
            let radius = (3.0 * self.bandwidth / dx.min(dy)).ceil() as isize;

            for j in (-radius)..=radius {
                let grid_y = gy + j;
                if grid_y < 0 || grid_y >= self.height as isize {
                    continue;
                }

                let cell_y = self.y0 + (grid_y as f64) * dy;
                let ky = self.kernel_value((py - cell_y) / dy);

                for i in (-radius)..=radius {
                    let grid_x = gx + i;
                    if grid_x < 0 || grid_x >= self.width as isize {
                        continue;
                    }

                    let cell_x = self.x0 + (grid_x as f64) * dx;
                    let kx = self.kernel_value((px - cell_x) / dx);

                    let idx = (grid_y as usize) * self.width + (grid_x as usize);
                    grid[idx] += kx * ky;
                }
            }
        }

        // Normalize by the number of points
        let n = points.len() as f64;
        for val in &mut grid {
            *val /= n;
        }

        grid
    }

    /// Estimate density with weighted points.
    pub fn estimate_weighted(&self, points: &[(f64, f64, f64)]) -> Vec<f64> {
        let mut grid = vec![0.0; self.width * self.height];

        if points.is_empty() {
            return grid;
        }

        let dx = (self.x1 - self.x0) / (self.width - 1) as f64;
        let dy = (self.y1 - self.y0) / (self.height - 1) as f64;

        let mut total_weight = 0.0;

        for &(px, py, weight) in points {
            total_weight += weight;

            let gx = ((px - self.x0) / dx) as isize;
            let gy = ((py - self.y0) / dy) as isize;

            let radius = (3.0 * self.bandwidth / dx.min(dy)).ceil() as isize;

            for j in (-radius)..=radius {
                let grid_y = gy + j;
                if grid_y < 0 || grid_y >= self.height as isize {
                    continue;
                }

                let cell_y = self.y0 + (grid_y as f64) * dy;
                let ky = self.kernel_value((py - cell_y) / dy);

                for i in (-radius)..=radius {
                    let grid_x = gx + i;
                    if grid_x < 0 || grid_x >= self.width as isize {
                        continue;
                    }

                    let cell_x = self.x0 + (grid_x as f64) * dx;
                    let kx = self.kernel_value((px - cell_x) / dx);

                    let idx = (grid_y as usize) * self.width + (grid_x as usize);
                    grid[idx] += weight * kx * ky;
                }
            }
        }

        // Normalize by total weight
        if total_weight > 0.0 {
            for val in &mut grid {
                *val /= total_weight;
            }
        }

        grid
    }

    /// Evaluate the kernel at a distance.
    fn kernel_value(&self, x: f64) -> f64 {
        match self.kernel {
            KernelType::Gaussian => gaussian_kernel(x, self.bandwidth),
            KernelType::Epanechnikov => epanechnikov_kernel(x, self.bandwidth),
        }
    }
}

/// Simple 2D density estimation function.
///
/// # Example
///
/// ```
/// use d3rs::contour::density_2d;
///
/// let points = vec![
///     (0.5, 0.5),
///     (0.6, 0.4),
///     (0.4, 0.6),
/// ];
///
/// let (grid, width, height) = density_2d(&points, 10, 10, 0.2);
/// assert_eq!(grid.len(), 100);
/// assert_eq!(width, 10);
/// assert_eq!(height, 10);
/// ```
pub fn density_2d(
    points: &[(f64, f64)],
    width: usize,
    height: usize,
    bandwidth: f64,
) -> (Vec<f64>, usize, usize) {
    // Determine extent from points
    let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);

    for &(x, y) in points {
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }

    // Add some padding
    let padding = bandwidth * 3.0;
    x0 -= padding;
    x1 += padding;
    y0 -= padding;
    y1 += padding;

    let estimator = DensityEstimator::new()
        .size(width, height)
        .x(x0, x1)
        .y(y0, y1)
        .bandwidth(bandwidth);

    (estimator.estimate(points), width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_kernel() {
        let k = gaussian_kernel(0.0, 1.0);
        assert!((k - 0.3989422804014327).abs() < 0.0001);
    }

    #[test]
    fn test_epanechnikov_kernel() {
        let k = epanechnikov_kernel(0.0, 1.0);
        assert!((k - 0.75).abs() < 0.0001);

        // Outside support should be zero
        assert_eq!(epanechnikov_kernel(1.5, 1.0), 0.0);
    }

    #[test]
    fn test_density_estimator() {
        let points = vec![(0.5, 0.5), (0.6, 0.4), (0.4, 0.6)];

        let estimator = DensityEstimator::new().bandwidth(0.2).size(10, 10);

        let grid = estimator.estimate(&points);
        assert_eq!(grid.len(), 100);

        // Maximum should be near center
        let max_idx = grid
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();

        let max_x = max_idx % 10;
        let max_y = max_idx / 10;
        assert!(max_x >= 3 && max_x <= 6);
        assert!(max_y >= 3 && max_y <= 6);
    }

    #[test]
    fn test_density_2d() {
        let points = vec![(0.0, 0.0), (1.0, 1.0)];
        let (grid, width, height) = density_2d(&points, 10, 10, 0.2);
        assert_eq!(grid.len(), width * height);
    }

    #[test]
    fn test_weighted_density() {
        let points = vec![(0.5, 0.5, 10.0), (0.0, 0.0, 1.0)];

        let estimator = DensityEstimator::new().bandwidth(0.2).size(10, 10);

        let grid = estimator.estimate_weighted(&points);
        assert_eq!(grid.len(), 100);
    }
}
