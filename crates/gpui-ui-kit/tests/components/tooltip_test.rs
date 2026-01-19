//! Tooltip component tests

use gpui::prelude::*;
use gpui_ui_kit::tooltip::{Tooltip, TooltipPlacement, WithTooltip};

#[test]
fn test_tooltip_placement() {
    let placements = [
        TooltipPlacement::Top,
        TooltipPlacement::Bottom,
        TooltipPlacement::Left,
        TooltipPlacement::Right,
    ];

    for placement in &placements {
        let tooltip = Tooltip::new("Help text").placement(*placement).delay(500);
        drop(tooltip);
    }
}

#[test]
fn test_with_tooltip_wrapper() {
    use gpui::div;

    let wrapper = WithTooltip::new(div().child("Hover me"), "Tooltip text")
        .placement(TooltipPlacement::Bottom)
        .show(true);

    drop(wrapper);
}

#[test]
fn test_tooltip_delay() {
    let instant = Tooltip::new("Instant").delay(0);
    let slow = Tooltip::new("Slow").delay(1000);

    drop(instant);
    drop(slow);
}
