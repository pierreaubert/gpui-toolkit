//! 3D to 2D projection systems for surface rendering

use super::SurfacePoint3D;
use std::f64::consts::PI;

/// A 2D point resulting from projection
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

impl Point2D {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Trait for 3D to 2D projections
pub trait Projection: Clone {
    /// Project a 3D point to 2D screen coordinates
    fn project(&self, x: f64, y: f64, z: f64) -> Point2D;

    /// Project a SurfacePoint3D to 2D
    fn project_point(&self, p: &SurfacePoint3D) -> Point2D {
        self.project(p.x, p.y, p.z)
    }

    /// Get the depth value for sorting (higher = further from camera)
    fn depth(&self, x: f64, y: f64, z: f64) -> f64;

    /// Get depth for a surface point
    fn point_depth(&self, p: &SurfacePoint3D) -> f64 {
        self.depth(p.x, p.y, p.z)
    }
}

/// Available projection types
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum ProjectionType {
    /// Standard isometric projection (30 degree angle)
    #[default]
    Isometric,
    /// Oblique projection with configurable depth
    Oblique,
    /// Orthographic projection with rotation
    Orthographic,
    /// Perspective projection (simulated vanishing point)
    Perspective,
}

/// Camera controls for view manipulation
#[derive(Clone, Debug)]
pub struct Camera2D {
    /// Rotation around X axis (pitch) in degrees
    pub rotation_x: f64,
    /// Rotation around Z axis (yaw) in degrees
    pub rotation_z: f64,
    /// Zoom factor (1.0 = normal)
    pub zoom: f64,
    /// Pan offset in screen coordinates
    pub pan: (f64, f64),
}

impl Default for Camera2D {
    fn default() -> Self {
        Self {
            rotation_x: 30.0,
            rotation_z: 45.0,
            zoom: 1.0,
            pan: (0.0, 0.0),
        }
    }
}

impl Camera2D {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn rotation(mut self, x: f64, z: f64) -> Self {
        self.rotation_x = x;
        self.rotation_z = z;
        self
    }

    pub fn zoom(mut self, zoom: f64) -> Self {
        self.zoom = zoom;
        self
    }

    pub fn pan(mut self, x: f64, y: f64) -> Self {
        self.pan = (x, y);
        self
    }
}

/// Isometric projection - the most common for scientific visualization
///
/// Uses a fixed 30-degree viewing angle that shows all three axes equally.
#[derive(Clone, Debug)]
pub struct IsometricProjection {
    /// Scale factor
    pub scale: f64,
    /// Screen origin (center of projection)
    pub origin: (f64, f64),
    /// Camera settings
    pub camera: Camera2D,
}

impl Default for IsometricProjection {
    fn default() -> Self {
        Self {
            scale: 100.0,
            origin: (0.0, 0.0),
            camera: Camera2D::default(),
        }
    }
}

impl IsometricProjection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    pub fn origin(mut self, x: f64, y: f64) -> Self {
        self.origin = (x, y);
        self
    }

    pub fn camera(mut self, camera: Camera2D) -> Self {
        self.camera = camera;
        self
    }
}

impl Projection for IsometricProjection {
    fn project(&self, x: f64, y: f64, z: f64) -> Point2D {
        let rx = self.camera.rotation_x * PI / 180.0;
        let rz = self.camera.rotation_z * PI / 180.0;

        // Rotate around Z axis (yaw)
        let x1 = x * rz.cos() - y * rz.sin();
        let y1 = x * rz.sin() + y * rz.cos();

        // Rotate around X axis (pitch) - affects Y and Z
        let y2 = y1 * rx.cos() - z * rx.sin();

        // Project to 2D (drop the depth coordinate after rotation)
        // For isometric, we use x1 for screen X and a combination for screen Y
        let screen_x = x1 * self.scale * self.camera.zoom + self.origin.0 + self.camera.pan.0;
        // Y is inverted because screen Y goes down
        let screen_y = -y2 * self.scale * self.camera.zoom + self.origin.1 + self.camera.pan.1;

        Point2D::new(screen_x, screen_y)
    }

    fn depth(&self, x: f64, y: f64, z: f64) -> f64 {
        let rx = self.camera.rotation_x * PI / 180.0;
        let rz = self.camera.rotation_z * PI / 180.0;

        // Rotate around Z axis
        let y1 = x * rz.sin() + y * rz.cos();

        // Rotate around X axis
        let z2 = y1 * rx.sin() + z * rx.cos();

        -z2 // Negative because larger Z should be further back
    }
}

