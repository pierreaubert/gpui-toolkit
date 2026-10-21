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
/// World positions use `f64`; local column-major matrices use `f32` for wgpu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Look-at target in world space.
    pub target: [f64; 3],
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
    /// Checks finite camera parameters and a nondegenerate viewing frustum.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid positions, angles, distances or projection.
    pub fn validate(&self) -> Result<(), crate::SceneError> {
        if self.target.iter().any(|v| !v.is_finite())
            || !self.azimuth.is_finite()
            || !self.elevation.is_finite()
        {
            return Err(crate::SceneError::InvalidCamera(
                "target and angles must be finite",
            ));
        }
        if !self.distance.is_finite()
            || self.distance < 0.0
            || !self.near.is_finite()
            || self.near <= 0.0
            || !self.far.is_finite()
            || self.far <= self.near
        {
            return Err(crate::SceneError::InvalidCamera(
                "distances require radius >= 0 and 0 < near < far",
            ));
        }
        let valid = match self.projection {
            Projection::Perspective { fov_y } => {
                fov_y.is_finite() && fov_y > 0.0 && fov_y < std::f32::consts::PI
            }
            Projection::Orthographic { half_height } => {
                half_height.is_finite() && half_height > 0.0
            }
        };
        if !valid {
            return Err(crate::SceneError::InvalidCamera(
                "invalid projection extent",
            ));
        }
        let projection = self.projection_matrix(1.0);
        let view = self.view_matrix_relative_to(self.target);
        if projection
            .iter()
            .chain(view.iter())
            .any(|value| !value.is_finite())
            || projection[0] == 0.0
            || projection[5] == 0.0
            || projection[10] == 0.0
            || (matches!(self.projection, Projection::Perspective { .. }) && projection[14] == 0.0)
        {
            return Err(crate::SceneError::InvalidCamera(
                "projection exceeds local f32 precision",
            ));
        }
        Ok(())
    }

    /// Fits the bounds' circumscribed sphere in a square viewport.
    ///
    /// Use [`Self::fitted_for_aspect`] for other viewport shapes and validated bounds.
    #[must_use]
    pub fn fitted(bounds: gpui_mesh::Bounds) -> Self {
        Self::fit(bounds, 1.0)
    }

    /// Fits bounds with a margin for the viewport's limiting field of view.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid bounds/aspect or unrepresentable camera parameters.
    pub fn fitted_for_aspect(
        bounds: gpui_mesh::Bounds,
        aspect: f32,
    ) -> Result<Self, crate::SceneError> {
        if !aspect.is_finite()
            || aspect <= 0.0
            || (0..3).any(|axis| {
                !bounds.min[axis].is_finite()
                    || !bounds.max[axis].is_finite()
                    || bounds.min[axis] > bounds.max[axis]
            })
        {
            return Err(crate::SceneError::InvalidCamera(
                "fitting requires finite ordered bounds and positive aspect",
            ));
        }
        let camera = Self::fit(bounds, aspect);
        camera.validate()?;
        Ok(camera)
    }

    fn fit(bounds: gpui_mesh::Bounds, aspect: f32) -> Self {
        let centre = bounds.center();
        let diagonal = bounds.diagonal() as f32;
        let camera = Self::default();
        let Projection::Perspective { fov_y } = camera.projection else {
            unreachable!("default perspective")
        };
        let vertical = f64::from(fov_y) * 0.5;
        let horizontal = (vertical.tan() * f64::from(aspect)).atan();
        let distance = (f64::from(diagonal) * 0.5 / vertical.min(horizontal).sin() * 1.05) as f32;
        Self {
            target: centre,
            distance: distance.max(1.0),
            near: (diagonal * 0.001).max(0.001),
            far: (diagonal * 100.0)
                .max((distance + diagonal) * 2.0)
                .max(10.0),
            ..camera
        }
    }

    /// Eye position in world space.
    #[must_use]
    pub fn eye(&self) -> [f64; 3] {
        let offset = self.eye_offset();
        std::array::from_fn(|axis| self.target[axis] + f64::from(offset[axis]))
    }

    fn eye_offset(&self) -> [f32; 3] {
        let elevation = self.elevation.clamp(-1.5, 1.5);
        let horizontal = elevation.cos() * self.distance;
        [
            horizontal * self.azimuth.sin(),
            elevation.sin() * self.distance,
            horizontal * self.azimuth.cos(),
        ]
    }

    /// Orbits by azimuth/elevation deltas in radians.
    pub fn orbit(&mut self, d_azimuth: f32, d_elevation: f32) {
        self.azimuth += d_azimuth;
        self.elevation = (self.elevation + d_elevation).clamp(-1.5, 1.5);
    }

    /// Dollies by a multiplicative factor (below 1 zooms in).
    /// Non-positive/non-finite factors and invalid cameras are ignored.
    pub fn dolly(&mut self, factor: f32) {
        if !factor.is_finite() || factor <= 0.0 || self.validate().is_err() {
            return;
        }
        let (mut min, mut max) = (self.near * 2.0, self.far * 0.5);
        if min > max {
            (min, max) = (self.near, self.far);
        }
        self.distance = (self.distance * factor).clamp(min, max);
    }

    /// Camera right direction in world space.
    #[must_use]
    pub fn right(&self) -> [f32; 3] {
        [self.azimuth.cos(), 0.0, -self.azimuth.sin()]
    }

    /// Camera up direction in world space.
    #[must_use]
    pub fn up(&self) -> [f32; 3] {
        cross(self.right(), self.forward())
    }

    fn forward(&self) -> [f32; 3] {
        let elevation = self.elevation.clamp(-1.5, 1.5);
        [
            -elevation.cos() * self.azimuth.sin(),
            -elevation.sin(),
            -elevation.cos() * self.azimuth.cos(),
        ]
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
            self.target[axis] += f64::from((-right[axis] * dx + up[axis] * dy) * scale);
        }
    }

    /// World-space view matrix, column-major.
    ///
    /// For large coordinates use [`Self::view_matrix_relative_to`] and local
    /// vertices; a world-space `f32` matrix cannot retain small world offsets.
    #[must_use]
    pub fn view_matrix(&self) -> [f32; 16] {
        self.view_matrix_relative_to([0.0; 3])
    }

    /// Builds a local view matrix without large-coordinate cancellation.
    #[must_use]
    pub fn view_matrix_relative_to(&self, origin: [f64; 3]) -> [f32; 16] {
        let offset = self.eye_offset();
        let eye: [f64; 3] =
            std::array::from_fn(|axis| self.target[axis] - origin[axis] + f64::from(offset[axis]));
        let (s, u, f) = (self.right(), self.up(), self.forward());
        view_from_eye(s, u, f, eye)
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
    /// Use [`Self::view_projection_relative_to`] for rebased large-coordinate geometry.
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
        multiply(
            self.projection_matrix(aspect),
            self.view_matrix_relative_to(origin),
        )
    }

    /// Checks a viewport-specific actor-relative matrix for GPU representation.
    ///
    /// Uses the actual positive aspect ratio, without the legacy raw matrix
    /// helper's minimum-aspect clamp. Returned coefficients are finite and all
    /// nonzero coefficients are normal f32 values. This conservative profile
    /// does not promise exact f64 geometry or a bound on float rounding error.
    ///
    /// # Errors
    ///
    /// Returns invalid-camera errors for invalid parameters/aspect, or unsupported
    /// precision when the viewport/origin cannot meet this GPU float profile.
    pub fn try_view_projection_relative_to(
        &self,
        origin: [f64; 3],
        aspect: f32,
    ) -> Result<[f32; 16], crate::SceneError> {
        self.gpu_frame(aspect)?
            .project_origin(origin, false)
            .map(|(matrix, _)| matrix)
    }

    pub(crate) fn gpu_frame(&self, aspect: f32) -> Result<CameraFrame, crate::SceneError> {
        self.validate()?;
        if !aspect.is_finite() || aspect <= 0.0 {
            return Err(crate::SceneError::InvalidCamera(
                "aspect must be finite and positive",
            ));
        }
        let projection = self.projection_at_aspect(aspect);
        if !projection_fits_gpu(&projection) {
            return Err(crate::SceneError::UnsupportedPrecision(
                "viewport projection exceeds the GPU float profile",
            ));
        }
        let basis = [self.right(), self.up(), self.forward()];
        if !basis.iter().all(|axis| normal_coefficients(axis)) {
            return Err(crate::SceneError::UnsupportedPrecision(
                "camera basis exceeds the GPU float profile",
            ));
        }
        Ok(CameraFrame {
            projection,
            basis,
            offset: self.eye_offset(),
            target: self.target,
        })
    }

    fn projection_at_aspect(&self, aspect: f32) -> [f32; 16] {
        match self.projection {
            Projection::Perspective { fov_y } => perspective(fov_y, aspect, self.near, self.far),
            Projection::Orthographic { half_height } => {
                orthographic(half_height, aspect, self.near, self.far)
            }
        }
    }

    pub(crate) fn needs_reference_projection(&self, aspect: f32) -> bool {
        !projection_fits_gpu(&self.projection_at_aspect(aspect))
    }

    // The render entry point validates the camera and viewport first.
    pub(crate) fn software_primitive_matrix(
        &self,
        origin: [f64; 3],
        aspect: f32,
    ) -> Result<[f64; 16], crate::SceneError> {
        let projection = self.projection_at_aspect(aspect);
        let raw = multiply(projection, self.view_matrix_relative_to(origin));
        if projection_fits_gpu(&projection) && raw.iter().all(|value| value.is_finite()) {
            // Preserve ordinary raster arithmetic exactly, promoting the same
            // f32 coefficients once instead of at every primitive vertex.
            return Ok(raw.map(f64::from));
        }
        let near = f64::from(self.near);
        let far = f64::from(self.far);
        let aspect = f64::from(aspect);
        let projection = match self.projection {
            Projection::Perspective { fov_y } => {
                let focal = 1.0 / (f64::from(fov_y) * 0.5).tan();
                let a = far / (near - far);
                [
                    focal / aspect,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    focal,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    a,
                    -1.0,
                    0.0,
                    0.0,
                    near * a,
                    0.0,
                ]
            }
            Projection::Orthographic { half_height } => {
                let height = f64::from(half_height);
                let a = -1.0 / (far - near);
                [
                    1.0 / (height * aspect),
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0 / height,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    a,
                    0.0,
                    0.0,
                    0.0,
                    near * a,
                    1.0,
                ]
            }
        };
        let offset = self.eye_offset();
        let eye: [f64; 3] =
            std::array::from_fn(|axis| self.target[axis] - origin[axis] + f64::from(offset[axis]));
        let (s, u, f) = (
            self.right().map(f64::from),
            self.up().map(f64::from),
            self.forward().map(f64::from),
        );
        let dot = |direction: [f64; 3]| {
            direction[0] * eye[0] + direction[1] * eye[1] + direction[2] * eye[2]
        };
        let view = [
            s[0],
            u[0],
            -f[0],
            0.0,
            s[1],
            u[1],
            -f[1],
            0.0,
            s[2],
            u[2],
            -f[2],
            0.0,
            -dot(s),
            -dot(u),
            dot(f),
            1.0,
        ];
        let matrix: [f64; 16] = std::array::from_fn(|index| {
            let row = index % 4;
            let column = index / 4;
            (0..4)
                .map(|k| projection[k * 4 + row] * view[column * 4 + k])
                .sum()
        });
        if matrix.iter().any(|value| !value.is_finite()) {
            return Err(crate::SceneError::UnsupportedPrecision(
                "software primitive camera exceeds f64 range",
            ));
        }
        Ok(matrix)
    }

    pub(crate) fn ray_grid_relative_to(
        &self,
        origin: [f64; 3],
        aspect: f32,
    ) -> Result<RayGrid, crate::SceneError> {
        self.validate()?;
        let offset = self.eye_offset();
        let eye: [f64; 3] =
            std::array::from_fn(|axis| self.target[axis] - origin[axis] + f64::from(offset[axis]));
        let forward = self.forward();
        let (right, up) = (self.right(), self.up());
        let extent = |distance: f32| match self.projection {
            Projection::Perspective { fov_y } => {
                f64::from((fov_y * 0.5).tan()) * f64::from(distance)
            }
            Projection::Orthographic { half_height } => f64::from(half_height),
        };
        let plane = |distance: f32| {
            let height = extent(distance);
            [
                std::array::from_fn(|axis| {
                    eye[axis] + f64::from(forward[axis]) * f64::from(distance)
                }),
                right.map(|value| f64::from(value) * height * f64::from(aspect)),
                up.map(|value| f64::from(value) * height),
            ]
        };
        Ok(RayGrid {
            near: plane(self.near),
            far: plane(self.far),
        })
    }
}

