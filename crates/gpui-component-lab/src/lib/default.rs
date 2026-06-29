use super::builtin::builtin_story_renderer;
use super::motion_preset::MotionPreset;
use super::rendered_preview_conformance::RenderedPreviewConformance;
use super::story_metadata_item::StoryMetadataItem;
use super::theme_preset::ThemePreset;
use super::viewport_preset::ViewportPreset;

pub(super) fn default_true() -> bool {
    true
}

pub(super) fn default_preview_min_width() -> f32 {
    RenderedPreviewConformance::default().min_width
}

pub(super) fn default_preview_min_height() -> f32 {
    RenderedPreviewConformance::default().min_height
}

pub(super) fn default_story_metadata(story_id: &str, crate_name: &str) -> Vec<StoryMetadataItem> {
    let renderer = builtin_story_renderer(story_id)
        .map(|renderer| renderer.label)
        .unwrap_or_else(|| "Metadata-only".into());
    vec![
        StoryMetadataItem::new("crate", "Crate", crate_name),
        StoryMetadataItem::new("story", "Story", story_id),
        StoryMetadataItem::new("renderer", "Renderer", renderer),
    ]
}

pub fn default_viewports() -> Vec<ViewportPreset> {
    vec![
        ViewportPreset::new("mobile", "Mobile", 390.0, 844.0),
        ViewportPreset::new("tablet", "Tablet", 834.0, 1112.0),
        ViewportPreset::new("desktop", "Desktop", 1280.0, 800.0),
        ViewportPreset::new("wide", "Wide", 1728.0, 1117.0),
    ]
}

pub fn default_theme_presets() -> Vec<ThemePreset> {
    vec![
        ThemePreset::new("neutral", "Neutral", "neutral", false),
        ThemePreset::new("apple-hig", "Apple HIG", "apple_hig", false),
        ThemePreset::new("material3", "Material 3", "material3", false),
        ThemePreset::new("fluent", "Fluent", "fluent", false),
        ThemePreset::new("reduced-motion", "Reduced Motion", "neutral", true),
    ]
}

pub fn default_motion_presets() -> Vec<MotionPreset> {
    vec![
        MotionPreset::new("system", "System", false),
        MotionPreset::new("reduced", "Reduced", true),
    ]
}
