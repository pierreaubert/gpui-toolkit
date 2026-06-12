//! SplitPane component tests

use gpui::div;
use gpui::prelude::ParentElement;
use gpui_ui_kit::split_pane::{SplitDirection, SplitPane};

#[test]
fn test_split_pane_creation() {
    let pane = SplitPane::new("test");
    drop(pane);
}

#[test]
fn test_split_pane_directions() {
    for dir in [SplitDirection::Horizontal, SplitDirection::Vertical] {
        let pane = SplitPane::new("test").direction(dir);
        drop(pane);
    }
}

#[test]
fn test_split_pane_ratio() {
    let pane = SplitPane::new("test").ratio(0.3);
    drop(pane);
}

#[test]
fn test_split_pane_ratio_clamped() {
    let pane = SplitPane::new("test").ratio(1.5);
    drop(pane);

    let pane = SplitPane::new("test").ratio(-0.5);
    drop(pane);
}

#[test]
fn test_split_pane_first() {
    let pane = SplitPane::new("test").first(div().child("Left panel"));
    drop(pane);
}

#[test]
fn test_split_pane_second() {
    let pane = SplitPane::new("test").second(div().child("Right panel"));
    drop(pane);
}

#[test]
fn test_split_pane_min_first() {
    let pane = SplitPane::new("test").min_first(gpui::px(150.0));
    drop(pane);
}

#[test]
fn test_split_pane_min_second() {
    let pane = SplitPane::new("test").min_second(gpui::px(200.0));
    drop(pane);
}

#[test]
fn test_split_pane_divider_width() {
    let pane = SplitPane::new("test").divider_width(gpui::px(6.0));
    drop(pane);
}

#[test]
fn test_split_pane_on_resize() {
    let pane = SplitPane::new("test").on_resize(|_ratio, _window, _cx| {});
    drop(pane);
}

#[test]
fn test_split_pane_full_configuration() {
    let pane = SplitPane::new("main-split")
        .direction(SplitDirection::Horizontal)
        .first(div().child("Left"))
        .second(div().child("Right"))
        .ratio(0.3)
        .min_first(gpui::px(150.0))
        .min_second(gpui::px(200.0))
        .divider_width(gpui::px(6.0))
        .on_resize(|_ratio, _window, _cx| {});
    drop(pane);
}
