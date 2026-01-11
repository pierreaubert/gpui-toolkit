//! Integration tests for NumberInput component
//!
//! Tests the number input component including:
//! - Basic rendering
//! - Size variants
//! - Value changes via buttons (+/-)
//! - Min/max bounds
//! - Step size
//! - Decimals formatting
//! - Unit display
//! - Label
//! - Disabled state
//! - Mouse click on +/- buttons
//! - Click on value field to edit
//! - Double-click to select all
//! - Keyboard input
//! - Scroll wheel
//! - Theme customization

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::number_input::{NumberInput, NumberInputSize, NumberInputTheme};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct NumberInputTestView;

impl Render for NumberInputTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            NumberInput::new("test-number-input")
                .value(50.0)
                .min(0.0)
                .max(100.0),
        )
    }
}

#[gpui::test]
async fn test_number_input_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| NumberInputTestView);
}

// ============================================================================
// Size Variant Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_sizes(cx: &mut TestAppContext) {
    struct SizeTestView;

    impl Render for SizeTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    NumberInput::new("sm-input")
                        .size(NumberInputSize::Sm)
                        .value(10.0),
                )
                .child(
                    NumberInput::new("md-input")
                        .size(NumberInputSize::Md)
                        .value(50.0),
                )
                .child(
                    NumberInput::new("lg-input")
                        .size(NumberInputSize::Lg)
                        .value(100.0),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizeTestView);
}

// ============================================================================
// Value Change Tests
// ============================================================================

/// View that tracks value changes
struct NumberInputChangeTestView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for NumberInputChangeTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            NumberInput::new("change-test-input")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .step(5.0)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_number_input_value_change(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputChangeTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initial value should be 50
    assert_eq!(*value.borrow(), 50.0);
}

// ============================================================================
// Mouse Button Click Tests (+/- buttons)
// ============================================================================

/// View for testing +/- button clicks
struct NumberInputButtonTestView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for NumberInputButtonTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            NumberInput::new("button-test-input")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .step(5.0)
                .width(150.0)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test clicking the increment (+) button increases value
#[gpui::test]
async fn test_number_input_increment_button_click(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputButtonTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Find the increment button (right side of the input)
    if let Some(bounds) = cx.debug_bounds("button-test-input") {
        // The + button is on the right side
        let button_x = bounds.right() - gpui::px(14.0); // Approximate center of + button
        let button_y = bounds.center().y;
        let inc_button_pos = gpui::point(button_x, button_y);

        // Click the + button
        cx.simulate_mouse_down(inc_button_pos, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(inc_button_pos, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Value should have increased by step (5.0)
        let new_value = *value.borrow();
        assert_eq!(
            new_value, 55.0,
            "Value should be 55.0 after clicking +, got {}",
            new_value
        );
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            1,
            "on_change should have been called once"
        );
    }
}

/// Test clicking the decrement (-) button decreases value
#[gpui::test]
async fn test_number_input_decrement_button_click(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputButtonTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Find the decrement button (left side of the input)
    if let Some(bounds) = cx.debug_bounds("button-test-input") {
        // The - button is on the left side
        let button_x = bounds.left() + gpui::px(14.0); // Approximate center of - button
        let button_y = bounds.center().y;
        let dec_button_pos = gpui::point(button_x, button_y);

        // Click the - button
        cx.simulate_mouse_down(dec_button_pos, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(dec_button_pos, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Value should have decreased by step (5.0)
        let new_value = *value.borrow();
        assert_eq!(
            new_value, 45.0,
            "Value should be 45.0 after clicking -, got {}",
            new_value
        );
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            1,
            "on_change should have been called once"
        );
    }
}

/// Test multiple button clicks
#[gpui::test]
async fn test_number_input_multiple_button_clicks(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputButtonTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("button-test-input") {
        let inc_button_pos = gpui::point(bounds.right() - gpui::px(14.0), bounds.center().y);

        // Click + button 3 times
        for _ in 0..3 {
            cx.simulate_mouse_down(inc_button_pos, MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_up(inc_button_pos, MouseButton::Left, Modifiers::default());
            cx.run_until_parked();
        }

        // Value should have increased by 15 (3 * 5.0)
        let new_value = *value.borrow();
        assert_eq!(
            new_value, 65.0,
            "Value should be 65.0 after 3 clicks on +, got {}",
            new_value
        );
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            3,
            "on_change should have been called 3 times"
        );
    }
}

// ============================================================================
// Bounds Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_min_max_bounds(cx: &mut TestAppContext) {
    struct BoundsTestView;

    impl Render for BoundsTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                NumberInput::new("bounds-input")
                    .value(150.0) // Over max, should be clamped
                    .min(0.0)
                    .max(100.0),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| BoundsTestView);
}

