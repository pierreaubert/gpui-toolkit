use super::default::default_true;
use super::rendered_preview_conformance::RenderedPreviewConformance;
use serde::{Deserialize, Serialize};

/// Machine-readable conformance expectations for a story.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoryConformance {
    #[serde(default = "default_true")]
    pub responsive: bool,
    #[serde(default)]
    pub rendered: RenderedPreviewConformance,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_touch_target: Option<f32>,
    #[serde(default)]
    pub focusable_count: usize,
    #[serde(default)]
    pub touch_target_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus_labels: Vec<String>,
    #[serde(default = "default_true")]
    pub builder_layout: bool,
}

impl StoryConformance {
    pub fn display(min_width: f32, min_height: f32) -> Self {
        Self {
            responsive: true,
            rendered: RenderedPreviewConformance::new(min_width, min_height),
            min_touch_target: None,
            focusable_count: 0,
            touch_target_count: 0,
            focus_labels: Vec::new(),
            builder_layout: true,
        }
    }

    pub fn interactive(
        min_width: f32,
        min_height: f32,
        min_touch_target: f32,
        focus_labels: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        let focus_labels = focus_labels.into_iter().map(Into::into).collect::<Vec<_>>();
        Self {
            responsive: true,
            rendered: RenderedPreviewConformance::new(min_width, min_height),
            min_touch_target: Some(min_touch_target),
            focusable_count: focus_labels.len().max(1),
            touch_target_count: focus_labels.len().max(1),
            focus_labels,
            builder_layout: true,
        }
    }

    pub fn scrollable_showcase(focus_labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut conformance = Self::interactive(320.0, 220.0, 48.0, focus_labels);
        conformance.rendered = conformance.rendered.scrollable(true);
        conformance
    }

    pub fn px_chart() -> Self {
        Self::display(220.0, 145.0)
    }

    pub fn for_story(story_id: &str, crate_name: &str) -> Self {
        match crate_name {
            "gpui-px" => Self::px_chart(),
            "gpui-audio-kit" => match story_id {
                "audio-kit.potentiometer"
                | "audio-kit.vertical-slider"
                | "audio-kit.volume-knob" => {
                    Self::interactive(140.0, 170.0, 48.0, ["Audio control"])
                }
                "audio-kit.horizontal-meter" => Self::display(320.0, 120.0),
                "audio-kit.spectrum-axis" => Self::display(360.0, 180.0),
                _ => Self::display(140.0, 170.0),
            },
            "gpui-ui-kit" => match story_id {
                "ui-kit.button" => Self::interactive(180.0, 120.0, 48.0, ["Primary action"]),
                "ui-kit.form" => Self::interactive(320.0, 260.0, 48.0, ["Form input"]),
                "ui-kit.status" => Self::display(320.0, 160.0),
                "ui-kit.navigation" => Self::interactive(320.0, 140.0, 48.0, ["Tabs"]),
                "ui-kit.feedback" => Self::display(320.0, 120.0),
                "ui-kit.card" => Self::display(320.0, 180.0),
                "ui-kit.showcase-component" => Self::scrollable_showcase(["Showcase"]),
                _ => Self::scrollable_showcase([story_id.replace("ui-kit.", "")]),
            },
            _ => Self::default(),
        }
    }
}

impl Default for StoryConformance {
    fn default() -> Self {
        Self::display(160.0, 120.0)
    }
}