/// Oblique projection - shows one face flat with depth at an angle
#[derive(Clone, Debug)]
pub struct ObliqueProjection {
    pub scale: f64,
    /// Angle of the depth axis in degrees (typically 30 or 45)
    pub angle: f64,
    /// Depth reduction factor (1.0 = cavalier, 0.5 = cabinet)
    pub depth_factor: f64,
    pub origin: (f64, f64),
    pub camera: Camera2D,
}

impl Default for ObliqueProjection {
    fn default() -> Self {
        Self {
            scale: 100.0,
            angle: 45.0,
            depth_factor: 0.5, // Cabinet projection by default
            origin: (0.0, 0.0),
            camera: Camera2D::default(),
        }
    }
}

impl ObliqueProjection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cavalier() -> Self {
        Self {
            depth_factor: 1.0,
            ..Self::default()
        }
    }

    pub fn cabinet() -> Self {
        Self {
            depth_factor: 0.5,
            ..Self::default()
        }
    }

    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    pub fn angle(mut self, angle: f64) -> Self {
        self.angle = angle;
        self
    }

    pub fn origin(mut self, x: f64, y: f64) -> Self {
        self.origin = (x, y);
        self
    }
}

impl Projection for ObliqueProjection {
    fn project(&self, x: f64, y: f64, z: f64) -> Point2D {
        let angle_rad = self.angle * PI / 180.0;
        let depth_offset_x = y * self.depth_factor * angle_rad.cos();
        let depth_offset_y = y * self.depth_factor * angle_rad.sin();

        let screen_x = (x + depth_offset_x) * self.scale * self.camera.zoom
            + self.origin.0
            + self.camera.pan.0;
        let screen_y = -(z + depth_offset_y) * self.scale * self.camera.zoom
            + self.origin.1
            + self.camera.pan.1;

        Point2D::new(screen_x, screen_y)
    }

    fn depth(&self, _x: f64, y: f64, _z: f64) -> f64 {
        -y // Depth is primarily determined by Y in oblique projection
    }
}

/// Orthographic projection with arbitrary rotation
#[derive(Clone, Debug)]
pub struct OrthographicProjection {
    pub scale: f64,
    /// Rotation angles in degrees (around X, Y, Z axes)
    pub rotation: (f64, f64, f64),
    pub origin: (f64, f64),
}

impl Default for OrthographicProjection {
    fn default() -> Self {
        Self {
            scale: 100.0,
            rotation: (30.0, 0.0, 45.0),
            origin: (0.0, 0.0),
        }
    }
}

impl OrthographicProjection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    pub fn rotation(mut self, rx: f64, ry: f64, rz: f64) -> Self {
        self.rotation = (rx, ry, rz);
        self
    }

    pub fn origin(mut self, x: f64, y: f64) -> Self {
        self.origin = (x, y);
        self
    }

    fn rotate_point(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let (rx, ry, rz) = self.rotation;
        let rx = rx * PI / 180.0;
        let ry = ry * PI / 180.0;
        let rz = rz * PI / 180.0;

        // Rotate around X
        let y1 = y * rx.cos() - z * rx.sin();
        let z1 = y * rx.sin() + z * rx.cos();

        // Rotate around Y
        let x2 = x * ry.cos() + z1 * ry.sin();
        let z2 = -x * ry.sin() + z1 * ry.cos();

        // Rotate around Z
        let x3 = x2 * rz.cos() - y1 * rz.sin();
        let y3 = x2 * rz.sin() + y1 * rz.cos();

        (x3, y3, z2)
    }
}

impl Projection for OrthographicProjection {
    fn project(&self, x: f64, y: f64, z: f64) -> Point2D {
        let (x_rot, y_rot, _z_rot) = self.rotate_point(x, y, z);

        let screen_x = x_rot * self.scale + self.origin.0;
        let screen_y = -y_rot * self.scale + self.origin.1;

        Point2D::new(screen_x, screen_y)
    }

    fn depth(&self, x: f64, y: f64, z: f64) -> f64 {
        let (_, _, z_rot) = self.rotate_point(x, y, z);
        -z_rot
    }
}

/// Perspective projection with a simulated vanishing point
#[derive(Clone, Debug)]
pub struct PerspectiveProjection {
    pub scale: f64,
    /// Field of view in degrees
    pub fov: f64,
    /// Distance from camera to origin
    pub distance: f64,
    /// Rotation angles in degrees
    pub rotation: (f64, f64, f64),
    pub origin: (f64, f64),
}