pub(crate) struct RayGrid {
    near: [[f64; 3]; 3],
    far: [[f64; 3]; 3],
}

impl RayGrid {
    pub(crate) fn segment(&self, ndc: [f32; 2]) -> [[f64; 3]; 2] {
        [&self.near, &self.far].map(|plane| {
            std::array::from_fn(|axis| {
                plane[0][axis]
                    + f64::from(ndc[0]) * plane[1][axis]
                    + f64::from(ndc[1]) * plane[2][axis]
            })
        })
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

// Projection and trigonometric basis are shared by all actors for one camera.
pub(crate) struct CameraFrame {
    projection: [f32; 16],
    basis: [[f32; 3]; 3],
    offset: [f32; 3],
    target: [f64; 3],
}

fn projection_fits_gpu(projection: &[f32; 16]) -> bool {
    normal_coefficients(projection)
        && [0, 5, 10, 14]
            .into_iter()
            .all(|index| projection[index].is_normal())
}

#[inline]
fn normal_coefficients(values: &[f32]) -> bool {
    // Clearing the sign leaves ordered IEEE magnitudes. A zero exponent with
    // nonzero fraction is subnormal; an all-ones exponent is infinity or NaN.
    // Bitwise boolean reduction permits vectorization of this small hot check.
    const MIN_NORMAL_BITS: u32 = f32::MIN_POSITIVE.to_bits();
    const INFINITY_BITS: u32 = f32::INFINITY.to_bits();
    values.iter().fold(0u32, |invalid, value| {
        let magnitude = value.to_bits() & 0x7fff_ffff;
        let nonzero = u32::from(magnitude != 0);
        let outside =
            u32::from(magnitude < MIN_NORMAL_BITS) | u32::from(magnitude >= INFINITY_BITS);
        invalid | (nonzero & outside)
    }) == 0
}

fn local_distance(eye: [f64; 3]) -> f64 {
    // Accepted local views have f32-range translations, so their f64 squares
    // cannot overflow. Scale only very small vectors to avoid double underflow.
    const SMALL_LENGTH: f64 = 1e-150;
    let scale = eye.into_iter().map(f64::abs).fold(0.0, f64::max);
    if scale == 0.0 {
        return 0.0;
    }
    if scale < SMALL_LENGTH {
        let scaled = eye.map(|value| value / scale);
        return scale
            * (scaled[0] * scaled[0] + scaled[1] * scaled[1] + scaled[2] * scaled[2]).sqrt();
    }
    (eye[0] * eye[0] + eye[1] * eye[1] + eye[2] * eye[2]).sqrt()
}

fn view_from_eye(s: [f32; 3], u: [f32; 3], f: [f32; 3], eye: [f64; 3]) -> [f32; 16] {
    let dot = |direction: [f32; 3]| {
        (f64::from(direction[0]) * eye[0]
            + f64::from(direction[1]) * eye[1]
            + f64::from(direction[2]) * eye[2]) as f32
    };
    [
        s[0],
        u[0],
        -f[0],
        0.0,
        s[1],
        u[1],
        -f[1],
        0.0,
        s[2],
        u[2],
        -f[2],
        0.0,
        -dot(s),
        -dot(u),
        dot(f),
        1.0,
    ]
}

impl CameraFrame {
    pub(crate) fn project_origin(
        &self,
        origin: [f64; 3],
        sort_transparent: bool,
    ) -> Result<([f32; 16], f64), crate::SceneError> {
        let eye: [f64; 3] = std::array::from_fn(|axis| {
            self.target[axis] - origin[axis] + f64::from(self.offset[axis])
        });
        let [s, u, f] = self.basis;
        let view = view_from_eye(s, u, f, eye);
        let matrix = multiply(self.projection, view);
        if !normal_coefficients(&matrix) {
            return Err(crate::SceneError::UnsupportedPrecision(
                "actor-relative camera matrix exceeds the GPU float profile",
            ));
        }
        // World-space eye addition can lose the orbit offset at large targets.
        // Only translucent actor ordering needs this length; opaque actors skip it.
        let distance = if sort_transparent {
            local_distance(eye)
        } else {
            0.0
        };
        Ok((matrix, distance))
    }
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

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coefficient_reduction_checks_ieee_boundaries_and_local_length_keeps_small_values() {
        for sign in [0u32, 0x8000_0000] {
            for exponent in 0..256 {
                for fraction in [0, 1, 0x7f_ffff] {
                    let value = f32::from_bits(sign | (exponent << 23) | fraction);
                    let expected = value == 0.0 || value.is_normal();
                    for index in [0, 7, 15] {
                        let mut values = [1.0; 16];
                        values[index] = value;
                        assert_eq!(
                            normal_coefficients(&values),
                            expected,
                            "bits {:08x}",
                            value.to_bits()
                        );
                    }
                }
            }
        }
        assert_eq!(local_distance([0.0; 3]), 0.0);
        assert!((local_distance([3.0, 4.0, 0.0]) - 5.0).abs() < 1e-15);
        for length in [1e-250, 1e38] {
            assert!((local_distance([length, 0.0, 0.0]) / length - 1.0).abs() < 1e-15);
        }
    }

    #[test]
    fn checked_gpu_frames_preserve_normal_matrices_and_local_orbit_distance() {
        for target in [[0.0; 3], [1e12 + 0.125, -1e12, 1e12]] {
            for aspect in [0.25, 1.0, 2.0] {
                for azimuth in [0.0, 0.7, 2.0] {
                    let camera = Camera {
                        target,
                        azimuth,
                        ..Default::default()
                    };
                    let frame = camera.gpu_frame(aspect).unwrap();
                    for origin in [target, target.map(|v| v - 2.0)] {
                        let (matrix, _) = frame.project_origin(origin, false).unwrap();
                        assert_eq!(
                            matrix.map(f32::to_bits),
                            camera
                                .view_projection_relative_to(origin, aspect)
                                .map(f32::to_bits)
                        );
                    }
                }
            }
        }
        let camera = Camera {
            target: [1e300; 3],
            ..Default::default()
        };
        let (_, distance) = camera
            .gpu_frame(1.0)
            .unwrap()
            .project_origin(camera.target, true)
            .unwrap();
        assert!(
            (distance - f64::from(camera.distance)).abs() < 1e-6,
            "world eye addition must not lose the local orbit offset"
        );
        assert_eq!(camera.eye(), camera.target); // The old world-space sort distance was zero.
        for aspect in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert!(matches!(
                camera.try_view_projection_relative_to(camera.target, aspect),
                Err(crate::SceneError::InvalidCamera(_))
            ));
        }
        assert!(matches!(
            camera.try_view_projection_relative_to([0.0; 3], 1.0),
            Err(crate::SceneError::UnsupportedPrecision(_))
        ));
    }

    #[test]
    fn checked_viewport_uses_actual_aspect_and_rejects_collapsed_or_subnormal_coefficients() {
        let camera = Camera {
            azimuth: 0.0,
            elevation: 0.0,
            ..Default::default()
        };
        let matrix = camera
            .try_view_projection_relative_to([0.0; 3], 1e-7)
            .unwrap();
        assert_eq!(matrix[0], camera.projection_matrix(1.0)[0] / 1e-7);
        assert_ne!(
            matrix[0],
            camera.view_projection_relative_to([0.0; 3], 1e-7)[0]
        );
        let tiny_basis = Camera {
            azimuth: 1e-40,
            ..camera
        };
        tiny_basis.validate().unwrap();
        assert!(matches!(
            tiny_basis.try_view_projection_relative_to([0.0; 3], 1.0),
            Err(crate::SceneError::UnsupportedPrecision(_))
        ));
        let shallow_bias = Camera {
            distance: 0.0,
            near: 1e-38,
            far: 1e36,
            projection: Projection::Orthographic { half_height: 1.0 },
            ..camera
        };
        shallow_bias.validate().unwrap();
        assert_eq!(shallow_bias.projection_matrix(1.0)[14], 0.0);
        assert!(matches!(
            shallow_bias.try_view_projection_relative_to([0.0; 3], 1.0),
            Err(crate::SceneError::UnsupportedPrecision(_))
        ));
        let wide = Camera {
            projection: Projection::Orthographic { half_height: 1e36 },
            ..camera
        };
        wide.validate().unwrap();
        assert!(matches!(
            wide.try_view_projection_relative_to([0.0; 3], 4096.0),
            Err(crate::SceneError::UnsupportedPrecision(_))
        ));
    }

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
    fn fitted_camera_contains_all_corners_in_square_and_portrait_viewports() {
        let bounds = gpui_mesh::Bounds {
            min: [-1.0; 3],
            max: [1.0; 3],
        };
        for aspect in [0.25, 1.0, 2.0] {
            let camera = Camera::fitted_for_aspect(bounds, aspect).unwrap();
            for corner in 0..8 {
                let point = std::array::from_fn(|axis| {
                    if corner & (1 << axis) == 0 {
                        bounds.min[axis] as f32
                    } else {
                        bounds.max[axis] as f32
                    }
                });
                let clip = transform(&camera.view_projection(aspect), point);
                assert!(
                    clip[0].abs() < clip[3]
                        && clip[1].abs() < clip[3]
                        && clip[2] > 0.0
                        && clip[2] < clip[3],
                    "aspect {aspect}, corner {corner}"
                );
            }
        }
        Camera::fitted_for_aspect(bounds, 0.0).unwrap_err();
        Camera::fitted_for_aspect(
            gpui_mesh::Bounds {
                min: [1.0; 3],
                max: [-1.0; 3],
            },
            1.0,
        )
        .unwrap_err();
    }

    #[test]
    fn orbiting_keeps_distance() {
        let mut camera = Camera::default();
        let before = camera.eye();
        camera.orbit(0.5, 0.0);
        let after = camera.eye();
        let dist = |p: [f64; 3]| (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
        assert!((dist(before) - dist(after)).abs() < 1e-5);
        assert_ne!(before, after);
    }

    #[test]
    fn rebased_projection_preserves_camera_offsets_at_large_origins() {
        let reference = Camera::default().view_projection(1.0);
        for center in [1000.125, 1_000_000_000.125, 1_000_000_000_000.125] {
            let camera = Camera {
                target: [center; 3],
                ..Camera::default()
            };
            assert_eq!(
                camera.view_projection_relative_to([center; 3], 1.0),
                reference
            );
        }
    }

    #[test]
    fn fitted_target_and_panning_preserve_fractional_world_offsets() {
        let base = Camera::fitted(gpui_mesh::Bounds {
            min: [-1.0, -0.5, -2.0],
            max: [1.5, 1.0, 1.0],
        });
        for offset in [1_000_000_000.125, 1_000_000_000_000.125] {
            let mut shifted = Camera::fitted(gpui_mesh::Bounds {
                min: [-1.0, -0.5, -2.0].map(|v| v + offset),
                max: [1.5, 1.0, 1.0].map(|v| v + offset),
            });
            assert_eq!(shifted.target.map(|v| v - offset), base.target);
            assert_eq!(shifted.right(), base.right());
            assert_eq!(shifted.up(), base.up());
            let before = shifted.target;
            shifted.pan(1.0, -0.5);
            let mut reference = base;
            reference.pan(1.0, -0.5);
            for (axis, before_axis) in before.into_iter().enumerate() {
                assert!(
                    ((shifted.target[axis] - before_axis)
                        - (reference.target[axis] - base.target[axis]))
                        .abs()
                        < 0.0002
                );
            }
        }
    }

    #[test]
    fn analytic_rays_match_both_projection_matrices_and_rebase_exactly() {
        for projection in [
            Projection::Perspective { fov_y: 1.0 },
            Projection::Orthographic { half_height: 2.0 },
        ] {
            let camera = Camera {
                near: 0.2,
                far: 20.0,
                projection,
                ..Camera::default()
            };
            let grid = camera.ray_grid_relative_to([0.0; 3], 1.5).unwrap();
            for ndc in [[0.0, 0.0], [0.7, -0.5], [-0.4, 0.8]] {
                for (index, point) in grid.segment(ndc).iter().enumerate() {
                    let clip = transform(&camera.view_projection(1.5), point.map(|v| v as f32));
                    assert!((clip[0] / clip[3] - ndc[0]).abs() < 0.0001);
                    assert!((clip[1] / clip[3] - ndc[1]).abs() < 0.0001);
                    assert!((clip[2] / clip[3] - index as f32).abs() < 0.0001);
                }
                let offset = [1e12 + 0.125; 3];
                let shifted = Camera {
                    target: offset,
                    ..camera
                };
                assert_eq!(
                    grid.segment(ndc),
                    shifted
                        .ray_grid_relative_to(offset, 1.5)
                        .unwrap()
                        .segment(ndc)
                );
            }
        }
    }

    #[test]
    fn invalid_frustums_fail_and_narrow_valid_frustums_can_dolly() {
        for projection in [
            Projection::Perspective { fov_y: 0.0 },
            Projection::Perspective {
                fov_y: f32::from_bits(1),
            },
            Projection::Perspective {
                fov_y: std::f32::consts::PI,
            },
            Projection::Orthographic { half_height: -1.0 },
            Projection::Orthographic {
                half_height: f32::from_bits(1),
            },
        ] {
            assert!(
                Camera {
                    projection,
                    ..Camera::default()
                }
                .validate()
                .is_err()
            );
        }
        let mut camera = Camera {
            near: 1.0,
            far: 1.5,
            ..Camera::default()
        };
        camera.dolly(0.5);
        camera.validate().unwrap();
        assert!((1.0..=1.5).contains(&camera.distance));
        let before = camera;
        camera.dolly(f32::NAN);
        assert_eq!(before, camera);
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
        let along_right = moved[0] * f64::from(right[0])
            + moved[1] * f64::from(right[1])
            + moved[2] * f64::from(right[2]);
        assert!(along_right < 0.0, "dragging right moves the world right");
    }
}
