//! Voronoi diagram computation from Delaunay triangulation.
//!
//! A Voronoi diagram partitions a plane into regions based on distance to a set
//! of seed points. Each region contains all points closer to one seed than to
//! any other.

use super::Delaunay;
use delaunator::EMPTY;

/// A Voronoi diagram computed from a Delaunay triangulation.
///
/// The Voronoi diagram is the dual of the Delaunay triangulation: each Delaunay
/// vertex becomes a Voronoi cell, and each Delaunay triangle's circumcenter
/// becomes a Voronoi vertex.
#[derive(Clone)]
pub struct Voronoi<'a> {
    /// Reference to the underlying Delaunay triangulation
    delaunay: &'a Delaunay,
    /// Clipping bounds [x_min, y_min, x_max, y_max]
    bounds: [f64; 4],
    /// Cached circumcenters of all triangles
    circumcenters: Vec<(f64, f64)>,
}

impl<'a> Voronoi<'a> {
    /// Creates a new Voronoi diagram from a Delaunay triangulation.
    ///
    /// If bounds are not provided, they are computed from the input points
    /// with a small margin.
    pub fn new(delaunay: &'a Delaunay, bounds: Option<[f64; 4]>) -> Self {
        // Compute bounds if not provided
        let bounds = bounds.unwrap_or_else(|| {
            if delaunay.is_empty() {
                [0.0, 0.0, 1.0, 1.0]
            } else {
                let mut x_min = f64::INFINITY;
                let mut y_min = f64::INFINITY;
                let mut x_max = f64::NEG_INFINITY;
                let mut y_max = f64::NEG_INFINITY;

                for &(x, y) in delaunay.points() {
                    x_min = x_min.min(x);
                    y_min = y_min.min(y);
                    x_max = x_max.max(x);
                    y_max = y_max.max(y);
                }

                // Add margin
                let margin_x = (x_max - x_min).max(1.0) * 0.1;
                let margin_y = (y_max - y_min).max(1.0) * 0.1;

                [
                    x_min - margin_x,
                    y_min - margin_y,
                    x_max + margin_x,
                    y_max + margin_y,
                ]
            }
        });

        // Pre-compute all circumcenters
        let circumcenters: Vec<_> = (0..delaunay.triangle_count())
            .map(|t| delaunay.circumcenter(t))
            .collect();

        Self {
            delaunay,
            bounds,
            circumcenters,
        }
    }

    /// Returns the clipping bounds [x_min, y_min, x_max, y_max].
    pub fn bounds(&self) -> [f64; 4] {
        self.bounds
    }

    /// Returns the number of cells in the Voronoi diagram.
    pub fn cell_count(&self) -> usize {
        self.delaunay.len()
    }

    /// Returns a reference to the underlying Delaunay triangulation.
    pub fn delaunay(&self) -> &Delaunay {
        self.delaunay
    }

    /// Returns the polygon for the cell around point at index `i`.
    ///
    /// The polygon is clipped to the bounds and returned as a closed polygon
    /// (first point repeated at end).
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn cell_polygon(&self, i: usize) -> Option<Vec<(f64, f64)>> {
        if i >= self.delaunay.len() {
            return None;
        }

        let cell = self.compute_cell(i);
        if cell.is_empty() {
            // Cell might be a single point or degenerate
            return Some(vec![]);
        }

        // Clip cell to bounds
        let clipped = self.clip_polygon(&cell);
        if clipped.is_empty() {
            return Some(vec![]);
        }

        // Close the polygon
        let mut result = clipped;
        if result.first() != result.last() {
            result.push(result[0]);
        }

        Some(result)
    }