/// Test that clicking - at minimum doesn't go below min
#[gpui::test]
async fn test_number_input_respects_min_bound(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(5.0)); // Close to min
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    struct MinBoundTestView {
        value: Rc<RefCell<f64>>,
        change_count: Arc<AtomicUsize>,
    }

    impl Render for MinBoundTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let current_value = *self.value.borrow();
            let value_rc = self.value.clone();
            let change_count = self.change_count.clone();

            div().size_full().child(
                NumberInput::new("min-bound-input")
                    .value(current_value)
                    .min(0.0)
                    .max(100.0)
                    .step(10.0)
                    .width(150.0)
                    .on_change(move |new_val, _window, _cx| {
                        *value_rc.borrow_mut() = new_val;
                        change_count.fetch_add(1, Ordering::SeqCst);
                    }),
            )
        }
    }

    let window = cx.add_window(move |_window, _cx| MinBoundTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("min-bound-input") {
        let dec_button_pos = gpui::point(bounds.left() + gpui::px(14.0), bounds.center().y);

        // Click - button - should clamp to 0
        cx.simulate_mouse_down(dec_button_pos, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(dec_button_pos, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        let new_value = *value.borrow();
        assert_eq!(
            new_value, 0.0,
            "Value should be clamped to min (0.0), got {}",
            new_value
        );
    }
}

/// Test that clicking + at maximum doesn't go above max
#[gpui::test]
async fn test_number_input_respects_max_bound(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(95.0)); // Close to max
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    struct MaxBoundTestView {
        value: Rc<RefCell<f64>>,
        change_count: Arc<AtomicUsize>,
    }

    impl Render for MaxBoundTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let current_value = *self.value.borrow();
            let value_rc = self.value.clone();
            let change_count = self.change_count.clone();

            div().size_full().child(
                NumberInput::new("max-bound-input")
                    .value(current_value)
                    .min(0.0)
                    .max(100.0)
                    .step(10.0)
                    .width(150.0)
                    .on_change(move |new_val, _window, _cx| {
                        *value_rc.borrow_mut() = new_val;
                        change_count.fetch_add(1, Ordering::SeqCst);
                    }),
            )
        }
    }

    let window = cx.add_window(move |_window, _cx| MaxBoundTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("max-bound-input") {
        let inc_button_pos = gpui::point(bounds.right() - gpui::px(14.0), bounds.center().y);

        // Click + button - should clamp to 100
        cx.simulate_mouse_down(inc_button_pos, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(inc_button_pos, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        let new_value = *value.borrow();
        assert_eq!(
            new_value, 100.0,
            "Value should be clamped to max (100.0), got {}",
            new_value
        );
    }
}

#[gpui::test]
async fn test_number_input_negative_range(cx: &mut TestAppContext) {
    struct NegativeRangeView;

    impl Render for NegativeRangeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                NumberInput::new("negative-input")
                    .value(-30.0)
                    .min(-60.0)
                    .max(12.0)
                    .unit("dB"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NegativeRangeView);
}

// ============================================================================
// Step Size Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_step_size(cx: &mut TestAppContext) {
    struct StepTestView;

    impl Render for StepTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    NumberInput::new("step-1")
                        .value(50.0)
                        .step(1.0)
                        .label("Step 1"),
                )
                .child(
                    NumberInput::new("step-5")
                        .value(50.0)
                        .step(5.0)
                        .label("Step 5"),
                )
                .child(
                    NumberInput::new("step-10")
                        .value(50.0)
                        .step(10.0)
                        .label("Step 10"),
                )
                .child(
                    NumberInput::new("step-0.1")
                        .value(0.5)
                        .step(0.1)
                        .decimals(1)
                        .label("Step 0.1"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| StepTestView);
}

// ============================================================================
// Decimals Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_decimals(cx: &mut TestAppContext) {
    struct DecimalsTestView;

    impl Render for DecimalsTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    NumberInput::new("decimals-0")
                        .value(12.34567)
                        .decimals(0)
                        .label("0 decimals"),
                )
                .child(
                    NumberInput::new("decimals-1")
                        .value(12.34567)
                        .decimals(1)
                        .label("1 decimal"),
                )
                .child(
                    NumberInput::new("decimals-2")
                        .value(12.34567)
                        .decimals(2)
                        .label("2 decimals"),
                )
                .child(
                    NumberInput::new("decimals-3")
                        .value(12.34567)
                        .decimals(3)
                        .label("3 decimals"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| DecimalsTestView);
}

