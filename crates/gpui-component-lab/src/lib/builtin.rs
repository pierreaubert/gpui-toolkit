use super::consts::BUILTIN_RENDERER_STORY_IDS;
use super::consts::ui_kit_showcase_story_id;
use super::register::register_audio_kit_stories;
use super::register::register_px_stories;
use super::register::register_ui_kit_stories;
use super::story_registry::StoryRegistry;
use super::story_renderer::StoryRenderer;
use super::story_renderer_kind::StoryRendererKind;
use super::story_renderer_registry::StoryRendererRegistry;
use anyhow::Result;

pub(super) fn builtin_renderer_kind(story_id: &str) -> StoryRendererKind {
    if story_id.starts_with("px.") {
        StoryRendererKind::Chart
    } else if story_id.starts_with("audio-kit.") {
        StoryRendererKind::Audio
    } else if ui_kit_showcase_story_id(story_id) {
        StoryRendererKind::Showcase
    } else {
        StoryRendererKind::Component
    }
}

pub fn builtin_story_renderer(story_id: &str) -> Option<StoryRenderer> {
    StoryRenderer::for_builtin(story_id)
}

pub fn builtin_story_renderers() -> Result<StoryRendererRegistry> {
    let mut registry = StoryRendererRegistry::new();
    for story_id in BUILTIN_RENDERER_STORY_IDS {
        if let Some(renderer) = builtin_story_renderer(story_id) {
            registry.register(renderer)?;
        }
    }
    Ok(registry)
}

pub fn builtin_story_has_renderer(story_id: &str) -> bool {
    builtin_story_renderer(story_id).is_some()
}

/// Build the default lab registry.
pub fn builtin_story_registry() -> Result<StoryRegistry> {
    let mut registry = StoryRegistry::new();
    register_ui_kit_stories(&mut registry)?;
    register_px_stories(&mut registry)?;
    register_audio_kit_stories(&mut registry)?;
    Ok(registry)
}
