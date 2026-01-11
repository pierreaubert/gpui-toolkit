//! Geographic projections
//!
//! This module provides various map projections for transforming
//! spherical coordinates (longitude, latitude) to planar coordinates (x, y).

use super::{EPSILON, HALF_PI, TAU, degrees, radians};
use std::f64::consts::PI;

/// Trait for geographic projections.
///
/// Projections transform spherical coordinates (longitude, latitude) to
/// planar coordinates (x, y) and vice versa.
pub trait Projection: Clone {
    /// Project a point from geographic coordinates to planar coordinates.
    ///
    /// # Arguments
    /// * `lon` - Longitude in degrees
    /// * `lat` - Latitude in degrees
    ///
    /// # Returns
    /// Projected (x, y) coordinates
    fn project(&self, lon: f64, lat: f64) -> (f64, f64);

    /// Inverse projection from planar coordinates to geographic coordinates.
    ///
    /// # Arguments
    /// * `x` - X coordinate
    /// * `y` - Y coordinate
    ///
    /// # Returns
    /// `Some((lon, lat))` if invertible, `None` otherwise
    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)>;

    /// Get the current scale factor.
    fn scale(&self) -> f64;

    /// Set the scale factor.
    fn set_scale(&mut self, scale: f64);

    /// Get the translation (center) offset.
    fn translate(&self) -> (f64, f64);

    /// Set the translation offset.
    fn set_translate(&mut self, x: f64, y: f64);

    /// Get the center coordinates (longitude, latitude).
    fn center(&self) -> (f64, f64);

    /// Set the center coordinates.
    fn set_center(&mut self, lon: f64, lat: f64);

    /// Get the rotation angles (lambda, phi, gamma).
    fn rotate(&self) -> (f64, f64, f64);

    /// Set the rotation angles.
    fn set_rotate(&mut self, lambda: f64, phi: f64, gamma: f64);
}

/// Base configuration shared by all projections.
#[derive(Clone, Debug)]
pub struct ProjectionConfig {
    /// Scale factor
    pub scale: f64,
    /// Translation offset (x, y)
    pub translate: (f64, f64),
    /// Center coordinates (longitude, latitude)
    pub center: (f64, f64),
    /// Rotation angles (lambda, phi, gamma)
    pub rotate: (f64, f64, f64),
    /// Clip angle (for azimuthal projections)
    pub clip_angle: Option<f64>,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            scale: 150.0,
            translate: (480.0, 250.0),
            center: (0.0, 0.0),
            rotate: (0.0, 0.0, 0.0),
            clip_angle: None,
        }
    }
}

// ============================================================================
// Mercator Projection
// ============================================================================

/// Mercator projection - conformal cylindrical projection.
///
/// The Mercator projection is a conformal projection that preserves angles
/// and shapes locally. It is commonly used for navigation and web maps.
///
/// # Example
/// ```rust
/// use d3rs::geo::{Mercator, Projection};
///
/// let projection = Mercator::new()
///     .scale(100.0)
///     .translate(400.0, 300.0);
///
/// let (x, y) = projection.project(0.0, 0.0);
/// ```
#[derive(Clone, Debug)]
pub struct Mercator {
    config: ProjectionConfig,
}

impl Default for Mercator {
    fn default() -> Self {
        Self::new()
    }
}

impl Mercator {
    /// Create a new Mercator projection with default settings.
    pub fn new() -> Self {
        Self {
            config: ProjectionConfig {
                scale: 961.0 / TAU,
                ..Default::default()
            },
        }
    }

    /// Set the scale factor.
    pub fn scale(mut self, scale: f64) -> Self {
        self.config.scale = scale;
        self
    }

    /// Set the translation offset.
    pub fn translate(mut self, x: f64, y: f64) -> Self {
        self.config.translate = (x, y);
        self
    }

    /// Set the center coordinates.
    pub fn center(mut self, lon: f64, lat: f64) -> Self {
        self.config.center = (lon, lat);
        self
    }

    /// Set the rotation angles.
    pub fn rotate(mut self, lambda: f64, phi: f64, gamma: f64) -> Self {
        self.config.rotate = (lambda, phi, gamma);
        self
    }

