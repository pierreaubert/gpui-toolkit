//! Versor — unit quaternion for 3D sphere rotation.
//!
//! Port of <https://github.com/Fil/versor> to Rust.
//! A versor (unit quaternion) represents a rotation on the sphere
//! without gimbal lock, and supports smooth interpolation (SLERP).

use std::f64::consts::PI;

const RADIANS: f64 = PI / 180.0;
const DEGREES: f64 = 180.0 / PI;

/// Unit quaternion [w, x, y, z] representing a 3D rotation.
///
/// Convention: `[a, b, c, d]` where `a` is the scalar (real) part
/// and `[b, c, d]` is the vector (imaginary) part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Versor {
    pub w: f64, // a (scalar)
    pub x: f64, // b (i)
    pub y: f64, // c (j)
    pub z: f64, // d (k)
}

impl Default for Versor {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Versor {
    /// Identity quaternion (no rotation).
    pub const IDENTITY: Self = Self {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    /// Create from components [w, x, y, z].
    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Self { w, x, y, z }
    }

    /// Create from array [w, x, y, z].
    pub fn from_array([w, x, y, z]: [f64; 4]) -> Self {
        Self { w, x, y, z }
    }

    /// Convert to array [w, x, y, z].
    pub fn to_array(self) -> [f64; 4] {
        [self.w, self.x, self.y, self.z]
    }

    /// Create unit quaternion from Euler rotation angles [λ, φ, γ] in degrees.
    ///
    /// Matches `versor.fromAngles([l, p, g])` from the JS library.
    pub fn from_angles(lambda_deg: f64, phi_deg: f64, gamma_deg: f64) -> Self {
        let l = lambda_deg * RADIANS / 2.0;
        let p = phi_deg * RADIANS / 2.0;
        let g = gamma_deg * RADIANS / 2.0;
        let (sl, cl) = l.sin_cos();
        let (sp, cp) = p.sin_cos();
        let (sg, cg) = g.sin_cos();
        Self {
            w: cl * cp * cg + sl * sp * sg,
            x: sl * cp * cg - cl * sp * sg,
            y: cl * sp * cg + sl * cp * sg,
            z: cl * cp * sg - sl * sp * cg,
        }
    }

    /// Convert quaternion to Euler rotation angles [λ, φ, γ] in degrees.
    ///
    /// Matches `versor.rotation(q)` from the JS library.
    pub fn to_angles(self) -> (f64, f64, f64) {
        let Self {
            w: a,
            x: b,
            y: c,
            z: d,
        } = self;
        let lambda = (2.0 * (a * b + c * d)).atan2(1.0 - 2.0 * (b * b + c * c)) * DEGREES;
        let phi = (2.0 * (a * c - d * b)).clamp(-1.0, 1.0).asin() * DEGREES;
        let gamma = (2.0 * (a * d + b * c)).atan2(1.0 - 2.0 * (c * c + d * d)) * DEGREES;
        (lambda, phi, gamma)
    }

    /// Create from Cartesian coordinates [x, y, z] on the unit sphere.
    ///
    /// Matches `versor.fromCartesian([x, y, z])`.
    pub fn from_cartesian(x: f64, y: f64, z: f64) -> Self {
        Self {
            w: 0.0,
            x: z,
            y: -y,
            z: x,
        }
    }

    /// Convert spherical coordinates [λ°, φ°] to Cartesian [x, y, z].
    ///
    /// Matches `versor.cartesian([lon, lat])`.
    pub fn spherical_to_cartesian(lon_deg: f64, lat_deg: f64) -> [f64; 3] {
        let l = lon_deg * RADIANS;
        let p = lat_deg * RADIANS;
        let cp = p.cos();
        [cp * l.cos(), cp * l.sin(), p.sin()]
    }

