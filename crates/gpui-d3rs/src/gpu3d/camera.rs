//! Camera system for 3D surface visualization

use crate::mesh::MeshBounds;
use glam::{Mat4, Vec3};
use std::f32::consts::PI;

/// The projection used by a [`Camera3D`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// A perspective projection with a vertical field of view in radians.
    Perspective { fov_y: f32 },
    /// An orthographic projection with the given vertical half-height.
    Orthographic { half_height: f64 },
}

/// A fixed camera orientation for common 3D views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardView {
    Front,
    Back,
    Left,
    Right,
    Top,
    Bottom,
    Isometric,
}

/// 3D camera.
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
    projection: Projection,
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
            projection: Projection::Perspective {
                fov_y: 45.0_f32.to_radians(),
            },
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
        self.projection = Projection::Perspective { fov_y: self.fov };
        self
    }

    /// Create a camera with the requested projection.
    pub fn with_projection(projection: Projection) -> Self {
        let mut camera = Self::default();
        if let Projection::Perspective { fov_y } = projection {
            camera.fov = fov_y;
        }
        camera.projection = projection;
        camera
    }

    /// Set aspect ratio
    pub fn with_aspect(mut self, aspect: f32) -> Self {
        self.aspect = aspect;
        self
    }

    /// Get view matrix (world to camera transformation)
    pub fn view_matrix(&self) -> Mat4 {
        let forward = self.forward();
        let up = if forward.cross(self.up).length_squared() < 1e-12 {
            // A standard front/back view looks along the default Y up axis.
            // Choose a stable screen-up axis instead of passing collinear
            // vectors to look_at_rh.
            if forward.cross(Vec3::Z).length_squared() >= 1e-12 {
                Vec3::Z
            } else {
                Vec3::X
            }
        } else {
            self.up
        };
        Mat4::look_at_rh(self.position, self.target, up)
    }

    /// Get projection matrix
    pub fn projection_matrix(&self) -> Mat4 {
        let aspect = self.aspect.max(1e-6);
        let near = self.near.max(1e-6);
        let far = self.far.max(near + 1e-6);
        match self.projection {
            Projection::Perspective { .. } => Mat4::perspective_rh(self.fov, aspect, near, far),
            Projection::Orthographic { half_height } => {
                let half_height = (half_height as f32).abs().max(1e-6);
                let half_width = half_height * aspect;
                Mat4::orthographic_rh(
                    -half_width,
                    half_width,
                    -half_height,
                    half_height,
                    near,
                    far,
                )
            }
        }
    }

    /// Get combined view-projection matrix
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }

    /// Get the direction the camera is looking
    pub fn forward(&self) -> Vec3 {
        let dir = self.target - self.position;
        if dir.length_squared() < 1e-12 {
            return Vec3::NEG_Z;
        }
        dir.normalize()
    }

    /// Get the right vector
    pub fn right(&self) -> Vec3 {
        let fwd = self.forward();
        let right = fwd.cross(self.up);
        if right.length_squared() < 1e-12 {
            return Vec3::X;
        }
        right.normalize()
    }

    /// Get camera-facing billboard plane axes in world space.
    pub fn billboard_axes(&self) -> (Vec3, Vec3) {
        let right = self.right();
        let up = right.cross(self.forward());
        if up.length_squared() < 1e-12 {
            return (right, self.up.normalize_or_zero());
        }
        (right, up.normalize())
    }

    /// Project a world point to screen coordinates (0..width, 0..height)
    /// Returns None if the point is behind the camera
    pub fn project_to_screen(&self, world_pos: Vec3, width: f32, height: f32) -> Option<Vec3> {
        let view_proj = self.view_projection_matrix();
        let clip_pos = view_proj * world_pos.extend(1.0);
        if clip_pos.w <= 1e-6 {
            return None;
        }

        let ndc = clip_pos.truncate() / clip_pos.w;
        if !(0.0..=1.0).contains(&ndc.z) {
            return None;
        }

        let x = (ndc.x + 1.0) * 0.5 * width;
        let y = (1.0 - ndc.y) * 0.5 * height;
        let z = ndc.z;

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

    /// Get the current view direction, from the camera toward its target.
    pub fn view_direction(&self) -> Vec3 {
        (self.target - self.camera_position()).normalize_or_zero()
    }

    /// Set one of the fixed camera orientations while preserving target and
    /// distance.
    pub fn set_standard_view(&mut self, view: StandardView) {
        let (azimuth, elevation) = match view {
            // The coordinate convention used by d3rs treats Z as the depth
            // axis for top/bottom views and Y as the front/back axis.
            StandardView::Front => (0.0, PI / 2.0),
            StandardView::Back => (0.0, -PI / 2.0),
            StandardView::Left => (-PI / 2.0, 0.0),
            StandardView::Right => (PI / 2.0, 0.0),
            StandardView::Top => (0.0, 0.0),
            StandardView::Bottom => (PI, 0.0),
            StandardView::Isometric => (PI / 4.0, PI / 6.0),
        };
        self.azimuth = azimuth;
        self.elevation = elevation;
    }

    /// Move the orbit target and camera far enough away to contain the bounds.
    ///
    /// The fit uses the bounding sphere and the default camera's 45-degree
    /// vertical field of view. The narrower of the vertical and horizontal
    /// view angles is used for non-wide viewports.
    pub fn fit_to_bounds(&mut self, bounds: MeshBounds, viewport_aspect: f32) {
        let origin = bounds.origin();
        self.target = Vec3::new(origin[0] as f32, origin[1] as f32, origin[2] as f32);

        let extent = [
            bounds.max[0] - bounds.min[0],
            bounds.max[1] - bounds.min[1],
            bounds.max[2] - bounds.min[2],
        ];
        let radius = 0.5
            * extent
                .iter()
                .map(|component| component * component)
                .sum::<f64>()
                .sqrt();
        let radius = (radius as f32).max(1e-6);

        let aspect = viewport_aspect.abs().max(1e-6);
        let fov_y = Camera3D::default().fov;
        let vertical_half_angle = (fov_y * 0.5).clamp(1e-3, PI * 0.5 - 1e-3);
        let horizontal_half_angle = (vertical_half_angle.tan() * aspect).atan();
        let half_angle = vertical_half_angle.min(horizontal_half_angle);
        let required_distance = radius / half_angle.sin();

        self.distance = required_distance.max(self.min_distance);
        self.max_distance = self.max_distance.max(self.distance);
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

    #[test]
    fn test_camera_zero_aspect_and_position() {
        // Zero aspect ratio used to produce NaN in projection matrix
        let mut camera = Camera3D::default();
        camera.aspect = 0.0;
        let proj = camera.projection_matrix();
        assert!(
            !proj.to_cols_array().iter().any(|x| x.is_nan()),
            "projection with zero aspect should not be NaN"
        );

        // Position equal to target used to produce zero-length forward vector
        camera.position = Vec3::new(1.0, 1.0, 1.0);
        camera.target = Vec3::new(1.0, 1.0, 1.0);
        let fwd = camera.forward();
        assert!(
            fwd.is_finite(),
            "forward with position==target should be finite"
        );
        assert!(fwd.length() > 0.0, "forward should not be zero vector");
    }
}
