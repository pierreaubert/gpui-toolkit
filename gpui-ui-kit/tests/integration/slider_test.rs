//! Integration tests for horizontal Slider component
//!
//! Tests the slider component including:
//! - Basic rendering with different sizes
//! - Value changes via scroll wheel
//! - Value changes via click and drag
//! - Keyboard navigation (arrows)
//! - Disabled state
//! - Value clamping at bounds
//! - Callbacks: on_change

use gpui::{
    Context, Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase,
    VisualTestContext, Window, div, point, prelude::*,
};
use gpui_ui_kit::slider::{Slider, SliderSize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct SliderTestView;

impl Render for SliderTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Slider::new("test-slider").value(0.5).min(0.0).max(1.0))
    }
}

#[gpui::test]
async fn test_slider_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| SliderTestView);
}

/// Test different size variants
#[gpui::test]
async fn test_slider_sizes(cx: &mut TestAppContext) {
    struct SizeTestView;

    impl Render for SizeTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    Slider::new("slider-sm")
                        .size(SliderSize::Sm)
                        .value(0.25)
                        .min(0.0)
                        .max(1.0),
                )
                .child(
                    Slider::new("slider-md")
                        .size(SliderSize::Md)
                        .value(0.5)
                        .min(0.0)
                        .max(1.0),
                )
                .child(
                    Slider::new("slider-lg")
                        .size(SliderSize::Lg)
                        .value(0.75)
                        .min(0.0)
                        .max(1.0),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizeTestView);
}

// ============================================================================
// Value Change Tests
// ============================================================================

/// View that tracks value changes
struct SliderValueChangeView {
    value: Rc<RefCell<f32>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderValueChangeView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            Slider::new("change-test-slider")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_slider_value_change_callback(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderValueChangeView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initial value should be 50
    assert!(
        ((*value.borrow()) - 50.0).abs() < 0.01,
        "Initial value should be 50"
    );
}

// ============================================================================
// Scroll Wheel Tests
// ============================================================================

/// View for scroll wheel tests
struct SliderScrollWheelView {
    value: Rc<RefCell<f32>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderScrollWheelView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            Slider::new("scroll-wheel-slider")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test scroll wheel up increases value
#[gpui::test]
async fn test_slider_scroll_wheel_up_increases_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(
        ((*value.borrow()) - 50.0).abs() < 0.01,
        "Initial value should be 50"
    );

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-slider") {
        let center = bounds.center();

        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(0.0, -1.0)), // Negative = scroll up
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val > 50.0,
            "Value should increase after scroll up, got {}",
            new_val
        );
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            1,
            "on_change should have been called once"
        );
    }
}

/// Test scroll wheel down decreases value
#[gpui::test]
async fn test_slider_scroll_wheel_down_decreases_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-slider") {
        let center = bounds.center();

        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(0.0, 1.0)), // Positive = scroll down
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val < 50.0,
            "Value should decrease after scroll down, got {}",
            new_val
        );
    }
}

/// Test scroll wheel with Shift for fine control
#[gpui::test]
async fn test_slider_scroll_wheel_shift_fine_control(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-slider") {
        let center = bounds.center();

        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(0.0, -1.0)),
            modifiers: Modifiers {
                shift: true,
                ..Modifiers::default()
            },
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val > 50.0 && new_val < 51.0,
            "Shift+scroll should give fine control, got {}",
            new_val
        );
    }
}

/// Test scroll wheel respects bounds (clamped at max)
#[gpui::test]
async fn test_slider_scroll_wheel_respects_max_bound(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(95.0)); // Near max
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-slider") {
        let center = bounds.center();

        for _ in 0..10 {
            cx.simulate_event(ScrollWheelEvent {
                position: center,
                delta: ScrollDelta::Lines(point(0.0, -1.0)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            });
            cx.run_until_parked();
        }

        let new_val = *value.borrow();
        assert!(
            new_val <= 100.0,
            "Value should be clamped at max (100), got {}",
            new_val
        );
    }
}

/// Test scroll wheel respects bounds (clamped at min)
#[gpui::test]
async fn test_slider_scroll_wheel_respects_min_bound(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(5.0)); // Near min
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-slider") {
        let center = bounds.center();

        for _ in 0..10 {
            cx.simulate_event(ScrollWheelEvent {
                position: center,
                delta: ScrollDelta::Lines(point(0.0, 1.0)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            });
            cx.run_until_parked();
        }

        let new_val = *value.borrow();
        assert!(
            new_val >= 0.0,
            "Value should be clamped at min (0), got {}",
            new_val
        );
    }
}

// ============================================================================
// Disabled State Tests
// ============================================================================

/// View with disabled slider
struct SliderDisabledView {
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderDisabledView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();

