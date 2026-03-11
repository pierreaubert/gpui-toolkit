//! Integration tests for Checkbox component
//!
//! Tests checkbox rendering, toggle on/off, disabled state, sizes,
//! indeterminate state, labels, and custom theming using VisualTestContext.

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::checkbox::{Checkbox, CheckboxSize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Toggle Tests
// ============================================================================

/// View that tracks checkbox state changes
struct CheckboxToggleTestView {
    checked: Rc<RefCell<bool>>,
    change_count: Arc<AtomicUsize>,
    disabled: bool,
    indeterminate: bool,
}

impl Render for CheckboxToggleTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let is_checked = *self.checked.borrow();
        let checked_rc = self.checked.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            Checkbox::new("test-checkbox")
                .checked(is_checked)
                .disabled(self.disabled)
                .indeterminate(self.indeterminate)
                .label("Test Checkbox")
                .on_change(move |new_state, _window, _cx| {
                    *checked_rc.borrow_mut() = new_state;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_checkbox_click_toggles_on(cx: &mut TestAppContext) {
    let checked = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let checked_clone = checked.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| CheckboxToggleTestView {
        checked: checked_clone,
        change_count: change_count_clone,
        disabled: false,
        indeterminate: false,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(!*checked.borrow(), "Should start unchecked");

    if let Some(bounds) = cx.debug_bounds("test-checkbox") {
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
async fn test_checkbox_click_toggles_off(cx: &mut TestAppContext) {
    let checked = Rc::new(RefCell::new(true));
    let change_count = Arc::new(AtomicUsize::new(0));
    let checked_clone = checked.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| CheckboxToggleTestView {
        checked: checked_clone,
        change_count: change_count_clone,
        disabled: false,
        indeterminate: false,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(*checked.borrow(), "Should start checked");

    if let Some(bounds) = cx.debug_bounds("test-checkbox") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(!*checked.borrow(), "Should be unchecked after click");
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            1,
            "on_change should have been called once"
        );
    }
}

#[gpui::test]
async fn test_checkbox_disabled_ignores_click(cx: &mut TestAppContext) {
    let checked = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let checked_clone = checked.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| CheckboxToggleTestView {
        checked: checked_clone,
        change_count: change_count_clone,
        disabled: true,
        indeterminate: false,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("test-checkbox") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
    }

    assert!(
        !*checked.borrow(),
        "Disabled checkbox should remain unchecked"
    );
    assert_eq!(
        change_count.load(Ordering::SeqCst),
        0,
        "Disabled checkbox should not trigger on_change"
    );
}

// ============================================================================
// Size Rendering Tests
// ============================================================================

#[gpui::test]
async fn test_checkbox_all_sizes(cx: &mut TestAppContext) {
    struct SizesView;

    impl Render for SizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Checkbox::new("cb-sm").size(CheckboxSize::Sm).label("Small"))
                .child(
                    Checkbox::new("cb-md")
                        .size(CheckboxSize::Md)
                        .label("Medium"),
                )
                .child(Checkbox::new("cb-lg").size(CheckboxSize::Lg).label("Large"))
        }
    }

    let _window = cx.add_window(|_window, _cx| SizesView);
}

// ============================================================================
// Indeterminate State Tests
// ============================================================================

#[gpui::test]
async fn test_checkbox_indeterminate(cx: &mut TestAppContext) {
    struct IndeterminateView;

    impl Render for IndeterminateView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Checkbox::new("indet-checkbox")
                    .indeterminate(true)
                    .label("Partially selected"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| IndeterminateView);
}

// ============================================================================
// Label Tests
// ============================================================================

#[gpui::test]
async fn test_checkbox_with_label(cx: &mut TestAppContext) {
    struct LabelView;

    impl Render for LabelView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Checkbox::new("labeled").label("Accept terms and conditions"))
                .child(Checkbox::new("no-label"))
        }
    }

    let _window = cx.add_window(|_window, _cx| LabelView);
}

// ============================================================================
// Multiple Toggles Test
// ============================================================================

#[gpui::test]
async fn test_checkbox_multiple_toggles(cx: &mut TestAppContext) {
    let checked = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let checked_clone = checked.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| CheckboxToggleTestView {
        checked: checked_clone,
        change_count: change_count_clone,
        disabled: false,
        indeterminate: false,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("test-checkbox") {
        let center = bounds.center();

        // Toggle on
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        assert!(*checked.borrow(), "Should be checked after first click");

        // Toggle off
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        assert!(!*checked.borrow(), "Should be unchecked after second click");

        assert_eq!(
            change_count.load(Ordering::SeqCst),
            2,
            "on_change should have been called twice"
        );
    }
}