// ============================================================================
// Unit Display Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_units(cx: &mut TestAppContext) {
    struct UnitsTestView;

    impl Render for UnitsTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    NumberInput::new("unit-hz")
                        .value(1000.0)
                        .unit("Hz")
                        .label("Frequency"),
                )
                .child(
                    NumberInput::new("unit-db")
                        .value(-6.0)
                        .unit("dB")
                        .decimals(1)
                        .label("Gain"),
                )
                .child(
                    NumberInput::new("unit-percent")
                        .value(75.0)
                        .unit("%")
                        .label("Amount"),
                )
                .child(
                    NumberInput::new("unit-ms")
                        .value(100.0)
                        .unit("ms")
                        .label("Delay"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| UnitsTestView);
}

// ============================================================================
// Label Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_with_label(cx: &mut TestAppContext) {
    struct LabelTestView;

    impl Render for LabelTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                NumberInput::new("labeled-input")
                    .value(42.0)
                    .label("Answer to Everything"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LabelTestView);
}

// ============================================================================
// Disabled State Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_disabled(cx: &mut TestAppContext) {
    struct DisabledTestView;

    impl Render for DisabledTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                NumberInput::new("disabled-input")
                    .value(50.0)
                    .disabled(true)
                    .label("Disabled"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DisabledTestView);
}

/// Test that disabled input doesn't trigger callbacks on button click
struct DisabledCallbackTestView {
    change_count: Arc<AtomicUsize>,
}

