//! d3-geo - Geographic projections and paths
//!
//! This module provides D3.js-style geographic projections for mapping
//! spherical coordinates (longitude, latitude) to planar coordinates (x, y).
//!
//! # Example
//!
//! ```rust
//! use d3rs::geo::{Mercator, Projection};
//!
//! let projection = Mercator::new()
//!     .scale(100.0)
//!     .translate(400.0, 300.0);
//!
//! // Project a point (longitude, latitude) to (x, y)
//! let (x, y) = projection.project(0.0, 0.0);
//!
//! // Inverse projection (x, y) to (longitude, latitude)
//! if let Some((lon, lat)) = projection.invert(x, y) {
//!     println!("Longitude: {}, Latitude: {}", lon, lat);
//! }
//! ```

mod graticule;
mod path;
pub mod projection;

pub use graticule::{Graticule, GraticuleConfig};
pub use path::{GeoJsonGeometry, GeoPath, GeoPathConfig};
pub use projection::{
    Albers, ConicEqualArea, Equirectangular, Mercator, Orthographic, Projection, Stereographic,
    TransverseMercator,
};

use std::f64::consts::PI;

/// Converts degrees to radians
#[inline]
pub fn radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

/// Converts radians to degrees
#[inline]
pub fn degrees(radians: f64) -> f64 {
    radians * 180.0 / PI
}

/// Half of PI
pub const HALF_PI: f64 = PI / 2.0;

/// Two times PI (tau)
pub const TAU: f64 = 2.0 * PI;

/// Small epsilon for floating point comparisons
pub const EPSILON: f64 = 1e-6;

/// Calculate the great-circle distance between two points on a sphere.
///
/// Uses the Haversine formula. Returns distance in radians.
/// Multiply by Earth's radius (6371 km) for kilometers.
///
/// # Arguments
/// * `lon1`, `lat1` - First point in degrees
/// * `lon2`, `lat2` - Second point in degrees
///
/// # Example
/// ```rust
/// use d3rs::geo::geo_distance;
/// let dist = geo_distance(0.0, 0.0, 90.0, 0.0);
/// assert!((dist - std::f64::consts::FRAC_PI_2).abs() < 1e-10);
/// ```
pub fn geo_distance(lon1: f64, lat1: f64, lon2: f64, lat2: f64) -> f64 {
    let lambda1 = radians(lon1);
    let phi1 = radians(lat1);
    let lambda2 = radians(lon2);
    let phi2 = radians(lat2);

    let sin_phi1 = phi1.sin();
    let cos_phi1 = phi1.cos();
    let sin_phi2 = phi2.sin();
    let cos_phi2 = phi2.cos();
    let delta_lambda = lambda2 - lambda1;
    let cos_delta_lambda = delta_lambda.cos();

    let sin_sigma = ((cos_phi2 * delta_lambda.sin()).powi(2)
        + (cos_phi1 * sin_phi2 - sin_phi1 * cos_phi2 * cos_delta_lambda).powi(2))
    .sqrt();
    let cos_sigma = sin_phi1 * sin_phi2 + cos_phi1 * cos_phi2 * cos_delta_lambda;

    sin_sigma.atan2(cos_sigma)
}

/// Calculate the length of a GeoJSON LineString or MultiLineString in radians.
///
/// # Arguments
/// * `coordinates` - Slice of (longitude, latitude) pairs in degrees
pub fn geo_length(coordinates: &[(f64, f64)]) -> f64 {
    if coordinates.len() < 2 {
        return 0.0;
    }

    let mut length = 0.0;
    for i in 1..coordinates.len() {
        let (lon1, lat1) = coordinates[i - 1];
        let (lon2, lat2) = coordinates[i];
        length += geo_distance(lon1, lat1, lon2, lat2);
    }
    length
}

