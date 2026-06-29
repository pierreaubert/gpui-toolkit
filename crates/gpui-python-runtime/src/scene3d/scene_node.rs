use super::light_spec::LightSpec;
use super::lines_spec::LinesSpec;
use super::mesh_spec::MeshSpec;
use super::surface_spec::SurfaceSpec;
use super::types::SceneFingerprints;
use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SceneNode {
    Surface(SurfaceSpec),
    Lines(LinesSpec),
    Mesh(MeshSpec),
    Light(LightSpec),
}

impl SceneNode {
    pub fn id(&self) -> &str {
        match self {
            Self::Surface(spec) => &spec.id,
            Self::Lines(spec) => &spec.id,
            Self::Mesh(spec) => &spec.id,
            Self::Light(spec) => &spec.id,
        }
    }

    pub fn validate(&self) -> Result<(), Scene3DError> {
        match self {
            Self::Surface(spec) => spec.validate(),
            Self::Lines(spec) => spec.validate(),
            Self::Mesh(spec) => spec.validate(),
            Self::Light(spec) => spec.validate(),
        }
    }

    pub(crate) fn fingerprints(&self) -> SceneFingerprints {
        match self {
            Self::Surface(spec) => spec.fingerprints(),
            Self::Lines(spec) => spec.fingerprints(),
            Self::Mesh(spec) => spec.fingerprints(),
            Self::Light(spec) => spec.fingerprints(),
        }
    }
}
