use super::hash::hash_f64_slice;
use super::validate::validate_finite_f64_slice;
use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridData {
    pub values: Vec<f64>,
    pub width: usize,
    pub height: usize,
}

impl GridData {
    #[must_use]
    pub fn from_flat(values: Vec<f64>, width: usize, height: usize) -> Self {
        Self {
            values,
            width,
            height,
        }
    }

    pub fn from_rows(rows: Vec<Vec<f64>>) -> Result<Self, Scene3DError> {
        if rows.is_empty() {
            return Err(Scene3DError::EmptyData { field: "z" });
        }
        let height = rows.len();
        let width = rows[0].len();
        if width == 0 {
            return Err(Scene3DError::EmptyData { field: "z" });
        }
        let mut values = Vec::new();
        for row in rows {
            if row.len() != width {
                return Err(Scene3DError::GridDimensionMismatch {
                    z_len: values.len() + row.len(),
                    width,
                    height,
                    expected: width.saturating_mul(height),
                });
            }
            values.extend(row);
        }
        Ok(Self {
            values,
            width,
            height,
        })
    }

    pub fn validate(&self) -> Result<(), Scene3DError> {
        if self.width == 0 || self.height == 0 || self.values.is_empty() {
            return Err(Scene3DError::EmptyData { field: "z" });
        }
        let expected = self.width.saturating_mul(self.height);
        if self.values.len() != expected {
            return Err(Scene3DError::GridDimensionMismatch {
                z_len: self.values.len(),
                width: self.width,
                height: self.height,
                expected,
            });
        }
        validate_finite_f64_slice(&self.values, "z")
    }

    /// Return the grid data as a borrowed flat slice together with its dimensions.
    #[must_use]
    pub fn as_flat(&self) -> (&[f64], usize, usize) {
        (&self.values, self.width, self.height)
    }

    /// Iterate over the grid rows as borrowed slices.
    pub fn rows(&self) -> impl Iterator<Item = &[f64]> {
        self.values.chunks(self.width)
    }

    pub(crate) fn hash_into(&self, h: &mut impl Hasher) {
        self.width.hash(h);
        self.height.hash(h);
        hash_f64_slice(&self.values, h);
    }
}