/// Spherical linear interpolation between two points on a sphere.
///
/// # Arguments
/// * `lon1`, `lat1` - First point in degrees
/// * `lon2`, `lat2` - Second point in degrees
/// * `t` - Interpolation parameter [0, 1]
///
/// # Returns
/// Interpolated point (longitude, latitude) in degrees
pub fn geo_interpolate(lon1: f64, lat1: f64, lon2: f64, lat2: f64, t: f64) -> (f64, f64) {
    let lambda1 = radians(lon1);
    let phi1 = radians(lat1);
    let lambda2 = radians(lon2);
    let phi2 = radians(lat2);

    let cos_phi1 = phi1.cos();
    let sin_phi1 = phi1.sin();
    let cos_phi2 = phi2.cos();
    let sin_phi2 = phi2.sin();
    let cos_lambda1 = lambda1.cos();
    let sin_lambda1 = lambda1.sin();
    let cos_lambda2 = lambda2.cos();
    let sin_lambda2 = lambda2.sin();

    // Cartesian coordinates
    let x1 = cos_phi1 * cos_lambda1;
    let y1 = cos_phi1 * sin_lambda1;
    let z1 = sin_phi1;

    let x2 = cos_phi2 * cos_lambda2;
    let y2 = cos_phi2 * sin_lambda2;
    let z2 = sin_phi2;

    // Spherical interpolation
    let d = geo_distance(lon1, lat1, lon2, lat2);

    if d.abs() < EPSILON {
        return (lon1, lat1);
    }

    let sin_d = d.sin();
    let a = ((1.0 - t) * d).sin() / sin_d;
    let b = (t * d).sin() / sin_d;

    let x = a * x1 + b * x2;
    let y = a * y1 + b * y2;
    let z = a * z1 + b * z2;

    let lon = degrees(y.atan2(x));
    let lat = degrees(z.atan2((x * x + y * y).sqrt()));

    (lon, lat)
}

/// Calculate the spherical area of a polygon.
///
/// Uses the spherical excess formula.
/// Returns area in steradians. Multiply by R^2 for area in square units.
///
/// # Arguments
/// * `coordinates` - Slice of (longitude, latitude) pairs forming a closed ring
pub fn geo_area(coordinates: &[(f64, f64)]) -> f64 {
    if coordinates.len() < 3 {
        return 0.0;
    }

    let n = coordinates.len();
    let mut area = 0.0;

    for i in 0..n {
        let (lon1, lat1) = coordinates[i];
        let (lon2, lat2) = coordinates[(i + 1) % n];

        let lambda1 = radians(lon1);
        let phi1 = radians(lat1);
        let lambda2 = radians(lon2);
        let phi2 = radians(lat2);

        area += (lambda2 - lambda1) * (2.0 + phi1.sin() + phi2.sin());
    }

    (area / 2.0).abs()
}

/// Calculate the bounding box of geographic coordinates.
///
/// # Arguments
/// * `coordinates` - Slice of (longitude, latitude) pairs
///
/// # Returns
/// `((min_lon, min_lat), (max_lon, max_lat))`
pub fn geo_bounds(coordinates: &[(f64, f64)]) -> ((f64, f64), (f64, f64)) {
    if coordinates.is_empty() {
        return ((f64::NAN, f64::NAN), (f64::NAN, f64::NAN));
    }

    let mut min_lon = f64::MAX;
    let mut max_lon = f64::MIN;
    let mut min_lat = f64::MAX;
    let mut max_lat = f64::MIN;

    for &(lon, lat) in coordinates {
        if lon < min_lon {
            min_lon = lon;
        }
        if lon > max_lon {
            max_lon = lon;
        }
        if lat < min_lat {
            min_lat = lat;
        }
        if lat > max_lat {
            max_lat = lat;
        }
    }

    ((min_lon, min_lat), (max_lon, max_lat))
}

/// Calculate the centroid of geographic coordinates.
///
/// # Arguments
/// * `coordinates` - Slice of (longitude, latitude) pairs
///
/// # Returns
/// Centroid (longitude, latitude)
pub fn geo_centroid(coordinates: &[(f64, f64)]) -> (f64, f64) {
    if coordinates.is_empty() {
        return (f64::NAN, f64::NAN);
    }

    let mut x_sum = 0.0;
    let mut y_sum = 0.0;
    let mut z_sum = 0.0;

    for &(lon, lat) in coordinates {
        let lambda = radians(lon);
        let phi = radians(lat);
        let cos_phi = phi.cos();

        x_sum += cos_phi * lambda.cos();
        y_sum += cos_phi * lambda.sin();
        z_sum += phi.sin();
    }

    let n = coordinates.len() as f64;
    x_sum /= n;
    y_sum /= n;
    z_sum /= n;

    let lon = degrees(y_sum.atan2(x_sum));
    let lat = degrees(z_sum.atan2((x_sum * x_sum + y_sum * y_sum).sqrt()));

    (lon, lat)
}

