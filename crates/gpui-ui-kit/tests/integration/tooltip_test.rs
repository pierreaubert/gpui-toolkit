//! Integration tests for Tooltip component
//!
//! Tests the Tooltip and WithTooltip components including:
//! - All placements (Top, Bottom, Left, Right)
//! - Custom delay
//! - WithTooltip wrapper

use gpui::{Context, TestAppContext, Window, div, prelude::*};
use gpui_ui_kit::tooltip::{Tooltip, TooltipPlacement, WithTooltip};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct TooltipTestView;

impl Render for TooltipTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Tooltip::new("This is a tooltip"))
    }
}

#[gpui::test]
async fn test_tooltip_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| TooltipTestView);
}

// ============================================================================
// Placement Tests
// ============================================================================

#[gpui::test]
async fn test_tooltip_placement_top(cx: &mut TestAppContext) {
    struct TopView;

    impl Render for TopView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tooltip::new("Top tooltip").placement(TooltipPlacement::Top))
        }
    }

    let _window = cx.add_window(|_window, _cx| TopView);
}

#[gpui::test]
async fn test_tooltip_placement_bottom(cx: &mut TestAppContext) {
    struct BottomView;

    impl Render for BottomView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tooltip::new("Bottom tooltip").placement(TooltipPlacement::Bottom))
        }
    }

    let _window = cx.add_window(|_window, _cx| BottomView);
}

#[gpui::test]
async fn test_tooltip_placement_left(cx: &mut TestAppContext) {
    struct LeftView;

    impl Render for LeftView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tooltip::new("Left tooltip").placement(TooltipPlacement::Left))
        }
    }

    let _window = cx.add_window(|_window, _cx| LeftView);
}

#[gpui::test]
async fn test_tooltip_placement_right(cx: &mut TestAppContext) {
    struct RightView;

    impl Render for RightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tooltip::new("Right tooltip").placement(TooltipPlacement::Right))
        }
    }

    let _window = cx.add_window(|_window, _cx| RightView);
}

#[gpui::test]
async fn test_tooltip_all_placements(cx: &mut TestAppContext) {
    struct AllPlacementsView;

    impl Render for AllPlacementsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(Tooltip::new("Top").placement(TooltipPlacement::Top))
                .child(Tooltip::new("Bottom").placement(TooltipPlacement::Bottom))
                .child(Tooltip::new("Left").placement(TooltipPlacement::Left))
                .child(Tooltip::new("Right").placement(TooltipPlacement::Right))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllPlacementsView);
}

// ============================================================================
// Delay Tests
// ============================================================================

#[gpui::test]
async fn test_tooltip_custom_delay(cx: &mut TestAppContext) {
    struct DelayView;

    impl Render for DelayView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tooltip::new("Delayed tooltip").delay(500))
        }
    }

    let _window = cx.add_window(|_window, _cx| DelayView);
}

#[gpui::test]
async fn test_tooltip_no_delay(cx: &mut TestAppContext) {
    struct NoDelayView;

    impl Render for NoDelayView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Tooltip::new("Instant tooltip").delay(0))
        }
    }

    let _window = cx.add_window(|_window, _cx| NoDelayView);
}

// ============================================================================
// WithTooltip Tests
// ============================================================================

#[gpui::test]
async fn test_with_tooltip_renders(cx: &mut TestAppContext) {
    struct WithTooltipView;

    impl Render for WithTooltipView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(WithTooltip::new(div().child("Hover me"), "Tooltip text"))
        }
    }

    let _window = cx.add_window(|_window, _cx| WithTooltipView);
}

#[gpui::test]
async fn test_with_tooltip_visible(cx: &mut TestAppContext) {
    struct VisibleTooltipView;

    impl Render for VisibleTooltipView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(WithTooltip::new(div().child("Target"), "Visible tooltip").show(true))
        }
    }

    let _window = cx.add_window(|_window, _cx| VisibleTooltipView);
}

#[gpui::test]
async fn test_with_tooltip_hidden(cx: &mut TestAppContext) {
    struct HiddenTooltipView;

    impl Render for HiddenTooltipView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(WithTooltip::new(div().child("Target"), "Hidden tooltip").show(false))
        }
    }

    let _window = cx.add_window(|_window, _cx| HiddenTooltipView);
}

#[gpui::test]
async fn test_with_tooltip_placement(cx: &mut TestAppContext) {
    struct TooltipPlacementView;

    impl Render for TooltipPlacementView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    WithTooltip::new(div().child("Top"), "Above")
                        .placement(TooltipPlacement::Top)
                        .show(true),
                )
                .child(
                    WithTooltip::new(div().child("Bottom"), "Below")
                        .placement(TooltipPlacement::Bottom)
                        .show(true),
                )
                .child(
                    WithTooltip::new(div().child("Left"), "To the left")
                        .placement(TooltipPlacement::Left)
                        .show(true),
                )
                .child(
                    WithTooltip::new(div().child("Right"), "To the right")
                        .placement(TooltipPlacement::Right)
                        .show(true),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| TooltipPlacementView);
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[gpui::test]
async fn test_tooltip_all_features(cx: &mut TestAppContext) {
    struct AllFeaturesView;

    impl Render for AllFeaturesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Tooltip::new("Full featured tooltip")
                    .placement(TooltipPlacement::Bottom)
                    .delay(300),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllFeaturesView);
}

#[gpui::test]
async fn test_with_tooltip_all_features(cx: &mut TestAppContext) {
    struct WithTooltipAllFeaturesView;

    impl Render for WithTooltipAllFeaturesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                WithTooltip::new(
                    div().px_4().py_2().bg(gpui::rgb(0x333333)).child("Button"),
                    "Click to perform action",
                )
                .placement(TooltipPlacement::Bottom)
                .show(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| WithTooltipAllFeaturesView);
}
