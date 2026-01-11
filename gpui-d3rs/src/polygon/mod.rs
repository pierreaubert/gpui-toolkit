//! Polygon utilities (d3-polygon)
//!
//! This module provides functions for computing geometric properties of polygons.
//!
//! # Example
//!
//! ```
//! use d3rs::polygon::{polygon_area, polygon_centroid, polygon_contains};
//!
//! let triangle = vec![(0.0, 0.0), (100.0, 0.0), (50.0, 100.0)];
//!
//! let area = polygon_area(&triangle);
//! assert!((area - 5000.0).abs() < 1e-6);
//!
//! let (cx, cy) = polygon_centroid(&triangle);
//! assert!((cx - 50.0).abs() < 1e-6);
//! ```

/// Compute the signed area of a polygon.
///
/// Returns a positive value for counter-clockwise polygons,
/// negative for clockwise polygons.
///
/// # Example
///
/// ```
/// use d3rs::polygon::polygon_area;
///
/// let square = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
/// assert!((polygon_area(&square) - 1.0).abs() < 1e-10);
/// ```
pub fn polygon_area(polygon: &[(f64, f64)]) -> f64 {
    let n = polygon.len();
    if n < 3 {
        return 0.0;
    }

    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += polygon[i].0 * polygon[j].1;
        sum -= polygon[j].0 * polygon[i].1;
    }

    sum.abs() / 2.0
}

/// Compute the signed area of a polygon (preserves sign).
///
/// Returns a positive value for counter-clockwise polygons,
/// negative for clockwise polygons.
pub fn polygon_area_signed(polygon: &[(f64, f64)]) -> f64 {
    let n = polygon.len();
    if n < 3 {
        return 0.0;
    }

    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += polygon[i].0 * polygon[j].1;
        sum -= polygon[j].0 * polygon[i].1;
    }

    sum / 2.0
}

/// Compute the centroid (center of mass) of a polygon.
///
/// # Example
///
/// ```
/// use d3rs::polygon::polygon_centroid;
///
/// let square = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
/// let (cx, cy) = polygon_centroid(&square);
/// assert!((cx - 1.0).abs() < 1e-10);
/// assert!((cy - 1.0).abs() < 1e-10);
/// ```
pub fn polygon_centroid(polygon: &[(f64, f64)]) -> (f64, f64) {
    let n = polygon.len();
    if n == 0 {
        return (f64::NAN, f64::NAN);
    }
    if n == 1 {
        return polygon[0];
    }
    if n == 2 {
        return (
            (polygon[0].0 + polygon[1].0) / 2.0,
            (polygon[0].1 + polygon[1].1) / 2.0,
        );
    }

    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut area = 0.0;

    for i in 0..n {
        let j = (i + 1) % n;
        let cross = polygon[i].0 * polygon[j].1 - polygon[j].0 * polygon[i].1;
        cx += (polygon[i].0 + polygon[j].0) * cross;
        cy += (polygon[i].1 + polygon[j].1) * cross;
        area += cross;
    }

    if area.abs() < 1e-10 {
        // Degenerate polygon, compute simple average
        let sum: (f64, f64) = polygon
            .iter()
            .fold((0.0, 0.0), |acc, p| (acc.0 + p.0, acc.1 + p.1));
        return (sum.0 / n as f64, sum.1 / n as f64);
    }

    area *= 3.0;
    (cx / area, cy / area)
}

/// Test if a point is inside a polygon.
///
/// Uses the ray casting algorithm.
///
/// # Example
///
/// ```
/// use d3rs::polygon::polygon_contains;
///
/// let square = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
/// assert!(polygon_contains(&square, (1.0, 1.0)));
/// assert!(!polygon_contains(&square, (3.0, 3.0)));
/// ```
pub fn polygon_contains(polygon: &[(f64, f64)], point: (f64, f64)) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }

    let (x, y) = point;
    let mut inside = false;

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

/// Compute the perimeter (total edge length) of a polygon.
///
/// # Example
///
/// ```
/// use d3rs::polygon::polygon_length;
///
/// let square = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
/// assert!((polygon_length(&square) - 4.0).abs() < 1e-10);
/// ```
pub fn polygon_length(polygon: &[(f64, f64)]) -> f64 {
    let n = polygon.len();
    if n < 2 {
        return 0.0;
    }

    let mut length = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let dx = polygon[j].0 - polygon[i].0;
        let dy = polygon[j].1 - polygon[i].1;
        length += (dx * dx + dy * dy).sqrt();
    }

    length
}

/// Compute the convex hull of a set of points.
///
/// Returns the vertices of the convex hull in counter-clockwise order.
/// Uses Andrew's monotone chain algorithm.
///
/// # Example
///
/// ```
/// use d3rs::polygon::polygon_hull;
///
/// let points = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 0.5), (1.0, 1.0), (0.0, 1.0)];
/// let hull = polygon_hull(&points);
/// assert_eq!(hull.len(), 4); // The square vertices, excluding the interior point
/// ```
pub fn polygon_hull(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }

    // Sort points lexicographically
    let mut sorted: Vec<(f64, f64)> = points.to_vec();
    sorted.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });

    fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    }

    // Build lower hull
    let mut lower = Vec::new();
    for &p in &sorted {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }

    // Build upper hull
    let mut upper = Vec::new();
    for &p in sorted.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }

    // Remove the last point of each half because it's repeated
    lower.pop();
    upper.pop();

    // Concatenate
    lower.extend(upper);
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polygon_area_triangle() {
        let triangle = vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)];
        assert!((polygon_area(&triangle) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_polygon_area_square() {
        let square = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!((polygon_area(&square) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_polygon_area_signed() {
        // Counter-clockwise
        let ccw = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!(polygon_area_signed(&ccw) > 0.0);

        // Clockwise
        let cw = vec![(0.0, 0.0), (0.0, 1.0), (1.0, 1.0), (1.0, 0.0)];
        assert!(polygon_area_signed(&cw) < 0.0);
    }

    #[test]
    fn test_polygon_centroid() {
        let square = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
        let (cx, cy) = polygon_centroid(&square);
        assert!((cx - 1.0).abs() < 1e-10);
        assert!((cy - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_polygon_contains() {
        let square = vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)];
        assert!(polygon_contains(&square, (1.0, 1.0)));
        assert!(polygon_contains(&square, (0.5, 0.5)));
        assert!(!polygon_contains(&square, (-1.0, 1.0)));
        assert!(!polygon_contains(&square, (3.0, 1.0)));
    }

    #[test]
    fn test_polygon_length() {
        let square = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        assert!((polygon_length(&square) - 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_polygon_hull() {
        // Points with interior point
        let points = vec![(0.0, 0.0), (1.0, 0.0), (0.5, 0.5), (1.0, 1.0), (0.0, 1.0)];
        let hull = polygon_hull(&points);
        assert_eq!(hull.len(), 4);

        // All points on hull
        let square = vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let hull = polygon_hull(&square);
        assert_eq!(hull.len(), 4);
    }

    #[test]
    fn test_polygon_hull_collinear() {
        let points = vec![(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)];
        let hull = polygon_hull(&points);
        assert!(hull.len() >= 2);
    }
}
