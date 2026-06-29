use super::material_spec::MaterialSpec;
use super::point3::Point3;
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
        self.material.validate()
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

        SceneFingerprints {
            geometry: geometry.finish(),
            material: material.finish(),
            camera: 0,
        }
    }
}