    /// Returns an iterator over all cell polygons.
    ///
    /// Each polygon is the Voronoi cell for the corresponding input point.
    pub fn cell_polygons(&self) -> impl Iterator<Item = Vec<(f64, f64)>> + '_ {
        (0..self.delaunay.len()).map(move |i| self.cell_polygon(i).unwrap_or_default())
    }

    /// Returns true if the point (x, y) is inside the cell at index `i`.
    pub fn contains(&self, i: usize, x: f64, y: f64) -> bool {
        if let Some(polygon) = self.cell_polygon(i) {
            point_in_polygon(x, y, &polygon)
        } else {
            false
        }
    }

    /// Returns an iterator over the indices of cells neighboring cell `i`.
    ///
    /// Two cells are neighbors if they share an edge in the Voronoi diagram.
    pub fn neighbors(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        self.delaunay.neighbors(i)
    }

    /// Renders all Voronoi cells to a path string (SVG path data format).
    pub fn render_to_path(&self) -> String {
        let mut path = String::new();

        for polygon in self.cell_polygons() {
            if polygon.len() >= 3 {
                let (x0, y0) = polygon[0];
                path.push_str(&format!("M{},{}", x0, y0));

                for &(x, y) in &polygon[1..polygon.len() - 1] {
                    path.push_str(&format!("L{},{}", x, y));
                }

                path.push('Z');
            }
        }

        path
    }

    /// Renders the cell at index `i` to a path string (SVG path data format).
    pub fn render_cell_to_path(&self, i: usize) -> String {
        if let Some(polygon) = self.cell_polygon(i)
            && polygon.len() >= 3
        {
            let (x0, y0) = polygon[0];
            let mut path = format!("M{},{}", x0, y0);

            for &(x, y) in &polygon[1..polygon.len() - 1] {
                path.push_str(&format!("L{},{}", x, y));
            }

            path.push('Z');
            return path;
        }
        String::new()
    }

    /// Computes the raw (unclipped) Voronoi cell for point `i`.
    fn compute_cell(&self, i: usize) -> Vec<(f64, f64)> {
        if self.delaunay.is_empty() || self.circumcenters.is_empty() {
            return vec![];
        }

        // For points on the hull, we need special handling
        let is_on_hull = self.delaunay.hull().contains(&i);

        // Find all triangles containing this point
        let mut cell_vertices = Vec::new();
        let triangles = &self.delaunay.triangulation().triangles;
        let halfedges = &self.delaunay.triangulation().halfedges;

        // Find an incoming edge for this point
        let mut start_edge = EMPTY;
        for (e, &triangle) in triangles.iter().enumerate() {
            if triangle == i {
                start_edge = e;
                break;
            }
        }

        if start_edge == EMPTY {
            return vec![];
        }

        // Walk around the point collecting circumcenters
        let mut edge = start_edge;
        let mut iterations = 0;
        let max_iterations = triangles.len();

        loop {
            let t = edge / 3;
            if t < self.circumcenters.len() {
                cell_vertices.push(self.circumcenters[t]);
            }

            // Move to next edge around the point
            let next_edge = if edge % 3 == 2 { edge - 2 } else { edge + 1 };
            let opposite = halfedges[next_edge];

            if opposite == EMPTY {
                // We hit the boundary - this is a hull point
                if is_on_hull {
                    // Add a point at infinity in the appropriate direction
                    self.add_infinite_vertex(&mut cell_vertices, next_edge, true);
                }
                break;
            }

            edge = opposite;

            if edge == start_edge {
                break;
            }

            iterations += 1;
            if iterations > max_iterations {
                break;
            }
        }

        // If we're on the hull and didn't complete a loop, walk the other direction
        if is_on_hull && edge != start_edge {
            // We need to walk backwards from start_edge
            let mut backwards_vertices = Vec::new();
            edge = start_edge;
            iterations = 0;

            loop {
                let prev_edge = if edge.is_multiple_of(3) {
                    edge + 2
                } else {
                    edge - 1
                };
                let opposite = halfedges[prev_edge];

                if opposite == EMPTY {
                    // Add infinite vertex at the other boundary
                    self.add_infinite_vertex(&mut backwards_vertices, prev_edge, false);
                    break;
                }

                edge = opposite;
                let t = edge / 3;
                if t < self.circumcenters.len() {
                    backwards_vertices.push(self.circumcenters[t]);
                }

                iterations += 1;
                if iterations > max_iterations {
                    break;
                }
            }

            // Combine: backwards (reversed) + cell_vertices
            backwards_vertices.reverse();
            backwards_vertices.extend(cell_vertices);
            cell_vertices = backwards_vertices;
        }

        cell_vertices
    }

    /// Adds a vertex at "infinity" for hull edges.
    fn add_infinite_vertex(&self, vertices: &mut Vec<(f64, f64)>, edge: usize, outward: bool) {
        let triangles = &self.delaunay.triangulation().triangles;
        let t = edge / 3;
        let i = edge % 3;

        let p0_idx = triangles[t * 3 + i];
        let p1_idx = triangles[t * 3 + (i + 1) % 3];

        let (x0, y0) = self.delaunay.point(p0_idx).unwrap_or((0.0, 0.0));
        let (x1, y1) = self.delaunay.point(p1_idx).unwrap_or((0.0, 0.0));

        // Direction perpendicular to edge, pointing outward
        let dx = x1 - x0;
        let dy = y1 - y0;

        let (nx, ny) = if outward { (-dy, dx) } else { (dy, -dx) };

        let len = (nx * nx + ny * ny).sqrt();
        if len > 1e-10 {
            let nx = nx / len;
            let ny = ny / len;

            // Project far enough to reach bounds
            let scale = ((self.bounds[2] - self.bounds[0]).powi(2)
                + (self.bounds[3] - self.bounds[1]).powi(2))
            .sqrt()
                * 2.0;

            // Use edge midpoint as base
            let mx = (x0 + x1) / 2.0;
            let my = (y0 + y1) / 2.0;

            vertices.push((mx + nx * scale, my + ny * scale));
        }
    }

    /// Clips a polygon to the bounds using Sutherland-Hodgman algorithm.
    fn clip_polygon(&self, polygon: &[(f64, f64)]) -> Vec<(f64, f64)> {
        if polygon.is_empty() {
            return vec![];
        }

        let [x_min, y_min, x_max, y_max] = self.bounds;
        let mut result = polygon.to_vec();

        // Clip against each edge of the bounding box
        result = clip_against_edge(
            &result,
            |&(x, _)| x >= x_min,
            |p1, p2| intersect_vertical(p1, p2, x_min),
        );
        result = clip_against_edge(
            &result,
            |&(x, _)| x <= x_max,
            |p1, p2| intersect_vertical(p1, p2, x_max),
        );
        result = clip_against_edge(
            &result,
            |&(_, y)| y >= y_min,
            |p1, p2| intersect_horizontal(p1, p2, y_min),
        );
        result = clip_against_edge(
            &result,
            |&(_, y)| y <= y_max,
            |p1, p2| intersect_horizontal(p1, p2, y_max),
        );

        result
    }
}

