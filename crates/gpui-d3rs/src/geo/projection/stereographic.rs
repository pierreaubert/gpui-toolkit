use super::super::radians;
use super::Projection;
use super::projection_config::ProjectionConfig;
use super::projection_config::apply_rotation;
use super::projection_config::clip_angle_visible;
use super::projection_config::invert_rotation;

/// Stereographic projection - conformal azimuthal projection.
///
/// The stereographic projection is a conformal projection that maps the
/// sphere to a plane by projecting from a point on the sphere.
#[derive(Clone, Debug)]
pub struct Stereographic {
    pub(super) config: ProjectionConfig,
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
    pub(super) fn project_raw(lambda: f64, phi: f64) -> (f64, f64) {
        let cy = phi.cos();
        let k = 1.0 + lambda.cos() * cy;
        (cy * lambda.sin() / k, phi.sin() / k)
    }

    /// Inverse raw stereographic projection.
    pub(super) fn invert_raw(x: f64, y: f64) -> (f64, f64) {
        let z = (x * x + y * y).sqrt();
        let c = 2.0 * z.atan();
        let cos_c = c.cos();
        let sin_c = c.sin();

        let lambda = (x * sin_c).atan2(z * cos_c);
        let phi = (y * sin_c / z).asin();

        (lambda, if phi.is_nan() { 0.0 } else { phi })
    }
}

impl Projection for Stereographic {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let (lambda, phi) = apply_rotation(&self.config, lon, lat);
        self.project_rotated(lambda, phi)
    }

    fn project_rotated(&self, lambda: f64, phi: f64) -> (f64, f64) {
        let (x, y) = Self::project_raw(lambda, phi);

        // Guard against division-by-zero at the exact antipode
        if !x.is_finite() || !y.is_finite() {
            return (f64::NAN, f64::NAN);
        }

        let (cx, cy) =
            Self::project_raw(radians(self.config.center.0), radians(self.config.center.1));

        (
            self.config.translate.0 + self.config.scale * (x - cx),
            self.config.translate.1 - self.config.scale * (y - cy),
        )
    }

    fn invert(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        let (cx, cy) =
            Self::project_raw(radians(self.config.center.0), radians(self.config.center.1));
        let x = (x - self.config.translate.0) / self.config.scale + cx;
        let y = -(y - self.config.translate.1) / self.config.scale + cy;
        let (lambda, phi) = Self::invert_raw(x, y);
        Some(invert_rotation(&self.config, lambda, phi))
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

    fn clip_angle(&self) -> Option<f64> {
        self.config.clip_angle
    }

    fn is_visible(&self, lon: f64, lat: f64) -> bool {
        clip_angle_visible(&self.config, lon, lat)
    }
}
