//! # d3-delaunay — Delaunay triangulation and Voronoi diagrams
//!
//! This module wraps [`math_delaunay`] with a D3.js-compatible API.
//! The implementation is a faithful port of d3-delaunay using delaunator.

pub mod voronoi;

pub use math_delaunay::Delaunay as MathDelaunay;
pub use math_delaunay::Voronoi as MathVoronoi;

use crate::util::scratch::with_path_scratch;
use std::fmt;

/// Recoverable errors for checked Delaunay and Voronoi operations.
#[derive(Debug, Clone, PartialEq)]
pub enum DelaunayError {
    /// Input points must have finite coordinates before triangulation.
    NonFinitePointCoordinate {
        index: usize,
        coordinate: &'static str,
        value: f64,
    },
    /// Query coordinates must be finite.
    NonFiniteQueryCoordinate {
        coordinate: &'static str,
        value: f64,
    },
    /// Radius-limited queries must use a finite radius.
    NonFiniteRadius { radius: f64 },
    /// Radius-limited queries cannot use a negative radius.
    NegativeRadius { radius: f64 },
    /// Voronoi clipping bounds must be finite.
    NonFiniteVoronoiBound {
        coordinate: &'static str,
        value: f64,
    },
    /// Voronoi clipping bounds must be ordered min <= max.
    ReversedVoronoiBounds {
        axis: &'static str,
        min: f64,
        max: f64,
    },
}

impl fmt::Display for DelaunayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePointCoordinate {
                index,
                coordinate,
                value,
            } => write!(
                f,
                "delaunay point coordinate {coordinate} at index {index} is not finite: {value}"
            ),
            Self::NonFiniteQueryCoordinate { coordinate, value } => {
                write!(
                    f,
                    "delaunay query coordinate {coordinate} is not finite: {value}"
                )
            }
            Self::NonFiniteRadius { radius } => {
                write!(f, "delaunay query radius is not finite: {radius}")
            }
            Self::NegativeRadius { radius } => {
                write!(f, "delaunay query radius is negative: {radius}")
            }
            Self::NonFiniteVoronoiBound { coordinate, value } => write!(
                f,
                "delaunay Voronoi bound coordinate {coordinate} is not finite: {value}"
            ),
            Self::ReversedVoronoiBounds { axis, min, max } => {
                write!(
                    f,
                    "delaunay Voronoi {axis} bounds are reversed: {min} > {max}"
                )
            }
        }
    }
}

impl std::error::Error for DelaunayError {}

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

    /// Create a Delaunay triangulation after validating all point coordinates.
    pub fn try_new(points: &[(f64, f64)]) -> Result<Self, DelaunayError> {
        validate_points(points)?;
        Ok(Self::new(points))
    }

    /// Create from an iterator of (x, y) tuples.
    pub fn from_points_iter<I: IntoIterator<Item = (f64, f64)>>(iter: I) -> Self {
        let points: Vec<(f64, f64)> = iter.into_iter().collect();
        Self::new(&points)
    }

    /// Create from an iterator after validating all point coordinates.
    pub fn try_from_points_iter<I: IntoIterator<Item = (f64, f64)>>(
        iter: I,
    ) -> Result<Self, DelaunayError> {
        let points: Vec<(f64, f64)> = iter.into_iter().collect();
        Self::try_new(&points)
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

    /// Find the nearest point after validating query coordinates.
    pub fn try_find(
        &self,
        x: f64,
        y: f64,
        start: Option<usize>,
    ) -> Result<Option<usize>, DelaunayError> {
        validate_query_coordinate("x", x)?;
        validate_query_coordinate("y", y)?;
        Ok(self.find(x, y, start))
    }

    /// Find the nearest point within a given radius.
    pub fn find_within_radius(&self, x: f64, y: f64, radius: f64) -> Option<usize> {
        let found = self.find(x, y, None)?;
        let (px, py) = self.points_tuples[found];
        let dist = ((px - x).powi(2) + (py - y).powi(2)).sqrt();
        if dist <= radius { Some(found) } else { None }
    }

    /// Find the nearest point within a radius after validating query inputs.
    pub fn try_find_within_radius(
        &self,
        x: f64,
        y: f64,
        radius: f64,
    ) -> Result<Option<usize>, DelaunayError> {
        validate_query_coordinate("x", x)?;
        validate_query_coordinate("y", y)?;
        if !radius.is_finite() {
            return Err(DelaunayError::NonFiniteRadius { radius });
        }
        if radius < 0.0 {
            return Err(DelaunayError::NegativeRadius { radius });
        }

        Ok(self.find_within_radius(x, y, radius))
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
        let bounds = bounds.unwrap_or_else(|| self.inferred_voronoi_bounds());
        voronoi::Voronoi::new(&self.inner, bounds)
    }

    /// Create a Voronoi diagram after validating explicit or inferred bounds.
    pub fn try_voronoi(
        &self,
        bounds: Option<[f64; 4]>,
    ) -> Result<voronoi::Voronoi<'_>, DelaunayError> {
        let bounds = bounds.unwrap_or_else(|| self.inferred_voronoi_bounds());
        validate_voronoi_bounds(bounds)?;
        Ok(voronoi::Voronoi::new(&self.inner, bounds))
    }

    /// Access the underlying math-delaunay Delaunay.
    pub fn inner(&self) -> &MathDelaunay {
        &self.inner
    }

    fn inferred_voronoi_bounds(&self) -> [f64; 4] {
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
    }
}

