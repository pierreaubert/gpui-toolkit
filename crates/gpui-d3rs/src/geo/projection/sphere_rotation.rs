use super::super::degrees;
use super::super::versor::Versor;

/// 3D sphere rotation using unit quaternions (versors).
///
/// Replaces the previous Euler-angle rotation with quaternion math,
/// eliminating gimbal lock and matching `d3.geoRotation([λ, φ, γ])` exactly.
///
/// Kept as a thin wrapper for backward compatibility of the public API
/// (`SphereRotation::from_degrees`, `.rotate()`, `.invert()`).
#[derive(Clone, Debug)]
pub struct SphereRotation {
    pub(super) forward: Versor,
    pub(super) inverse: Versor,
}

impl SphereRotation {
    /// Create a rotation from angles in degrees.
    /// Matches D3's `rotateRadians(deltaLambda, deltaPhi, deltaGamma)`.
    pub fn from_degrees(lambda_deg: f64, phi_deg: f64, gamma_deg: f64) -> Self {
        let forward = Versor::from_angles(lambda_deg, phi_deg, gamma_deg);
        Self {
            inverse: forward.conjugate(),
            forward,
        }
    }

    /// Create a rotation from angles in radians.
    pub fn from_radians(delta_lambda: f64, delta_phi: f64, delta_gamma: f64) -> Self {
        Self::from_degrees(
            degrees(delta_lambda),
            degrees(delta_phi),
            degrees(delta_gamma),
        )
    }

    /// Identity rotation (no-op).
    pub fn identity() -> Self {
        Self {
            forward: Versor::IDENTITY,
            inverse: Versor::IDENTITY,
        }
    }

    /// Apply the forward rotation to spherical coordinates (in radians).
    ///
    /// Input: (lambda, phi) in radians
    /// Output: (rotated_lambda, rotated_phi) in radians
    pub fn rotate(&self, lambda: f64, phi: f64) -> (f64, f64) {
        self.forward.rotate_spherical(lambda, phi)
    }

    /// Apply the inverse rotation to spherical coordinates (in radians).
    pub fn invert(&self, lambda: f64, phi: f64) -> (f64, f64) {
        self.inverse.rotate_spherical(lambda, phi)
    }
}