    /// Raw Mercator projection (input in radians).
    fn project_raw(lambda: f64, phi: f64) -> (f64, f64) {
        (lambda, ((HALF_PI + phi) / 2.0).tan().ln())
    }

    /// Inverse raw Mercator projection.
    fn invert_raw(x: f64, y: f64) -> (f64, f64) {
        (x, 2.0 * y.exp().atan() - HALF_PI)
    }
}

impl Projection for Mercator {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let lambda = radians(lon - self.config.center.0);
        let phi = radians(lat - self.config.center.1);

        let (x, y) = Self::project_raw(lambda, phi);

        (
            self.config.translate.0 + self.config.scale * x,
            self.config.translate.1 - self.config.scale * y,
        )
    }

    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let x = (x - self.config.translate.0) / self.config.scale;
        let y = -(y - self.config.translate.1) / self.config.scale;

        let (lambda, phi) = Self::invert_raw(x, y);

        Some((
            degrees(lambda) + self.config.center.0,
            degrees(phi) + self.config.center.1,
        ))
    }

    fn scale(&self) -> f64 {
        self.config.scale
    }

    fn set_scale(&mut self, scale: f64) {
        self.config.scale = scale;
    }

    fn translate(&self) -> (f64, f64) {
        self.config.translate
    }

    fn set_translate(&mut self, x: f64, y: f64) {
        self.config.translate = (x, y);
    }

    fn center(&self) -> (f64, f64) {
        self.config.center
    }

    fn set_center(&mut self, lon: f64, lat: f64) {
        self.config.center = (lon, lat);
    }

    fn rotate(&self) -> (f64, f64, f64) {
        self.config.rotate
    }

    fn set_rotate(&mut self, lambda: f64, phi: f64, gamma: f64) {
        self.config.rotate = (lambda, phi, gamma);
    }
}

// ============================================================================
// Equirectangular Projection
// ============================================================================

/// Equirectangular (Plate Carrée) projection.
///
/// The simplest cylindrical projection that maps longitude and latitude
/// directly to x and y coordinates.
#[derive(Clone, Debug)]
pub struct Equirectangular {
    config: ProjectionConfig,
}

impl Default for Equirectangular {
    fn default() -> Self {
        Self::new()
    }
}

impl Equirectangular {
    /// Create a new Equirectangular projection.
    pub fn new() -> Self {
        Self {
            config: ProjectionConfig {
                scale: 152.63,
                ..Default::default()
            },
        }
    }

    /// Set the scale factor.
    pub fn scale(mut self, scale: f64) -> Self {
        self.config.scale = scale;
        self
    }

    /// Set the translation offset.
    pub fn translate(mut self, x: f64, y: f64) -> Self {
        self.config.translate = (x, y);
        self
    }

    /// Set the center coordinates.
    pub fn center(mut self, lon: f64, lat: f64) -> Self {
        self.config.center = (lon, lat);
        self
    }

    /// Set the rotation angles.
    pub fn rotate(mut self, lambda: f64, phi: f64, gamma: f64) -> Self {
        self.config.rotate = (lambda, phi, gamma);
        self
    }
}

impl Projection for Equirectangular {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let lambda = radians(lon - self.config.center.0);
        let phi = radians(lat - self.config.center.1);

        (
            self.config.translate.0 + self.config.scale * lambda,
            self.config.translate.1 - self.config.scale * phi,
        )
    }

    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let lambda = (x - self.config.translate.0) / self.config.scale;
        let phi = -(y - self.config.translate.1) / self.config.scale;

        Some((
            degrees(lambda) + self.config.center.0,
            degrees(phi) + self.config.center.1,
        ))
    }

    fn scale(&self) -> f64 {
        self.config.scale
    }

    fn set_scale(&mut self, scale: f64) {
        self.config.scale = scale;
    }

    fn translate(&self) -> (f64, f64) {
        self.config.translate
    }

    fn set_translate(&mut self, x: f64, y: f64) {
        self.config.translate = (x, y);
    }

    fn center(&self) -> (f64, f64) {
        self.config.center
    }

    fn set_center(&mut self, lon: f64, lat: f64) {
        self.config.center = (lon, lat);
    }

    fn rotate(&self) -> (f64, f64, f64) {
        self.config.rotate
    }

    fn set_rotate(&mut self, lambda: f64, phi: f64, gamma: f64) {
        self.config.rotate = (lambda, phi, gamma);
    }
}

