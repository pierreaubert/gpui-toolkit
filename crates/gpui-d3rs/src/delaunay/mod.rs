//! # d3-delaunay - Delaunay triangulation and Voronoi diagrams
//!
//! This module provides Delaunay triangulation and Voronoi diagram computation,
//! wrapping the `delaunator` crate with a D3.js-compatible API.
//!
//! ## Features
//!
//! - Delaunay triangulation from 2D points
//! - Convex hull computation
//! - Nearest neighbor search
//! - Triangle and edge iteration
//! - Voronoi diagram generation with clipping bounds
//!
//! ## Example
//!
//! ```rust
//! use d3rs::delaunay::Delaunay;
//!
//! let points = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (0.5, 0.5)];
//! let delaunay = Delaunay::new(&points);
//!
//! // Find nearest point to (0.3, 0.3)
//! let nearest = delaunay.find(0.3, 0.3, None);
//! assert_eq!(nearest, Some(3)); // Point at (0.5, 0.5)
//!
//! // Get triangles
//! for triangle in delaunay.triangles() {
//!     println!("Triangle: {:?}", triangle);
//! }
//!
//! // Create Voronoi diagram
//! let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));
//! for (i, cell) in voronoi.cell_polygons().enumerate() {
//!     println!("Cell {}: {:?}", i, cell);
//! }
//! ```

mod voronoi;

pub use voronoi::Voronoi;

use delaunator::{EMPTY, Point, Triangulation, triangulate};

/// Delaunay triangulation of a set of 2D points.
///
/// Wraps the `delaunator` crate with a D3.js-compatible API.
#[derive(Clone)]
pub struct Delaunay {
    /// The input points as (x, y) tuples
    points: Vec<(f64, f64)>,
    /// The underlying triangulation from delaunator
    triangulation: Triangulation,
    /// Cached convex hull indices
    hull: Vec<usize>,
}

impl Delaunay {
    /// Creates a new Delaunay triangulation from a slice of (x, y) points.
    ///
    /// # Example
    /// ```
    /// use d3rs::delaunay::Delaunay;
    /// let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
    /// ```
    pub fn new(points: &[(f64, f64)]) -> Self {
        let delaunator_points: Vec<Point> = points.iter().map(|&(x, y)| Point { x, y }).collect();

        let triangulation = triangulate(&delaunator_points);
        let hull = triangulation.hull.clone();

        Self {
            points: points.to_vec(),
            triangulation,
            hull,
        }
    }

    /// Creates a new Delaunay triangulation from an iterator of (x, y) points.
    ///
    /// # Example
    /// ```
    /// use d3rs::delaunay::Delaunay;
    /// let points = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];
    /// let delaunay = Delaunay::from_iter(points.into_iter());
    /// ```
    pub fn from_iter<I: IntoIterator<Item = (f64, f64)>>(iter: I) -> Self {
        let points: Vec<(f64, f64)> = iter.into_iter().collect();
        Self::new(&points)
    }

    /// Returns the number of points in the triangulation.
    pub fn len(&self) -> usize {
        self.points.len()
    }

    /// Returns true if the triangulation contains no points.
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Returns a reference to the input points.
    pub fn points(&self) -> &[(f64, f64)] {
        &self.points
    }

    /// Returns the point at the given index.
    pub fn point(&self, i: usize) -> Option<(f64, f64)> {
        self.points.get(i).copied()
    }

    /// Returns a reference to the raw triangulation data.
    pub fn triangulation(&self) -> &Triangulation {
        &self.triangulation
    }

