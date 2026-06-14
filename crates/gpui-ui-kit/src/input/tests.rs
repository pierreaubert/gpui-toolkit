use super::{EditState, Input};
use gpui::SharedString;

#[test]
fn effective_label_prefers_aria_label_then_label_then_placeholder() {
    let input = Input::new("a").aria_label("aria");
    assert_eq!(input.aria_label, Some(SharedString::from("aria")));
    assert!(input.label.is_none());
    assert!(input.placeholder.is_none());

    let input = Input::new("b").label("label").placeholder("placeholder");
    assert!(input.aria_label.is_none());
    assert_eq!(input.label, Some(SharedString::from("label")));
    assert_eq!(input.placeholder, Some(SharedString::from("placeholder")));

    let input = Input::new("c").placeholder("placeholder");
    assert!(input.aria_label.is_none());
    assert!(input.label.is_none());
    assert_eq!(input.placeholder, Some(SharedString::from("placeholder")));
}

#[test]
fn insert_char_does_not_allocate_string_and_matches_insert_text() {
    let mut state = EditState::new("ab");
    state.clear_selection();
    state.cursor = 1;

    let mut via_char = state.clone();
    via_char.insert_char('X');

    let mut via_text = state.clone();
    via_text.insert_text("X");

    assert_eq!(via_char.text, "aXb");
    assert_eq!(via_text.text, "aXb");
    assert_eq!(via_char.cursor, 2);
    assert_eq!(via_text.cursor, 2);
}
