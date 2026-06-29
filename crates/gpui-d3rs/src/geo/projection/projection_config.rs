use super::super::{degrees, radians};
use super::sphere_rotation::SphereRotation;

/// Base configuration shared by all projections.
#[derive(Clone, Debug)]
pub struct ProjectionConfig {
    /// Scale factor
    pub scale: f64,
    /// Translation offset (x, y)
    pub translate: (f64, f64),
    /// Center coordinates (longitude, latitude)
    pub center: (f64, f64),
    /// Rotation angles (lambda, phi, gamma)
    pub rotate: (f64, f64, f64),
    /// Clip angle (for azimuthal projections)
    pub clip_angle: Option<f64>,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            scale: 150.0,
            translate: (480.0, 250.0),
            center: (0.0, 0.0),
            rotate: (0.0, 0.0, 0.0),
            clip_angle: None,
        }
    }
}

/// Build a SphereRotation from a ProjectionConfig's rotate fields.
///
/// This matches D3's internal `projection.rotate([λ, φ, γ])` rotation, which is
/// `geoRotation([λ, φ, γ])`. The projection center is therefore the point that
/// this rotation maps to the origin.
pub(super) fn build_rotation(config: &ProjectionConfig) -> SphereRotation {
    SphereRotation::from_degrees(config.rotate.0, config.rotate.1, config.rotate.2)
}

/// Apply the configured rotation to input (lon, lat) in degrees.
///
/// This matches D3's pre-projection rotation stage. The projection center is
/// handled as a post-projection planar offset (see individual projections).
/// Returns (lambda, phi) in radians, ready for project_raw.
pub(super) fn apply_rotation(config: &ProjectionConfig, lon: f64, lat: f64) -> (f64, f64) {
    let rotation = build_rotation(config);
    rotation.rotate(radians(lon), radians(lat))
}

/// Invert the configured rotation from (lambda, phi) in radians
/// back to (lon, lat) in degrees.
pub(super) fn invert_rotation(config: &ProjectionConfig, lambda: f64, phi: f64) -> (f64, f64) {
    let rotation = build_rotation(config);
    let (rl, rp) = rotation.invert(lambda, phi);
    (degrees(rl), degrees(rp))
}

/// Check visibility against clip_angle after applying rotation.
///
/// Returns `true` when the point's angular distance from the projection
/// center is within `clip_angle`, or when no clip_angle is set.
pub(super) fn clip_angle_visible(config: &ProjectionConfig, lon: f64, lat: f64) -> bool {
    match config.clip_angle {
        None => true,
        Some(clip_deg) => {
            let rotation = build_rotation(config);
            let (lambda, phi) = rotation.rotate(radians(lon), radians(lat));
            let cos_clip = clip_deg.to_radians().cos();
            let cos_dist = phi.cos() * lambda.cos();
            cos_dist >= cos_clip
        }
    }
}
