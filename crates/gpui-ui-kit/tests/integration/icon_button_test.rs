//! Integration tests for IconButton component
//!
//! Tests the IconButton component including:
//! - Click callback with VisualTestContext
//! - Disabled state ignoring clicks
//! - All sizes rendering
//! - All variants rendering
//! - Selected state
//! - Rounded full styling

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div,
};
use gpui_ui_kit::icon_button::{IconButton, IconButtonSize, IconButtonVariant};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Click Callback Tests
// ============================================================================

struct IconButtonClickView {
    click_count: Arc<AtomicUsize>,
}

impl Render for IconButtonClickView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let click_count = self.click_count.clone();

        div()
            .size_full()
            .child(
                IconButton::new("test-icon-btn", "✓").on_click(move |_window, _cx| {
                    click_count.fetch_add(1, Ordering::SeqCst);
                }),
            )
    }
}

#[gpui::test]
async fn test_icon_button_click_triggers_callback(cx: &mut TestAppContext) {
    let click_count = Arc::new(AtomicUsize::new(0));
    let click_count_clone = click_count.clone();

    let window = cx.add_window(move |_window, _cx| IconButtonClickView {
        click_count: click_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("test-icon-btn") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            click_count.load(Ordering::SeqCst),
            1,
            "on_click should have been called once"
        );
    }
}

// ============================================================================
// Disabled State Test
// ============================================================================

struct DisabledIconButtonView {
    click_count: Arc<AtomicUsize>,
}

impl Render for DisabledIconButtonView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let click_count = self.click_count.clone();

        div().size_full().child(
            IconButton::new("disabled-icon-btn", "✓")
                .disabled(true)
                .on_click(move |_window, _cx| {
                    click_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_icon_button_disabled_ignores_click(cx: &mut TestAppContext) {
    let click_count = Arc::new(AtomicUsize::new(0));
    let click_count_clone = click_count.clone();

    let window = cx.add_window(move |_window, _cx| DisabledIconButtonView {
        click_count: click_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("disabled-icon-btn") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            click_count.load(Ordering::SeqCst),
            0,
            "Disabled icon button should not fire on_click"
        );
    }
}

// ============================================================================
// Multiple Clicks Test
// ============================================================================

#[gpui::test]
async fn test_icon_button_multiple_clicks(cx: &mut TestAppContext) {
    let click_count = Arc::new(AtomicUsize::new(0));
    let click_count_clone = click_count.clone();

    let window = cx.add_window(move |_window, _cx| IconButtonClickView {
        click_count: click_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("test-icon-btn") {
        let center = bounds.center();
        for _ in 0..3 {
            cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
            cx.run_until_parked();
        }

        assert_eq!(
            click_count.load(Ordering::SeqCst),
            3,
            "on_click should have been called three times"
        );
    }
}

// ============================================================================
// All Sizes Rendering Tests
// ============================================================================

#[gpui::test]
async fn test_icon_button_all_sizes_render(cx: &mut TestAppContext) {
    struct AllSizesView;

    impl Render for AllSizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_2()
                .child(IconButton::new("ib-xs", "✓").size(IconButtonSize::Xs))
                .child(IconButton::new("ib-sm", "✓").size(IconButtonSize::Sm))
                .child(IconButton::new("ib-md", "✓").size(IconButtonSize::Md))
                .child(IconButton::new("ib-lg", "✓").size(IconButtonSize::Lg))
                .child(IconButton::new("ib-xl", "✓").size(IconButtonSize::Xl))
                .child(IconButton::new("ib-custom", "✓").size(IconButtonSize::Custom(32)))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllSizesView);
}

// ============================================================================
// All Variants Rendering Tests
// ============================================================================

#[gpui::test]
async fn test_icon_button_all_variants_render(cx: &mut TestAppContext) {
    struct AllVariantsView;

    impl Render for AllVariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_2()
                .child(IconButton::new("ib-ghost", "✓").variant(IconButtonVariant::Ghost))
                .child(IconButton::new("ib-filled", "✓").variant(IconButtonVariant::Filled))
                .child(IconButton::new("ib-outline", "✓").variant(IconButtonVariant::Outline))
        }
    }

    let _window = cx.add_window(|_window, _cx| AllVariantsView);
}

// ============================================================================
// Selected State Tests
// ============================================================================

#[gpui::test]
async fn test_icon_button_selected_state(cx: &mut TestAppContext) {
    struct SelectedView;

    impl Render for SelectedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_2()
                .child(IconButton::new("ib-selected", "✓").selected(true))
                .child(IconButton::new("ib-not-selected", "✓").selected(false))
        }
    }

    let _window = cx.add_window(|_window, _cx| SelectedView);
}

// ============================================================================
// Rounded Full Test
// ============================================================================

#[gpui::test]
async fn test_icon_button_rounded_full(cx: &mut TestAppContext) {
    struct RoundedFullView;

    impl Render for RoundedFullView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(IconButton::new("ib-rounded", "✓").rounded_full())
        }
    }

    let _window = cx.add_window(|_window, _cx| RoundedFullView);
}
