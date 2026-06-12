//! StatusBar component tests

use gpui::div;
use gpui::prelude::ParentElement;
use gpui_ui_kit::status_bar::{StatusBar, StatusBarPosition};

#[test]
fn test_status_bar_creation() {
    let bar = StatusBar::new("footer");
    drop(bar);
}

#[test]
fn test_status_bar_both_positions() {
    let positions = [StatusBarPosition::Top, StatusBarPosition::Bottom];

    for position in &positions {
        let bar = StatusBar::new("test").position(*position);
        drop(bar);
    }
}

#[test]
fn test_status_bar_left_section() {
    let bar = StatusBar::new("test").left(div().child("Playback controls"));
    drop(bar);
}

#[test]
fn test_status_bar_center_section() {
    let bar = StatusBar::new("test").center(div().child("Track info"));
    drop(bar);
}

#[test]
fn test_status_bar_right_section() {
    let bar = StatusBar::new("test").right(div().child("Volume control"));
    drop(bar);
}

#[test]
fn test_status_bar_height() {
    let bar = StatusBar::new("test").height(gpui::px(40.0));
    drop(bar);
}

#[test]
fn test_status_bar_full_configuration() {
    let bar = StatusBar::new("footer")
        .position(StatusBarPosition::Bottom)
        .height(gpui::px(36.0))
        .left(div().child("Left"))
        .center(div().child("Center"))
        .right(div().child("Right"));
    drop(bar);
}
