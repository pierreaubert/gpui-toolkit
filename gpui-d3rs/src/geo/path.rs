//! GeoPath - Rendering GeoJSON to paths
//!
//! This module provides functionality for rendering GeoJSON features
//! to SVG path strings or other path representations.

use super::projection::Projection;

/// Configuration for GeoPath rendering.
#[derive(Clone, Debug)]
pub struct GeoPathConfig {
    /// Number of decimal places for path coordinates
    pub digits: usize,
    /// Radius for point features
    pub point_radius: f64,
}

impl Default for GeoPathConfig {
    fn default() -> Self {
        Self {
            digits: 3,
            point_radius: 4.5,
        }
    }
}

/// A GeoJSON geometry type.
#[derive(Clone, Debug)]
pub enum GeoJsonGeometry {
    /// A single point
    Point(f64, f64),
    /// Multiple points
    MultiPoint(Vec<(f64, f64)>),
    /// A line string (series of connected points)
    LineString(Vec<(f64, f64)>),
    /// Multiple line strings
    MultiLineString(Vec<Vec<(f64, f64)>>),
    /// A polygon (closed ring with optional holes)
    Polygon(Vec<Vec<(f64, f64)>>),
    /// Multiple polygons
    MultiPolygon(Vec<Vec<Vec<(f64, f64)>>>),
}

/// A path generator for GeoJSON geometries.
///
/// GeoPath projects geographic coordinates using a projection and generates
/// SVG path data or other path representations.
#[derive(Clone)]
pub struct GeoPath<P: Projection> {
    projection: P,
    config: GeoPathConfig,
}

impl<P: Projection> GeoPath<P> {
    /// Create a new GeoPath with the given projection.
    pub fn new(projection: P) -> Self {
        Self {
            projection,
            config: GeoPathConfig::default(),
        }
    }

    /// Set the number of decimal places for path coordinates.
    pub fn digits(mut self, digits: usize) -> Self {
        self.config.digits = digits;
        self
    }

    /// Set the point radius for point features.
    pub fn point_radius(mut self, radius: f64) -> Self {
        self.config.point_radius = radius;
        self
    }

    /// Get a reference to the projection.
    pub fn projection(&self) -> &P {
        &self.projection
    }

    /// Get a mutable reference to the projection.
    pub fn projection_mut(&mut self) -> &mut P {
        &mut self.projection
    }

    /// Render a GeoJSON geometry to an SVG path string.
    pub fn render(&self, geometry: &GeoJsonGeometry) -> String {
        match geometry {
            GeoJsonGeometry::Point(lon, lat) => self.render_point(*lon, *lat),
            GeoJsonGeometry::MultiPoint(points) => self.render_multi_point(points),
            GeoJsonGeometry::LineString(coords) => self.render_line_string(coords),
            GeoJsonGeometry::MultiLineString(lines) => self.render_multi_line_string(lines),
            GeoJsonGeometry::Polygon(rings) => self.render_polygon(rings),
            GeoJsonGeometry::MultiPolygon(polygons) => self.render_multi_polygon(polygons),
        }
    }

    /// Render a point as a circle.
    fn render_point(&self, lon: f64, lat: f64) -> String {
        let (x, y) = self.projection.project(lon, lat);
        let r = self.config.point_radius;
        let d = self.config.digits;

        // Create a circle path
        format!(
            "M{:.d$},{:.d$}m0,{:.d$}a{:.d$},{:.d$} 0 1,1 0,-{:.d$}a{:.d$},{:.d$} 0 1,1 0,{:.d$}z",
            x,
            y,
            r,
            r,
            r,
            2.0 * r,
            r,
            r,
            2.0 * r,
            d = d
        )
    }

