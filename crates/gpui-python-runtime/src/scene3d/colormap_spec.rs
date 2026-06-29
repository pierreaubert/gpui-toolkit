use crate::error::Scene3DError;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColormapSpec {
    #[default]
    Viridis,
    Plasma,
    Inferno,
    Turbo,
    CoolWarm,
}

impl ColormapSpec {
    pub fn parse(value: &str) -> Result<Self, Scene3DError> {
        match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "viridis" => Ok(Self::Viridis),
            "plasma" => Ok(Self::Plasma),
            "inferno" => Ok(Self::Inferno),
            "turbo" => Ok(Self::Turbo),
            "coolwarm" | "cool_warm" => Ok(Self::CoolWarm),
            _ => Err(Scene3DError::UnknownColormap {
                name: value.to_string(),
            }),
        }
    }
}