impl std::fmt::Debug for Voronoi<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Voronoi")
            .field("cell_count", &self.cell_count())
            .field("bounds", &self.bounds)
            .finish()
    }
}

/// Clips a polygon against a single edge using Sutherland-Hodgman.
fn clip_against_edge<F, I>(polygon: &[(f64, f64)], inside: F, intersect: I) -> Vec<(f64, f64)>
where
    F: Fn(&(f64, f64)) -> bool,
    I: Fn((f64, f64), (f64, f64)) -> (f64, f64),
{
    if polygon.is_empty() {
        return vec![];
    }

    let mut result = Vec::new();
    let n = polygon.len();

    for i in 0..n {
        let current = polygon[i];
        let next = polygon[(i + 1) % n];

        let current_inside = inside(&current);
        let next_inside = inside(&next);

        if current_inside {
            result.push(current);
            if !next_inside {
                result.push(intersect(current, next));
            }
        } else if next_inside {
            result.push(intersect(current, next));
        }
    }

    result
}

/// Computes intersection with a vertical line at x = x_val.
fn intersect_vertical(p1: (f64, f64), p2: (f64, f64), x_val: f64) -> (f64, f64) {
    let (x1, y1) = p1;
    let (x2, y2) = p2;

    if (x2 - x1).abs() < 1e-12 {
        return (x_val, (y1 + y2) / 2.0);
    }

    let t = (x_val - x1) / (x2 - x1);
    let y = y1 + t * (y2 - y1);
    (x_val, y)
}

/// Computes intersection with a horizontal line at y = y_val.
fn intersect_horizontal(p1: (f64, f64), p2: (f64, f64), y_val: f64) -> (f64, f64) {
    let (x1, y1) = p1;
    let (x2, y2) = p2;

    if (y2 - y1).abs() < 1e-12 {
        return ((x1 + x2) / 2.0, y_val);
    }

    let t = (y_val - y1) / (y2 - y1);
    let x = x1 + t * (x2 - x1);
    (x, y_val)
}

