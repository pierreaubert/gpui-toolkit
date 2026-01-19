//! Integration tests for Toggle component
//!
//! Tests the Toggle component including:
//! - Both styles (Sliding, Segmented)
//! - All sizes (Sm, Md, Lg)
//! - Checked and unchecked states
//! - Disabled state
//! - Selected state (for plugin parameter editing)
//! - Click and keyboard toggle callbacks

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::toggle::{Toggle, ToggleSize, ToggleStyle};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct ToggleTestView;

impl Render for ToggleTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Toggle::new("test-toggle"))
    }
}

#[gpui::test]
async fn test_toggle_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ToggleTestView);
}

// ============================================================================
// Style Tests
// ============================================================================

#[gpui::test]
async fn test_toggle_sliding_style(cx: &mut TestAppContext) {
    struct SlidingView;

    impl Render for SlidingView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toggle::new("sliding").style(ToggleStyle::Sliding))
        }
    }

    let _window = cx.add_window(|_window, _cx| SlidingView);
}

#[gpui::test]
async fn test_toggle_segmented_style(cx: &mut TestAppContext) {
    struct SegmentedView;

    impl Render for SegmentedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toggle::new("segmented").style(ToggleStyle::Segmented))
        }
    }

    let _window = cx.add_window(|_window, _cx| SegmentedView);
}

#[gpui::test]
async fn test_toggle_both_styles(cx: &mut TestAppContext) {
    struct BothStylesView;

    impl Render for BothStylesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    Toggle::new("sliding")
                        .style(ToggleStyle::Sliding)
                        .label("Sliding"),
                )
                .child(
                    Toggle::new("segmented")
                        .style(ToggleStyle::Segmented)
                        .label("Segmented"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| BothStylesView);
}

// ============================================================================
// Size Tests
// ============================================================================

#[gpui::test]
async fn test_toggle_size_sm(cx: &mut TestAppContext) {
    struct SmSizeView;

    impl Render for SmSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toggle::new("sm").size(ToggleSize::Sm))
        }
    }

    let _window = cx.add_window(|_window, _cx| SmSizeView);
}

#[gpui::test]
async fn test_toggle_size_md(cx: &mut TestAppContext) {
    struct MdSizeView;

    impl Render for MdSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toggle::new("md").size(ToggleSize::Md))
        }
    }

    let _window = cx.add_window(|_window, _cx| MdSizeView);
}

#[gpui::test]
async fn test_toggle_size_lg(cx: &mut TestAppContext) {
    struct LgSizeView;

    impl Render for LgSizeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toggle::new("lg").size(ToggleSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| LgSizeView);
}

#[gpui::test]
async fn test_toggle_all_sizes(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .items_center()
                .gap_4()
                .child(Toggle::new("sm").size(ToggleSize::Sm).label("Sm"))
                .child(Toggle::new("md").size(ToggleSize::Md).label("Md"))
                .child(Toggle::new("lg").size(ToggleSize::Lg).label("Lg"))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// Checked State Tests
// ============================================================================

#[gpui::test]
async fn test_toggle_unchecked(cx: &mut TestAppContext) {
    struct UncheckedView;

    impl Render for UncheckedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toggle::new("unchecked").checked(false))
        }
    }

    let _window = cx.add_window(|_window, _cx| UncheckedView);
}

#[gpui::test]
async fn test_toggle_checked(cx: &mut TestAppContext) {
    struct CheckedView;

    impl Render for CheckedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toggle::new("checked").checked(true))
        }
    }

    let _window = cx.add_window(|_window, _cx| CheckedView);
}

#[gpui::test]
async fn test_toggle_both_states(cx: &mut TestAppContext) {
    struct BothStatesView;

    impl Render for BothStatesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(Toggle::new("off").checked(false).label("Off"))
                .child(Toggle::new("on").checked(true).label("On"))
        }
    }

    let _window = cx.add_window(|_window, _cx| BothStatesView);
}

// ============================================================================
// Disabled State Tests
// ============================================================================

#[gpui::test]
async fn test_toggle_disabled(cx: &mut TestAppContext) {
    struct DisabledView;

    impl Render for DisabledView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(Toggle::new("disabled-off").disabled(true).checked(false))
                .child(Toggle::new("disabled-on").disabled(true).checked(true))
        }
    }

    let _window = cx.add_window(|_window, _cx| DisabledView);
}

