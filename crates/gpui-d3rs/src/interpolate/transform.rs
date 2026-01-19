//! 2D Transform interpolation
//!
//! Provides interpolation for 2D affine transforms using matrix decomposition.

use std::f64::consts::PI;

/// A 2D affine transform represented as a decomposed form.
///
/// This allows for smooth interpolation of transforms by interpolating
/// the individual components (translate, rotate, scale, skew).
#[derive(Debug, Clone, Copy, Default)]
pub struct Transform2D {
    /// Translation in X
    pub translate_x: f64,
    /// Translation in Y
    pub translate_y: f64,
    /// Rotation in radians
    pub rotate: f64,
    /// Scale in X
    pub scale_x: f64,
    /// Scale in Y
    pub scale_y: f64,
    /// Skew in X (radians)
    pub skew_x: f64,
}

impl Transform2D {
    /// Create an identity transform.
    pub fn identity() -> Self {
        Self {
            translate_x: 0.0,
            translate_y: 0.0,
            rotate: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            skew_x: 0.0,
        }
    }

    /// Create a translation transform.
    pub fn translate(x: f64, y: f64) -> Self {
        Self {
            translate_x: x,
            translate_y: y,
            ..Self::identity()
        }
    }

    /// Create a rotation transform (in degrees).
    pub fn rotate_deg(degrees: f64) -> Self {
        Self {
            rotate: degrees * PI / 180.0,
            ..Self::identity()
        }
    }

    /// Create a rotation transform (in radians).
    pub fn rotate_rad(radians: f64) -> Self {
        Self {
            rotate: radians,
            ..Self::identity()
        }
    }

    /// Create a scale transform.
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            scale_x: sx,
            scale_y: sy,
            ..Self::identity()
        }
    }

    /// Create a uniform scale transform.
    pub fn scale_uniform(s: f64) -> Self {
        Self::scale(s, s)
    }

    /// Create a skew transform (in degrees).
    pub fn skew_x_deg(degrees: f64) -> Self {
        Self {
            skew_x: degrees * PI / 180.0,
            ..Self::identity()
        }
    }

    /// Decompose a 2D transformation matrix [a, b, c, d, e, f] into components.
    ///
    /// The matrix is:
    /// ```text
    /// | a c e |
    /// | b d f |
    /// | 0 0 1 |
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// use d3rs::interpolate::Transform2D;
    ///
    /// // Identity matrix
    /// let t = Transform2D::from_matrix(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    /// assert!((t.scale_x - 1.0).abs() < 0.001);
    /// assert!((t.rotate).abs() < 0.001);
    /// ```
    pub fn from_matrix(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        let translate_x = e;
        let translate_y = f;

        // Decompose rotation and scale
        let scale_x = (a * a + b * b).sqrt();
        let scale_y = (c * c + d * d).sqrt();

        // Determine sign of scale_x from determinant
        let det = a * d - b * c;
        let scale_x = if det < 0.0 { -scale_x } else { scale_x };

        let rotate = b.atan2(a);

        // Calculate skew
        let skew_x = if scale_x.abs() > 1e-10 && scale_y.abs() > 1e-10 {
            (a * c + b * d) / (scale_x * scale_y)
        } else {
            0.0
        };
        let skew_x = skew_x.asin();

        Self {
            translate_x,
            translate_y,
            rotate,
            scale_x,
            scale_y,
            skew_x,
        }
    }

    /// Convert back to a transformation matrix [a, b, c, d, e, f].
    pub fn to_matrix(&self) -> [f64; 6] {
        let cos_r = self.rotate.cos();
        let sin_r = self.rotate.sin();
        let tan_skew = self.skew_x.tan();

        let a = self.scale_x * cos_r;
        let b = self.scale_x * sin_r;
        let c = self.scale_y * (-sin_r + cos_r * tan_skew);
        let d = self.scale_y * (cos_r + sin_r * tan_skew);

        [a, b, c, d, self.translate_x, self.translate_y]
    }

    /// Transform a point (x, y) using this transform.
    pub fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, e, f] = self.to_matrix();
        (a * x + c * y + e, b * x + d * y + f)
    }

    /// Interpolate between two transforms.
    pub fn interpolate(&self, other: &Transform2D, t: f64) -> Transform2D {
        // Handle rotation wrap-around
        let mut rotate_diff = other.rotate - self.rotate;
        if rotate_diff > PI {
            rotate_diff -= 2.0 * PI;
        } else if rotate_diff < -PI {
            rotate_diff += 2.0 * PI;
        }

        Transform2D {
            translate_x: self.translate_x + (other.translate_x - self.translate_x) * t,
            translate_y: self.translate_y + (other.translate_y - self.translate_y) * t,
            rotate: self.rotate + rotate_diff * t,
            scale_x: self.scale_x + (other.scale_x - self.scale_x) * t,
            scale_y: self.scale_y + (other.scale_y - self.scale_y) * t,
            skew_x: self.skew_x + (other.skew_x - self.skew_x) * t,
        }
    }

    /// Convert to CSS transform string.
    pub fn to_css(&self) -> String {
        let rotate_deg = self.rotate * 180.0 / PI;
        let skew_deg = self.skew_x * 180.0 / PI;

        let mut parts = Vec::new();

        if self.translate_x.abs() > 1e-10 || self.translate_y.abs() > 1e-10 {
            parts.push(format!(
                "translate({:.3}px, {:.3}px)",
                self.translate_x, self.translate_y
            ));
        }
        if rotate_deg.abs() > 1e-10 {
            parts.push(format!("rotate({:.3}deg)", rotate_deg));
        }
        if (self.scale_x - 1.0).abs() > 1e-10 || (self.scale_y - 1.0).abs() > 1e-10 {
            parts.push(format!("scale({:.3}, {:.3})", self.scale_x, self.scale_y));
        }
        if skew_deg.abs() > 1e-10 {
            parts.push(format!("skewX({:.3}deg)", skew_deg));
        }

        if parts.is_empty() {
            "none".to_string()
        } else {
            parts.join(" ")
        }
    }

    /// Convert to SVG transform attribute string.
    pub fn to_svg(&self) -> String {
        let [a, b, c, d, e, f] = self.to_matrix();
        format!(
            "matrix({:.6}, {:.6}, {:.6}, {:.6}, {:.6}, {:.6})",
            a, b, c, d, e, f
        )
    }
}