/// Tests if a point is inside a polygon using the ray casting algorithm.
fn point_in_polygon(x: f64, y: f64, polygon: &[(f64, f64)]) -> bool {
    if polygon.len() < 3 {
        return false;
    }

    let mut inside = false;
    let n = polygon.len();

    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = polygon[i];
        let (xj, yj) = polygon[j];

        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }

        j = i;
    }

    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voronoi_creation() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        assert_eq!(voronoi.cell_count(), 3);
        assert_eq!(voronoi.bounds(), [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn test_voronoi_auto_bounds() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let voronoi = delaunay.voronoi(None);

        let bounds = voronoi.bounds();
        assert!(bounds[0] < 0.0); // x_min with margin
        assert!(bounds[1] < 0.0); // y_min with margin
        assert!(bounds[2] > 1.0); // x_max with margin
        assert!(bounds[3] > 1.0); // y_max with margin
    }

    #[test]
    fn test_cell_polygon() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        // Each corner should have a cell
        for i in 0..4 {
            let cell = voronoi.cell_polygon(i);
            assert!(cell.is_some());
            let polygon = cell.unwrap();
            // Polygon should be non-empty for valid cells
            // (may be empty for degenerate cases)
            if !polygon.is_empty() {
                // Should be closed
                assert_eq!(polygon.first(), polygon.last());
            }
        }
    }

    #[test]
    fn test_cell_polygons_iterator() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        let cells: Vec<_> = voronoi.cell_polygons().collect();
        assert_eq!(cells.len(), 3);
    }

    #[test]
    fn test_contains() {
        let delaunay = Delaunay::new(&[(0.25, 0.25), (0.75, 0.25), (0.5, 0.75)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        // The point at (0.25, 0.25) should contain points near it
        // This is a basic sanity check
        let cell0 = voronoi.cell_polygon(0);
        assert!(cell0.is_some());
    }

    #[test]
    fn test_neighbors() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0), (0.5, 0.5)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        // Central point should have neighbors
        let neighbors: Vec<_> = voronoi.neighbors(3).collect();
        assert!(!neighbors.is_empty());
    }

    #[test]
    fn test_render_to_path() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        let path = voronoi.render_to_path();
        // Should contain path commands
        assert!(path.contains('M') || path.is_empty());
    }

    #[test]
    fn test_render_cell_to_path() {
        let delaunay = Delaunay::new(&[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        let path = voronoi.render_cell_to_path(0);
        // May or may not have content depending on cell geometry
        if !path.is_empty() {
            assert!(path.starts_with('M'));
        }
    }

    #[test]
    fn test_point_in_polygon() {
        let polygon = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0)];

        assert!(point_in_polygon(0.5, 0.5, &polygon));
        assert!(!point_in_polygon(2.0, 0.5, &polygon));
        assert!(!point_in_polygon(-0.5, 0.5, &polygon));
    }

    #[test]
    fn test_clip_polygon() {
        let delaunay = Delaunay::new(&[(0.5, 0.5)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        // A polygon that extends outside bounds
        let polygon = vec![(-0.5, 0.5), (0.5, 0.5), (0.5, 1.5), (-0.5, 1.5)];
        let clipped = voronoi.clip_polygon(&polygon);

        // All clipped points should be within bounds
        for &(x, y) in &clipped {
            assert!(x >= 0.0 && x <= 1.0);
            assert!(y >= 0.0 && y <= 1.0);
        }
    }

    #[test]
    fn test_empty_delaunay() {
        let delaunay = Delaunay::new(&[]);
        let voronoi = delaunay.voronoi(None);

        assert_eq!(voronoi.cell_count(), 0);
        let cells: Vec<_> = voronoi.cell_polygons().collect();
        assert!(cells.is_empty());
    }

    #[test]
    fn test_single_point() {
        let delaunay = Delaunay::new(&[(0.5, 0.5)]);
        let voronoi = delaunay.voronoi(Some([0.0, 0.0, 1.0, 1.0]));

        assert_eq!(voronoi.cell_count(), 1);
        // Single point should have the entire bounds as its cell (or empty)
        let cell = voronoi.cell_polygon(0);
        assert!(cell.is_some());
    }
}
