use super::default::default_motion_presets;
use super::default::default_story_metadata;
use super::default::default_theme_presets;
use super::default::default_viewports;
use super::motion_preset::MotionPreset;
use super::story_conformance::StoryConformance;
use super::story_metadata_item::StoryMetadataItem;
use super::story_prop::StoryProp;
use super::theme_preset::ThemePreset;
use super::viewport_preset::ViewportPreset;
use serde::{Deserialize, Serialize};

/// Prop-driven component story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentStory {
    pub id: String,
    pub crate_name: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metadata: Vec<StoryMetadataItem>,
    #[serde(default)]
    pub props: Vec<StoryProp>,
    #[serde(default)]
    pub viewports: Vec<ViewportPreset>,
    #[serde(default)]
    pub themes: Vec<ThemePreset>,
    #[serde(default)]
    pub motions: Vec<MotionPreset>,
    #[serde(default)]
    pub conformance: StoryConformance,
}

impl ComponentStory {
    pub fn new(
        id: impl Into<String>,
        crate_name: impl Into<String>,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let id = id.into();
        let crate_name = crate_name.into();
        let conformance = StoryConformance::for_story(&id, &crate_name);
        let metadata = default_story_metadata(&id, &crate_name);
        Self {
            id,
            crate_name,
            title: title.into(),
            description: description.into(),
            metadata,
            props: Vec::new(),
            viewports: default_viewports(),
            themes: default_theme_presets(),
            motions: default_motion_presets(),
            conformance,
        }
    }

    pub fn props(mut self, props: impl IntoIterator<Item = StoryProp>) -> Self {
        self.props = props.into_iter().collect();
        self
    }

    pub fn metadata(mut self, metadata: impl IntoIterator<Item = StoryMetadataItem>) -> Self {
        self.metadata = metadata.into_iter().collect();
        self
    }

    pub fn metadata_item(mut self, item: StoryMetadataItem) -> Self {
        self.metadata.push(item);
        self
    }

    pub fn conformance(mut self, conformance: StoryConformance) -> Self {
        self.conformance = conformance;
        self
    }
}