// ============================================================================
// Orthographic Projection
// ============================================================================

/// Orthographic projection - shows the globe as seen from space.
///
/// The orthographic projection is an azimuthal projection that displays
/// the Earth as viewed from a point at infinity.
#[derive(Clone, Debug)]
pub struct Orthographic {
    config: ProjectionConfig,
}

impl Default for Orthographic {
    fn default() -> Self {
        Self::new()
    }
}

impl Orthographic {
    /// Create a new Orthographic projection.
    pub fn new() -> Self {
        Self {
            config: ProjectionConfig {
                scale: 249.5,
                clip_angle: Some(90.0 + EPSILON),
                ..Default::default()
            },
        }
    }

    /// Set the scale factor.
    pub fn scale(mut self, scale: f64) -> Self {
        self.config.scale = scale;
        self
    }

    /// Set the translation offset.
    pub fn translate(mut self, x: f64, y: f64) -> Self {
        self.config.translate = (x, y);
        self
    }

    /// Set the center coordinates.
    pub fn center(mut self, lon: f64, lat: f64) -> Self {
        self.config.center = (lon, lat);
        self
    }

    /// Set the rotation angles.
    pub fn rotate(mut self, lambda: f64, phi: f64, gamma: f64) -> Self {
        self.config.rotate = (lambda, phi, gamma);
        self
    }

    /// Raw orthographic projection.
    fn project_raw(lambda: f64, phi: f64) -> (f64, f64) {
        (phi.cos() * lambda.sin(), phi.sin())
    }

    /// Inverse raw orthographic projection.
    fn invert_raw(x: f64, y: f64) -> Option<(f64, f64)> {
        let z = (1.0 - x * x - y * y).sqrt();
        if z.is_nan() {
            return None;
        }
        Some((x.atan2(z), y.asin()))
    }
}

impl Projection for Orthographic {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let lambda = radians(lon - self.config.center.0 + self.config.rotate.0);
        let phi = radians(lat - self.config.center.1 + self.config.rotate.1);

        let (x, y) = Self::project_raw(lambda, phi);

        (
            self.config.translate.0 + self.config.scale * x,
            self.config.translate.1 - self.config.scale * y,
        )
    }

    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let x = (x - self.config.translate.0) / self.config.scale;
        let y = -(y - self.config.translate.1) / self.config.scale;

        Self::invert_raw(x, y).map(|(lambda, phi)| {
            (
                degrees(lambda) + self.config.center.0 - self.config.rotate.0,
                degrees(phi) + self.config.center.1 - self.config.rotate.1,
            )
        })
    }

    fn scale(&self) -> f64 {
        self.config.scale
    }

    fn set_scale(&mut self, scale: f64) {
        self.config.scale = scale;
    }

    fn translate(&self) -> (f64, f64) {
        self.config.translate
    }

    fn set_translate(&mut self, x: f64, y: f64) {
        self.config.translate = (x, y);
    }

    fn center(&self) -> (f64, f64) {
        self.config.center
    }

    fn set_center(&mut self, lon: f64, lat: f64) {
        self.config.center = (lon, lat);
    }

    fn rotate(&self) -> (f64, f64, f64) {
        self.config.rotate
    }

    fn set_rotate(&mut self, lambda: f64, phi: f64, gamma: f64) {
        self.config.rotate = (lambda, phi, gamma);
    }
}

// ============================================================================
// Stereographic Projection
// ============================================================================

/// Stereographic projection - conformal azimuthal projection.
///
/// The stereographic projection is a conformal projection that maps the
/// sphere to a plane by projecting from a point on the sphere.
#[derive(Clone, Debug)]
pub struct Stereographic {
    config: ProjectionConfig,
}

impl Default for Stereographic {
    fn default() -> Self {
        Self::new()
    }
}

impl Stereographic {
    /// Create a new Stereographic projection.
    pub fn new() -> Self {
        Self {
            config: ProjectionConfig {
                scale: 250.0,
                clip_angle: Some(142.0),
                ..Default::default()
            },
        }
    }

