/// A single point on a 3D surface with a color-mapping value
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePoint3D {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Z coordinate (height)
    pub z: f64,
    /// Value for color mapping (can be different from z)
    pub t: f64,
}

impl SurfacePoint3D {
    /// Create a new surface point
    pub fn new(x: f64, y: f64, z: f64, t: f64) -> Self {
        Self { x, y, z, t }
    }

    /// Create a point where t equals z
    pub fn from_xyz(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, t: z }
    }
}
