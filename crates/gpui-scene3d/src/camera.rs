//! Orbit camera with perspective and orthographic projection.

// Rust guideline compliant 2026-02-21

/// Projection mode.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    /// Perspective with vertical field of view in radians.
    Perspective {
        /// Vertical field of view in radians.
        fov_y: f32,
    },
    /// Orthographic with vertical half-height in world units.
    Orthographic {
        /// Half of the visible height in world units.
        half_height: f32,
    },
}

/// Orbit camera: position on a sphere around a target.
///
/// All math is `f32` 4x4 matrices in column-major order, ready for wgpu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Look-at target in world space.
    pub target: [f32; 3],
    /// Azimuth in radians around the up axis.
    pub azimuth: f32,
    /// Elevation in radians, clamped away from the poles.
    pub elevation: f32,
    /// Distance from target.
    pub distance: f32,
    /// Projection mode.
    pub projection: Projection,
    /// Near plane distance.
    pub near: f32,
    /// Far plane distance.
    pub far: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: [0.0; 3],
            azimuth: 0.7,
            elevation: 0.5,
            distance: 3.0,
            projection: Projection::Perspective {
                fov_y: 45.0_f32.to_radians(),
            },
            near: 0.01,
            far: 1000.0,
        }
    }
}

impl Camera {
    /// Creates a camera fitted to bounds: target at the centre, distance
    /// covering the diagonal.
    #[must_use]
    pub fn fitted(bounds: gpui_mesh::Bounds) -> Self {
        let centre = bounds.center();
        let diagonal = bounds.diagonal() as f32;
        Self {
            target: [centre[0] as f32, centre[1] as f32, centre[2] as f32],
            distance: (diagonal * 1.2).max(1.0),
            near: (diagonal * 0.001).max(0.001),
            far: (diagonal * 100.0).max(10.0),
            ..Self::default()
        }
    }

    /// Eye position in world space.
    #[must_use]
    pub fn eye(&self) -> [f32; 3] {
        let elevation = self.elevation.clamp(-1.5, 1.5);
        let horizontal = elevation.cos() * self.distance;
        [
            self.target[0] + horizontal * self.azimuth.sin(),
            self.target[1] + elevation.sin() * self.distance,
            self.target[2] + horizontal * self.azimuth.cos(),
        ]
    }

    /// Orbits by azimuth/elevation deltas in radians.
    pub fn orbit(&mut self, d_azimuth: f32, d_elevation: f32) {
        self.azimuth += d_azimuth;
        self.elevation = (self.elevation + d_elevation).clamp(-1.5, 1.5);
    }

    /// Dollies by a multiplicative factor (below 1 zooms in).
    pub fn dolly(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(self.near * 2.0, self.far * 0.5);
    }

    /// Camera right direction in world space.
    #[must_use]
    pub fn right(&self) -> [f32; 3] {
        let forward = normalize(sub(self.target, self.eye()));
        normalize(cross(forward, [0.0, 1.0, 0.0]))
    }

    /// Camera up direction in world space.
    #[must_use]
    pub fn up(&self) -> [f32; 3] {
        let forward = normalize(sub(self.target, self.eye()));
        cross(self.right(), forward)
    }

    /// Pans the target in the camera plane (grab-the-world dragging).
    ///
    /// `dx`/`dy` are pointer deltas in pixels; the world moves with the
    /// pointer, scaled by the current distance so panning feels uniform
    /// at any zoom level.
    pub fn pan(&mut self, dx: f32, dy: f32) {
        let scale = self.distance * 0.0016;
        let right = self.right();
        let up = self.up();
        for axis in 0..3 {
            self.target[axis] += (-right[axis] * dx + up[axis] * dy) * scale;
        }
    }

    /// View matrix, column-major.
    #[must_use]
    pub fn view_matrix(&self) -> [f32; 16] {
        look_at(self.eye(), self.target, [0.0, 1.0, 0.0])
    }

    /// Projection matrix for an aspect ratio, column-major, wgpu depth [0,1].
    #[must_use]
    pub fn projection_matrix(&self, aspect: f32) -> [f32; 16] {
        match self.projection {
            Projection::Perspective { fov_y } => {
                perspective(fov_y, aspect.max(1e-6), self.near, self.far)
            }
            Projection::Orthographic { half_height } => {
                orthographic(half_height, aspect.max(1e-6), self.near, self.far)
            }
        }
    }

    /// Combined view-projection matrix, column-major.
    #[must_use]
    pub fn view_projection(&self, aspect: f32) -> [f32; 16] {
        multiply(self.projection_matrix(aspect), self.view_matrix())
    }