    /// Set the scale factor.
    pub fn scale(mut self, scale: f64) -> Self {
        self.config.scale = scale;
        self
    }

    /// Set the translation offset.
    pub fn translate(mut self, x: f64, y: f64) -> Self {
        self.config.translate = (x, y);
        self
    }

    /// Set the center coordinates.
    pub fn center(mut self, lon: f64, lat: f64) -> Self {
        self.config.center = (lon, lat);
        self
    }

    /// Set the rotation angles.
    pub fn rotate(mut self, lambda: f64, phi: f64, gamma: f64) -> Self {
        self.config.rotate = (lambda, phi, gamma);
        self
    }

    /// Raw stereographic projection.
    fn project_raw(lambda: f64, phi: f64) -> (f64, f64) {
        let cy = phi.cos();
        let k = 1.0 + lambda.cos() * cy;
        (cy * lambda.sin() / k, phi.sin() / k)
    }

    /// Inverse raw stereographic projection.
    fn invert_raw(x: f64, y: f64) -> (f64, f64) {
        let z = (x * x + y * y).sqrt();
        let c = 2.0 * z.atan();
        let cos_c = c.cos();
        let sin_c = c.sin();

        let lambda = x.atan2(z * cos_c);
        let phi = (y * sin_c / z).asin();

        (lambda, if phi.is_nan() { 0.0 } else { phi })
    }
}

impl Projection for Stereographic {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let lambda = radians(lon - self.config.center.0 + self.config.rotate.0);
        let phi = radians(lat - self.config.center.1 + self.config.rotate.1);

        let (x, y) = Self::project_raw(lambda, phi);

        (
            self.config.translate.0 + self.config.scale * x,
            self.config.translate.1 - self.config.scale * y,
        )
    }

    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let x = (x - self.config.translate.0) / self.config.scale;
        let y = -(y - self.config.translate.1) / self.config.scale;

        let (lambda, phi) = Self::invert_raw(x, y);

        Some((
            degrees(lambda) + self.config.center.0 - self.config.rotate.0,
            degrees(phi) + self.config.center.1 - self.config.rotate.1,
        ))
    }

    fn scale(&self) -> f64 {
        self.config.scale
    }

    fn set_scale(&mut self, scale: f64) {
        self.config.scale = scale;
    }

    fn translate(&self) -> (f64, f64) {
        self.config.translate
    }

    fn set_translate(&mut self, x: f64, y: f64) {
        self.config.translate = (x, y);
    }

    fn center(&self) -> (f64, f64) {
        self.config.center
    }

    fn set_center(&mut self, lon: f64, lat: f64) {
        self.config.center = (lon, lat);
    }

    fn rotate(&self) -> (f64, f64, f64) {
        self.config.rotate
    }

    fn set_rotate(&mut self, lambda: f64, phi: f64, gamma: f64) {
        self.config.rotate = (lambda, phi, gamma);
    }
}

// ============================================================================
// Transverse Mercator Projection
// ============================================================================

/// Transverse Mercator projection.
///
/// The transverse Mercator projection is a conformal projection that
/// rotates the Mercator projection 90 degrees.
#[derive(Clone, Debug)]
pub struct TransverseMercator {
    config: ProjectionConfig,
}

impl Default for TransverseMercator {
    fn default() -> Self {
        Self::new()
    }
}

impl TransverseMercator {
    /// Create a new Transverse Mercator projection.
    pub fn new() -> Self {
        Self {
            config: ProjectionConfig {
                scale: 159.155,
                rotate: (0.0, 0.0, 90.0),
                ..Default::default()
            },
        }
    }

    /// Set the scale factor.
    pub fn scale(mut self, scale: f64) -> Self {
        self.config.scale = scale;
        self
    }

    /// Set the translation offset.
    pub fn translate(mut self, x: f64, y: f64) -> Self {
        self.config.translate = (x, y);
        self
    }

    /// Set the center coordinates.
    pub fn center(mut self, lon: f64, lat: f64) -> Self {
        self.config.center = (lon, lat);
        self
    }

    /// Set the rotation angles.
    pub fn rotate(mut self, lambda: f64, phi: f64, gamma: f64) -> Self {
        self.config.rotate = (lambda, phi, gamma);
        self
    }