impl Render for DisabledCallbackTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();

        div().size_full().child(
            NumberInput::new("disabled-callback-input")
                .value(50.0)
                .disabled(true)
                .width(150.0)
                .on_change(move |_, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_number_input_disabled_no_callback(cx: &mut TestAppContext) {
    let change_count = Arc::new(AtomicUsize::new(0));
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| DisabledCallbackTestView {
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("disabled-callback-input") {
        // Try clicking + button on disabled input
        let inc_button_pos = gpui::point(bounds.right() - gpui::px(14.0), bounds.center().y);
        cx.simulate_mouse_down(inc_button_pos, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(inc_button_pos, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Try clicking - button on disabled input
        let dec_button_pos = gpui::point(bounds.left() + gpui::px(14.0), bounds.center().y);
        cx.simulate_mouse_down(dec_button_pos, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(dec_button_pos, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
    }

    // Change count should remain 0 since input is disabled
    assert_eq!(
        change_count.load(Ordering::SeqCst),
        0,
        "Disabled input should not trigger callbacks"
    );
}

// ============================================================================
// Edit Mode Tests - Click on value to edit
// ============================================================================

/// View for testing click-to-edit
struct NumberInputEditTestView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for NumberInputEditTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            NumberInput::new("edit-test-input")
                .value(current_value)
                .min(0.0)
                .max(1000.0)
                .step(1.0)
                .width(150.0)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test clicking on value field starts editing
#[gpui::test]
async fn test_number_input_click_to_edit(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputEditTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("edit-test-input") {
        // Click on the value field (center of the input)
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Type a new value
        cx.simulate_input("123");
        cx.run_until_parked();

        // Press Enter to confirm
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();

        // Value should have changed to 123
        let new_value = *value.borrow();
        assert_eq!(
            new_value, 123.0,
            "Value should be 123.0 after editing, got {}",
            new_value
        );
    }
}

/// Test double-click selects all text
#[gpui::test]
async fn test_number_input_double_click_selects_all(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputEditTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("edit-test-input") {
        let center = bounds.center();

        // Double-click to select all
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Type new value - should replace selected text
        cx.simulate_input("999");
        cx.run_until_parked();

        // Press Enter to confirm
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();

        // Value should be 999 (replaced the selected "50")
        let new_value = *value.borrow();
        assert_eq!(
            new_value, 999.0,
            "Value should be 999.0 after double-click and type, got {}",
            new_value
        );
    }
}

/// Test Escape cancels editing
#[gpui::test]
async fn test_number_input_escape_cancels_edit(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputEditTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("edit-test-input") {
        let center = bounds.center();

        // Click to edit
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Type a new value
        cx.simulate_input("999");
        cx.run_until_parked();

        // Press Escape to cancel
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();

        // Value should remain unchanged (50)
        let new_value = *value.borrow();
        assert_eq!(
            new_value, 50.0,
            "Value should still be 50.0 after Escape, got {}",
            new_value
        );
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            0,
            "on_change should not have been called on Escape"
        );
    }
}

// ============================================================================
// Keyboard Navigation Tests (Arrow keys in non-edit mode)
// ============================================================================

/// Test arrow keys adjust value when not in edit mode
#[gpui::test]
async fn test_number_input_arrow_keys_adjust_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputChangeTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("change-test-input") {
        let center = bounds.center();

        // Click to focus
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Ensure we are NOT in edit mode (edit mode captures arrows for text nav)
        // Usually clicking text focuses it and selects all, but doesn't necessarily enter "edit mode"
        // in a way that captures arrows unless cursor is moving.
        // However, NumberInput often treats focus as "ready to type".
        // Let's check how NumberInput handles arrows.
        // If it follows standard UI patterns, Up/Down should increment/decrement value
        // when focused, unless strictly in text editing mode.

        // Press Up Arrow
        cx.simulate_keystrokes("up");
        cx.run_until_parked();

        // Value should increase by step (5.0) -> 55.0
        assert_eq!(*value.borrow(), 55.0, "Up arrow should increment value");

        // Press Down Arrow
        cx.simulate_keystrokes("down");
        cx.run_until_parked();

        // Value should decrease by step (5.0) -> 50.0
        assert_eq!(*value.borrow(), 50.0, "Down arrow should decrement value");
    }
}

// ============================================================================
// Scroll Wheel Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_scroll_wheel(cx: &mut TestAppContext) {
    use gpui::{ScrollDelta, ScrollWheelEvent, TouchPhase, point};

    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputChangeTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("change-test-input") {
        let center = bounds.center();

        // Simulate scroll up (negative Y delta) -> Increase value
        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(0.0, -1.0)),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        assert_eq!(*value.borrow(), 55.0, "Scroll up should increment value");

        // Simulate scroll down (positive Y delta) -> Decrease value
        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(0.0, 1.0)),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        assert_eq!(*value.borrow(), 50.0, "Scroll down should decrement value");
    }
}

// ============================================================================
// Invalid Input Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_invalid_text_reverts(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let value_clone = value.clone();

    let window = cx.add_window(move |_window, _cx| NumberInputChangeTestView {
        value: value_clone,
        change_count: Arc::new(AtomicUsize::new(0)),
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("change-test-input") {
        let center = bounds.center();

        // Click to focus and edit
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Type invalid non-numeric text
        cx.simulate_input("abc");
        cx.run_until_parked();

        // Press Enter to confirm
        cx.simulate_keystrokes("enter");
        cx.run_until_parked();

        // Value should remain 50.0 (original value), ignoring invalid input
        assert_eq!(
            *value.borrow(),
            50.0,
            "Should revert to original value on invalid input"
        );
    }
}

#[gpui::test]
async fn test_number_input_with_custom_theme(cx: &mut TestAppContext) {
    struct ThemedView;

    impl Render for ThemedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let custom_theme = NumberInputTheme {
                background: gpui::rgba(0x1a1a1aff),
                text: gpui::rgba(0xffffffff),
                button_bg: gpui::rgba(0x2a2a2aff),
                button_hover: gpui::rgba(0x3a3a3aff),
                button_active: gpui::rgba(0xff6600ff),
                button_text: gpui::rgba(0xccccccff),
                border: gpui::rgba(0x444444ff),
                border_focus: gpui::rgba(0xff6600ff),
                label: gpui::rgba(0xaaaaaaff),
                disabled_opacity: 0.4,
            };

            div().child(
                NumberInput::new("themed-input")
                    .theme(custom_theme)
                    .value(42.0)
                    .label("Themed Input"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ThemedView);
}

// ============================================================================
// Width Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_fixed_width(cx: &mut TestAppContext) {
    struct WidthTestView;

    impl Render for WidthTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    NumberInput::new("narrow-input")
                        .value(10.0)
                        .width(80.0)
                        .label("Narrow"),
                )
                .child(
                    NumberInput::new("wide-input")
                        .value(10.0)
                        .width(200.0)
                        .label("Wide"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| WidthTestView);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[gpui::test]
async fn test_number_input_zero_value(cx: &mut TestAppContext) {
    struct ZeroValueView;

    impl Render for ZeroValueView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                NumberInput::new("zero-input")
                    .value(0.0)
                    .min(-100.0)
                    .max(100.0),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ZeroValueView);
}

#[gpui::test]
async fn test_number_input_large_values(cx: &mut TestAppContext) {
    struct LargeValuesView;

    impl Render for LargeValuesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                NumberInput::new("large-input")
                    .value(1000000.0)
                    .min(0.0)
                    .max(10000000.0)
                    .step(1000.0),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LargeValuesView);
}

#[gpui::test]
async fn test_number_input_small_step(cx: &mut TestAppContext) {
    struct SmallStepView;

    impl Render for SmallStepView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                NumberInput::new("small-step-input")
                    .value(0.5)
                    .min(0.0)
                    .max(1.0)
                    .step(0.01)
                    .decimals(2),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SmallStepView);
}

// Note: Scroll wheel tests are not included because VisualTestContext
// does not currently support simulate_scroll(). Scroll wheel functionality
// should be tested manually.
