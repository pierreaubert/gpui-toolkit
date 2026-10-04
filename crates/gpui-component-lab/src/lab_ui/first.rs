use crate::ComponentStory;

pub(super) fn first_viewport_id(story: &ComponentStory) -> String {
    story
        .viewports
        .first()
        .map_or_else(|| "desktop".to_string(), |viewport| viewport.id.to_string())
}

pub(super) fn first_theme_id(story: &ComponentStory) -> String {
    story
        .themes
        .first()
        .map_or_else(|| "neutral".to_string(), |theme| theme.id.to_string())
}

pub(super) fn first_motion_id(story: &ComponentStory) -> String {
    story
        .motions
        .first()
        .map_or_else(|| "system".to_string(), |motion| motion.id.to_string())
}
