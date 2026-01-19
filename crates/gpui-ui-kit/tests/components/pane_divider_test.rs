//! PaneDivider component tests

use gpui_ui_kit::pane_divider::{CollapseDirection, PaneDivider};

#[test]
fn test_pane_divider_vertical() {
    let divider = PaneDivider::vertical("v-div", CollapseDirection::Left)
        .label("Sidebar")
        .collapsed(false)
        .thickness(gpui::px(8.0))
        .on_toggle(|collapsed, _window, _cx| {
            println!("Collapsed: {}", collapsed);
        })
        .on_drag_start(|pos, _window, _cx| {
            println!("Drag start x: {}", pos);
        });

    drop(divider);
}

#[test]
fn test_pane_divider_horizontal() {
    let divider = PaneDivider::horizontal("h-div", CollapseDirection::Down)
        .collapsed(true)
        .collapsed_size(gpui::px(32.0));

    drop(divider);
}

#[test]
fn test_collapse_direction_logic() {
    let left = CollapseDirection::Left;
    assert_eq!(left.opposite(), CollapseDirection::Right);
    assert!(left.is_horizontal());

    let up = CollapseDirection::Up;
    assert_eq!(up.opposite(), CollapseDirection::Down);
    assert!(!up.is_horizontal());
}