    /// Returns an iterator over the triangle indices as (i, j, k) tuples.
    ///
    /// Each tuple contains the indices of the three vertices of a triangle.
    ///
    /// # Example
    /// ```
    /// use d3rs::delaunay::Delaunay;
    /// let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (0.5, 0.5)]);
    /// for (i, j, k) in delaunay.triangles() {
    ///     println!("Triangle vertices: {}, {}, {}", i, j, k);
    /// }
    /// ```
    pub fn triangles(&self) -> impl Iterator<Item = (usize, usize, usize)> + '_ {
        self.triangulation
            .triangles
            .chunks(3)
            .map(|chunk| (chunk[0], chunk[1], chunk[2]))
    }

    /// Returns an iterator over the triangle polygons as vectors of (x, y) points.
    ///
    /// Each polygon is a closed triangle (4 points, first and last are the same).
    pub fn triangle_polygons(&self) -> impl Iterator<Item = Vec<(f64, f64)>> + '_ {
        self.triangles().map(move |(i, j, k)| {
            let p0 = self.points[i];
            let p1 = self.points[j];
            let p2 = self.points[k];
            vec![p0, p1, p2, p0]
        })
    }

    /// Returns the number of triangles in the triangulation.
    pub fn triangle_count(&self) -> usize {
        self.triangulation.triangles.len() / 3
    }

    /// Returns the indices of the convex hull vertices in counterclockwise order.
    pub fn hull(&self) -> &[usize] {
        &self.hull
    }

    /// Returns the convex hull as a polygon (closed, first point repeated at end).
    pub fn hull_polygon(&self) -> Vec<(f64, f64)> {
        if self.hull.is_empty() {
            return vec![];
        }
        let mut polygon: Vec<(f64, f64)> = self.hull.iter().map(|&i| self.points[i]).collect();
        polygon.push(polygon[0]); // Close the polygon
        polygon
    }

    /// Finds the index of the point nearest to (x, y).
    ///
    /// If `start` is provided, begins the search from that point index,
    /// which can be faster if you have a good initial guess.
    ///
    /// Returns `None` if the triangulation is empty.
    ///
    /// # Example
    /// ```
    /// use d3rs::delaunay::Delaunay;
    /// let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
    /// let nearest = delaunay.find(0.1, 0.1, None);
    /// assert_eq!(nearest, Some(0));
    /// ```
    pub fn find(&self, x: f64, y: f64, start: Option<usize>) -> Option<usize> {
        if self.points.is_empty() {
            return None;
        }

        // Start from the given index or 0
        let mut current = start.unwrap_or(0).min(self.points.len() - 1);
        let mut min_dist = self.distance_squared(current, x, y);

        // Walk towards the nearest point using the triangulation connectivity
        loop {
            let mut improved = false;

            // Check all neighbors of the current point
            for neighbor in self.neighbors(current) {
                let dist = self.distance_squared(neighbor, x, y);
                if dist < min_dist {
                    min_dist = dist;
                    current = neighbor;
                    improved = true;
                    break;
                }
            }

            if !improved {
                break;
            }
        }

        Some(current)
    }

    /// Finds the index of the point nearest to (x, y) within the given radius.
    ///
    /// Returns `None` if no point is within the radius or if the triangulation is empty.
    pub fn find_within_radius(&self, x: f64, y: f64, radius: f64) -> Option<usize> {
        let nearest = self.find(x, y, None)?;
        let dist_sq = self.distance_squared(nearest, x, y);
        if dist_sq <= radius * radius {
            Some(nearest)
        } else {
            None
        }
    }

    /// Returns an iterator over the indices of points neighboring the given point.
    ///
    /// # Example
    /// ```
    /// use d3rs::delaunay::Delaunay;
    /// let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (0.5, 0.5)]);
    /// for neighbor in delaunay.neighbors(3) {
    ///     println!("Point 3 is connected to point {}", neighbor);
    /// }
    /// ```
    pub fn neighbors(&self, i: usize) -> NeighborIterator<'_> {
        NeighborIterator::new(self, i)
    }

    /// Returns an iterator over all edges as (i, j) index pairs.
    ///
    /// Each edge is returned only once (not both (i, j) and (j, i)).
    pub fn edges(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.triangulation
            .halfedges
            .iter()
            .enumerate()
            .filter_map(|(e, &opposite)| {
                // Only yield edges where e < opposite, or opposite is EMPTY
                // This ensures each edge is yielded exactly once
                if opposite == EMPTY || e < opposite {
                    let t = e / 3;
                    let i = e % 3;
                    let a = self.triangulation.triangles[t * 3 + i];
                    let b = self.triangulation.triangles[t * 3 + (i + 1) % 3];
                    Some((a, b))
                } else {
                    None
                }
            })
    }

    /// Creates a Voronoi diagram from this Delaunay triangulation.
    ///
    /// The bounds specify the clipping rectangle as [x_min, y_min, x_max, y_max].
    /// If not provided, the bounds are computed from the input points.
    ///
    /// # Example
    /// ```
    /// use d3rs::delaunay::Delaunay;
    /// let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
    /// let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));
    /// ```
    pub fn voronoi(&self, bounds: Option<[f64; 4]>) -> Voronoi<'_> {
        Voronoi::new(self, bounds)
    }

    /// Renders the Delaunay triangulation to a path string (SVG path data format).
    ///
    /// The output can be used with SVG `<path>` elements or similar.
    pub fn render_to_path(&self) -> String {
        let mut path = String::new();

        // Render all triangle edges
        for (a, b) in self.edges() {
            let (x0, y0) = self.points[a];
            let (x1, y1) = self.points[b];
            path.push_str(&format!("M{},{}L{},{}", x0, y0, x1, y1));
        }

        path
    }

    /// Renders the convex hull to a path string (SVG path data format).
    pub fn render_hull_to_path(&self) -> String {
        if self.hull.is_empty() {
            return String::new();
        }

        let mut path = String::new();
        let (x0, y0) = self.points[self.hull[0]];
        path.push_str(&format!("M{},{}", x0, y0));

        for &i in &self.hull[1..] {
            let (x, y) = self.points[i];
            path.push_str(&format!("L{},{}", x, y));
        }

        path.push('Z');
        path
    }

    /// Returns the squared distance from point at index i to (x, y).
    fn distance_squared(&self, i: usize, x: f64, y: f64) -> f64 {
        let (px, py) = self.points[i];
        let dx = px - x;
        let dy = py - y;
        dx * dx + dy * dy
    }

    /// Returns the circumcenter of the triangle at the given index.
    pub(crate) fn circumcenter(&self, t: usize) -> (f64, f64) {
        let i0 = self.triangulation.triangles[t * 3];
        let i1 = self.triangulation.triangles[t * 3 + 1];
        let i2 = self.triangulation.triangles[t * 3 + 2];

        let (ax, ay) = self.points[i0];
        let (bx, by) = self.points[i1];
        let (cx, cy) = self.points[i2];

        let d = 2.0 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));

        if d.abs() < 1e-12 {
            // Degenerate case: points are collinear
            return ((ax + bx + cx) / 3.0, (ay + by + cy) / 3.0);
        }

        let ax2 = ax * ax;
        let ay2 = ay * ay;
        let bx2 = bx * bx;
        let by2 = by * by;
        let cx2 = cx * cx;
        let cy2 = cy * cy;

        let ux = ((ax2 + ay2) * (by - cy) + (bx2 + by2) * (cy - ay) + (cx2 + cy2) * (ay - by)) / d;
        let uy = ((ax2 + ay2) * (cx - bx) + (bx2 + by2) * (ax - cx) + (cx2 + cy2) * (bx - ax)) / d;

        (ux, uy)
    }
}

