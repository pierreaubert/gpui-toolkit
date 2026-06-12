//! Voronoi Stippling — <https://observablehq.com/@mbostock/voronoi-stippling>
//!
//! Weighted Lloyd's relaxation: iteratively moves points toward the
//! density-weighted centroids of their Voronoi cells.

use crate::delaunay::Delaunay;
use crate::shape::path::{Path, PathBuilder};

/// Mutable stippling state that supports incremental iteration.
pub struct StipplingState {
    pub points: Vec<f64>, // flat [x0, y0, x1, y1, ...]
    pub n: usize,
    pub width: usize,
    pub height: usize,
    pub iteration: usize,
    seed: u64,
}

impl StipplingState {
    /// Create initial state: seed N points via rejection sampling.
    pub fn new(density: &[f64], width: usize, height: usize, n: usize) -> Self {
        let w = width as f64;
        let h = height as f64;
        let mut seed = 42u64;
        let mut next_rand = || -> f64 {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as f64 / (1u64 << 31) as f64
        };

        let mut points = vec![0.0f64; n * 2];
        for i in 0..n {
            for _ in 0..30 {
                let x = (next_rand() * w).floor();
                let y = (next_rand() * h).floor();
                points[i * 2] = x;
                points[i * 2 + 1] = y;
                let ix = x as usize;
                let iy = y as usize;
                if ix < width && iy < height && next_rand() < density[iy * width + ix] {
                    break;
                }
            }
        }

        Self {
            points,
            n,
            width,
            height,
            iteration: 0,
            seed,
        }
    }

    /// Run one Lloyd relaxation step. Call this repeatedly to animate.
    pub fn step(&mut self, density: &[f64]) {
        let n = self.n;
        let width = self.width;
        let height = self.height;
        let w = width as f64;
        let h = height as f64;

        let tuples: Vec<(f64, f64)> = (0..n)
            .map(|i| (self.points[i * 2], self.points[i * 2 + 1]))
            .collect();
        let delaunay = Delaunay::new(&tuples);

        let mut c = vec![0.0f64; n * 2];
        let mut s = vec![0.0f64; n];

        // Scan pixels — use stride to skip pixels for speed
        // Larger stride = faster but less accurate centroid
        let stride = if width * height > 500_000 {
            4
        } else if width * height > 100_000 {
            3
        } else {
            1
        };
        let mut last_found = 0usize;
        for y in (0..height).step_by(stride) {
            for x in (0..width).step_by(stride) {
                let weight = density[y * width + x];
                if weight < 0.001 {
                    continue;
                }
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;
                if let Some(i) = delaunay.find(px, py, Some(last_found)) {
                    last_found = i;
                    if i < n {
                        let area = (stride * stride) as f64; // compensate for stride
                        s[i] += weight * area;
                        c[i * 2] += weight * area * px;
                        c[i * 2 + 1] += weight * area * py;
                    }
                }
            }
        }

        let k = self.iteration as f64;
        let jitter = (k + 1.0).powf(-0.8) * 10.0;
        let mut next_rand = || -> f64 {
            self.seed = self
                .seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.seed >> 33) as f64 / (1u64 << 31) as f64
        };

        for i in 0..n {
            let x0 = self.points[i * 2];
            let y0 = self.points[i * 2 + 1];
            let (x1, y1) = if s[i] > 0.0 {
                (c[i * 2] / s[i], c[i * 2 + 1] / s[i])
            } else {
                (x0, y0)
            };
            let jx = (next_rand() - 0.5) * jitter;
            let jy = (next_rand() - 0.5) * jitter;
            self.points[i * 2] = (x0 + (x1 - x0) * 1.8 + jx).clamp(0.0, w - 1.0);
            self.points[i * 2 + 1] = (y0 + (y1 - y0) * 1.8 + jy).clamp(0.0, h - 1.0);
        }

        self.iteration += 1;
    }

    /// Get current dot positions as (x, y) pairs.
    pub fn get_points(&self) -> Vec<(f64, f64)> {
        (0..self.n)
            .map(|i| (self.points[i * 2], self.points[i * 2 + 1]))
            .collect()
    }

    /// Build dot paths for rendering.
    pub fn build_dot_paths(&self) -> Vec<Path> {
        let dot_r = 1.5;
        let n_sides = 8;
        (0..self.n)
            .map(|i| {
                let px = self.points[i * 2];
                let py = self.points[i * 2 + 1];
                let mut builder = PathBuilder::new();
                for v in 0..n_sides {
                    let angle = std::f64::consts::TAU * v as f64 / n_sides as f64;
                    let x = px + dot_r * angle.cos();
                    let y = py + dot_r * angle.sin();
                    if v == 0 {
                        builder = builder.move_to(x, y);
                    } else {
                        builder = builder.line_to(x, y);
                    }
                }
                builder = builder.close_path();
                builder.build()
            })
            .collect()
    }
}

/// Convenience: run all iterations at once (for tests / non-interactive use).
pub fn compute(
    density: &[f64],
    width: usize,
    height: usize,
    n: usize,
    iterations: usize,
) -> StipplingResult {
    let mut state = StipplingState::new(density, width, height, n);
    for _ in 0..iterations {
        state.step(density);
    }
    StipplingResult {
        width: width as f64,
        height: height as f64,
        points: state.get_points(),
        dot_paths: state.build_dot_paths(),
        point_count: n,
        iterations,
    }
}

#[derive(Debug)]
pub struct StipplingResult {
    pub width: f64,
    pub height: f64,
    pub points: Vec<(f64, f64)>,
    pub dot_paths: Vec<Path>,
    pub point_count: usize,
    pub iterations: usize,
}
