//! Tag component tests

use gpui_ui_kit::tag::{Tag, TagSize, TagVariant};

#[test]
fn test_tag_creation() {
    let tag = Tag::new("tag-1", "FLAC");
    drop(tag);
}

#[test]
fn test_tag_all_variants() {
    for variant in [
        TagVariant::Default,
        TagVariant::Primary,
        TagVariant::Success,
        TagVariant::Warning,
        TagVariant::Error,
        TagVariant::Outlined,
    ] {
        let tag = Tag::new("tag-v", "Label").variant(variant);
        drop(tag);
    }
}

#[test]
fn test_tag_all_sizes() {
    for size in [TagSize::Sm, TagSize::Md, TagSize::Lg] {
        let tag = Tag::new("tag-s", "Label").size(size);
        drop(tag);
    }
}

#[test]
fn test_tag_icon() {
    let tag = Tag::new("tag-icon", "Audio").icon("🎵");
    drop(tag);
}

#[test]
fn test_tag_removable() {
    let tag = Tag::new("tag-rm", "Tag").removable(true);
    drop(tag);
}

#[test]
fn test_tag_on_click() {
    let tag = Tag::new("tag-click", "Clickable").on_click(|_window, _cx| {});
    drop(tag);
}

#[test]
fn test_tag_on_remove() {
    let tag = Tag::new("tag-remove", "Removable")
        .removable(true)
        .on_remove(|_window, _cx| {});
    drop(tag);
}

#[test]
fn test_tag_full_configuration() {
    let tag = Tag::new("tag-full", "FLAC")
        .variant(TagVariant::Success)
        .size(TagSize::Lg)
        .icon("🎵")
        .removable(true)
        .on_click(|_window, _cx| {})
        .on_remove(|_window, _cx| {});
    drop(tag);
}