/// Check if a point is inside a geographic polygon using ray casting.
///
/// # Arguments
/// * `coordinates` - Polygon ring coordinates (closed)
/// * `lon`, `lat` - Test point
pub fn geo_contains(coordinates: &[(f64, f64)], lon: f64, lat: f64) -> bool {
    if coordinates.len() < 3 {
        return false;
    }

    let n = coordinates.len();
    let mut inside = false;

    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = coordinates[i];
        let (xj, yj) = coordinates[j];

        if ((yi > lat) != (yj > lat)) && (lon < (xj - xi) * (lat - yi) / (yj - yi) + xi) {
            inside = !inside;
        }

        j = i;
    }

    inside
}

/// Rotation transform for spherical coordinates.
///
/// Rotates coordinates by the specified angles (in degrees).
#[derive(Clone, Debug)]
pub struct Rotation {
    /// Rotation around the z-axis (longitude rotation)
    pub lambda: f64,
    /// Rotation around the y-axis (latitude rotation)
    pub phi: f64,
    /// Rotation around the x-axis (gamma rotation)
    pub gamma: f64,
}

impl Default for Rotation {
    fn default() -> Self {
        Self::new()
    }
}

impl Rotation {
    /// Create a new rotation with zero angles.
    pub fn new() -> Self {
        Self {
            lambda: 0.0,
            phi: 0.0,
            gamma: 0.0,
        }
    }

    /// Set the rotation angles in degrees.
    pub fn angles(mut self, lambda: f64, phi: f64, gamma: f64) -> Self {
        self.lambda = lambda;
        self.phi = phi;
        self.gamma = gamma;
        self
    }

    /// Apply the rotation to a point (longitude, latitude in degrees).
    pub fn rotate(&self, lon: f64, lat: f64) -> (f64, f64) {
        let mut lambda = radians(lon);
        let mut phi = radians(lat);

        // Apply rotations
        lambda += radians(self.lambda);

        if self.phi.abs() > EPSILON || self.gamma.abs() > EPSILON {
            let cos_phi = phi.cos();
            let sin_phi = phi.sin();
            let cos_lambda = lambda.cos();
            let sin_lambda = lambda.sin();

            // Rotation around y-axis
            let phi_rad = radians(self.phi);
            let cos_phi_rot = phi_rad.cos();
            let sin_phi_rot = phi_rad.sin();

            let x = cos_phi * cos_lambda;
            let y = cos_phi * sin_lambda;
            let z = sin_phi;

            let x2 = x * cos_phi_rot - z * sin_phi_rot;
            let z2 = x * sin_phi_rot + z * cos_phi_rot;

            lambda = y.atan2(x2);
            phi = z2.asin();

            // Rotation around x-axis (gamma)
            if self.gamma.abs() > EPSILON {
                let gamma_rad = radians(self.gamma);
                let cos_gamma = gamma_rad.cos();
                let sin_gamma = gamma_rad.sin();

                let cos_phi = phi.cos();
                let sin_phi = phi.sin();
                let sin_lambda = lambda.sin();
                let cos_lambda = lambda.cos();

                let y = cos_phi * sin_lambda;
                let z = sin_phi;

                let y2 = y * cos_gamma - z * sin_gamma;
                let z2 = y * sin_gamma + z * cos_gamma;

                lambda = y2.atan2(cos_phi * cos_lambda);
                phi = z2.asin();
            }
        }

        (degrees(lambda), degrees(phi))
    }

