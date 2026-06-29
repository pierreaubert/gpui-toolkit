//! Interactive camera state for 3D surface plots.

use super::projection::Camera2D;

/// Mouse interaction sensitivity and limits for a surface camera.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceCameraLimits {
    /// Minimum pitch in degrees.
    pub min_rotation_x: f64,
    /// Maximum pitch in degrees.
    pub max_rotation_x: f64,
    /// Minimum zoom factor (1.0 = normal).
    pub min_zoom: f64,
    /// Maximum zoom factor.
    pub max_zoom: f64,
    /// Pixels of mouse movement per degree of rotation.
    pub rotate_pixels_per_degree: f64,
    /// Zoom multiplier applied per scroll tick.
    pub zoom_factor_per_tick: f64,
}

impl Default for SurfaceCameraLimits {
    fn default() -> Self {
        Self {
            min_rotation_x: -90.0,
            max_rotation_x: 90.0,
            min_zoom: 0.25,
            max_zoom: 5.0,
            rotate_pixels_per_degree: 2.0,
            zoom_factor_per_tick: 1.1,
        }
    }
}

/// Owns the camera state for an interactive surface plot and applies pointer deltas.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SurfaceCamera {
    pub camera: Camera2D,
    pub limits: SurfaceCameraLimits,
}

impl SurfaceCamera {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the initial rotation in degrees (pitch, yaw).
    pub fn with_rotation(mut self, pitch: f64, yaw: f64) -> Self {
        self.camera.rotation_x = pitch;
        self.camera.rotation_z = yaw;
        self
    }

    /// Set the initial zoom factor (will be clamped to limits).
    pub fn with_zoom(mut self, zoom: f64) -> Self {
        self.camera.zoom = zoom.clamp(self.limits.min_zoom, self.limits.max_zoom);
        self
    }

    /// Apply a mouse drag delta in pixels.
    ///
    /// Horizontal movement changes yaw; vertical movement changes pitch.
    pub fn apply_drag(&mut self, dx: f64, dy: f64) {
        let degrees_per_pixel = 1.0 / self.limits.rotate_pixels_per_degree;
        self.camera.rotation_z += dx * degrees_per_pixel;
        self.camera.rotation_x -= dy * degrees_per_pixel;
        self.camera.rotation_x = self
            .camera
            .rotation_x
            .clamp(self.limits.min_rotation_x, self.limits.max_rotation_x);
    }

    /// Apply a scroll wheel delta.
    ///
    /// Positive delta zooms in; negative delta zooms out.
    pub fn apply_scroll(&mut self, delta: f64) {
        let factor = self.limits.zoom_factor_per_tick.powf(delta);
        self.camera.zoom =
            (self.camera.zoom * factor).clamp(self.limits.min_zoom, self.limits.max_zoom);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_applies_drag() {
        let mut camera = SurfaceCamera::new().with_rotation(30.0, 45.0);
        camera.apply_drag(10.0, -5.0);

        // 10 px / 2 px-per-degree = +5 degrees yaw
        assert!((camera.camera.rotation_z - 50.0).abs() < 1e-9);
        // -5 px / 2 px-per-degree = +2.5 degrees pitch (negated in apply_drag)
        assert!((camera.camera.rotation_x - 32.5).abs() < 1e-9);
    }

    #[test]
    fn test_camera_pitch_is_clamped() {
        let mut camera = SurfaceCamera::new();
        camera.apply_drag(0.0, -1000.0);
        assert_eq!(camera.camera.rotation_x, camera.limits.max_rotation_x);

        camera.apply_drag(0.0, 2000.0);
        assert_eq!(camera.camera.rotation_x, camera.limits.min_rotation_x);
    }

    #[test]
    fn test_camera_zoom_is_clamped() {
        let mut camera = SurfaceCamera::new().with_zoom(1.0);
        camera.apply_scroll(100.0);
        assert_eq!(camera.camera.zoom, camera.limits.max_zoom);

        camera.apply_scroll(-1000.0);
        assert_eq!(camera.camera.zoom, camera.limits.min_zoom);
    }

    #[test]
    fn test_camera_zoom_direction() {
        let mut camera = SurfaceCamera::new().with_zoom(1.0);
        camera.apply_scroll(1.0);
        assert!(camera.camera.zoom > 1.0);

        camera.apply_scroll(-1.0);
        assert!((camera.camera.zoom - 1.0).abs() < 1e-9);
    }
}