    /// Quaternion multiplication (Hamilton product): self * other.
    ///
    /// Matches `versor.multiply(q0, q1)`.
    pub fn multiply(self, other: Self) -> Self {
        let Self {
            w: a1,
            x: b1,
            y: c1,
            z: d1,
        } = self;
        let Self {
            w: a2,
            x: b2,
            y: c2,
            z: d2,
        } = other;
        Self {
            w: a1 * a2 - b1 * b2 - c1 * c2 - d1 * d2,
            x: a1 * b2 + b1 * a2 + c1 * d2 - d1 * c2,
            y: a1 * c2 - b1 * d2 + c1 * a2 + d1 * b2,
            z: a1 * d2 + b1 * c2 - c1 * b2 + d1 * a2,
        }
    }

    /// Dot product of two quaternions.
    pub fn dot(self, other: Self) -> f64 {
        self.w * other.w + self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Quaternion norm (length).
    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    /// Normalize to unit quaternion.
    pub fn normalize(self) -> Self {
        let n = self.norm();
        if n < 1e-15 {
            return Self::IDENTITY;
        }
        Self {
            w: self.w / n,
            x: self.x / n,
            y: self.y / n,
            z: self.z / n,
        }
    }

    /// Conjugate (inverse for unit quaternions).
    pub fn conjugate(self) -> Self {
        Self {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }

    /// Compute the quaternion that rotates between two Cartesian points on the sphere.
    ///
    /// Matches `versor.delta(v0, v1)`.
    pub fn delta(v0: [f64; 3], v1: [f64; 3]) -> Self {
        Self::delta_alpha(v0, v1, 1.0)
    }

    /// Compute the quaternion that rotates between two Cartesian points,
    /// scaled by alpha ∈ [0, 1] for tweening.
    ///
    /// Matches `versor.delta(v0, v1, alpha)`.
    pub fn delta_alpha(v0: [f64; 3], v1: [f64; 3], alpha: f64) -> Self {
        // Cross product
        let w = [
            v0[1] * v1[2] - v0[2] * v1[1],
            v0[2] * v1[0] - v0[0] * v1[2],
            v0[0] * v1[1] - v0[1] * v1[0],
        ];
        let l = (w[0] * w[0] + w[1] * w[1] + w[2] * w[2]).sqrt();
        if l < 1e-15 {
            return Self::IDENTITY;
        }
        // Dot product
        let d = (v0[0] * v1[0] + v0[1] * v1[1] + v0[2] * v1[2]).clamp(-1.0, 1.0);
        let t = alpha * d.acos() / 2.0; // θ/2
        let s = t.sin();
        Self {
            w: t.cos(),
            x: w[2] / l * s,
            y: -w[1] / l * s,
            z: w[0] / l * s,
        }
    }

    /// SLERP interpolation between two quaternions.
    ///
    /// Matches `versor.interpolate(q0, q1)(t)`.
    pub fn slerp(self, other: Self, t: f64) -> Self {
        let mut other = other;
        let mut dot = self.dot(other);

        // Ensure shortest path
        if dot < 0.0 {
            other = Self::new(-other.w, -other.x, -other.y, -other.z);
            dot = -dot;
        }

        // Linear interpolation for very close quaternions
        if dot > 0.9995 {
            return Self::new(
                self.w + t * (other.w - self.w),
                self.x + t * (other.x - self.x),
                self.y + t * (other.y - self.y),
                self.z + t * (other.z - self.z),
            )
            .normalize();
        }

        let theta0 = dot.clamp(-1.0, 1.0).acos();
        let theta = theta0 * t;
        let sin_theta = theta.sin();
        let sin_theta0 = theta0.sin();

        let s0 = (theta0 - theta).sin() / sin_theta0;
        let s1 = sin_theta / sin_theta0;

        Self::new(
            self.w * s0 + other.w * s1,
            self.x * s0 + other.x * s1,
            self.y * s0 + other.y * s1,
            self.z * s0 + other.z * s1,
        )
    }

    /// Rotate a point on the sphere using this quaternion.
    ///
    /// Uses the versor convention: Cartesian [x, y, z] maps to quaternion [0, z, -y, x].
    /// This matches D3's versor library.
    ///
    /// Input: (lambda, phi) in radians.
    /// Output: (rotated_lambda, rotated_phi) in radians.
    pub fn rotate_spherical(self, lambda: f64, phi: f64) -> (f64, f64) {
        // Convert spherical to Cartesian
        let cos_phi = phi.cos();
        let cx = cos_phi * lambda.cos();
        let cy = cos_phi * lambda.sin();
        let cz = phi.sin();

        // Map Cartesian to quaternion using versor convention: [0, z, -y, x]
        let pq = Self::new(0.0, cz, -cy, cx);

        // Rotate: q * p * q⁻¹
        let rotated = self.multiply(pq).multiply(self.conjugate());

        // Map quaternion back to Cartesian: x = rotated.z, y = -rotated.y, z = rotated.x
        let rx = rotated.z;
        let ry = -rotated.y;
        let rz = rotated.x;

        // Convert Cartesian back to spherical
        let out_lambda = ry.atan2(rx);
        let out_phi = rz.clamp(-1.0, 1.0).asin();
        (out_lambda, out_phi)
    }

    /// Rotate a point using Euler angles [λ, φ, γ] in degrees.
    ///
    /// Input: (lon, lat) in degrees.
    /// Output: (rotated_lon, rotated_lat) in degrees.
    pub fn rotate_degrees(rotation_angles: (f64, f64, f64), lon: f64, lat: f64) -> (f64, f64) {
        let q = Self::from_angles(rotation_angles.0, rotation_angles.1, rotation_angles.2);
        let (rl, rp) = q.rotate_spherical(lon * RADIANS, lat * RADIANS);
        (rl * DEGREES, rp * DEGREES)
    }
}

impl std::ops::Mul for Versor {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        self.multiply(rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 1e-10;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < TOL
    }

    #[test]
    fn test_identity() {
        let q = Versor::IDENTITY;
        assert!(approx(q.norm(), 1.0));
        let (l, p, g) = q.to_angles();
        assert!(approx(l, 0.0) && approx(p, 0.0) && approx(g, 0.0));
    }

    #[test]
    fn test_from_angles_roundtrip() {
        let angles = [(30.0, 45.0, 0.0), (0.0, -90.0, 0.0), (120.0, -30.0, 15.0)];
        for (l, p, g) in angles {
            let q = Versor::from_angles(l, p, g);
            assert!(approx(q.norm(), 1.0), "not unit: norm={}", q.norm());
            let (rl, rp, rg) = q.to_angles();
            let roundtrip = Versor::from_angles(rl, rp, rg);
            assert!(
                approx(q.dot(roundtrip).abs(), 1.0),
                "roundtrip ({l},{p},{g}) -> ({rl},{rp},{rg}) changed rotation"
            );
        }
    }

    #[test]
    fn test_multiply_identity() {
        let q = Versor::from_angles(45.0, -30.0, 10.0);
        let r = q * Versor::IDENTITY;
        assert!(approx(r.w, q.w) && approx(r.x, q.x));
    }

    #[test]
    fn test_rotate_spherical_identity() {
        let q = Versor::IDENTITY;
        let (rl, rp) = q.rotate_spherical(0.5, 0.3);
        assert!(approx(rl, 0.5) && approx(rp, 0.3));
    }

    #[test]
    fn test_delta() {
        let v0 = Versor::spherical_to_cartesian(0.0, 0.0);
        let v1 = Versor::spherical_to_cartesian(90.0, 0.0);
        let q = Versor::delta(v0, v1);
        assert!(approx(q.norm(), 1.0));
        // Applying this rotation to v0 should give v1
        let (rl, rp) = q.rotate_spherical(0.0, 0.0);
        assert!(
            approx(rl * DEGREES, 90.0) && approx(rp * DEGREES, 0.0),
            "delta rotation: got ({}, {})",
            rl * DEGREES,
            rp * DEGREES
        );
    }

    #[test]
    fn test_slerp() {
        let q0 = Versor::from_angles(0.0, 0.0, 0.0);
        let q1 = Versor::from_angles(90.0, 0.0, 0.0);
        let mid = q0.slerp(q1, 0.5);
        let (l, p, g) = mid.to_angles();
        assert!(approx(l, 45.0), "slerp midpoint: lambda={l}");
        assert!(approx(p, 0.0));
        assert!(approx(g, 0.0));
    }
}