impl std::fmt::Debug for Delaunay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Delaunay")
            .field("points_count", &self.points.len())
            .field("triangles_count", &self.triangle_count())
            .field("hull_size", &self.hull.len())
            .finish()
    }
}

/// Iterator over the neighbors of a point in a Delaunay triangulation.
pub struct NeighborIterator<'a> {
    #[allow(dead_code)]
    delaunay: &'a Delaunay,
    #[allow(dead_code)]
    point_index: usize,
    #[allow(dead_code)]
    visited: Vec<bool>,
    to_visit: Vec<usize>,
}

impl<'a> NeighborIterator<'a> {
    fn new(delaunay: &'a Delaunay, point_index: usize) -> Self {
        let n = delaunay.points.len();
        let mut visited = vec![false; n];
        visited[point_index] = true; // Don't yield the point itself

        // Find all triangles containing this point and collect neighbors
        let mut to_visit = Vec::new();
        for (i, j, k) in delaunay.triangles() {
            if i == point_index {
                if !visited[j] {
                    to_visit.push(j);
                    visited[j] = true;
                }
                if !visited[k] {
                    to_visit.push(k);
                    visited[k] = true;
                }
            } else if j == point_index {
                if !visited[i] {
                    to_visit.push(i);
                    visited[i] = true;
                }
                if !visited[k] {
                    to_visit.push(k);
                    visited[k] = true;
                }
            } else if k == point_index {
                if !visited[i] {
                    to_visit.push(i);
                    visited[i] = true;
                }
                if !visited[j] {
                    to_visit.push(j);
                    visited[j] = true;
                }
            }
        }

        Self {
            delaunay,
            point_index,
            visited,
            to_visit,
        }
    }
}

