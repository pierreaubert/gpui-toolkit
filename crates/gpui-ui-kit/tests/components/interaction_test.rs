use gpui::{ElementId, Modifiers, ScrollDelta, point, px};
use gpui_ui_kit::interaction::{
    DragState, InteractionConfig, clear_drag_state, drag_has_moved, get_drag_state, handle_drag,
    handle_keyboard, handle_scroll, mark_drag_moved, store_drag_state,
};
use gpui_ui_kit::scale::Scale;

fn vertical_config() -> InteractionConfig {
    InteractionConfig::vertical(0.0, 100.0, Scale::Linear, 100.0)
}

#[test]
fn drag_state_round_trips_with_element_id() {
    let id = ElementId::from("drag-test");
    let state = DragState {
        start_pos: 10.0,
        start_value: 42.0,
    };

    store_drag_state(id.clone(), state.start_pos, state.start_value);
    assert_eq!(get_drag_state(&id), Some(state));
    clear_drag_state(id.clone());
    assert_eq!(get_drag_state(&id), None);
}

#[test]
fn drag_movement_state_is_reset_for_each_gesture() {
    let id = ElementId::from("drag-movement-test");
    store_drag_state(id.clone(), 10.0, 42.0);
    assert!(!drag_has_moved(&id));

    mark_drag_moved(&id);
    assert!(drag_has_moved(&id));

    store_drag_state(id.clone(), 20.0, 42.0);
    assert!(!drag_has_moved(&id));

    clear_drag_state(id.clone());
    assert!(!drag_has_moved(&id));
}

#[test]
fn handle_drag_respects_threshold() {
    let config = vertical_config();
    let state = DragState {
        start_pos: 50.0,
        start_value: 50.0,
    };

    assert!(handle_drag(51.0, &state, &config).is_none());
    assert!(handle_drag(30.0, &state, &config).is_some());
}

#[test]
fn handle_drag_ignores_zero_or_non_finite_track_sizes() {
    let state = DragState {
        start_pos: 50.0,
        start_value: 50.0,
    };
    for track_size in [0.0, f32::NAN, f32::INFINITY] {
        let config = InteractionConfig::vertical(0.0, 100.0, Scale::Linear, track_size);
        assert!(handle_drag(30.0, &state, &config).is_none());
    }
}

#[test]
fn horizontal_drag_right_increases_value() {
    let config = InteractionConfig::horizontal(0.0, 100.0, Scale::Linear, 200.0);
    let state = DragState {
        start_pos: 100.0,
        start_value: 50.0,
    };

    let value = handle_drag(150.0, &state, &config).unwrap();

    assert!(value > 50.0);
}

#[test]
fn handle_keyboard_steps() {
    let config = vertical_config();
    let modifiers = Modifiers::default();
    let up = handle_keyboard("up", &modifiers, 50.0, &config).unwrap();
    let down = handle_keyboard("down", &modifiers, up, &config).unwrap();

    assert!(up > 50.0);
    assert!(down < up);
}

#[test]
fn media_key_aliases_adjust_volume_when_enabled() {
    let config = InteractionConfig::rotational(0.0, 1.0, Scale::Linear, 48.0).with_media_keys();

    assert_eq!(
        handle_keyboard("f11", &Modifiers::default(), 0.5, &config),
        Some(0.45)
    );
    assert_eq!(
        handle_keyboard("f12", &Modifiers::default(), 0.5, &config),
        Some(0.55)
    );
    assert_eq!(
        handle_keyboard("audiolowervolume", &Modifiers::default(), 0.5, &config),
        Some(0.45)
    );
    assert_eq!(
        handle_keyboard("audioraisevolume", &Modifiers::default(), 0.5, &config),
        Some(0.55)
    );
    assert_eq!(
        handle_keyboard("f10", &Modifiers::default(), 0.5, &config),
        None
    );
}

#[test]
fn handle_scroll_changes_value() {
    let config = vertical_config();
    let delta = ScrollDelta::Pixels(point(px(0.0), px(-10.0)));
    let new_value = handle_scroll(&delta, &Modifiers::default(), 50.0, &config);

    assert!(new_value.is_some());
}

#[test]
fn horizontal_scroll_right_increases_value() {
    let config = InteractionConfig::horizontal(0.0, 100.0, Scale::Linear, 200.0);
    let delta = ScrollDelta::Pixels(point(px(10.0), px(0.0)));
    let new_value = handle_scroll(&delta, &Modifiers::default(), 50.0, &config).unwrap();

    assert!(new_value > 50.0);
}