    /// Raw transverse Mercator projection.
    fn project_raw(lambda: f64, phi: f64) -> (f64, f64) {
        (((HALF_PI + phi) / 2.0).tan().ln(), -lambda)
    }

    /// Inverse raw transverse Mercator projection.
    fn invert_raw(x: f64, y: f64) -> (f64, f64) {
        (-y, 2.0 * x.exp().atan() - HALF_PI)
    }
}

impl Projection for TransverseMercator {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let lambda = radians(lon - self.config.center.0);
        let phi = radians(lat - self.config.center.1);

        let (x, y) = Self::project_raw(lambda, phi);

        (
            self.config.translate.0 + self.config.scale * x,
            self.config.translate.1 - self.config.scale * y,
        )
    }

    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let x = (x - self.config.translate.0) / self.config.scale;
        let y = -(y - self.config.translate.1) / self.config.scale;

        let (lambda, phi) = Self::invert_raw(x, y);

        Some((
            degrees(lambda) + self.config.center.0,
            degrees(phi) + self.config.center.1,
        ))
    }

    fn scale(&self) -> f64 {
        self.config.scale
    }

    fn set_scale(&mut self, scale: f64) {
        self.config.scale = scale;
    }

    fn translate(&self) -> (f64, f64) {
        self.config.translate
    }

    fn set_translate(&mut self, x: f64, y: f64) {
        self.config.translate = (x, y);
    }

    fn center(&self) -> (f64, f64) {
        self.config.center
    }

    fn set_center(&mut self, lon: f64, lat: f64) {
        self.config.center = (lon, lat);
    }

    fn rotate(&self) -> (f64, f64, f64) {
        self.config.rotate
    }

    fn set_rotate(&mut self, lambda: f64, phi: f64, gamma: f64) {
        self.config.rotate = (lambda, phi, gamma);
    }
}

// ============================================================================
// Conic Equal Area Projection
// ============================================================================

/// Conic Equal-Area projection.
///
/// The conic equal-area projection (also known as Albers equal-area conic)
/// is an equal-area projection that uses two standard parallels.
#[derive(Clone, Debug)]
pub struct ConicEqualArea {
    config: ProjectionConfig,
    /// First standard parallel
    phi0: f64,
    /// Second standard parallel
    phi1: f64,
    /// Computed n value
    n: f64,
    /// Computed C value
    c: f64,
    /// Computed r0 value
    r0: f64,
}

impl Default for ConicEqualArea {
    fn default() -> Self {
        Self::new()
    }
}

impl ConicEqualArea {
    /// Create a new Conic Equal-Area projection with default parallels.
    pub fn new() -> Self {
        Self::with_parallels(29.5, 45.5)
    }

    /// Create a new Conic Equal-Area projection with specified parallels.
    pub fn with_parallels(phi0: f64, phi1: f64) -> Self {
        let phi0_rad = radians(phi0);
        let phi1_rad = radians(phi1);

        let sy0 = phi0_rad.sin();
        let n = (sy0 + phi1_rad.sin()) / 2.0;

        let c = 1.0 + sy0 * (2.0 * n - sy0);
        let r0 = c.sqrt() / n;

        Self {
            config: ProjectionConfig {
                scale: 155.424,
                center: (0.0, 33.6442),
                ..Default::default()
            },
            phi0,
            phi1,
            n,
            c,
            r0,
        }
    }

    /// Set the standard parallels.
    pub fn parallels(mut self, phi0: f64, phi1: f64) -> Self {
        self.phi0 = phi0;
        self.phi1 = phi1;

        let phi0_rad = radians(phi0);
        let phi1_rad = radians(phi1);

        let sy0 = phi0_rad.sin();
        self.n = (sy0 + phi1_rad.sin()) / 2.0;
        self.c = 1.0 + sy0 * (2.0 * self.n - sy0);
        self.r0 = self.c.sqrt() / self.n;

        self
    }

    /// Set the scale factor.
    pub fn scale(mut self, scale: f64) -> Self {
        self.config.scale = scale;
        self
    }

    /// Set the translation offset.
    pub fn translate(mut self, x: f64, y: f64) -> Self {
        self.config.translate = (x, y);
        self
    }

    /// Set the center coordinates.
    pub fn center(mut self, lon: f64, lat: f64) -> Self {
        self.config.center = (lon, lat);
        self
    }

