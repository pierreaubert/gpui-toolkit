//! Integration tests for StepIndicator component

use gpui::{Context, IntoElement, ParentElement, Render, Styled, TestAppContext, Window, div};
use gpui_ui_kit::step_indicator::{
    StepIndicator, StepIndicatorSize, StepItem, StepItemStatus, StepOrientation,
};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct StepIndicatorTestView;

impl Render for StepIndicatorTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(StepIndicator::new(
            "test-steps",
            vec![StepItem::new("Step 1"), StepItem::new("Step 2")],
        ))
    }
}

#[gpui::test]
async fn test_step_indicator_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| StepIndicatorTestView);
}

// ============================================================================
// Status Tests
// ============================================================================

#[gpui::test]
async fn test_step_indicator_statuses(cx: &mut TestAppContext) {
    struct StatusView;

    impl Render for StatusView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(StepIndicator::new(
                "status-steps",
                vec![
                    StepItem::new("Done").status(StepItemStatus::Completed),
                    StepItem::new("Current").status(StepItemStatus::Active),
                    StepItem::new("Failed").status(StepItemStatus::Error),
                    StepItem::new("Pending").status(StepItemStatus::NotVisited),
                ],
            ))
        }
    }

    let _window = cx.add_window(|_window, _cx| StatusView);
}

// ============================================================================
// Orientation Tests
// ============================================================================

#[gpui::test]
async fn test_step_indicator_horizontal(cx: &mut TestAppContext) {
    struct HorizontalView;

    impl Render for HorizontalView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                StepIndicator::new("h-steps", vec![StepItem::new("A"), StepItem::new("B")])
                    .orientation(StepOrientation::Horizontal),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| HorizontalView);
}

#[gpui::test]
async fn test_step_indicator_vertical(cx: &mut TestAppContext) {
    struct VerticalView;

    impl Render for VerticalView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                StepIndicator::new("v-steps", vec![StepItem::new("A"), StepItem::new("B")])
                    .orientation(StepOrientation::Vertical),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| VerticalView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_step_indicator_sizes(cx: &mut TestAppContext) {
    struct SizeView;

    impl Render for SizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    StepIndicator::new("sm-steps", vec![StepItem::new("A"), StepItem::new("B")])
                        .size(StepIndicatorSize::Sm),
                )
                .child(
                    StepIndicator::new("md-steps", vec![StepItem::new("A"), StepItem::new("B")])
                        .size(StepIndicatorSize::Md),
                )
                .child(
                    StepIndicator::new("lg-steps", vec![StepItem::new("A"), StepItem::new("B")])
                        .size(StepIndicatorSize::Lg),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizeView);
}

// ============================================================================
// Full Configuration Tests
// ============================================================================

#[gpui::test]
async fn test_step_indicator_full_config(cx: &mut TestAppContext) {
    struct FullConfigView;

    impl Render for FullConfigView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                StepIndicator::new(
                    "setup-steps",
                    vec![
                        StepItem::new("Account").status(StepItemStatus::Completed),
                        StepItem::new("Profile")
                            .status(StepItemStatus::Active)
                            .icon("👤"),
                        StepItem::new("Review").status(StepItemStatus::NotVisited),
                        StepItem::new("Confirm").status(StepItemStatus::NotVisited),
                    ],
                )
                .orientation(StepOrientation::Horizontal)
                .size(StepIndicatorSize::Lg),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| FullConfigView);
}
