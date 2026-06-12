//! StepIndicator component tests

use gpui_ui_kit::step_indicator::{
    StepIndicator, StepIndicatorSize, StepItem, StepItemStatus, StepOrientation,
};

#[test]
fn test_step_indicator_creation() {
    let indicator = StepIndicator::new("test", vec![StepItem::new("Step 1")]);
    drop(indicator);
}

#[test]
fn test_step_item_statuses() {
    let statuses = [
        StepItemStatus::NotVisited,
        StepItemStatus::Active,
        StepItemStatus::Completed,
        StepItemStatus::Error,
    ];

    for status in statuses {
        let item = StepItem::new("Step").status(status);
        drop(item);
    }
}

#[test]
fn test_step_indicator_orientations() {
    for orientation in [StepOrientation::Horizontal, StepOrientation::Vertical] {
        let indicator =
            StepIndicator::new("test", vec![StepItem::new("Step 1")]).orientation(orientation);
        drop(indicator);
    }
}

#[test]
fn test_step_indicator_sizes() {
    for size in [
        StepIndicatorSize::Sm,
        StepIndicatorSize::Md,
        StepIndicatorSize::Lg,
    ] {
        let indicator = StepIndicator::new("test", vec![StepItem::new("Step 1")]).size(size);
        drop(indicator);
    }
}

#[test]
fn test_step_item_with_icon() {
    let item = StepItem::new("Account").icon("👤");
    drop(item);
}

#[test]
fn test_step_indicator_on_click() {
    let indicator = StepIndicator::new("test", vec![StepItem::new("Step 1")])
        .on_click(|_index, _window, _cx| {});
    drop(indicator);
}

#[test]
fn test_step_indicator_multiple_steps() {
    let indicator = StepIndicator::new(
        "test",
        vec![
            StepItem::new("Account").status(StepItemStatus::Completed),
            StepItem::new("Profile").status(StepItemStatus::Active),
            StepItem::new("Confirm").status(StepItemStatus::NotVisited),
        ],
    );
    drop(indicator);
}

#[test]
fn test_step_indicator_full_configuration() {
    let indicator = StepIndicator::new(
        "setup-steps",
        vec![
            StepItem::new("Account")
                .status(StepItemStatus::Completed)
                .icon("✓"),
            StepItem::new("Profile").status(StepItemStatus::Active),
            StepItem::new("Review").status(StepItemStatus::NotVisited),
            StepItem::new("Confirm").status(StepItemStatus::NotVisited),
        ],
    )
    .orientation(StepOrientation::Horizontal)
    .size(StepIndicatorSize::Lg)
    .on_click(|_index, _window, _cx| {});
    drop(indicator);
}
