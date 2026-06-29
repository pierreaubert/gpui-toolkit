use super::point3::Point3;
use super::validate::validate_positive_f32;
use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerspectiveCameraSpec {
    pub position: Point3,
    pub target: Point3,
    pub up: Point3,
    pub fov_y_deg: f32,
    pub near: f32,
    pub far: f32,
}

impl Default for PerspectiveCameraSpec {
    fn default() -> Self {
        Self {
            position: Point3::new(2.0, 2.0, 2.0),
            target: Point3::ZERO,
            up: Point3::new(0.0, 1.0, 0.0),
            fov_y_deg: 45.0,
            near: 0.1,
            far: 100.0,
        }
    }
}

impl PerspectiveCameraSpec {
    pub(super) fn validate(&self) -> Result<(), Scene3DError> {
        self.position.validate("camera.position")?;
        self.target.validate("camera.target")?;
        self.up.validate("camera.up")?;
        validate_positive_f32(self.fov_y_deg, "camera.fov_y_deg")?;
        validate_positive_f32(self.near, "camera.near")?;
        validate_positive_f32(self.far, "camera.far")?;
        if self.near >= self.far {
            return Err(Scene3DError::InvalidData {
                field: "camera.far",
                reason: "must be greater than near",
            });
        }
        Ok(())
    }

    pub(super) fn hash_into(&self, h: &mut impl Hasher) {
        self.position.hash_into(h);
        self.target.hash_into(h);
        self.up.hash_into(h);
        self.fov_y_deg.to_bits().hash(h);
        self.near.to_bits().hash(h);
        self.far.to_bits().hash(h);
    }
}
