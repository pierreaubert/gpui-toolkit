use super::first::first_motion_id;
use super::first::first_theme_id;
use super::first::first_viewport_id;
use super::misc::layout_string;
use super::preview_layout_constraints::PreviewLayoutConstraints;
use crate::StoryDocument;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct InitialLabState {
    pub(super) viewport_id: String,
    pub(super) theme_id: String,
    pub(super) motion_id: String,
    pub(super) matrix_mode: bool,
    pub(super) layout_constraints: PreviewLayoutConstraints,
}

impl InitialLabState {
    pub(super) fn from_document(document: &StoryDocument) -> Self {
        let story = &document.story;
        Self {
            viewport_id: layout_string(&document.layout, "viewport")
                .filter(|id| {
                    story
                        .viewports
                        .iter()
                        .any(|viewport| viewport.id.as_str() == id.as_str())
                })
                .unwrap_or_else(|| first_viewport_id(story)),
            theme_id: layout_string(&document.layout, "theme")
                .filter(|id| {
                    story
                        .themes
                        .iter()
                        .any(|theme| theme.id.as_str() == id.as_str())
                })
                .unwrap_or_else(|| first_theme_id(story)),
            motion_id: layout_string(&document.layout, "motion")
                .filter(|id| {
                    story
                        .motions
                        .iter()
                        .any(|motion| motion.id.as_str() == id.as_str())
                })
                .unwrap_or_else(|| first_motion_id(story)),
            matrix_mode: document
                .layout
                .get("matrix")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            layout_constraints: PreviewLayoutConstraints::from_layout(&document.layout),
        }
    }
}
