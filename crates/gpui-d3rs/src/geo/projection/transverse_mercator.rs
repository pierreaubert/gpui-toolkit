use super::super::{HALF_PI, degrees, radians};
use super::Projection;
use super::projection_config::ProjectionConfig;
use super::sphere_rotation::SphereRotation;

/// d3 composes transverse Mercator from a plain Mercator core with wrapped
/// setters: user `rotate([λ, φ, γ])` behaves as an internal rotation of
/// `[λ, φ, γ + 90]`, and user `center([lon, lat])` behaves as an internal
/// center of `[-lat, lon]`. The stored config keeps user values (so the
/// getters match d3); the +90/swapped forms are applied at use sites.
fn effective_rotation(config: &ProjectionConfig) -> SphereRotation {
    SphereRotation::from_degrees(config.rotate.0, config.rotate.1, config.rotate.2 + 90.0)
}

/// Internal center corresponding to the user-facing center, in degrees.
fn effective_center(config: &ProjectionConfig) -> (f64, f64) {
    (-config.center.1, config.center.0)
}

fn apply_effective_rotation(config: &ProjectionConfig, lon: f64, lat: f64) -> (f64, f64) {
    effective_rotation(config).rotate(radians(lon), radians(lat))
}

fn invert_effective_rotation(config: &ProjectionConfig, lambda: f64, phi: f64) -> (f64, f64) {
    let (rl, rp) = effective_rotation(config).invert(lambda, phi);
    (degrees(rl), degrees(rp))
}

/// Transverse Mercator projection.
///
/// The transverse Mercator projection is a conformal projection that
/// rotates the Mercator projection 90 degrees.
#[derive(Clone, Debug)]
pub struct TransverseMercator {
    pub(super) config: ProjectionConfig,
}

impl Default for TransverseMercator {
    fn default() -> Self {
        Self::new()
    }
}

impl TransverseMercator {
    /// Create a new Transverse Mercator projection.
    ///
    /// The default rotation is `[0, 0, 0]` like d3 (the internal +90 gamma
    /// offset is applied at projection time).
    pub fn new() -> Self {
        Self {
            config: ProjectionConfig {
                scale: 159.155,
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

    /// Set the rotation angles (stored as given; d3's +90 internal gamma
    /// offset is applied at projection time).
    pub fn rotate(mut self, lambda: f64, phi: f64, gamma: f64) -> Self {
        self.config.rotate = (lambda, phi, gamma);
        self
    }

    /// Maximum absolute latitude (radians) kept by the raw projection.
    ///
    /// Mirrors `Mercator::MAX_PHI`: d3 cuts transverse Mercator output to a
    /// ±π·scale square via its default post-clip extent, which is equivalent
    /// to clamping `mercator(φ)` to ±π. Without this, resampling dives into
    /// the pole singularity and world bounds blow up.
    const MAX_PHI: f64 = 85.05112877980659_f64.to_radians();

    /// Raw transverse Mercator projection.
    pub(super) fn project_raw(lambda: f64, phi: f64) -> (f64, f64) {
        let phi = phi.clamp(-Self::MAX_PHI, Self::MAX_PHI);
        (((HALF_PI + phi) / 2.0).tan().ln(), -lambda)
    }

    /// Inverse raw transverse Mercator projection.
    pub(super) fn invert_raw(x: f64, y: f64) -> (f64, f64) {
        (-y, 2.0 * x.exp().atan() - HALF_PI)
    }
}

impl Projection for TransverseMercator {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let (lambda, phi) = apply_effective_rotation(&self.config, lon, lat);
        self.project_rotated(lambda, phi)
    }

    fn project_rotated(&self, lambda: f64, phi: f64) -> (f64, f64) {
        let (x, y) = Self::project_raw(lambda, phi);
        let (clon, clat) = effective_center(&self.config);
        let (cx, cy) = Self::project_raw(radians(clon), radians(clat));

        (
            self.config.translate.0 + self.config.scale * (x - cx),
            self.config.translate.1 - self.config.scale * (y - cy),
        )
    }

    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let (clon, clat) = effective_center(&self.config);
        let (cx, cy) = Self::project_raw(radians(clon), radians(clat));
        let x = (x - self.config.translate.0) / self.config.scale + cx;
        let y = -(y - self.config.translate.1) / self.config.scale + cy;
        let (lambda, phi) = Self::invert_raw(x, y);
        Some(invert_effective_rotation(&self.config, lambda, phi))
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

    fn stream_rotation(&self) -> (f64, f64, f64) {
        (
            self.config.rotate.0,
            self.config.rotate.1,
            self.config.rotate.2 + 90.0,
        )
    }

    fn set_rotate(&mut self, lambda: f64, phi: f64, gamma: f64) {
        self.config.rotate = (lambda, phi, gamma);
    }

    fn longitude_unwrap_center(&self) -> Option<f64> {
        Some(self.config.rotate.0 + self.config.center.0)
    }
}
