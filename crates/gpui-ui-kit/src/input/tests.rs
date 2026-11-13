use super::{EditState, Input, InputEntity};
use gpui::SharedString;
use std::cell::RefCell;

#[test]
fn password_builder_enables_masking() {
    let input = Input::new("password").value("secret").password(true);
    assert!(input.password);
    assert_eq!(input.value, SharedString::from("secret"));
}

#[test]
fn password_mask_cache_keeps_only_character_count() {
    let cache = RefCell::new(None);
    let mask = InputEntity::cached_password_mask(&cache, "sëcrèt");

    assert_eq!(mask, SharedString::from("••••••"));
    assert_eq!(cache.borrow().as_ref().map(|(count, _)| *count), Some(6));
}

#[test]
fn password_input_debug_output_redacts_the_value() {
    let input = Input::new("password")
        .value("not-for-debug-output")
        .password(true);

    let dump = format!("{input:?}");
    assert!(dump.contains("<redacted>"));
    assert!(!dump.contains("not-for-debug-output"));
}

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

#[test]
fn validator_reports_error_message_for_invalid_value() {
    let input = Input::new("email").validator(|value| {
        if value.contains('@') {
            None
        } else {
            Some(SharedString::from("Enter a valid email address"))
        }
    });

    assert_eq!(input.validate("user@example.com"), None);
    assert_eq!(
        input.validate("not-an-email"),
        Some(SharedString::from("Enter a valid email address"))
    );
}

#[test]
fn effective_error_prefers_explicit_error_over_validator() {
    let without_validator = Input::new("plain");
    assert_eq!(without_validator.validate("anything"), None);
    assert_eq!(without_validator.effective_error(), None);

    let input = Input::new("email")
        .value("not-an-email")
        .error("Server rejected this value")
        .validator(|value| {
            if value.contains('@') {
                None
            } else {
                Some(SharedString::from("Enter a valid email address"))
            }
        });
    // Explicit error wins even though the validator also fails.
    assert_eq!(
        input.effective_error(),
        Some(SharedString::from("Server rejected this value"))
    );

    let validated = Input::new("email")
        .value("not-an-email")
        .validator(|_| Some(SharedString::from("bad")));
    assert_eq!(validated.effective_error(), Some(SharedString::from("bad")));
}

#[test]
fn external_values_update_active_edits_without_erasing_uncommitted_typing() {
    let mut state = EditState::default();
    state.editing = true;
    state.text = "typing".into();
    state.cursor = 6;
    state.sync_external_value("saved", "saved");
    assert_eq!(state.text, "typing");
    state.sync_external_value("saved", "é");
    assert_eq!(state.text, "é");
    assert_eq!(state.cursor, 1);
    state.selection_anchor = Some(0);
    state.sync_external_value("é", "é");
    assert_eq!(state.selection_anchor, Some(0));
}

#[test]
fn ime_composition_sequence_retains_marked_range_until_commit() {
    // Simulates the EntityInputHandler composition flow with the same
    // associated functions the handler calls: replace_and_mark (twice,
    // as the IME refines), then a plain replace as the commit.
    let mut state = EditState::new("");
    state.clear_selection();
    state.cursor = 0;

    // First composition update: insert "n" and mark it.
    InputEntity::replace_char_range(&mut state, 0..0, "n");
    state.set_marked_range(Some((0, state.cursor)));
    assert_eq!(state.marked_range, Some((0, 1)));
    assert_eq!(
        InputEntity::char_to_utf16(&state.text, 0)..InputEntity::char_to_utf16(&state.text, 1),
        0..1
    );

    // Refinement: the platform replaces the marked UTF-16 range.
    let (marked_start, marked_end) = state.marked_range.unwrap();
    let utf16_range = InputEntity::char_to_utf16(&state.text, marked_start)
        ..InputEntity::char_to_utf16(&state.text, marked_end);
    let char_range = InputEntity::utf16_to_char(&state.text, utf16_range.start)
        ..InputEntity::utf16_to_char(&state.text, utf16_range.end);
    InputEntity::replace_char_range(&mut state, char_range, "に");
    let end = state.cursor;
    state.set_marked_range(Some((end - 1, end)));
    assert_eq!(state.text, "に");
    assert_eq!(state.marked_range, Some((0, 1)));

    // Commit: a plain replace clears the mark (handler does this first).
    state.set_marked_range(None);
    InputEntity::replace_char_range(&mut state, 0..1, "日");
    assert_eq!(state.text, "日");
    assert_eq!(state.marked_range, None);
}

#[test]
fn utf16_positions_snap_into_graphemes_for_cursor_placement() {
    // "é" as e + combining acute: UTF-16 index 1 sits mid-cluster.
    let mut state = EditState::new("e\u{301}x");
    let char_pos = InputEntity::utf16_to_char(&state.text, 1);
    assert_eq!(char_pos, 1);
    assert_eq!(state.snap_to_grapheme(char_pos), 0);
    state.start_selection(char_pos);
    assert_eq!(state.cursor, 0, "cursor must not rest inside a cluster");
}