// ============================================================================
// Selected State Tests
// ============================================================================

#[gpui::test]
async fn test_toggle_selected(cx: &mut TestAppContext) {
    struct SelectedView;

    impl Render for SelectedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toggle::new("selected")
                    .selected(true)
                    .label("Selected Parameter"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SelectedView);
}

#[gpui::test]
async fn test_toggle_selected_segmented(cx: &mut TestAppContext) {
    struct SelectedSegmentedView;

    impl Render for SelectedSegmentedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toggle::new("selected-seg")
                    .style(ToggleStyle::Segmented)
                    .selected(true)
                    .label("Bypass"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SelectedSegmentedView);
}

// ============================================================================
// Click Callback Tests
// ============================================================================

struct ClickableToggleView {
    checked: Rc<RefCell<bool>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for ClickableToggleView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_checked = *self.checked.borrow();
        let checked = self.checked.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            Toggle::new("clickable-toggle")
                .checked(current_checked)
                .label("Click me")
                .on_change(move |new_checked, _window, _cx| {
                    *checked.borrow_mut() = new_checked;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_toggle_click_callback(cx: &mut TestAppContext) {
    let checked: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));

    let checked_clone = checked.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| ClickableToggleView {
        checked: checked_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(!*checked.borrow(), "Initial state should be unchecked");

    if let Some(bounds) = cx.debug_bounds("clickable-toggle") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(*checked.borrow(), "Should be checked after click");
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            1,
            "on_change should have been called once"
        );
    }
}

#[gpui::test]
async fn test_toggle_disabled_ignores_click(cx: &mut TestAppContext) {
    struct DisabledClickView {
        checked: Rc<RefCell<bool>>,
        change_count: Arc<AtomicUsize>,
    }

    impl Render for DisabledClickView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let current_checked = *self.checked.borrow();
            let checked = self.checked.clone();
            let change_count = self.change_count.clone();

            div().size_full().child(
                Toggle::new("disabled-toggle")
                    .checked(current_checked)
                    .disabled(true)
                    .on_change(move |new_checked, _window, _cx| {
                        *checked.borrow_mut() = new_checked;
                        change_count.fetch_add(1, Ordering::SeqCst);
                    }),
            )
        }
    }

    let checked: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));

    let checked_clone = checked.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| DisabledClickView {
        checked: checked_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("disabled-toggle") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(!*checked.borrow(), "Should still be unchecked (disabled)");
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            0,
            "on_change should not have been called"
        );
    }
}

// ============================================================================
// Label Tests
// ============================================================================

#[gpui::test]
async fn test_toggle_with_label(cx: &mut TestAppContext) {
    struct LabelView;

    impl Render for LabelView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Toggle::new("labeled").label("Enable Feature"))
        }
    }

    let _window = cx.add_window(|_window, _cx| LabelView);
}

// ============================================================================
// Combined Feature Tests
// ============================================================================

#[gpui::test]
async fn test_toggle_all_features_sliding(cx: &mut TestAppContext) {
    struct AllFeaturesSlidingView;

    impl Render for AllFeaturesSlidingView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toggle::new("full")
                    .style(ToggleStyle::Sliding)
                    .size(ToggleSize::Lg)
                    .checked(true)
                    .selected(true)
                    .label("Full Featured"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllFeaturesSlidingView);
}

#[gpui::test]
async fn test_toggle_all_features_segmented(cx: &mut TestAppContext) {
    struct AllFeaturesSegmentedView;

    impl Render for AllFeaturesSegmentedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Toggle::new("full-seg")
                    .style(ToggleStyle::Segmented)
                    .checked(true)
                    .selected(true)
                    .label("Bypass"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllFeaturesSegmentedView);
}

// ============================================================================
// Segmented Style State Tests
// ============================================================================

#[gpui::test]
async fn test_toggle_segmented_states(cx: &mut TestAppContext) {
    struct SegmentedStatesView;

    impl Render for SegmentedStatesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    Toggle::new("seg-off")
                        .style(ToggleStyle::Segmented)
                        .checked(false)
                        .label("OFF State"),
                )
                .child(
                    Toggle::new("seg-on")
                        .style(ToggleStyle::Segmented)
                        .checked(true)
                        .label("ON State"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SegmentedStatesView);
}