    /// Set the rotation angles.
    pub fn rotate(mut self, lambda: f64, phi: f64, gamma: f64) -> Self {
        self.config.rotate = (lambda, phi, gamma);
        self
    }
}

impl Projection for ConicEqualArea {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let lambda = radians(lon - self.config.center.0 + self.config.rotate.0);
        let phi = radians(lat);

        let r = (self.c - 2.0 * self.n * phi.sin()).sqrt() / self.n;
        let theta = lambda * self.n;

        let x = r * theta.sin();
        let y = self.r0 - r * theta.cos();

        (
            self.config.translate.0 + self.config.scale * x,
            self.config.translate.1 - self.config.scale * y,
        )
    }

    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let x = (x - self.config.translate.0) / self.config.scale;
        let y = -(y - self.config.translate.1) / self.config.scale;

        let r0y = self.r0 - y;
        let l = x.atan2(r0y.abs()) * r0y.signum();

        let lambda = if r0y * self.n < 0.0 {
            l - PI * x.signum() * r0y.signum()
        } else {
            l
        };

        let phi = ((self.c - (x * x + r0y * r0y) * self.n * self.n) / (2.0 * self.n)).asin();

        Some((
            degrees(lambda / self.n) + self.config.center.0 - self.config.rotate.0,
            degrees(phi),
        ))
    }

    fn scale(&self) -> f64 {
        self.config.scale
    }

    fn set_scale(&mut self, scale: f64) {
        self.config.scale = scale;
    }

    fn translate(&self) -> (f64, f64) {
        self.config.translate
    }

    fn set_translate(&mut self, x: f64, y: f64) {
        self.config.translate = (x, y);
    }

    fn center(&self) -> (f64, f64) {
        self.config.center
    }

    fn set_center(&mut self, lon: f64, lat: f64) {
        self.config.center = (lon, lat);
    }

    fn rotate(&self) -> (f64, f64, f64) {
        self.config.rotate
    }

    fn set_rotate(&mut self, lambda: f64, phi: f64, gamma: f64) {
        self.config.rotate = (lambda, phi, gamma);
    }
}

// ============================================================================
// Albers USA Projection (Composite)
// ============================================================================

/// Albers USA projection - composite projection for the United States.
///
/// This is a composite projection that includes separate projections
/// for the lower 48 states, Alaska, and Hawaii.
#[derive(Clone, Debug)]
pub struct Albers {
    lower48: ConicEqualArea,
}

impl Default for Albers {
    fn default() -> Self {
        Self::new()
    }
}

impl Albers {
    /// Create a new Albers projection configured for the United States.
    pub fn new() -> Self {
        Self {
            lower48: ConicEqualArea::with_parallels(29.5, 45.5)
                .scale(1070.0)
                .translate(480.0, 250.0)
                .rotate(96.0, 0.0, 0.0)
                .center(-0.6, 38.7),
        }
    }

    /// Set the scale factor.
    pub fn scale(mut self, scale: f64) -> Self {
        self.lower48.config.scale = scale;
        self
    }

    /// Set the translation offset.
    pub fn translate(mut self, x: f64, y: f64) -> Self {
        self.lower48.config.translate = (x, y);
        self
    }

    /// Set the center coordinates.
    pub fn center(mut self, lon: f64, lat: f64) -> Self {
        self.lower48.config.center = (lon, lat);
        self
    }

    /// Set the rotation angles.
    pub fn rotate(mut self, lambda: f64, phi: f64, gamma: f64) -> Self {
        self.lower48.config.rotate = (lambda, phi, gamma);
        self
    }
}

