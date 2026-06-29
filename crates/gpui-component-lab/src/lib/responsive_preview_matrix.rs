use super::component_story::ComponentStory;
use super::types::ResponsivePreviewCell;
use serde::{Deserialize, Serialize};

/// Matrix of story previews across viewport/theme combinations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponsivePreviewMatrix {
    pub cells: Vec<ResponsivePreviewCell>,
}

impl ResponsivePreviewMatrix {
    pub fn for_story(story: &ComponentStory) -> Self {
        let mut cells = Vec::new();
        for viewport in &story.viewports {
            for theme in &story.themes {
                cells.push(ResponsivePreviewCell {
                    story_id: story.id.clone(),
                    viewport: viewport.clone(),
                    theme: theme.clone(),
                });
            }
        }
        Self { cells }
    }
}
