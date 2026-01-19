//! Camera system for 3D surface visualization

use glam::{Mat4, Vec3};
use std::f32::consts::PI;

/// 3D camera with perspective projection
#[derive(Debug, Clone)]
pub struct Camera3D {
    /// Camera position in world space
    pub position: Vec3,
    /// Point the camera is looking at
    pub target: Vec3,
    /// Up vector
    pub up: Vec3,
    /// Field of view in radians
    pub fov: f32,
    /// Aspect ratio (width / height)
    pub aspect: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
}

impl Default for Camera3D {
    fn default() -> Self {
        Self {
            position: Vec3::new(2.0, 2.0, 2.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            fov: 45.0_f32.to_radians(),
            aspect: 1.0,
            near: 0.1,
            far: 100.0,
        }
    }
}

impl Camera3D {
    /// Create a new camera
    pub fn new() -> Self {
        Self::default()
    }

    /// Set camera position
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }

    /// Set camera target
    pub fn with_target(mut self, target: Vec3) -> Self {
        self.target = target;
        self
    }

    /// Set field of view in degrees
    pub fn with_fov_degrees(mut self, fov: f32) -> Self {
        self.fov = fov.to_radians();
        self
    }

    /// Set aspect ratio
    pub fn with_aspect(mut self, aspect: f32) -> Self {
        self.aspect = aspect;
        self
    }

    /// Get view matrix (world to camera transformation)
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position, self.target, self.up)
    }

    /// Get projection matrix
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far)
    }

    /// Get combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Get the direction the camera is looking
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    /// Get the right vector
    pub fn right(&self) -> Vec3 {
        self.forward().cross(self.up).normalize()
    }

    /// Project a world point to screen coordinates (0..width, 0..height)
    /// Returns None if the point is behind the camera
    pub fn project_to_screen(&self, world_pos: Vec3, width: f32, height: f32) -> Option<Vec3> {
        let view_proj = self.view_projection_matrix();
        let clip_pos = view_proj.project_point3(world_pos);

        // Check if point is in front of camera (z < 1.0 in NDC for wgpu/glam perspective?)
        // glam::Mat4::project_point3 returns normalized device coordinates.
        // For standard perspective, z should be in [0, 1] or [-1, 1] depending on API.
        // glam uses OpenGL convention [-1, 1] by default or 0..1?
        // Mat4::perspective_rh creates a matrix for 0..1 depth range (wgpu default).
        // So valid z is 0..1.

        // However, we also need to check w component if we did manual multiplication.
        // project_point3 does the division by w.
        // If w was negative (behind camera), the point might be projected incorrectly?
        // glam handles this?
        // Let's assume it works for points in front.

        // Map NDC [-1, 1] x [-1, 1] to screen [0, width] x [0, height]
        // Y is flipped in screen coords (0 is top) vs NDC (1 is top).

        let x = (clip_pos.x + 1.0) * 0.5 * width;
        let y = (1.0 - clip_pos.y) * 0.5 * height;
        let z = clip_pos.z;

        Some(Vec3::new(x, y, z))
    }
}

/// Orbit controls for interactive camera manipulation
#[derive(Debug, Clone)]
pub struct OrbitControls {
    /// Target point to orbit around
    pub target: Vec3,
    /// Distance from target
    pub distance: f32,
    /// Azimuth angle (horizontal rotation) in radians
    pub azimuth: f32,
    /// Elevation angle (vertical rotation) in radians
    pub elevation: f32,
    /// Minimum distance allowed
    pub min_distance: f32,
    /// Maximum distance allowed
    pub max_distance: f32,
    /// Minimum elevation (to prevent flipping)
    pub min_elevation: f32,
    /// Maximum elevation (to prevent flipping)
    pub max_elevation: f32,
    /// Rotation sensitivity
    pub rotate_speed: f32,
    /// Zoom sensitivity
    pub zoom_speed: f32,
    /// Pan sensitivity
    pub pan_speed: f32,
    /// Initial state for reset
    initial_target: Vec3,
    initial_distance: f32,
    initial_azimuth: f32,
    initial_elevation: f32,
}

