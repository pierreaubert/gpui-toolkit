//! Canonical triangle-mesh data model and geometry validation.
//!
//! All positions are `f64` at every API boundary. Validation is pure — it
//! inspects `&self` and returns structured errors, never panics and never
//! mutates retained state.

use std::sync::Arc;

/// Canonical indexed-triangle mesh. Deliberately independent of FEM/BEM,
/// ndarray, NumPy, and num_complex (spec §5).
#[derive(Debug, Clone, PartialEq)]
pub struct TriangleMesh {
    pub id: Arc<str>,
    pub positions: Arc<[[f64; 3]]>,
    /// Zero-based vertex indices; every index must be in range.
    pub triangles: Arc<[[u32; 3]]>,
    pub vertex_ids: Option<Arc<[u64]>>,
    pub cell_ids: Option<Arc<[u64]>>,
}

/// Real scalar field over mesh vertices or cells (spec §5.2).
#[derive(Debug, Clone)]
pub struct ScalarField {
    pub id: Arc<str>,
    pub label: Arc<str>,
    pub unit: Option<Arc<str>>,
    pub values: Arc<[f64]>,
    pub association: ScalarAssociation,
    /// Explicit validity mask; `false` entries are excluded from rendering,
    /// contours, and inspection. NaN is accepted only where masked.
    pub valid: Option<Arc<[bool]>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarAssociation {
    Vertex,
    Cell,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MeshValidationError {
    #[error("positions array is empty")]
    EmptyPositions,
    #[error("triangles array is empty")]
    EmptyTriangles,
    #[error("position {index} is not finite")]
    NonFinitePosition { index: usize },
    #[error("triangle {triangle} slot {slot} has index {index} out of range (len {len})")]
    IndexOutOfRange {
        triangle: usize,
        slot: usize,
        index: u32,
        len: usize,
    },
    #[error("triangle {triangle} has a repeated vertex index")]
    RepeatedIndex { triangle: usize },
    #[error("triangle {triangle} has zero area")]
    ZeroAreaTriangle { triangle: usize },
    #[error("vertex_ids length {ids} != positions length {positions}")]
    VertexIdLengthMismatch { ids: usize, positions: usize },
    #[error("cell_ids length {ids} != triangles length {triangles}")]
    CellIdLengthMismatch {
        ids: usize,
        triangles: usize,
    },
    #[error("field '{field_id}' has {values} values, expected {expected} for {association:?}")]
    FieldLengthMismatch {
        field_id: String,
        values: usize,
        expected: usize,
        association: ScalarAssociation,
    },
    #[error("validity mask length {mask} != values length {values}")]
    MaskLengthMismatch { mask: usize, values: usize },
    #[error("value {index} is not finite and not masked (infinities are never drawable)")]
    NonFiniteValue { index: usize },
    #[error("axisymmetric radius at vertex {index} below tolerance: {value}")]
    InvalidRadius { index: usize, value: f64 },
    #[error("contours require a vertex-associated field")]
    ContoursRequireVertexField,
    #[error("contour levels must be finite, strictly increasing, and unique")]
    InvalidContourLevels,
    #[error("revolve sweep must be in (0, 2π] with segments >= 3")]
    InvalidRevolveSpec,
}

impl TriangleMesh {
    /// Full validation; call before mutating any retained state (spec §11).
    pub fn validate(&self) -> Result<(), MeshValidationError> {
        if self.positions.is_empty() {
            return Err(MeshValidationError::EmptyPositions);
        }
        if self.triangles.is_empty() {
            return Err(MeshValidationError::EmptyTriangles);
        }
        for (index, p) in self.positions.iter().enumerate() {
            if !p.iter().all(|c| c.is_finite()) {
                return Err(MeshValidationError::NonFinitePosition { index });
            }
        }
        let len = self.positions.len();
        for (t, tri) in self.triangles.iter().enumerate() {
            for (slot, &idx) in tri.iter().enumerate() {
                if idx as usize >= len {
                    return Err(MeshValidationError::IndexOutOfRange {
                        triangle: t,
                        slot,
                        index: idx,
                        len,
                    });
                }
            }
            if tri[0] == tri[1] || tri[1] == tri[2] || tri[0] == tri[2] {
                return Err(MeshValidationError::RepeatedIndex { triangle: t });
            }
            if triangle_area2(
                self.positions[tri[0] as usize],
                self.positions[tri[1] as usize],
                self.positions[tri[2] as usize],
            ) <= ZERO_AREA_EPS
            {
                return Err(MeshValidationError::ZeroAreaTriangle { triangle: t });
            }
        }
        if let Some(ids) = &self.vertex_ids
            && ids.len() != self.positions.len()
        {
            return Err(MeshValidationError::VertexIdLengthMismatch {
                ids: ids.len(),
                positions: self.positions.len(),
            });
        }
        if let Some(ids) = &self.cell_ids
            && ids.len() != self.triangles.len()
        {
            return Err(MeshValidationError::CellIdLengthMismatch {
                ids: ids.len(),
                triangles: self.triangles.len(),
            });
        }
        Ok(())
    }
}

/// Twice the signed area via cross-product magnitude, dimensionless guard.
const ZERO_AREA_EPS: f64 = 1e-30;

fn triangle_area2(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    n[0] * n[0] + n[1] * n[1] + n[2] * n[2]
}

impl ScalarField {
    /// Validate this field against a mesh: length must match the association
    /// target, the mask (if any) must match the values length, and every
    /// unmasked value must be finite. Masked entries may be NaN; infinities
    /// are never drawable (spec §5.2).
    pub fn validate(&self, mesh: &TriangleMesh) -> Result<(), MeshValidationError> {
        let expected = match self.association {
            ScalarAssociation::Vertex => mesh.positions.len(),
            ScalarAssociation::Cell => mesh.triangles.len(),
        };
        if self.values.len() != expected {
            return Err(MeshValidationError::FieldLengthMismatch {
                field_id: self.id.to_string(),
                values: self.values.len(),
                expected,
                association: self.association,
            });
        }
        let mask = self.valid.as_deref();
        if let Some(m) = mask
            && m.len() != self.values.len()
        {
            return Err(MeshValidationError::MaskLengthMismatch {
                mask: m.len(),
                values: self.values.len(),
            });
        }
        for (index, v) in self.values.iter().enumerate() {
            // No mask means every entry is valid; mask length == values length
            // was checked above, so direct indexing is safe.
            let ok = mask.is_none_or(|m| m[index]);
            // Masked entries may be NaN; infinities are never drawable (spec §5.2).
            if (ok && !v.is_finite()) || (!ok && v.is_infinite()) {
                return Err(MeshValidationError::NonFiniteValue { index });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(positions: &[[f64; 3]], triangles: &[[u32; 3]]) -> TriangleMesh {
        TriangleMesh {
            id: "m".into(),
            positions: positions.to_vec().into(),
            triangles: triangles.to_vec().into(),
            vertex_ids: None,
            cell_ids: None,
        }
    }

    #[test]
    fn valid_single_triangle_passes() {
        let m = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        assert!(m.validate().is_ok());
    }

    #[test]
    fn empty_positions_rejected() {
        let m = mesh(&[], &[[0, 1, 2]]);
        assert_eq!(m.validate(), Err(MeshValidationError::EmptyPositions));
    }

    #[test]
    fn empty_triangles_rejected() {
        let m = mesh(&[[0.0, 0.0, 0.0]], &[]);
        assert_eq!(m.validate(), Err(MeshValidationError::EmptyTriangles));
    }

    #[test]
    fn non_finite_position_rejected() {
        let m = mesh(
            &[[0.0, 0.0, 0.0], [f64::NAN, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        assert_eq!(
            m.validate(),
            Err(MeshValidationError::NonFinitePosition { index: 1 })
        );
    }

    #[test]
    fn index_out_of_range_rejected() {
        let m = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 3]],
        );
        assert_eq!(
            m.validate(),
            Err(MeshValidationError::IndexOutOfRange {
                triangle: 0,
                slot: 2,
                index: 3,
                len: 3
            })
        );
    }

    #[test]
    fn repeated_index_rejected() {
        let m = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 1]],
        );
        assert_eq!(
            m.validate(),
            Err(MeshValidationError::RepeatedIndex { triangle: 0 })
        );
    }

    #[test]
    fn zero_area_rejected() {
        // collinear vertices
        let m = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            &[[0, 1, 2]],
        );
        assert_eq!(
            m.validate(),
            Err(MeshValidationError::ZeroAreaTriangle { triangle: 0 })
        );
    }

    #[test]
    fn vertex_field_length_must_match_positions() {
        let m = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        let f = ScalarField {
            id: "f".into(),
            label: "f".into(),
            unit: None,
            values: vec![1.0, 2.0].into(),
            association: ScalarAssociation::Vertex,
            valid: None,
        };
        assert_eq!(
            f.validate(&m),
            Err(MeshValidationError::FieldLengthMismatch {
                field_id: "f".to_string(),
                values: 2,
                expected: 3,
                association: ScalarAssociation::Vertex,
            })
        );
    }

    #[test]
    fn cell_field_validates_against_triangle_count() {
        let m = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        let f = ScalarField {
            id: "f".into(),
            label: "f".into(),
            unit: None,
            values: vec![7.0].into(),
            association: ScalarAssociation::Cell,
            valid: None,
        };
        assert!(f.validate(&m).is_ok());
    }

    #[test]
    fn nan_rejected_unless_masked() {
        let m = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        let f = ScalarField {
            id: "f".into(),
            label: "f".into(),
            unit: None,
            values: vec![1.0, f64::NAN, 3.0].into(),
            association: ScalarAssociation::Vertex,
            valid: Some(vec![true, false, true].into()),
        };
        assert!(f.validate(&m).is_ok());
        let f_unmasked = ScalarField {
            valid: None,
            ..f.clone()
        };
        assert_eq!(
            f_unmasked.validate(&m),
            Err(MeshValidationError::NonFiniteValue { index: 1 })
        );
    }

    #[test]
    fn infinity_never_accepted() {
        let m = mesh(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[[0, 1, 2]],
        );
        let f = ScalarField {
            id: "f".into(),
            label: "f".into(),
            unit: None,
            values: vec![1.0, f64::INFINITY, 3.0].into(),
            association: ScalarAssociation::Vertex,
            valid: Some(vec![true, false, true].into()),
        };
        assert_eq!(
            f.validate(&m),
            Err(MeshValidationError::NonFiniteValue { index: 1 })
        );
    }
}
