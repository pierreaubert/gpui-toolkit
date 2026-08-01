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

pub use graticule::{Graticule, GraticuleConfig};
pub use path::{GeoJsonGeometry, GeoPath, GeoPathConfig};
pub use projection::{
    Albers, ConicEqualArea, Equirectangular, Mercator, Orthographic, Projection, Stereographic,
    TransverseMercator,
};

mod graticule;
mod path;
pub mod projection;
pub mod projection_stream;
pub mod stream;
pub mod topojson;
pub mod versor;

pub use topojson::{TopoJsonBudget, TopoJsonError, parse_land, parse_land_with_budget};

mod consts;
mod misc;
mod rotation;
#[cfg(test)]
mod tests;

pub use consts::*;
pub use misc::*;
pub use rotation::*;
pub use stream::*;

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
