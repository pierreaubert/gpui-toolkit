use super::super::{EPSILON, radians};
use super::Projection;
use super::projection_config::ProjectionConfig;
use super::projection_config::apply_rotation;
use super::projection_config::clip_angle_visible;
use super::projection_config::invert_rotation;

/// Orthographic projection - shows the globe as seen from space.
///
/// The orthographic projection is an azimuthal projection that displays
/// the Earth as viewed from a point at infinity.
#[derive(Clone, Debug)]
pub struct Orthographic {
    pub(super) config: ProjectionConfig,
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
    pub(super) fn project_raw(lambda: f64, phi: f64) -> (f64, f64) {
        (phi.cos() * lambda.sin(), phi.sin())
    }

    /// Inverse raw orthographic projection.
    pub(super) fn invert_raw(x: f64, y: f64) -> Option<(f64, f64)> {
        let z = (1.0 - x * x - y * y).sqrt();
        if z.is_nan() {
            return None;
        }
        Some((x.atan2(z), y.asin()))
    }
}

impl Projection for Orthographic {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let (lambda, phi) = apply_rotation(&self.config, lon, lat);
        self.project_rotated(lambda, phi)
    }

    fn project_rotated(&self, lambda: f64, phi: f64) -> (f64, f64) {
        let (x, y) = Self::project_raw(lambda, phi);
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
        Self::invert_raw(x, y).map(|(lambda, phi)| invert_rotation(&self.config, lambda, phi))
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