    /// Render multiple points.
    fn render_multi_point(&self, points: &[(f64, f64)]) -> String {
        points
            .iter()
            .map(|&(lon, lat)| self.render_point(lon, lat))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Render a line string.
    fn render_line_string(&self, coords: &[(f64, f64)]) -> String {
        if coords.is_empty() {
            return String::new();
        }

        let d = self.config.digits;
        let mut path = String::new();
        let mut prev_lon: Option<f64> = None;

        for (i, &(lon, lat)) in coords.iter().enumerate() {
            let (x, y) = self.projection.project(lon, lat);

            // Check if coordinates are valid
            if !x.is_finite() || !y.is_finite() {
                prev_lon = None;
                continue;
            }

            // Detect antimeridian crossing: if longitude jumps > 180 degrees
            let crosses_antimeridian = if let Some(prev) = prev_lon {
                (lon - prev).abs() > 180.0
            } else {
                false
            };

            if i == 0 || crosses_antimeridian {
                path.push_str(&format!("M{:.d$},{:.d$}", x, y, d = d));
            } else {
                path.push_str(&format!("L{:.d$},{:.d$}", x, y, d = d));
            }

            prev_lon = Some(lon);
        }

        path
    }

    /// Render multiple line strings.
    fn render_multi_line_string(&self, lines: &[Vec<(f64, f64)>]) -> String {
        lines
            .iter()
            .map(|line| self.render_line_string(line))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Render a polygon.
    fn render_polygon(&self, rings: &[Vec<(f64, f64)>]) -> String {
        if rings.is_empty() {
            return String::new();
        }

        let d = self.config.digits;
        let mut path = String::new();

        for ring in rings {
            if ring.is_empty() {
                continue;
            }

            let mut prev_lon: Option<f64> = None;
            let mut ring_started = false;

            for (i, &(lon, lat)) in ring.iter().enumerate() {
                let (x, y) = self.projection.project(lon, lat);

                // Check if coordinates are valid
                if !x.is_finite() || !y.is_finite() {
                    prev_lon = None;
                    if ring_started {
                        path.push('Z');
                        ring_started = false;
                    }
                    continue;
                }

                // Detect antimeridian crossing: if longitude jumps > 180 degrees
                let crosses_antimeridian = if let Some(prev) = prev_lon {
                    (lon - prev).abs() > 180.0
                } else {
                    false
                };

                if i == 0 || crosses_antimeridian {
                    // Close previous segment if we're breaking due to antimeridian
                    if ring_started && crosses_antimeridian {
                        path.push('Z');
                    }
                    path.push_str(&format!("M{:.d$},{:.d$}", x, y, d = d));
                    ring_started = true;
                } else {
                    path.push_str(&format!("L{:.d$},{:.d$}", x, y, d = d));
                }

                prev_lon = Some(lon);
            }

            // Close the ring
            if ring_started {
                path.push('Z');
            }
        }

        path
    }

    /// Render multiple polygons.
    fn render_multi_polygon(&self, polygons: &[Vec<Vec<(f64, f64)>>]) -> String {
        polygons
            .iter()
            .map(|polygon| self.render_polygon(polygon))
            .collect::<Vec<_>>()
            .join("")
    }

    /// Render coordinates to a vector of projected (x, y) points.
    pub fn project_coords(&self, coords: &[(f64, f64)]) -> Vec<(f64, f64)> {
        coords
            .iter()
            .map(|&(lon, lat)| self.projection.project(lon, lat))
            .collect()
    }

    /// Calculate the bounds of a geometry after projection.
    ///
    /// Returns `((min_x, min_y), (max_x, max_y))`
    pub fn bounds(&self, geometry: &GeoJsonGeometry) -> ((f64, f64), (f64, f64)) {
        let coords = self.geometry_coordinates(geometry);
        if coords.is_empty() {
            return ((f64::NAN, f64::NAN), (f64::NAN, f64::NAN));
        }

        let mut min_x = f64::MAX;
        let mut max_x = f64::MIN;
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;

        for (lon, lat) in coords {
            let (x, y) = self.projection.project(lon, lat);
            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }

        ((min_x, min_y), (max_x, max_y))
    }

    /// Calculate the centroid of a geometry after projection.
    pub fn centroid(&self, geometry: &GeoJsonGeometry) -> (f64, f64) {
        let coords = self.geometry_coordinates(geometry);
        if coords.is_empty() {
            return (f64::NAN, f64::NAN);
        }

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let n = coords.len() as f64;

        for (lon, lat) in coords {
            let (x, y) = self.projection.project(lon, lat);
            sum_x += x;
            sum_y += y;
        }

        (sum_x / n, sum_y / n)
    }

    /// Extract all coordinates from a geometry.
    fn geometry_coordinates(&self, geometry: &GeoJsonGeometry) -> Vec<(f64, f64)> {
        match geometry {
            GeoJsonGeometry::Point(lon, lat) => vec![(*lon, *lat)],
            GeoJsonGeometry::MultiPoint(points) => points.clone(),
            GeoJsonGeometry::LineString(coords) => coords.clone(),
            GeoJsonGeometry::MultiLineString(lines) => lines.iter().flatten().copied().collect(),
            GeoJsonGeometry::Polygon(rings) => rings.iter().flatten().copied().collect(),
            GeoJsonGeometry::MultiPolygon(polygons) => {
                polygons.iter().flatten().flatten().copied().collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::projection::Equirectangular;

    #[test]
    fn test_geo_path_point() {
        let proj = Equirectangular::new().scale(100.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::Point(0.0, 0.0);
        let svg = path.render(&geometry);
        assert!(svg.starts_with('M'));
        assert!(svg.contains('a')); // arc commands for circle
    }

    #[test]
    fn test_geo_path_line_string() {
        let proj = Equirectangular::new().scale(100.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::LineString(vec![(0.0, 0.0), (10.0, 10.0), (20.0, 0.0)]);
        let svg = path.render(&geometry);
        assert!(svg.starts_with('M'));
        assert!(svg.contains('L'));
    }

    #[test]
    fn test_geo_path_polygon() {
        let proj = Equirectangular::new().scale(100.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::Polygon(vec![vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 0.0),
        ]]);
        let svg = path.render(&geometry);
        assert!(svg.starts_with('M'));
        assert!(svg.ends_with('Z'));
    }

    #[test]
    fn test_geo_path_bounds() {
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::LineString(vec![(0.0, 0.0), (90.0, 45.0)]);
        let ((min_x, min_y), (max_x, max_y)) = path.bounds(&geometry);

        assert!(min_x < max_x);
        assert!(min_y < max_y);
    }

    #[test]
    fn test_geo_path_centroid() {
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::LineString(vec![(0.0, 0.0), (10.0, 0.0)]);
        let (cx, cy) = path.centroid(&geometry);

        // Centroid of a line from 0 to 10 should be at 5
        assert!((cx - 5.0 * std::f64::consts::PI / 180.0).abs() < 1e-6);
        assert!(cy.abs() < 1e-6);
    }

    #[test]
    fn test_geo_path_project_coords() {
        let proj = Equirectangular::new().scale(100.0).translate(500.0, 300.0);
        let path = GeoPath::new(proj);

        let coords = vec![(0.0, 0.0), (90.0, 0.0)];
        let projected = path.project_coords(&coords);

        assert_eq!(projected.len(), 2);
        // Origin should project to translate point
        assert!((projected[0].0 - 500.0).abs() < 1e-6);
        assert!((projected[0].1 - 300.0).abs() < 1e-6);
    }

    #[test]
    fn test_antimeridian_line_string_crossing() {
        // Line crossing from Russia (170°E) to Alaska (-170°W)
        // This should create two separate path segments
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::LineString(vec![
            (170.0, 60.0),  // Eastern Russia
            (-170.0, 60.0), // Alaska
        ]);
        let svg = path.render(&geometry);

        // Should have two M commands (one for start, one after crossing)
        let m_count = svg.matches('M').count();
        assert_eq!(
            m_count, 2,
            "Should have 2 move commands for antimeridian crossing"
        );

        // Should NOT have L command connecting them
        let l_count = svg.matches('L').count();
        assert_eq!(
            l_count, 0,
            "Should have no line commands across antimeridian"
        );
    }

    #[test]
    fn test_antimeridian_line_string_no_crossing() {
        // Line NOT crossing antimeridian (normal case)
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::LineString(vec![(10.0, 60.0), (20.0, 60.0), (30.0, 60.0)]);
        let svg = path.render(&geometry);

        // Should have only 1 M command at start
        let m_count = svg.matches('M').count();
        assert_eq!(m_count, 1, "Should have 1 move command for normal line");

        // Should have L commands connecting points
        let l_count = svg.matches('L').count();
        assert_eq!(l_count, 2, "Should have 2 line commands for 3-point line");
    }

    #[test]
    fn test_antimeridian_polygon_crossing() {
        // Polygon crossing antimeridian (like part of Russia)
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::Polygon(vec![vec![
            (170.0, 65.0),
            (175.0, 60.0),
            (-175.0, 60.0), // Crosses antimeridian here
            (-170.0, 65.0),
            (170.0, 65.0), // Close the ring
        ]]);
        let svg = path.render(&geometry);

        // Should have multiple M commands due to antimeridian breaks
        let m_count = svg.matches('M').count();
        assert!(
            m_count >= 2,
            "Should have at least 2 move commands for crossing polygon"
        );

        // Should have multiple Z commands (closed segments)
        let z_count = svg.matches('Z').count();
        assert!(
            z_count >= 2,
            "Should have at least 2 close commands for crossing polygon"
        );
    }

    #[test]
    fn test_antimeridian_polygon_no_crossing() {
        // Normal polygon not crossing antimeridian
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::Polygon(vec![vec![
            (10.0, 50.0),
            (20.0, 50.0),
            (20.0, 60.0),
            (10.0, 60.0),
            (10.0, 50.0),
        ]]);
        let svg = path.render(&geometry);

        // Should have only 1 M command
        let m_count = svg.matches('M').count();
        assert_eq!(m_count, 1, "Should have 1 move command for normal polygon");

        // Should have exactly 1 Z command (closed ring)
        let z_count = svg.matches('Z').count();
        assert_eq!(z_count, 1, "Should have 1 close command for normal polygon");
    }

    #[test]
    fn test_antimeridian_multipolygon_with_crossing() {
        // MultiPolygon with one polygon crossing antimeridian
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::MultiPolygon(vec![
            // First polygon: normal (Europe)
            vec![vec![
                (0.0, 50.0),
                (10.0, 50.0),
                (10.0, 55.0),
                (0.0, 55.0),
                (0.0, 50.0),
            ]],
            // Second polygon: crosses antimeridian (Russia/Alaska)
            vec![vec![
                (170.0, 60.0),
                (175.0, 60.0),
                (-175.0, 60.0), // Crossing
                (-170.0, 60.0),
                (170.0, 60.0),
            ]],
        ]);
        let svg = path.render(&geometry);

        // Should have M commands for both polygons plus breaks
        let m_count = svg.matches('M').count();
        assert!(
            m_count >= 3,
            "Should have at least 3 move commands (2 polygons + crossing)"
        );

        // Should have multiple Z commands
        let z_count = svg.matches('Z').count();
        assert!(z_count >= 3, "Should have at least 3 close commands");
    }

    #[test]
    fn test_invalid_coordinates_handling() {
        // Test that infinite/NaN coordinates are handled gracefully
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        // LineString with valid and invalid coordinates
        let geometry = GeoJsonGeometry::LineString(vec![
            (0.0, 0.0),
            (10.0, 10.0),
            (f64::NAN, f64::NAN), // Invalid
            (20.0, 20.0),
            (30.0, 30.0),
        ]);
        let svg = path.render(&geometry);

        // Should successfully render (not panic)
        assert!(!svg.is_empty(), "Should render despite invalid coordinates");

        // Should have M commands before and after invalid point
        let m_count = svg.matches('M').count();
        assert!(m_count >= 1, "Should have at least 1 move command");
    }

    #[test]
    fn test_antimeridian_with_orthographic_projection() {
        // Test antimeridian handling with non-Equirectangular projection
        use crate::geo::projection::Orthographic;

        let proj = Orthographic::new().scale(100.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::LineString(vec![(170.0, 0.0), (-170.0, 0.0)]);
        let svg = path.render(&geometry);

        // Should detect crossing even with different projection
        let m_count = svg.matches('M').count();
        assert!(
            m_count >= 1,
            "Should handle antimeridian with Orthographic projection"
        );
    }

    #[test]
    fn test_antimeridian_multiple_crossings() {
        // Line with multiple antimeridian crossings
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        let geometry = GeoJsonGeometry::LineString(vec![
            (170.0, 0.0),
            (-170.0, 0.0), // First crossing
            (-160.0, 0.0),
            (160.0, 0.0), // Second crossing (going back)
        ]);
        let svg = path.render(&geometry);

        // Should have multiple M commands for multiple crossings
        let m_count = svg.matches('M').count();
        assert!(
            m_count >= 3,
            "Should have at least 3 move commands for 2 crossings"
        );
    }

    #[test]
    fn test_antimeridian_edge_cases() {
        let proj = Equirectangular::new().scale(1.0).translate(0.0, 0.0);
        let path = GeoPath::new(proj);

        // Test exactly at +180
        let geometry1 =
            GeoJsonGeometry::LineString(vec![(179.0, 0.0), (180.0, 0.0), (-179.0, 0.0)]);
        let svg1 = path.render(&geometry1);
        assert!(!svg1.is_empty(), "Should handle +180 boundary");

        // Test exactly at -180
        let geometry2 =
            GeoJsonGeometry::LineString(vec![(-179.0, 0.0), (-180.0, 0.0), (179.0, 0.0)]);
        let svg2 = path.render(&geometry2);
        assert!(!svg2.is_empty(), "Should handle -180 boundary");
    }
}
