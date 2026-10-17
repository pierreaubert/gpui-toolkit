//! Integration tests for Checkbox component
//!
//! Tests checkbox rendering, toggle on/off, disabled state, sizes,
//! indeterminate state, labels, and custom theming using `VisualTestContext`.

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div,
};
use gpui_ui_kit::checkbox::{Checkbox, CheckboxSize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[gpui::test]
async fn test_checkbox_tab_navigation_and_focus_survive_redraw(cx: &mut TestAppContext) {
    struct Checks(Arc<AtomicUsize>);
    impl Render for Checks {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let mut row = div().size_full();
            for (id, disabled, increment) in [
                ("keyboard-first", false, 1),
                ("keyboard-disabled", true, 100),
                ("keyboard-last", false, 10),
            ] {
                let weak = cx.entity().downgrade();
                row = row.child(Checkbox::new(id).disabled(disabled).label(id).on_change(
                    move |_, _, cx| {
                        let _ = weak.update(cx, |view, cx| {
                            view.0.fetch_add(increment, Ordering::SeqCst);
                            cx.notify();
                        });
                    },
                ));
            }
            row
        }
    }
    let count = Arc::new(AtomicUsize::new(0));
    let state = count.clone();
    let window = cx.add_window(move |_, _| Checks(state));
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    cx.update(gpui::Window::focus_next);
    cx.simulate_keystrokes("space");
    cx.run_until_parked();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    cx.simulate_keystrokes("tab enter");
    cx.run_until_parked();
    assert_eq!(count.load(Ordering::SeqCst), 11);
    cx.simulate_keystrokes("shift-tab space");
    assert_eq!(count.load(Ordering::SeqCst), 12);
}

#[gpui::test]
async fn test_question_keyboard_selection_requires_separate_confirmation(cx: &mut TestAppContext) {
    use gpui_ui_kit::{Button, RadioGroup, RadioOption};
    struct Question {
        selected: Option<gpui::SharedString>,
        confirmed: Arc<AtomicUsize>,
    }
    impl Render for Question {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let weak = cx.entity().downgrade();
            let confirmed = self.confirmed.clone();
            div()
                .size_full()
                .child(
                    RadioGroup::new("keyboard-question")
                        .options(vec![
                            RadioOption::new("a", "Alpha"),
                            RadioOption::new("b", "Beta"),
                        ])
                        .selected(self.selected.clone())
                        .wrap_labels(true)
                        .on_change(move |value, _, cx| {
                            let _ = weak.update(cx, |view, cx| {
                                view.selected = Some(value);
                                cx.notify();
                            });
                        }),
                )
                .child(
                    Button::new("keyboard-confirm", "Confirm")
                        .disabled(self.selected.is_none())
                        .on_click(move |_, _| {
                            confirmed.fetch_add(1, Ordering::SeqCst);
                        }),
                )
        }
    }
    let confirmed = Arc::new(AtomicUsize::new(0));
    let state = confirmed.clone();
    let window = cx.add_window(move |_, _| Question {
        selected: None,
        confirmed: state,
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    cx.update(gpui::Window::focus_next);
    cx.simulate_keystrokes("down");
    cx.run_until_parked();
    assert_eq!(confirmed.load(Ordering::SeqCst), 0);
    cx.simulate_keystrokes("tab enter");
    assert_eq!(confirmed.load(Ordering::SeqCst), 1);
}

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