/// Create an interpolator between two Transform2D values.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::{Transform2D, interpolate_transform};
///
/// let start = Transform2D::translate(0.0, 0.0);
/// let end = Transform2D::translate(100.0, 50.0);
///
/// let interp = interpolate_transform(start, end);
/// let mid = interp(0.5);
///
/// assert!((mid.translate_x - 50.0).abs() < 0.001);
/// assert!((mid.translate_y - 25.0).abs() < 0.001);
/// ```
pub fn interpolate_transform(a: Transform2D, b: Transform2D) -> impl Fn(f64) -> Transform2D {
    move |t| a.interpolate(&b, t)
}

/// Interpolate between two SVG transform matrices.
///
/// Takes matrices as [a, b, c, d, e, f] arrays.
///
/// # Example
///
/// ```
/// use d3rs::interpolate::interpolate_transform_svg;
///
/// let identity = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
/// let translated = [1.0, 0.0, 0.0, 1.0, 100.0, 50.0];
///
/// let interp = interpolate_transform_svg(identity, translated);
/// let mid = interp(0.5);
///
/// assert!((mid[4] - 50.0).abs() < 0.001);
/// assert!((mid[5] - 25.0).abs() < 0.001);
/// ```
pub fn interpolate_transform_svg(a: [f64; 6], b: [f64; 6]) -> impl Fn(f64) -> [f64; 6] {
    let a_t = Transform2D::from_matrix(a[0], a[1], a[2], a[3], a[4], a[5]);
    let b_t = Transform2D::from_matrix(b[0], b[1], b[2], b[3], b[4], b[5]);

    move |t| a_t.interpolate(&b_t, t).to_matrix()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_identity() {
        let t = Transform2D::identity();
        let (x, y) = t.apply(10.0, 20.0);
        assert!((x - 10.0).abs() < 0.001);
        assert!((y - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_translate() {
        let t = Transform2D::translate(5.0, 10.0);
        let (x, y) = t.apply(10.0, 20.0);
        assert!((x - 15.0).abs() < 0.001);
        assert!((y - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_scale() {
        let t = Transform2D::scale(2.0, 3.0);
        let (x, y) = t.apply(10.0, 20.0);
        assert!((x - 20.0).abs() < 0.001);
        assert!((y - 60.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_rotate() {
        let t = Transform2D::rotate_deg(90.0);
        let (x, y) = t.apply(10.0, 0.0);
        assert!(x.abs() < 0.001);
        assert!((y - 10.0).abs() < 0.001);
    }

    #[test]
    fn test_transform_roundtrip() {
        let original = Transform2D {
            translate_x: 10.0,
            translate_y: 20.0,
            rotate: 0.5,
            scale_x: 2.0,
            scale_y: 1.5,
            skew_x: 0.1,
        };

        let matrix = original.to_matrix();
        let recovered = Transform2D::from_matrix(
            matrix[0], matrix[1], matrix[2], matrix[3], matrix[4], matrix[5],
        );

        assert!((original.translate_x - recovered.translate_x).abs() < 0.001);
        assert!((original.translate_y - recovered.translate_y).abs() < 0.001);
        assert!((original.rotate - recovered.rotate).abs() < 0.001);
        assert!((original.scale_x - recovered.scale_x).abs() < 0.01);
        assert!((original.scale_y - recovered.scale_y).abs() < 0.01);
    }

    #[test]
    fn test_interpolate_transform() {
        let start = Transform2D::translate(0.0, 0.0);
        let end = Transform2D::translate(100.0, 50.0);

        let interp = interpolate_transform(start, end);

        let mid = interp(0.5);
        assert!((mid.translate_x - 50.0).abs() < 0.001);
        assert!((mid.translate_y - 25.0).abs() < 0.001);
    }

    #[test]
    fn test_interpolate_rotation() {
        let start = Transform2D::rotate_deg(0.0);
        let end = Transform2D::rotate_deg(90.0);

        let interp = interpolate_transform(start, end);

        let mid = interp(0.5);
        let mid_deg = mid.rotate * 180.0 / PI;
        assert!((mid_deg - 45.0).abs() < 0.001);
    }
}
