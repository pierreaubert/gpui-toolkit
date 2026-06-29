use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionMode {
    Orbit,
    Pan,
    Zoom,
    Reset,
    HitTest,
}

impl InteractionMode {
    pub fn parse(value: &str) -> Result<Self, Scene3DError> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "orbit" => Ok(Self::Orbit),
            "pan" => Ok(Self::Pan),
            "zoom" => Ok(Self::Zoom),
            "reset" => Ok(Self::Reset),
            "hit_test" | "hittest" => Ok(Self::HitTest),
            _ => Err(Scene3DError::UnknownInteraction {
                name: value.to_string(),
            }),
        }
    }
}
