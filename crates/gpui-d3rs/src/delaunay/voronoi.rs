//! Voronoi diagram — wrapper around [`math_delaunay::Voronoi`].

use math_delaunay::{Delaunay as MathDelaunay, Voronoi as MathVoronoi};

use super::{polygon_to_path, polygon_to_path_into};
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

    /// Get the clipping bounds as a closed polygon.
    pub fn bounds_polygon(&self) -> [(f64, f64); 5] {
        let [x0, y0, x1, y1] = self.bounds();
        [(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)]
    }

    /// Number of cells (= number of input points).
    pub fn cell_count(&self) -> usize {
        self.inner.delaunay().len()
    }

    /// Get the polygon for cell i, clipped to bounds.
    pub fn cell_polygon(&self, i: usize) -> Option<Vec<(f64, f64)>> {
        if i >= self.cell_count() {
            return None;
        }
        self.inner.cell_polygon(i)
    }

    /// Iterate over all cell polygons.
    pub fn cell_polygons(&self) -> impl Iterator<Item = Vec<(f64, f64)>> + '_ {
        (0..self.cell_count()).filter_map(|i| self.cell_polygon(i))
    }

    /// Iterate over all cell polygons with their source point index.
    pub fn indexed_cell_polygons(&self) -> impl Iterator<Item = (usize, Vec<(f64, f64)>)> + '_ {
        (0..self.cell_count()).filter_map(|i| self.cell_polygon(i).map(|cell| (i, cell)))
    }

    /// Render all Voronoi cells as SVG path data.
    pub fn render_to_path(&self) -> String {
        with_path_scratch(|scratch| {
            self.render_to_path_into(scratch);
            scratch.clone()
        })
    }

    /// Render all Voronoi cells into `buf`.
    pub fn render_to_path_into(&self, buf: &mut String) {
        for cell in self.cell_polygons() {
            polygon_to_path_into(&cell, buf);
        }
    }

    /// Render the clipping bounds as SVG path data.
    pub fn render_bounds_to_path(&self) -> String {
        polygon_to_path(&self.bounds_polygon())
    }

    /// Render the clipping bounds into `buf`.
    pub fn render_bounds_to_path_into(&self, buf: &mut String) {
        polygon_to_path_into(&self.bounds_polygon(), buf);
    }

    /// Render one Voronoi cell as SVG path data.
    pub fn render_cell_to_path(&self, i: usize) -> Option<String> {
        self.cell_polygon(i).map(|cell| polygon_to_path(&cell))
    }

    /// Render one Voronoi cell into `buf`.
    ///
    /// Returns `true` when the cell exists and was appended.
    pub fn render_cell_to_path_into(&self, i: usize, buf: &mut String) -> bool {
        let Some(cell) = self.cell_polygon(i) else {
            return false;
        };
        polygon_to_path_into(&cell, buf);
        true
    }

    /// Test if point (x, y) is inside cell i.
    pub fn contains(&self, i: usize, x: f64, y: f64) -> bool {
        if i >= self.cell_count() {
            return false;
        }
        self.inner.contains(i, x, y)
    }

    /// Neighboring cell indices.
    pub fn neighbors(&self, i: usize) -> impl Iterator<Item = usize> {
        if i >= self.cell_count() {
            Vec::new().into_iter()
        } else {
            self.inner.delaunay().neighbors(i).into_iter()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::Delaunay;

    #[test]
    fn render_to_path_into_matches_allocating_render() {
        let delaunay = Delaunay::try_new(&[(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)]).unwrap();
        let voronoi = delaunay.try_voronoi(Some([0.0, 0.0, 10.0, 10.0])).unwrap();

        let mut path = String::new();
        voronoi.render_to_path_into(&mut path);

        assert_eq!(path, voronoi.render_to_path());
        assert!(path.starts_with('M'));
        assert!(path.contains('L'));
    }

    #[test]
    fn render_bounds_helpers_emit_closed_bounds_polygon() {
        let delaunay = Delaunay::try_new(&[(0.0, 0.0), (10.0, 0.0)]).unwrap();
        let voronoi = delaunay.try_voronoi(Some([0.0, 1.0, 10.0, 11.0])).unwrap();

        let mut path = String::from("prefix");
        voronoi.render_bounds_to_path_into(&mut path);

        assert_eq!(
            voronoi.bounds_polygon(),
            [
                (0.0, 1.0),
                (10.0, 1.0),
                (10.0, 11.0),
                (0.0, 11.0),
                (0.0, 1.0)
            ]
        );
        assert_eq!(voronoi.render_bounds_to_path(), "M0,1L10,1L10,11L0,11L0,1");
        assert_eq!(path, "prefixM0,1L10,1L10,11L0,11L0,1");
    }

    #[test]
    fn render_cell_helpers_match_cell_polygons() {
        let delaunay = Delaunay::try_new(&[(0.0, 0.0), (10.0, 0.0), (5.0, 10.0)]).unwrap();
        let voronoi = delaunay.try_voronoi(Some([0.0, 0.0, 10.0, 10.0])).unwrap();

        let indexed: Vec<_> = voronoi.indexed_cell_polygons().collect();
        assert_eq!(indexed.len(), voronoi.cell_count());

        let (cell_index, cell) = &indexed[0];
        let mut path = String::new();

        assert!(voronoi.render_cell_to_path_into(*cell_index, &mut path));
        assert_eq!(Some(path.clone()), voronoi.render_cell_to_path(*cell_index));
        assert_eq!(path, super::polygon_to_path(cell));
        assert_eq!(voronoi.render_cell_to_path(voronoi.cell_count()), None);
        assert!(!voronoi.render_cell_to_path_into(voronoi.cell_count(), &mut path));
        assert_eq!(voronoi.cell_polygon(voronoi.cell_count()), None);
        assert!(!voronoi.contains(voronoi.cell_count(), 0.0, 0.0));
        assert_eq!(voronoi.neighbors(voronoi.cell_count()).count(), 0);
    }
}
