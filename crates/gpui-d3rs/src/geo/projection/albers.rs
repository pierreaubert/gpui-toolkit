use super::Projection;
use super::conic_equal_area::ConicEqualArea;

/// Albers USA projection - composite projection for the United States.
///
/// This is a composite projection that includes separate projections
/// for the lower 48 states, Alaska, and Hawaii.
#[derive(Clone, Debug)]
pub struct Albers {
    pub(super) lower48: ConicEqualArea,
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

    fn project_rotated(&self, lambda: f64, phi: f64) -> (f64, f64) {
        self.lower48.project_rotated(lambda, phi)
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
