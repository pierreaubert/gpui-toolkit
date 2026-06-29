use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Point3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub(crate) fn validate(&self, field: &'static str) -> Result<(), Scene3DError> {
        if !self.x.is_finite() || !self.y.is_finite() || !self.z.is_finite() {
            return Err(Scene3DError::InvalidData {
                field,
                reason: "contains NaN or Infinity",
            });
        }
        Ok(())
    }

    pub(crate) fn hash_into(&self, h: &mut impl Hasher) {
        self.x.to_bits().hash(h);
        self.y.to_bits().hash(h);
        self.z.to_bits().hash(h);
    }
}
