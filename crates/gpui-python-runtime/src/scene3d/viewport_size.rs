use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewportSize {
    pub width: f32,
    pub height: f32,
}

impl ViewportSize {
    #[must_use]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub(crate) fn validate(&self) -> Result<(), Scene3DError> {
        if !self.width.is_finite() || self.width <= 0.0 {
            return Err(Scene3DError::InvalidData {
                field: "width",
                reason: "must be positive and finite",
            });
        }
        if !self.height.is_finite() || self.height <= 0.0 {
            return Err(Scene3DError::InvalidData {
                field: "height",
                reason: "must be positive and finite",
            });
        }
        Ok(())
    }

    pub(crate) fn hash_into(&self, h: &mut impl Hasher) {
        self.width.to_bits().hash(h);
        self.height.to_bits().hash(h);
    }
}