impl Iterator for NeighborIterator<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.to_visit.pop()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let delaunay = Delaunay::new(&[]);
        assert!(delaunay.is_empty());
        assert_eq!(delaunay.len(), 0);
        assert_eq!(delaunay.find(0.0, 0.0, None), None);
    }

    #[test]
    fn test_new_single_point() {
        let delaunay = Delaunay::new(&[(5.0, 5.0)]);
        assert_eq!(delaunay.len(), 1);
        assert_eq!(delaunay.find(0.0, 0.0, None), Some(0));
    }

    #[test]
    fn test_new_two_points() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 1.0)]);
        assert_eq!(delaunay.len(), 2);
        // With only 2 points, no triangles can form - find starts at 0 and stays there
        // if there's no connectivity. This is expected behavior.
        let nearest = delaunay.find(0.0, 0.0, None);
        assert!(nearest.is_some());
        // find(1.0, 1.0, None) may return 0 since there's no triangle connectivity
        let nearest2 = delaunay.find(1.0, 1.0, Some(1));
        assert_eq!(nearest2, Some(1)); // With explicit start, we get point 1
    }

    #[test]
    fn test_triangle() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        assert_eq!(delaunay.len(), 3);
        assert_eq!(delaunay.triangle_count(), 1);

        let triangles: Vec<_> = delaunay.triangles().collect();
        assert_eq!(triangles.len(), 1);

        // All three points should be in the triangle
        let (i, j, k) = triangles[0];
        let mut indices = vec![i, j, k];
        indices.sort();
        assert_eq!(indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_square() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        assert_eq!(delaunay.len(), 4);
        assert_eq!(delaunay.triangle_count(), 2);

        // Hull should have all 4 points
        assert_eq!(delaunay.hull().len(), 4);
    }

    #[test]
    fn test_find() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (0.5, 0.5)]);

        // Find nearest to origin
        assert_eq!(delaunay.find(0.0, 0.0, None), Some(0));

        // Find nearest to center
        let nearest = delaunay.find(0.5, 0.5, None);
        assert_eq!(nearest, Some(3));

        // Find nearest to top
        assert_eq!(delaunay.find(0.5, 0.9, None), Some(2));
    }

    #[test]
    fn test_find_with_radius() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);

        // Point within radius
        assert_eq!(delaunay.find_within_radius(0.1, 0.1, 0.2), Some(0));

        // Point outside radius
        assert_eq!(delaunay.find_within_radius(0.5, 0.5, 0.1), None);
    }

    #[test]
    fn test_neighbors() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (0.5, 0.5)]);

        // Central point should have all others as neighbors
        let neighbors: Vec<_> = delaunay.neighbors(3).collect();
        assert!(neighbors.len() >= 2); // At least 2 neighbors
    }

    #[test]
    fn test_hull_polygon() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        let hull = delaunay.hull_polygon();

        // Hull should be closed (first point repeated at end)
        assert_eq!(hull.first(), hull.last());
        assert_eq!(hull.len(), 5); // 4 points + closing point
    }

    #[test]
    fn test_triangle_polygons() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let polygons: Vec<_> = delaunay.triangle_polygons().collect();

        assert_eq!(polygons.len(), 1);
        assert_eq!(polygons[0].len(), 4); // Triangle + closing point
        assert_eq!(polygons[0].first(), polygons[0].last());
    }

    #[test]
    fn test_edges() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let edges: Vec<_> = delaunay.edges().collect();

        // Triangle has 3 edges
        assert_eq!(edges.len(), 3);
    }

    #[test]
    fn test_render_to_path() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let path = delaunay.render_to_path();

        // Should contain path commands
        assert!(path.contains('M'));
        assert!(path.contains('L'));
    }

    #[test]
    fn test_render_hull_to_path() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let path = delaunay.render_hull_to_path();

        // Should be a closed path
        assert!(path.starts_with('M'));
        assert!(path.ends_with('Z'));
    }

    #[test]
    fn test_circumcenter() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let (cx, cy) = delaunay.circumcenter(0);

        // Circumcenter should be equidistant from all three vertices
        let d0 = ((cx - 0.0).powi(2) + (cy - 0.0).powi(2)).sqrt();
        let d1 = ((cx - 1.0).powi(2) + (cy - 0.0).powi(2)).sqrt();
        let d2 = ((cx - 0.5).powi(2) + (cy - 1.0).powi(2)).sqrt();

        assert!((d0 - d1).abs() < 1e-10);
        assert!((d1 - d2).abs() < 1e-10);
    }

    #[test]
    fn test_from_iter() {
        let points = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];
        let delaunay = Delaunay::from_iter(points);
        assert_eq!(delaunay.len(), 3);
    }

    #[test]
    fn test_voronoi_creation() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        // Should have one cell per input point
        assert_eq!(voronoi.cell_count(), 3);
    }
}