impl Default for PerspectiveProjection {
    fn default() -> Self {
        Self {
            scale: 100.0,
            fov: 60.0,
            distance: 5.0,
            rotation: (30.0, 0.0, 45.0),
            origin: (0.0, 0.0),
        }
    }
}

impl PerspectiveProjection {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    pub fn fov(mut self, fov: f64) -> Self {
        self.fov = fov;
        self
    }

    pub fn distance(mut self, distance: f64) -> Self {
        self.distance = distance;
        self
    }

    pub fn rotation(mut self, rx: f64, ry: f64, rz: f64) -> Self {
        self.rotation = (rx, ry, rz);
        self
    }

    pub fn origin(mut self, x: f64, y: f64) -> Self {
        self.origin = (x, y);
        self
    }

    fn rotate_point(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let (rx, ry, rz) = self.rotation;
        let rx = rx * PI / 180.0;
        let ry = ry * PI / 180.0;
        let rz = rz * PI / 180.0;

        // Rotate around X
        let y1 = y * rx.cos() - z * rx.sin();
        let z1 = y * rx.sin() + z * rx.cos();

        // Rotate around Y
        let x2 = x * ry.cos() + z1 * ry.sin();
        let z2 = -x * ry.sin() + z1 * ry.cos();

        // Rotate around Z
        let x3 = x2 * rz.cos() - y1 * rz.sin();
        let y3 = x2 * rz.sin() + y1 * rz.cos();

        (x3, y3, z2)
    }
}

impl Projection for PerspectiveProjection {
    fn project(&self, x: f64, y: f64, z: f64) -> Point2D {
        let (x_rot, y_rot, z_rot) = self.rotate_point(x, y, z);

        // Apply perspective division
        let z_offset = self.distance + z_rot;
        let perspective_factor = if z_offset.abs() > 0.001 {
            self.distance / z_offset
        } else {
            self.distance / 0.001
        };

        let screen_x = x_rot * perspective_factor * self.scale + self.origin.0;
        let screen_y = -y_rot * perspective_factor * self.scale + self.origin.1;

        Point2D::new(screen_x, screen_y)
    }

    fn depth(&self, x: f64, y: f64, z: f64) -> f64 {
        let (_, _, z_rot) = self.rotate_point(x, y, z);
        -(self.distance + z_rot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isometric_projection() {
        let proj = IsometricProjection::new()
            .scale(1.0)
            .origin(0.0, 0.0)
            .camera(Camera2D::new().rotation(0.0, 0.0));

        let p = proj.project(1.0, 0.0, 0.0);
        assert!((p.x - 1.0).abs() < 1e-10);
        assert!(p.y.abs() < 1e-10);
    }

    #[test]
    fn test_oblique_projection() {
        let proj = ObliqueProjection::cabinet().scale(1.0).origin(0.0, 0.0);

        let p = proj.project(1.0, 0.0, 0.0);
        assert!((p.x - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_perspective_projection() {
        let proj = PerspectiveProjection::new()
            .scale(1.0)
            .distance(5.0)
            .rotation(0.0, 0.0, 0.0);

        // Points further from camera should appear smaller
        let p_near = proj.project(1.0, 0.0, -1.0);
        let p_far = proj.project(1.0, 0.0, 1.0);

        assert!(p_near.x.abs() > p_far.x.abs());
    }

    #[test]
    fn test_camera_controls() {
        let camera = Camera2D::new()
            .rotation(45.0, 30.0)
            .zoom(2.0)
            .pan(10.0, 20.0);

        assert_eq!(camera.rotation_x, 45.0);
        assert_eq!(camera.rotation_z, 30.0);
        assert_eq!(camera.zoom, 2.0);
        assert_eq!(camera.pan, (10.0, 20.0));
    }

    #[test]
    fn test_depth_sorting() {
        let proj = IsometricProjection::new().camera(Camera2D::new().rotation(30.0, 45.0));

        let p1 = SurfacePoint3D::new(0.0, 0.0, 0.0, 0.0);
        let p2 = SurfacePoint3D::new(0.0, 1.0, 0.0, 0.0);

        let d1 = proj.point_depth(&p1);
        let d2 = proj.point_depth(&p2);

        // p2 should be further back (higher depth)
        assert!(d2 > d1);
    }
}
