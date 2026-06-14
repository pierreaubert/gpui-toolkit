//! Voronoi diagram — wrapper around [`math_delaunay::Voronoi`].

use math_delaunay::{Delaunay as MathDelaunay, Voronoi as MathVoronoi};

use super::polygon_to_path_into;
use crate::util::scratch::with_path_scratch;

/// Voronoi diagram with D3-compatible API.
pub struct Voronoi<'a> {
    inner: MathVoronoi<'a>,
}

impl<'a> Voronoi<'a> {
    /// Create from a math-delaunay Delaunay and bounds.
    pub fn new(delaunay: &'a MathDelaunay, bounds: [f64; 4]) -> Self {
        Self {
            inner: MathVoronoi::new(delaunay, bounds),
        }
    }

    /// Get the clipping bounds.
    pub fn bounds(&self) -> [f64; 4] {
        self.inner.bounds()
    }

    /// Number of cells (= number of input points).
    pub fn cell_count(&self) -> usize {
        self.inner.delaunay().len()
    }

    /// Get the polygon for cell i, clipped to bounds.
    pub fn cell_polygon(&self, i: usize) -> Option<Vec<(f64, f64)>> {
        self.inner.cell_polygon(i)
    }

    /// Iterate over all cell polygons.
    pub fn cell_polygons(&self) -> impl Iterator<Item = Vec<(f64, f64)>> + '_ {
        (0..self.cell_count()).filter_map(|i| self.cell_polygon(i))
    }

    /// Render all Voronoi cells as SVG path data.
    pub fn render_to_path(&self) -> String {
        with_path_scratch(|scratch| {
            for cell in self.cell_polygons() {
                polygon_to_path_into(&cell, scratch);
            }
            scratch.clone()
        })
    }

    /// Test if point (x, y) is inside cell i.
    pub fn contains(&self, i: usize, x: f64, y: f64) -> bool {
        self.inner.contains(i, x, y)
    }

    /// Neighboring cell indices.
    pub fn neighbors(&self, i: usize) -> impl Iterator<Item = usize> {
        self.inner.delaunay().neighbors(i).into_iter()
    }
}