    /// Apply the inverse rotation to a point.
    pub fn invert(&self, lon: f64, lat: f64) -> (f64, f64) {
        // Inverse rotation is rotation with negated angles in reverse order
        let inv = Rotation::new().angles(-self.gamma, -self.phi, -self.lambda);

        let mut lambda = radians(lon);
        let mut phi = radians(lat);

        // Apply inverse gamma
        if self.gamma.abs() > EPSILON {
            let gamma_rad = radians(-self.gamma);
            let cos_gamma = gamma_rad.cos();
            let sin_gamma = gamma_rad.sin();

            let cos_phi = phi.cos();
            let sin_phi = phi.sin();
            let sin_lambda = lambda.sin();
            let cos_lambda = lambda.cos();

            let y = cos_phi * sin_lambda;
            let z = sin_phi;

            let y2 = y * cos_gamma - z * sin_gamma;
            let z2 = y * sin_gamma + z * cos_gamma;

            lambda = y2.atan2(cos_phi * cos_lambda);
            phi = z2.asin();
        }

        // Apply inverse phi rotation
        if self.phi.abs() > EPSILON {
            let phi_rad = radians(-self.phi);
            let cos_phi_rot = phi_rad.cos();
            let sin_phi_rot = phi_rad.sin();

            let cos_phi = phi.cos();
            let sin_phi = phi.sin();
            let cos_lambda = lambda.cos();
            let sin_lambda = lambda.sin();

            let x = cos_phi * cos_lambda;
            let z = sin_phi;

            let x2 = x * cos_phi_rot - z * sin_phi_rot;
            let z2 = x * sin_phi_rot + z * cos_phi_rot;

            lambda = (cos_phi * sin_lambda).atan2(x2);
            phi = z2.asin();
        }

        // Apply inverse lambda rotation
        lambda -= radians(self.lambda);

        let _ = inv;
        (degrees(lambda), degrees(phi))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radians_degrees() {
        assert!((radians(180.0) - PI).abs() < EPSILON);
        assert!((degrees(PI) - 180.0).abs() < EPSILON);
        assert!((radians(90.0) - HALF_PI).abs() < EPSILON);
    }

    #[test]
    fn test_geo_distance() {
        // Distance from (0,0) to (90,0) should be pi/2
        let d = geo_distance(0.0, 0.0, 90.0, 0.0);
        assert!((d - HALF_PI).abs() < 1e-10);

        // Distance from (0,0) to (0,90) should be pi/2
        let d = geo_distance(0.0, 0.0, 0.0, 90.0);
        assert!((d - HALF_PI).abs() < 1e-10);

        // Distance from (0,0) to (180,0) should be pi
        let d = geo_distance(0.0, 0.0, 180.0, 0.0);
        assert!((d - PI).abs() < 1e-10);
    }

    #[test]
    fn test_geo_length() {
        let coords = vec![(0.0, 0.0), (90.0, 0.0)];
        let len = geo_length(&coords);
        assert!((len - HALF_PI).abs() < 1e-10);

        let coords = vec![(0.0, 0.0), (90.0, 0.0), (180.0, 0.0)];
        let len = geo_length(&coords);
        assert!((len - PI).abs() < 1e-10);
    }

    #[test]
    fn test_geo_interpolate() {
        // Midpoint between (0,0) and (90,0)
        let (lon, lat) = geo_interpolate(0.0, 0.0, 90.0, 0.0, 0.5);
        assert!((lon - 45.0).abs() < 1e-6);
        assert!(lat.abs() < 1e-6);

        // Start point
        let (lon, lat) = geo_interpolate(0.0, 0.0, 90.0, 0.0, 0.0);
        assert!((lon - 0.0).abs() < 1e-6);
        assert!((lat - 0.0).abs() < 1e-6);

        // End point
        let (lon, lat) = geo_interpolate(0.0, 0.0, 90.0, 0.0, 1.0);
        assert!((lon - 90.0).abs() < 1e-6);
        assert!(lat.abs() < 1e-6);
    }

    #[test]
    fn test_geo_bounds() {
        let coords = vec![(-10.0, -20.0), (30.0, 40.0), (0.0, 0.0)];
        let ((min_lon, min_lat), (max_lon, max_lat)) = geo_bounds(&coords);
        assert!((min_lon - (-10.0)).abs() < EPSILON);
        assert!((min_lat - (-20.0)).abs() < EPSILON);
        assert!((max_lon - 30.0).abs() < EPSILON);
        assert!((max_lat - 40.0).abs() < EPSILON);
    }

    #[test]
    fn test_geo_centroid() {
        let coords = vec![(0.0, 0.0), (90.0, 0.0), (45.0, 45.0)];
        let (lon, lat) = geo_centroid(&coords);
        // Centroid should be somewhere in the middle
        assert!(lon > 0.0 && lon < 90.0);
        assert!(lat > 0.0 && lat < 45.0);
    }

    #[test]
    fn test_geo_contains() {
        // Simple square polygon
        let coords = vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 0.0),
        ];

        assert!(geo_contains(&coords, 5.0, 5.0));
        assert!(!geo_contains(&coords, 15.0, 5.0));
        assert!(!geo_contains(&coords, -5.0, 5.0));
    }

    #[test]
    fn test_rotation() {
        let rot = Rotation::new().angles(90.0, 0.0, 0.0);
        let (lon, lat) = rot.rotate(0.0, 0.0);
        assert!((lon - 90.0).abs() < 1e-6);
        assert!(lat.abs() < 1e-6);
    }

    #[test]
    fn test_rotation_inverse() {
        let rot = Rotation::new().angles(45.0, 30.0, 0.0);
        let original = (10.0, 20.0);
        let rotated = rot.rotate(original.0, original.1);
        let inverted = rot.invert(rotated.0, rotated.1);
        assert!((inverted.0 - original.0).abs() < 1e-6);
        assert!((inverted.1 - original.1).abs() < 1e-6);
    }
}