    /// Projects positions expressed relative to a world-space origin.
    ///
    /// Subtracts the origin before constructing the view matrix, avoiding
    /// cancellation between large translation terms in `f32` matrices.
    #[must_use]
    pub fn view_projection_relative_to(&self, origin: [f64; 3], aspect: f32) -> [f32; 16] {
        let mut local = *self;
        local.target =
            std::array::from_fn(|axis| (f64::from(self.target[axis]) - origin[axis]) as f32);
        local.view_projection(aspect)
    }
}

/// Multiplies two column-major 4x4 matrices: `a * b`.
#[must_use]
pub fn multiply(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut out = [0.0f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a[k * 4 + row] * b[col * 4 + k];
            }
            out[col * 4 + row] = sum;
        }
    }
    out
}

/// Transforms a point by a column-major matrix, returning clip space.
#[must_use]
pub fn transform(m: &[f32; 16], p: [f32; 3]) -> [f32; 4] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
        m[3] * p[0] + m[7] * p[1] + m[11] * p[2] + m[15],
    ]
}

fn look_at(eye: [f32; 3], target: [f32; 3], up: [f32; 3]) -> [f32; 16] {
    let f = normalize(sub(target, eye));
    let s = normalize(cross(f, up));
    let u = cross(s, f);
    [
        s[0],
        u[0],
        -f[0],
        0.0, //
        s[1],
        u[1],
        -f[1],
        0.0, //
        s[2],
        u[2],
        -f[2],
        0.0, //
        -dot(s, eye),
        -dot(u, eye),
        dot(f, eye),
        1.0,
    ]
}

fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    // Right-handed, wgpu depth [0, 1]: w = -z_view, z_ndc is 0 at the near
    // plane and 1 at the far plane.
    let f = 1.0 / (fov_y * 0.5).tan();
    let a = far / (near - far);
    [
        f / aspect,
        0.0,
        0.0,
        0.0, //
        0.0,
        f,
        0.0,
        0.0, //
        0.0,
        0.0,
        a,
        -1.0, //
        0.0,
        0.0,
        near * a,
        0.0,
    ]
}

fn orthographic(half_height: f32, aspect: f32, near: f32, far: f32) -> [f32; 16] {
    let half_width = half_height * aspect;
    let depth = far - near;
    [
        1.0 / half_width,
        0.0,
        0.0,
        0.0, //
        0.0,
        1.0 / half_height,
        0.0,
        0.0, //
        0.0,
        0.0,
        -1.0 / depth,
        0.0, //
        0.0,
        0.0,
        -near / depth,
        1.0,
    ]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = dot(v, v).sqrt();
    if len == 0.0 {
        return v;
    }
    [v[0] / len, v[1] / len, v[2] / len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fitted_camera_looks_at_bounds_centre() {
        let camera = Camera::fitted(gpui_mesh::Bounds {
            min: [0.0; 3],
            max: [2.0, 1.0, 1.0],
        });
        assert_eq!(camera.target, [1.0, 0.5, 0.5]);
        let clip = transform(&camera.view_projection(1.0), [1.0, 0.5, 0.5]);
        assert!(clip[3] > 0.0);
        assert!(clip[0].abs() < 0.01 && clip[1].abs() < 0.01);
    }

    #[test]
    fn orbiting_keeps_distance() {
        let mut camera = Camera::default();
        let before = camera.eye();
        camera.orbit(0.5, 0.0);
        let after = camera.eye();
        let dist = |p: [f32; 3]| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        assert!((dist(before) - dist(after)).abs() < 1e-5);
        assert_ne!(before, after);
    }

    #[test]
    fn rebased_projection_preserves_camera_offsets_at_large_origins() {
        let reference = Camera::default().view_projection(1.0);
        for center in [1000.0_f32, 1_000_000_000.0] {
            let camera = Camera {
                target: [center; 3],
                ..Camera::default()
            };
            assert_eq!(
                camera.view_projection_relative_to([f64::from(center); 3], 1.0),
                reference
            );
        }
    }
}

#[cfg(test)]
mod pan_tests {
    use super::*;

    #[test]
    fn pan_moves_target_in_camera_plane() {
        let mut camera = Camera::default();
        let before = camera.target;
        camera.pan(100.0, 0.0);
        assert_ne!(camera.target, before);
        // Panning is in-plane: distance to the eye direction is preserved.
        let right = camera.right();
        let moved = [
            camera.target[0] - before[0],
            camera.target[1] - before[1],
            camera.target[2] - before[2],
        ];
        let along_right = moved[0] * right[0] + moved[1] * right[1] + moved[2] * right[2];
        assert!(along_right < 0.0, "dragging right moves the world right");
    }
}
