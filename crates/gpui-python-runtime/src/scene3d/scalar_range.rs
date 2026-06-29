use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ScalarRange {
    pub min: f64,
    pub max: f64,
}

impl ScalarRange {
    #[must_use]
    pub const fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    pub(crate) fn validate(&self, field: &'static str) -> Result<(), Scene3DError> {
        if !self.min.is_finite() || !self.max.is_finite() {
            return Err(Scene3DError::InvalidData {
                field,
                reason: "range contains NaN or Infinity",
            });
        }
        if !matches!(
            self.min.partial_cmp(&self.max),
            Some(std::cmp::Ordering::Less)
        ) {
            return Err(Scene3DError::InvalidData {
                field,
                reason: "range min must be less than max",
            });
        }
        Ok(())
    }

    pub(crate) fn validate_positive(&self, field: &'static str) -> Result<(), Scene3DError> {
        self.validate(field)?;
        if self.min <= 0.0 || self.max <= 0.0 {
            return Err(Scene3DError::InvalidData {
                field,
                reason: "log range must be strictly positive",
            });
        }
        Ok(())
    }

    pub(crate) fn hash_into(&self, h: &mut impl Hasher) {
        self.min.to_bits().hash(h);
        self.max.to_bits().hash(h);
    }
}
