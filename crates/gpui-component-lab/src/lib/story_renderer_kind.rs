use serde::{Deserialize, Serialize};

/// Renderer category used by the component lab preview surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryRendererKind {
    Component,
    Showcase,
    Chart,
    Audio,
}

impl StoryRendererKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Component => "Component",
            Self::Showcase => "Showcase",
            Self::Chart => "PX Chart",
            Self::Audio => "Audio",
        }
    }
}