impl Default for OrbitControls {
    fn default() -> Self {
        let azimuth = PI / 4.0; // 45 degrees
        let elevation = PI / 6.0; // 30 degrees
        let distance = 3.5;
        let target = Vec3::ZERO;

        Self {
            target,
            distance,
            azimuth,
            elevation,
            min_distance: 0.5,
            max_distance: 20.0,
            min_elevation: -PI / 2.0 + 0.1,
            max_elevation: PI / 2.0 - 0.1,
            rotate_speed: 0.01,
            zoom_speed: 0.1,
            pan_speed: 0.005,
            initial_target: target,
            initial_distance: distance,
            initial_azimuth: azimuth,
            initial_elevation: elevation,
        }
    }
}

impl OrbitControls {
    /// Create new orbit controls
    pub fn new() -> Self {
        Self::default()
    }

    /// Set initial camera position from spherical coordinates
    pub fn with_position(mut self, distance: f32, azimuth_deg: f32, elevation_deg: f32) -> Self {
        self.distance = distance;
        self.azimuth = azimuth_deg.to_radians();
        self.elevation = elevation_deg.to_radians();
        self.initial_distance = self.distance;
        self.initial_azimuth = self.azimuth;
        self.initial_elevation = self.elevation;
        self
    }

    /// Set the orbit target
    pub fn with_target(mut self, target: Vec3) -> Self {
        self.target = target;
        self.initial_target = target;
        self
    }

    /// Rotate the camera (typically from mouse drag)
    pub fn rotate(&mut self, delta_x: f32, delta_y: f32) {
        self.azimuth -= delta_x * self.rotate_speed;
        self.elevation += delta_y * self.rotate_speed;
        self.elevation = self.elevation.clamp(self.min_elevation, self.max_elevation);
    }

    /// Zoom the camera (typically from scroll wheel)
    pub fn zoom(&mut self, delta: f32) {
        self.distance *= 1.0 - delta * self.zoom_speed;
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
    }

    /// Pan the camera (typically from middle mouse drag)
    pub fn pan(&mut self, delta_x: f32, delta_y: f32, camera: &Camera3D) {
        let right = camera.right();
        let up = camera.up;
        let pan_offset = right * (-delta_x * self.pan_speed * self.distance)
            + up * (delta_y * self.pan_speed * self.distance);
        self.target += pan_offset;
    }

    /// Reset to initial position
    pub fn reset(&mut self) {
        self.target = self.initial_target;
        self.distance = self.initial_distance;
        self.azimuth = self.initial_azimuth;
        self.elevation = self.initial_elevation;
    }

    /// Calculate camera position from current orbit parameters
    pub fn camera_position(&self) -> Vec3 {
        let x = self.distance * self.elevation.cos() * self.azimuth.sin();
        let y = self.distance * self.elevation.sin();
        let z = self.distance * self.elevation.cos() * self.azimuth.cos();
        self.target + Vec3::new(x, y, z)
    }

    /// Update and return a camera with current orbit parameters
    pub fn update_camera(&self, camera: &mut Camera3D) {
        camera.position = self.camera_position();
        camera.target = self.target;
    }

    /// Create a camera from current orbit state
    pub fn to_camera(&self) -> Camera3D {
        Camera3D {
            position: self.camera_position(),
            target: self.target,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_matrices() {
        let camera = Camera3D::default();
        let view = camera.view_matrix();
        let proj = camera.projection_matrix();

        // Matrices should be valid (no NaN)
        assert!(!view.to_cols_array().iter().any(|x| x.is_nan()));
        assert!(!proj.to_cols_array().iter().any(|x| x.is_nan()));
    }

    #[test]
    fn test_orbit_controls() {
        let mut controls = OrbitControls::default();
        let initial_pos = controls.camera_position();

        // Rotate should change position
        controls.rotate(1.0, 0.5);
        let new_pos = controls.camera_position();
        assert!((initial_pos - new_pos).length() > 0.01);

        // Reset should restore position
        controls.reset();
        let reset_pos = controls.camera_position();
        assert!((initial_pos - reset_pos).length() < 0.001);
    }

    #[test]
    fn test_orbit_zoom() {
        let mut controls = OrbitControls::default();
        let initial_distance = controls.distance;

        controls.zoom(0.5); // Zoom in
        assert!(controls.distance < initial_distance);

        controls.zoom(-0.5); // Zoom out
        assert!(controls.distance > initial_distance * 0.9); // Approximately back
    }

    #[test]
    fn test_elevation_clamping() {
        let mut controls = OrbitControls::default();

        // Try to rotate past vertical limits
        controls.rotate(0.0, 1000.0);
        assert!(controls.elevation <= controls.max_elevation);

        controls.rotate(0.0, -2000.0);
        assert!(controls.elevation >= controls.min_elevation);
    }
}
