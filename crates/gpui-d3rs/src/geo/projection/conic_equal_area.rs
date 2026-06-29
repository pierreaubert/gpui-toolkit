use super::super::{degrees, radians};
use super::Projection;
use super::projection_config::ProjectionConfig;
use super::projection_config::build_rotation;
use std::f64::consts::PI;

/// Conic Equal-Area projection.
///
/// The conic equal-area projection (also known as Albers equal-area conic)
/// is an equal-area projection that uses two standard parallels.
#[derive(Clone, Debug)]
pub struct ConicEqualArea {
    pub(super) config: ProjectionConfig,
    /// First standard parallel
    pub(super) phi0: f64,
    /// Second standard parallel
    pub(super) phi1: f64,
    /// Computed n value
    pub(super) n: f64,
    /// Computed C value
    pub(super) c: f64,
    /// Computed r0 value
    pub(super) r0: f64,
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

impl ConicEqualArea {
    /// Raw conic equal-area projection (no rotation, center, or scale).
    pub(super) fn project_raw_conic(&self, lambda: f64, phi: f64) -> Option<(f64, f64)> {
        let rho_sq = self.c - 2.0 * self.n * phi.sin();
        if rho_sq < 0.0 {
            return None;
        }
        let r = rho_sq.sqrt() / self.n;
        let theta = lambda * self.n;
        Some((r * theta.sin(), self.r0 - r * theta.cos()))
    }
}

impl Projection for ConicEqualArea {
    fn project(&self, lon: f64, lat: f64) -> (f64, f64) {
        let rotation = build_rotation(&self.config);

        // D3 pipeline:
        // 1. Rotate input
        let (rl, rp) = rotation.rotate(radians(lon), radians(lat));

        self.project_rotated(rl, rp)
    }

    fn project_rotated(&self, lambda: f64, phi: f64) -> (f64, f64) {
        let (x, y) = match self.project_raw_conic(lambda, phi) {
            Some(v) => v,
            None => return (f64::NAN, f64::NAN),
        };

        // D3 recenter: offset output so that the configured geographic center
        // maps to translate.  The center is projected without rotation, exactly
        // as D3's recenter() does.
        let cl = radians(self.config.center.0);
        let cp = radians(self.config.center.1);
        let (cx, cy) = self.project_raw_conic(cl, cp).unwrap_or((0.0, 0.0));

        (
            self.config.translate.0 + self.config.scale * (x - cx),
            self.config.translate.1 - self.config.scale * (y - cy),
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