fn validate_points(points: &[(f64, f64)]) -> Result<(), DelaunayError> {
    for (index, &(x, y)) in points.iter().enumerate() {
        if !x.is_finite() {
            return Err(DelaunayError::NonFinitePointCoordinate {
                index,
                coordinate: "x",
                value: x,
            });
        }
        if !y.is_finite() {
            return Err(DelaunayError::NonFinitePointCoordinate {
                index,
                coordinate: "y",
                value: y,
            });
        }
    }

    Ok(())
}

fn validate_query_coordinate(coordinate: &'static str, value: f64) -> Result<(), DelaunayError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(DelaunayError::NonFiniteQueryCoordinate { coordinate, value })
    }
}

fn validate_voronoi_bounds(bounds: [f64; 4]) -> Result<(), DelaunayError> {
    let [x0, y0, x1, y1] = bounds;
    validate_voronoi_bound("x0", x0)?;
    validate_voronoi_bound("y0", y0)?;
    validate_voronoi_bound("x1", x1)?;
    validate_voronoi_bound("y1", y1)?;

    if x0 > x1 {
        return Err(DelaunayError::ReversedVoronoiBounds {
            axis: "x",
            min: x0,
            max: x1,
        });
    }
    if y0 > y1 {
        return Err(DelaunayError::ReversedVoronoiBounds {
            axis: "y",
            min: y0,
            max: y1,
        });
    }

    Ok(())
}

fn validate_voronoi_bound(coordinate: &'static str, value: f64) -> Result<(), DelaunayError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(DelaunayError::NonFiniteVoronoiBound { coordinate, value })
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

    #[test]
    fn checked_delaunay_matches_permissive_for_valid_points() {
        let points = &[(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)];
        let permissive = Delaunay::new(points);
        let checked = Delaunay::try_new(points).unwrap();

        assert_eq!(checked.len(), permissive.len());
        assert_eq!(checked.points(), permissive.points());
        assert_eq!(checked.triangle_count(), permissive.triangle_count());
    }

    #[test]
    fn checked_delaunay_rejects_non_finite_points() {
        assert_eq!(
            Delaunay::try_new(&[(0.0, 0.0), (f64::INFINITY, 1.0)])
                .err()
                .unwrap(),
            DelaunayError::NonFinitePointCoordinate {
                index: 1,
                coordinate: "x",
                value: f64::INFINITY
            }
        );

        assert!(matches!(
            Delaunay::try_from_points_iter([(0.0, 0.0), (1.0, f64::NAN)]),
            Err(DelaunayError::NonFinitePointCoordinate {
                index: 1,
                coordinate: "y",
                value,
            }) if value.is_nan()
        ));
    }

    #[test]
    fn checked_delaunay_validates_query_inputs() {
        let delaunay = Delaunay::try_new(&[(0.0, 0.0), (10.0, 0.0)]).unwrap();

        assert_eq!(delaunay.try_find(1.0, 0.0, None).unwrap(), Some(0));
        assert_eq!(
            delaunay.try_find(f64::INFINITY, 0.0, None).unwrap_err(),
            DelaunayError::NonFiniteQueryCoordinate {
                coordinate: "x",
                value: f64::INFINITY
            }
        );
        assert_eq!(
            delaunay.try_find_within_radius(1.0, 0.0, -1.0).unwrap_err(),
            DelaunayError::NegativeRadius { radius: -1.0 }
        );
        assert!(matches!(
            delaunay.try_find_within_radius(1.0, 0.0, f64::NAN),
            Err(DelaunayError::NonFiniteRadius { radius }) if radius.is_nan()
        ));
    }

    #[test]
    fn checked_delaunay_validates_voronoi_bounds() {
        let delaunay = Delaunay::try_new(&[(0.0, 0.0), (10.0, 0.0)]).unwrap();
        let voronoi = delaunay.try_voronoi(Some([0.0, 0.0, 10.0, 10.0])).unwrap();
        assert_eq!(voronoi.bounds(), [0.0, 0.0, 10.0, 10.0]);

        assert_eq!(
            delaunay
                .try_voronoi(Some([10.0, 0.0, 0.0, 10.0]))
                .err()
                .unwrap(),
            DelaunayError::ReversedVoronoiBounds {
                axis: "x",
                min: 10.0,
                max: 0.0
            }
        );
        assert_eq!(
            delaunay
                .try_voronoi(Some([0.0, f64::INFINITY, 10.0, 10.0]))
                .err()
                .unwrap(),
            DelaunayError::NonFiniteVoronoiBound {
                coordinate: "y0",
                value: f64::INFINITY
            }
        );
    }
}
