use gpui::{ElementId, Modifiers, ScrollDelta, point, px};
use gpui_ui_kit::interaction::{
    DragState, InteractionConfig, clear_drag_state, get_drag_state, handle_drag, handle_keyboard,
    handle_scroll, store_drag_state,
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
