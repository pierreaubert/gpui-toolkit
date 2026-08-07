use super::colormap_spec::ColormapSpec;
use super::material_spec::MaterialSpec;
use super::point3::Point3;
use super::scalar_range::ScalarRange;
use super::types::SceneFingerprints;
use super::validate::validate_id;
use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshSpec {
    pub id: String,
    pub vertices: Vec<Point3>,
    pub indices: Vec<u32>,
    #[serde(default)]
    pub material: MaterialSpec,
    pub scalar_field: Option<MeshScalarField>,
}

/// Scalar values associated with a mesh. Values live in the material dirty
/// domain: changing a result field recolors retained geometry without treating
/// it as an entirely new mesh upload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeshScalarField {
    pub values: Vec<f64>,
    #[serde(default)]
    pub association: ScalarAssociation,
    #[serde(default)]
    pub colormap: ColormapSpec,
    pub range: Option<ScalarRange>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalarAssociation {
    #[default]
    Vertex,
    Cell,
}

impl MeshSpec {
    pub fn validate(&self) -> Result<(), Scene3DError> {
        validate_id(&self.id, "mesh.id")?;
        if self.vertices.is_empty() {
            return Err(Scene3DError::EmptyData {
                field: "mesh.vertices",
            });
        }
        if self.indices.is_empty() {
            return Err(Scene3DError::EmptyData {
                field: "mesh.indices",
            });
        }
        if !self.indices.len().is_multiple_of(3) {
            return Err(Scene3DError::InvalidData {
                field: "mesh.indices",
                reason: "triangle indices must be a multiple of 3",
            });
        }
        for vertex in &self.vertices {
            vertex.validate("mesh.vertices")?;
        }
        for (position, index) in self.indices.iter().copied().enumerate() {
            if index as usize >= self.vertices.len() {
                return Err(Scene3DError::InvalidMeshIndex {
                    position,
                    index,
                    vertex_count: self.vertices.len(),
                });
            }
        }
        self.material.validate()?;
        if let Some(field) = &self.scalar_field {
            let expected = match field.association {
                ScalarAssociation::Vertex => self.vertices.len(),
                ScalarAssociation::Cell => self.indices.len() / 3,
            };
            if field.values.len() != expected {
                return Err(Scene3DError::InvalidData {
                    field: "mesh.scalar_field.values",
                    reason: "scalar value count does not match mesh association",
                });
            }
            if field.values.iter().any(|value| !value.is_finite()) {
                return Err(Scene3DError::InvalidData {
                    field: "mesh.scalar_field.values",
                    reason: "scalar values contain NaN or Infinity",
                });
            }
            if let Some(range) = field.range {
                range.validate("mesh.scalar_field.range")?;
            }
        }
        Ok(())
    }

    pub(crate) fn fingerprints(&self) -> SceneFingerprints {
        let mut geometry = DefaultHasher::new();
        self.id.hash(&mut geometry);
        for vertex in &self.vertices {
            vertex.hash_into(&mut geometry);
        }
        self.indices.hash(&mut geometry);

        let mut material = DefaultHasher::new();
        self.material.hash_into(&mut material);
        if let Some(field) = &self.scalar_field {
            field.association.hash(&mut material);
            field.colormap.hash(&mut material);
            for value in &field.values {
                value.to_bits().hash(&mut material);
            }
            if let Some(range) = field.range {
                range.hash_into(&mut material);
            }
            field.label.hash(&mut material);
        }

        SceneFingerprints {
            geometry: geometry.finish(),
            material: material.finish(),
            camera: 0,
        }
    }
}
