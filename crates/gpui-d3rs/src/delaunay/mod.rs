//! # d3-delaunay — Delaunay triangulation and Voronoi diagrams
//!
//! This module wraps [`math_delaunay`] with a D3.js-compatible API.
//! The implementation is a faithful port of d3-delaunay using delaunator.

pub mod voronoi;

pub use math_delaunay::Delaunay as MathDelaunay;
pub use math_delaunay::Voronoi as MathVoronoi;

use crate::util::scratch::with_path_scratch;

/// Delaunay triangulation with backward-compatible API.
///
/// Wraps [`math_delaunay::Delaunay`] with the original d3rs interface.
pub struct Delaunay {
    inner: MathDelaunay,
    /// Cached tuple-format points for the old `.points()` API.
    points_tuples: Vec<(f64, f64)>,
}

impl Delaunay {
    /// Create a Delaunay triangulation from (x, y) tuples.
    pub fn new(points: &[(f64, f64)]) -> Self {
        let inner = MathDelaunay::from_points(points);
        Self {
            points_tuples: points.to_vec(),
            inner,
        }
    }

    /// Create from an iterator of (x, y) tuples.
    pub fn from_points_iter<I: IntoIterator<Item = (f64, f64)>>(iter: I) -> Self {
        let points: Vec<(f64, f64)> = iter.into_iter().collect();
        Self::new(&points)
    }

    /// Number of points.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the triangulation is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Access input points as (x, y) tuples.
    pub fn points(&self) -> &[(f64, f64)] {
        &self.points_tuples
    }

    /// Get point i as (x, y).
    pub fn point(&self, i: usize) -> Option<(f64, f64)> {
        self.points_tuples.get(i).copied()
    }

    /// Find the nearest point to (x, y).
    pub fn find(&self, x: f64, y: f64, start: Option<usize>) -> Option<usize> {
        if x.is_nan() || y.is_nan() {
            return None;
        }
        let result = self.inner.find(x, y, start.unwrap_or(0));
        if result >= self.inner.len() {
            None
        } else {
            Some(result)
        }
    }

    /// Find the nearest point within a given radius.
    pub fn find_within_radius(&self, x: f64, y: f64, radius: f64) -> Option<usize> {
        let found = self.find(x, y, None)?;
        let (px, py) = self.points_tuples[found];
        let dist = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
        if dist <= radius { Some(found) } else { None }
    }

    /// Iterate over neighbors of point i.
    pub fn neighbors(&self, i: usize) -> impl Iterator<Item = usize> {
        self.inner.neighbors(i).into_iter()
    }

    /// Iterate over triangles as (i, j, k) tuples.
    pub fn triangles(&self) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        let tris = self.inner.triangles();
        (0..tris.len() / 3).map(move |t| (tris[t * 3], tris[t * 3 + 1], tris[t * 3 + 2]))
    }

    /// Number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.inner.triangles().len() / 3
    }

    /// Convex hull point indices.
    pub fn hull(&self) -> &[usize] {
        self.inner.hull()
    }

    /// Hull as a closed polygon of (x, y) points.
    pub fn hull_polygon(&self) -> Vec<(f64, f64)> {
        let mut poly: Vec<(f64, f64)> = self
            .inner
            .hull()
            .iter()
            .map(|&i| self.inner.point(i))
            .collect();
        if !poly.is_empty() {
            poly.push(poly[0]);
        }
        poly
    }

    /// Render triangulation edges as SVG path data.
    pub fn render_to_path(&self) -> String {
        with_path_scratch(|scratch| {
            self.render_to_path_into(scratch);
            scratch.clone()
        })
    }

    /// Render triangulation edges into `buf`.
    pub fn render_to_path_into(&self, buf: &mut String) {
        use std::fmt::Write;
        for (a, b) in self.edges() {
            if let (Some((x0, y0)), Some((x1, y1))) = (self.point(a), self.point(b)) {
                write!(buf, "M{x0},{y0}L{x1},{y1}").unwrap();
            }
        }
    }

    /// Render the convex hull as SVG path data.
    pub fn render_hull_to_path(&self) -> String {
        polygon_to_path(&self.hull_polygon())
    }

    /// Iterate over unique edges as (i, j) pairs.
    pub fn edges(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let halfedges = self.inner.halfedges();
        let triangles = self.inner.triangles();
        (0..halfedges.len()).filter_map(move |e| {
            let j = halfedges[e];
            if j == delaunator::EMPTY || e < j {
                Some((
                    triangles[e],
                    triangles[if j == delaunator::EMPTY {
                        if e % 3 == 2 { e - 2 } else { e + 1 }
                    } else {
                        j
                    }],
                ))
            } else {
                None
            }
        })
    }

    /// Create a Voronoi diagram.
    pub fn voronoi(&self, bounds: Option<[f64; 4]>) -> voronoi::Voronoi<'_> {
        let bounds = bounds.unwrap_or_else(|| {
            if self.points_tuples.is_empty() {
                return [0.0, 0.0, 960.0, 500.0];
            }
            let mut xmin = f64::INFINITY;
            let mut ymin = f64::INFINITY;
            let mut xmax = f64::NEG_INFINITY;
            let mut ymax = f64::NEG_INFINITY;
            for &(x, y) in &self.points_tuples {
                xmin = xmin.min(x);
                ymin = ymin.min(y);
                xmax = xmax.max(x);
                ymax = ymax.max(y);
            }
            let margin = ((xmax - xmin).max(ymax - ymin)) * 0.1 + 1.0;
            [xmin - margin, ymin - margin, xmax + margin, ymax + margin]
        });
        voronoi::Voronoi::new(&self.inner, bounds)
    }

    /// Access the underlying math-delaunay Delaunay.
    pub fn inner(&self) -> &MathDelaunay {
        &self.inner
    }
}

pub(crate) fn polygon_to_path(points: &[(f64, f64)]) -> String {
    with_path_scratch(|scratch| {
        polygon_to_path_into(points, scratch);
        scratch.clone()
    })
}

pub(crate) fn polygon_to_path_into(points: &[(f64, f64)], buf: &mut String) {
    use std::fmt::Write;
    let Some((x0, y0)) = points.first().copied() else {
        return;
    };

    buf.reserve(points.len() * 24);
    write!(buf, "M{x0},{y0}").unwrap();
    for &(x, y) in &points[1..] {
        write!(buf, "L{x},{y}").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_to_path() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let path = delaunay.render_to_path();
        assert!(path.starts_with('M'));
        assert!(path.contains('L'));
    }

    #[test]
    fn test_render_to_path_into_matches() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let expected = delaunay.render_to_path();
        let mut buf = String::new();
        delaunay.render_to_path_into(&mut buf);
        assert_eq!(buf, expected);
    }

    #[test]
    fn test_polygon_to_path() {
        let points = &[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];
        let path = super::polygon_to_path(points);
        assert!(path.starts_with('M'));
        assert!(!path.contains("format"));
    }

    #[test]
    fn test_polygon_to_path_into_matches() {
        let points = &[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];
        let expected = super::polygon_to_path(points);
        let mut buf = String::new();
        super::polygon_to_path_into(points, &mut buf);
        assert_eq!(buf, expected);
    }
}
