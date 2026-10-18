//! Point and cell field data.

// Rust guideline compliant 2026-02-21

use std::sync::Arc;

/// Whether a field is stored per point or per cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Association {
    /// One value per position.
    Point,
    /// One value per cell.
    Cell,
}

/// Storage for one named field.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldData {
    /// One real scalar per entry.
    Scalar(Arc<[f64]>),
    /// One 3-vector per entry (velocity, displacement, force).
    Vector(Arc<[[f64; 3]]>),
    /// One symmetric 6-component tensor per entry, ordered xx, yy, zz, xy, yz, xz.
    Tensor(Arc<[[f64; 6]]>),
    /// One complex scalar per entry (acoustic / frequency-domain pressure).
    Complex(Arc<[[f64; 2]]>),
}

impl FieldData {
    /// Number of entries (points or cells), independent of component count.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Scalar(v) => v.len(),
            Self::Vector(v) => v.len(),
            Self::Tensor(v) => v.len(),
            Self::Complex(v) => v.len(),
        }
    }

    /// Whether the field has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Components per entry: 1, 3, 6, or 2.
    #[must_use]
    pub fn components(&self) -> usize {
        match self {
            Self::Scalar(_) => 1,
            Self::Vector(_) => 3,
            Self::Tensor(_) => 6,
            Self::Complex(_) => 2,
        }
    }

    /// Whether an existing entry has only finite components.
    #[inline]
    #[must_use]
    pub fn entry_is_finite(&self, index: usize) -> bool {
        match self {
            Self::Scalar(values) => values.get(index).is_some_and(|v| v.is_finite()),
            Self::Vector(values) => values
                .get(index)
                .is_some_and(|v| v.iter().all(|c| c.is_finite())),
            Self::Tensor(values) => values
                .get(index)
                .is_some_and(|v| v.iter().all(|c| c.is_finite())),
            Self::Complex(values) => values
                .get(index)
                .is_some_and(|v| v.iter().all(|c| c.is_finite())),
        }
    }

    /// Checked byte size of the backing storage, without allocation.
    #[must_use]
    pub fn byte_len(&self) -> u64 {
        let n = self.len() as u64;
        match self {
            Self::Scalar(_) => n * 8,
            Self::Vector(_) => n * 24,
            Self::Tensor(_) => n * 48,
            Self::Complex(_) => n * 16,
        }
    }
}

/// A named field attached to points or cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Stable identifier, unique within its association on a dataset.
    pub name: Arc<str>,
    /// Human-readable label for legends and colorbars.
    pub label: Arc<str>,
    /// Physical unit, when known (for example `Pa` or `m/s`).
    pub unit: Option<Arc<str>>,
    /// Point or cell association.
    pub association: Association,
    /// Backing values.
    pub data: FieldData,
    /// Explicit validity mask; `false` entries are excluded everywhere.
    /// Masked entries may be NaN. Dataset validation rejects infinities.
    pub valid: Option<Arc<[bool]>>,
}

impl Field {
    /// Whether an entry exists, is unmasked and has only finite components.
    ///
    /// Missing mask entries fail closed; dataset validation separately reports
    /// malformed lengths. Raw data views do not apply this predicate.
    #[inline]
    #[must_use]
    pub fn is_valid(&self, index: usize) -> bool {
        self.valid
            .as_ref()
            .is_none_or(|mask| mask.get(index) == Some(&true))
            && self.data.entry_is_finite(index)
    }

    /// Creates a scalar point field.
    pub fn scalar(name: impl Into<Arc<str>>, values: impl Into<Arc<[f64]>>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            unit: None,
            association: Association::Point,
            data: FieldData::Scalar(values.into()),
            valid: None,
        }
    }

    /// Creates a vector point field.
    pub fn vector(name: impl Into<Arc<str>>, values: impl Into<Arc<[[f64; 3]]>>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            unit: None,
            association: Association::Point,
            data: FieldData::Vector(values.into()),
            valid: None,
        }
    }

    /// Creates a symmetric-tensor point field (xx, yy, zz, xy, yz, xz).
    pub fn tensor(name: impl Into<Arc<str>>, values: impl Into<Arc<[[f64; 6]]>>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            unit: None,
            association: Association::Point,
            data: FieldData::Tensor(values.into()),
            valid: None,
        }
    }

    /// Creates a complex point field (re, im per entry).
    pub fn complex(name: impl Into<Arc<str>>, values: impl Into<Arc<[[f64; 2]]>>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            unit: None,
            association: Association::Point,
            data: FieldData::Complex(values.into()),
            valid: None,
        }
    }

    /// Sets the display label and unit.
    #[must_use]
    pub fn labelled(mut self, label: impl Into<Arc<str>>, unit: impl Into<Arc<str>>) -> Self {
        self.label = label.into();
        self.unit = Some(unit.into());
        self
    }

    /// Moves this field to cell association.
    #[must_use]
    pub fn on_cells(mut self) -> Self {
        self.association = Association::Cell;
        self
    }

    /// Scalar view, when this field is scalar.
    #[must_use]
    pub fn as_scalar(&self) -> Option<&[f64]> {
        match &self.data {
            FieldData::Scalar(v) => Some(v),
            _ => None,
        }
    }

    /// Vector view, when this field is a 3-vector field.
    #[must_use]
    pub fn as_vector(&self) -> Option<&[[f64; 3]]> {
        match &self.data {
            FieldData::Vector(v) => Some(v),
            _ => None,
        }
    }

    /// Tensor view, when this field is a symmetric-tensor field.
    #[must_use]
    pub fn as_tensor(&self) -> Option<&[[f64; 6]]> {
        match &self.data {
            FieldData::Tensor(v) => Some(v),
            _ => None,
        }
    }

    /// Complex view, when this field is a complex field.
    #[must_use]
    pub fn as_complex(&self) -> Option<&[[f64; 2]]> {
        match &self.data {
            FieldData::Complex(v) => Some(v),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validity_requires_existing_unmasked_finite_components_for_every_shape() {
        let data = [
            FieldData::Scalar(vec![1.0, 99.0, f64::NAN, f64::INFINITY].into()),
            FieldData::Vector(vec![[1.0; 3], [99.0; 3], [f64::NAN; 3], [f64::INFINITY; 3]].into()),
            FieldData::Tensor(vec![[1.0; 6], [99.0; 6], [f64::NAN; 6], [f64::INFINITY; 6]].into()),
            FieldData::Complex(vec![[1.0; 2], [99.0; 2], [f64::NAN; 2], [f64::INFINITY; 2]].into()),
        ];
        for data in data {
            let mut field = Field::scalar("test", Vec::<f64>::new());
            field.data = data;
            field.valid = Some(vec![true, false, true, true].into());
            assert!(field.is_valid(0));
            assert!((1..=4).all(|index| !field.is_valid(index)));
            field.valid = Some(vec![true].into());
            assert!(!field.is_valid(1), "short masks fail closed");
        }
    }
}