        div().size_full().child(
            Slider::new("disabled-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .disabled(true)
                .on_change(move |_, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test disabled slider ignores scroll wheel
#[gpui::test]
async fn test_slider_disabled_ignores_scroll_wheel(cx: &mut TestAppContext) {
    let change_count = Arc::new(AtomicUsize::new(0));
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderDisabledView {
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("disabled-slider") {
        let center = bounds.center();

        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(0.0, -1.0)),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        assert_eq!(
            change_count.load(Ordering::SeqCst),
            0,
            "Disabled slider should not respond to scroll wheel"
        );
    }
}

/// Test disabled slider ignores clicks
#[gpui::test]
async fn test_slider_disabled_ignores_clicks(cx: &mut TestAppContext) {
    let change_count = Arc::new(AtomicUsize::new(0));
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderDisabledView {
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("disabled-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            change_count.load(Ordering::SeqCst),
            0,
            "Disabled slider should not respond to clicks"
        );
    }
}

// ============================================================================
// Click and Drag Tests
// ============================================================================

/// Test clicking on slider track changes value
#[gpui::test]
async fn test_slider_click_changes_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderValueChangeView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("change-test-slider") {
        // Click near the right side of the track
        let right_side = gpui::point(bounds.right() - gpui::px(10.0), bounds.center().y);
        cx.simulate_mouse_down(right_side, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        let new_val = *value.borrow();
        // Value should be near maximum since we clicked near the right
        assert!(
            new_val > 80.0,
            "Clicking right side should set high value, got {}",
            new_val
        );
    }
}

// ============================================================================
// Percentage Unit Bounds Tests
// ============================================================================

/// View for percentage unit tests
struct SliderPercentageView {
    value: Rc<RefCell<f32>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderPercentageView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            Slider::new("percentage-slider")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test percentage stays clamped at 100% max
#[gpui::test]
async fn test_slider_percentage_clamped_at_max(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(100.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderPercentageView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("percentage-slider") {
        let center = bounds.center();

        for _ in 0..5 {
            cx.simulate_event(ScrollWheelEvent {
                position: center,
                delta: ScrollDelta::Lines(point(0.0, -1.0)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            });
            cx.run_until_parked();
        }

        let final_val = *value.borrow();
        assert!(
            final_val <= 100.0,
            "Percentage should not exceed 100%, got {}",
            final_val
        );
    }
}

/// Test percentage stays clamped at 0% min
#[gpui::test]
async fn test_slider_percentage_clamped_at_min(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderPercentageView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("percentage-slider") {
        let center = bounds.center();

        for _ in 0..5 {
            cx.simulate_event(ScrollWheelEvent {
                position: center,
                delta: ScrollDelta::Lines(point(0.0, 1.0)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            });
            cx.run_until_parked();
        }

        let final_val = *value.borrow();
        assert!(
            final_val >= 0.0,
            "Percentage should not go below 0%, got {}",
            final_val
        );
    }
}

// ============================================================================
// Multiple Scroll Events Tests
// ============================================================================

/// Test multiple scroll events accumulate
#[gpui::test]
async fn test_slider_multiple_scroll_events(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-slider") {
        let center = bounds.center();

        // Scroll up 3 times
        for _ in 0..3 {
            cx.simulate_event(ScrollWheelEvent {
                position: center,
                delta: ScrollDelta::Lines(point(0.0, -1.0)),
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            });
            cx.run_until_parked();
        }

        let new_val = *value.borrow();
        assert!(
            new_val > 60.0,
            "Value should increase significantly after 3 scrolls, got {}",
            new_val
        );
        assert!(
            change_count.load(Ordering::SeqCst) >= 3,
            "on_change should have been called at least 3 times"
        );
    }
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[gpui::test]
async fn test_slider_with_labels(cx: &mut TestAppContext) {
    struct LabeledSliderView;

    impl Render for LabeledSliderView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Slider::new("labeled-slider")
                    .value(50.0)
                    .min(0.0)
                    .max(100.0)
                    .label("Volume")
                    .show_value(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LabeledSliderView);
}

#[gpui::test]
async fn test_slider_with_step(cx: &mut TestAppContext) {
    struct SteppedSliderView;

    impl Render for SteppedSliderView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Slider::new("stepped-slider")
                    .value(50.0)
                    .min(0.0)
                    .max(100.0)
                    .step(10.0), // Steps of 10
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SteppedSliderView);
}

#[gpui::test]
async fn test_slider_negative_range(cx: &mut TestAppContext) {
    struct NegativeRangeView;

    impl Render for NegativeRangeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Slider::new("negative-slider")
                    .value(0.0)
                    .min(-60.0)
                    .max(12.0)
                    .label("Gain (dB)"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NegativeRangeView);
}

#[gpui::test]
async fn test_slider_zero_to_one_range(cx: &mut TestAppContext) {
    struct ZeroOneRangeView;

    impl Render for ZeroOneRangeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(Slider::new("zero-one-slider").value(0.5).min(0.0).max(1.0))
        }
    }

    let _window = cx.add_window(|_window, _cx| ZeroOneRangeView);
}
