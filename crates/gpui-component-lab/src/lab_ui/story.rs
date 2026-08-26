use crate::{ComponentStory, StoryPropValue};
use gpui::SharedString;

pub(super) fn story_file_name(story_id: &str) -> String {
    let mut name = String::with_capacity(story_id.len().saturating_mul(3) + ".story.json".len());
    for byte in story_id.bytes() {
        if byte.is_ascii_lowercase() || byte.is_ascii_digit() {
            name.push(byte as char);
        } else {
            use std::fmt::Write as _;
            write!(&mut name, "~{byte:02x}").expect("write into String cannot fail");
        }
    }
    name.push_str(".story.json");
    name
}

/// Legacy lossy name used before story filenames escaped their raw ids.
/// Existing files are migrated on the next save when their JSON confirms the
/// same story id.
pub(super) fn legacy_story_file_name(story_id: &str) -> String {
    let mut name = String::with_capacity(story_id.len() + ".story.json".len());
    for ch in story_id.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch);
        } else {
            name.push('_');
        }
    }
    name.push_str(".story.json");
    name
}

pub(super) fn story_prop<'a>(story: &'a ComponentStory, name: &str) -> Option<&'a StoryPropValue> {
    story
        .props
        .iter()
        .find(|prop| prop.name == name)
        .map(|prop| &prop.value)
}

pub(super) fn text_prop(story: &ComponentStory, name: &str, fallback: &str) -> SharedString {
    match story_prop(story, name) {
        Some(StoryPropValue::Text(value)) | Some(StoryPropValue::Color(value)) => value.clone(),
        Some(StoryPropValue::Choice(value)) => value.clone(),
        _ => SharedString::new(fallback),
    }
}

pub(super) fn choice_prop(story: &ComponentStory, name: &str, fallback: &str) -> SharedString {
    match story_prop(story, name) {
        Some(StoryPropValue::Choice(value)) => value.clone(),
        _ => SharedString::new(fallback),
    }
}

pub(super) fn bool_prop(story: &ComponentStory, name: &str, fallback: bool) -> bool {
    match story_prop(story, name) {
        Some(StoryPropValue::Bool(value)) => *value,
        _ => fallback,
    }
}
