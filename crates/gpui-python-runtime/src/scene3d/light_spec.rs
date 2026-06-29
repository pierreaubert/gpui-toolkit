use super::color_rgba::ColorRgba;
use super::default::default_light_intensity;
use super::point3::Point3;
use super::types::SceneFingerprints;
use super::validate::validate_id;
use super::validate::validate_positive_f32;
use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightSpec {
    pub id: String,
    pub direction: Point3,
    #[serde(default = "default_light_intensity")]
    pub intensity: f32,
    #[serde(default)]
    pub color: ColorRgba,
}

impl LightSpec {
    pub fn validate(&self) -> Result<(), Scene3DError> {
        validate_id(&self.id, "light.id")?;
        self.direction.validate("light.direction")?;
        validate_positive_f32(self.intensity, "light.intensity")?;
        self.color.validate("light.color")
    }

    pub(crate) fn fingerprints(&self) -> SceneFingerprints {
        let mut material = DefaultHasher::new();
        self.id.hash(&mut material);
        self.direction.hash_into(&mut material);
        self.intensity.to_bits().hash(&mut material);
        self.color.hash_into(&mut material);

        SceneFingerprints {
            geometry: 0,
            material: material.finish(),
            camera: 0,
        }
    }
}