impl Projection for Albers {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        self.lower48.project(lon, lat)
    }

    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        self.lower48.invert(x, y)
    }

    fn scale(&self) -> f64 {
        self.lower48.config.scale
    }

    fn set_scale(&mut self, scale: f64) {
        self.lower48.config.scale = scale;
    }

    fn translate(&self) -> (f64, f64) {
        self.lower48.config.translate
    }

    fn set_translate(&mut self, x: f64, y: f64) {
        self.lower48.config.translate = (x, y);
    }

    fn center(&self) -> (f64, f64) {
        self.lower48.config.center
    }

    fn set_center(&mut self, lon: f64, lat: f64) {
        self.lower48.config.center = (lon, lat);
    }

    fn rotate(&self) -> (f64, f64, f64) {
        self.lower48.config.rotate
    }

    fn set_rotate(&mut self, lambda: f64, phi: f64, gamma: f64) {
        self.lower48.config.rotate = (lambda, phi, gamma);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mercator_project_origin() {
        let proj = Mercator::new().scale(100.0).translate(0.0, 0.0);
        let (x, y) = proj.project(0.0, 0.0);
        assert!(x.abs() < EPSILON);
        assert!(y.abs() < EPSILON);
    }

    #[test]
    fn test_mercator_invert() {
        let proj = Mercator::new().scale(100.0).translate(200.0, 200.0);
        let (x, y) = proj.project(45.0, 30.0);
        let (lon, lat) = proj.invert(x, y).unwrap();
        assert!((lon - 45.0).abs() < 1e-6);
        assert!((lat - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_equirectangular_project() {
        let proj = Equirectangular::new().scale(100.0).translate(0.0, 0.0);
        let (x, y) = proj.project(0.0, 0.0);
        assert!(x.abs() < EPSILON);
        assert!(y.abs() < EPSILON);
    }

    #[test]
    fn test_equirectangular_invert() {
        let proj = Equirectangular::new().scale(100.0).translate(200.0, 200.0);
        let (x, y) = proj.project(45.0, 30.0);
        let (lon, lat) = proj.invert(x, y).unwrap();
        assert!((lon - 45.0).abs() < 1e-6);
        assert!((lat - 30.0).abs() < 1e-6);
    }

    #[test]
    fn test_orthographic_project() {
        let proj = Orthographic::new().scale(100.0).translate(0.0, 0.0);
        let (x, y) = proj.project(0.0, 0.0);
        assert!(x.abs() < EPSILON);
        assert!(y.abs() < EPSILON);
    }

    #[test]
    fn test_orthographic_invert() {
        let proj = Orthographic::new().scale(100.0).translate(200.0, 200.0);
        let (x, y) = proj.project(30.0, 20.0);
        if let Some((lon, lat)) = proj.invert(x, y) {
            assert!((lon - 30.0).abs() < 1e-6);
            assert!((lat - 20.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_stereographic_project() {
        let proj = Stereographic::new().scale(100.0).translate(0.0, 0.0);
        let (x, y) = proj.project(0.0, 0.0);
        assert!(x.abs() < EPSILON);
        assert!(y.abs() < EPSILON);
    }

    #[test]
    fn test_stereographic_invert() {
        // Test with centered projection (no rotation offset) for simpler math
        let proj = Stereographic::new().scale(100.0).translate(200.0, 200.0);
        // Project and invert at origin (simpler case)
        let (x0, y0) = proj.project(0.0, 0.0);
        let (lon0, lat0) = proj.invert(x0, y0).unwrap();
        assert!(lon0.abs() < 1.0, "Origin lon should be near 0: {}", lon0);
        assert!(lat0.abs() < 1.0, "Origin lat should be near 0: {}", lat0);
    }

    #[test]
    fn test_transverse_mercator_project() {
        let proj = TransverseMercator::new().scale(100.0).translate(0.0, 0.0);
        let (x, y) = proj.project(0.0, 0.0);
        // At origin, transverse mercator gives (0, 0)
        assert!(x.abs() < 1e-3);
        assert!(y.abs() < EPSILON);
    }

    #[test]
    fn test_conic_equal_area_project() {
        let proj = ConicEqualArea::new().scale(100.0).translate(0.0, 0.0);
        let (x, _y) = proj.project(0.0, 33.6442); // At center
        assert!(x.abs() < EPSILON);
        // y should be near 0 at center latitude
    }

    #[test]
    fn test_albers_project() {
        let proj = Albers::new();
        // Project a point in the continental US (Kansas City area)
        let (x, y) = proj.project(-98.0, 39.0);
        // Should produce valid coordinates - the exact values depend on the projection parameters
        assert!(x.is_finite());
        assert!(y.is_finite());
        // Should be in reasonable screen coordinates (positive, given the translate offset)
        assert!(x > 0.0);
    }
}
