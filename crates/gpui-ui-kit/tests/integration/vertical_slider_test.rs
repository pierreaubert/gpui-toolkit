//! Integration tests for VerticalSlider component
//!
//! Tests the vertical slider component including:
//! - Basic rendering
//! - Size variants
//! - Value changes
//! - Linear and logarithmic scales
//! - Ticks display
//! - Selected state
//! - Disabled state
//! - Drag callbacks
//! - Reset callback
//! - Theme customization
//! - Scroll wheel interactions
//! - Keyboard navigation

use gpui::{
    Context, Modifiers, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase,
    VisualTestContext, Window, div, point, prelude::*, px,
};
use gpui_ui_kit::audio::vertical_slider::{
    VerticalSlider, VerticalSliderSize, VerticalSliderTheme,
};
use gpui_ui_kit::scale::Scale;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct VerticalSliderTestView;

impl Render for VerticalSliderTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            VerticalSlider::new("test-vslider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Volume"),
        )
    }
}

#[gpui::test]
async fn test_vertical_slider_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| VerticalSliderTestView);
}

// ============================================================================
// Size Variant Tests
// ============================================================================

#[gpui::test]
async fn test_vertical_slider_sizes(cx: &mut TestAppContext) {
    struct SizeTestView;

    impl Render for SizeTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VerticalSlider::new("sm-slider")
                        .size(VerticalSliderSize::Sm)
                        .value(50.0)
                        .label("Small"),
                )
                .child(
                    VerticalSlider::new("md-slider")
                        .size(VerticalSliderSize::Md)
                        .value(50.0)
                        .label("Medium"),
                )
                .child(
                    VerticalSlider::new("lg-slider")
                        .size(VerticalSliderSize::Lg)
                        .value(50.0)
                        .label("Large"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizeTestView);
}

#[gpui::test]
async fn test_vertical_slider_custom_height(cx: &mut TestAppContext) {
    struct CustomHeightView;

    impl Render for CustomHeightView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VerticalSlider::new("custom-height")
                    .height(200.0)
                    .value(50.0)
                    .label("Custom Height"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| CustomHeightView);
}

// ============================================================================
// Value Change Tests
// ============================================================================

/// View that tracks value changes
struct SliderChangeTestView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderChangeTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            VerticalSlider::new("change-test-slider")
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
async fn test_vertical_slider_value_change(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f64>> = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderChangeTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initial value should be 50
    assert_eq!(*value.borrow(), 50.0);
}

// ============================================================================
// Scale Tests
// ============================================================================

#[gpui::test]
async fn test_vertical_slider_linear_scale(cx: &mut TestAppContext) {
    struct LinearScaleView;

    impl Render for LinearScaleView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VerticalSlider::new("linear-slider")
                    .scale(Scale::Linear)
                    .value(50.0)
                    .min(0.0)
                    .max(100.0)
                    .label("Linear"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LinearScaleView);
}

#[gpui::test]
async fn test_vertical_slider_logarithmic_scale(cx: &mut TestAppContext) {
    struct LogScaleView;

    impl Render for LogScaleView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VerticalSlider::new("log-slider")
                    .scale(Scale::Logarithmic)
                    .value(1000.0)
                    .min(20.0)
                    .max(20000.0)
                    .unit("Hz")
                    .label("Frequency"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LogScaleView);
}

// ============================================================================
// Ticks Tests
// ============================================================================

#[gpui::test]
async fn test_vertical_slider_with_ticks(cx: &mut TestAppContext) {
    struct TicksView;

    impl Render for TicksView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VerticalSlider::new("ticks-linear")
                        .scale(Scale::Linear)
                        .with_ticks()
                        .value(50.0)
                        .min(0.0)
                        .max(100.0)
                        .label("Linear Ticks"),
                )
                .child(
                    VerticalSlider::new("ticks-log")
                        .scale(Scale::Logarithmic)
                        .with_ticks()
                        .value(1000.0)
                        .min(20.0)
                        .max(20000.0)
                        .label("Log Ticks"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| TicksView);
}

// ============================================================================
// Selected State Tests
// ============================================================================

#[gpui::test]
async fn test_vertical_slider_selected(cx: &mut TestAppContext) {
    struct SelectedView;

    impl Render for SelectedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VerticalSlider::new("not-selected")
                        .value(50.0)
                        .selected(false)
                        .label("Not Selected"),
                )
                .child(
                    VerticalSlider::new("selected")
                        .value(50.0)
                        .selected(true)
                        .label("Selected"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SelectedView);
}

/// Test on_select callback
struct SelectCallbackTestView {
    select_count: Arc<AtomicUsize>,
}

impl Render for SelectCallbackTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let select_count = self.select_count.clone();

        div().size_full().child(
            VerticalSlider::new("select-callback-slider")
                .value(50.0)
                .on_select(move |_window, _cx| {
                    select_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_vertical_slider_on_select(cx: &mut TestAppContext) {
    let select_count = Arc::new(AtomicUsize::new(0));
    let select_count_clone = select_count.clone();

    let _window = cx.add_window(move |_window, _cx| SelectCallbackTestView {
        select_count: select_count_clone,
    });

    // Initial state - no selections yet
    assert_eq!(select_count.load(Ordering::SeqCst), 0);
}

// ============================================================================
// Disabled State Tests
// ============================================================================

#[gpui::test]
async fn test_vertical_slider_disabled(cx: &mut TestAppContext) {
    struct DisabledView;

    impl Render for DisabledView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VerticalSlider::new("disabled-slider")
                    .value(50.0)
                    .disabled(true)
                    .label("Disabled"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DisabledView);
}

// ============================================================================
// Unit Display Tests
// ============================================================================

#[gpui::test]
async fn test_vertical_slider_units(cx: &mut TestAppContext) {
    struct UnitsView;

    impl Render for UnitsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VerticalSlider::new("unit-db")
                        .value(-6.0)
                        .min(-60.0)
                        .max(12.0)
                        .unit("dB")
                        .label("Gain"),
                )
                .child(
                    VerticalSlider::new("unit-hz")
                        .value(1000.0)
                        .min(20.0)
                        .max(20000.0)
                        .unit("Hz")
                        .scale(Scale::Logarithmic)
                        .label("Freq"),
                )
                .child(
                    VerticalSlider::new("unit-percent")
                        .value(0.5)
                        .min(0.0)
                        .max(1.0)
                        .unit("%")
                        .label("Mix"),
                )
                .child(
                    VerticalSlider::new("unit-ratio")
                        .value(4.0)
                        .min(1.0)
                        .max(20.0)
                        .unit(":1")
                        .label("Ratio"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| UnitsView);
}

// ============================================================================
// Shortcut Key Tests
// ============================================================================

#[gpui::test]
async fn test_vertical_slider_shortcut_key(cx: &mut TestAppContext) {
    struct ShortcutView;

    impl Render for ShortcutView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VerticalSlider::new("shortcut-g")
                        .value(0.0)
                        .min(-12.0)
                        .max(12.0)
                        .shortcut_key('g')
                        .label("Gain"),
                )
                .child(
                    VerticalSlider::new("shortcut-f")
                        .value(1000.0)
                        .min(20.0)
                        .max(20000.0)
                        .shortcut_key('f')
                        .label("Frequency"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| ShortcutView);
}

// ============================================================================
// Drag Callback Tests
// ============================================================================

/// Test on_drag_start callback
struct DragCallbackTestView {
    drag_started: Arc<AtomicBool>,
}

impl Render for DragCallbackTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let drag_started = self.drag_started.clone();

        div().size_full().child(
            VerticalSlider::new("drag-callback-slider")
                .value(50.0)
                .on_drag_start(move |_y, _value, _window, _cx| {
                    drag_started.store(true, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_vertical_slider_on_drag_start(cx: &mut TestAppContext) {
    let drag_started = Arc::new(AtomicBool::new(false));
    let drag_started_clone = drag_started.clone();

    let _window = cx.add_window(move |_window, _cx| DragCallbackTestView {
        drag_started: drag_started_clone,
    });

    // Initially drag hasn't started
    assert!(!drag_started.load(Ordering::SeqCst));
}

// ============================================================================
// Reset Callback Tests
// ============================================================================

/// Test on_reset callback (double-click)
struct ResetCallbackTestView {
    reset_count: Arc<AtomicUsize>,
}

impl Render for ResetCallbackTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let reset_count = self.reset_count.clone();

        div().size_full().child(
            VerticalSlider::new("reset-callback-slider")
                .value(75.0)
                .on_reset(move |_window, _cx| {
                    reset_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_vertical_slider_on_reset(cx: &mut TestAppContext) {
    let reset_count = Arc::new(AtomicUsize::new(0));
    let reset_count_clone = reset_count.clone();

    let _window = cx.add_window(move |_window, _cx| ResetCallbackTestView {
        reset_count: reset_count_clone,
    });

    // Initially no resets
    assert_eq!(reset_count.load(Ordering::SeqCst), 0);
}

// ============================================================================
// Theme Tests
// ============================================================================

#[gpui::test]
async fn test_vertical_slider_with_custom_theme(cx: &mut TestAppContext) {
    struct ThemedView;

    impl Render for ThemedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let custom_theme = VerticalSliderTheme {
                surface: gpui::rgba(0x2a2a2aff),
                surface_hover: gpui::rgba(0x3a3a3aff),
                track_bg: gpui::rgba(0x1a1a1aff),
                accent: gpui::rgba(0xff6600ff),
                accent_muted: gpui::rgba(0xff660033),
                border: gpui::rgba(0x444444ff),
                text_secondary: gpui::rgba(0xaaaaaaff),
                text_primary: gpui::rgba(0xffffffff),
                text_muted: gpui::rgba(0x888888ff),
                text_on_accent: gpui::rgba(0xffffffff),
                background_secondary: gpui::rgba(0x2a2a2aff),
                peak_marker: gpui::rgba(0xff6b6bff),
            };

            div().child(
                VerticalSlider::new("themed-slider")
                    .theme(custom_theme)
                    .value(50.0)
                    .label("Themed"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ThemedView);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[gpui::test]
async fn test_vertical_slider_value_clamping(cx: &mut TestAppContext) {
    struct ClampingView;

    impl Render for ClampingView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VerticalSlider::new("over-max")
                        .value(150.0) // Over max
                        .min(0.0)
                        .max(100.0)
                        .label("Over Max"),
                )
                .child(
                    VerticalSlider::new("under-min")
                        .value(-50.0) // Under min
                        .min(0.0)
                        .max(100.0)
                        .label("Under Min"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| ClampingView);
}

#[gpui::test]
async fn test_vertical_slider_zero_range(cx: &mut TestAppContext) {
    struct ZeroRangeView;

    impl Render for ZeroRangeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VerticalSlider::new("zero-range")
                    .value(50.0)
                    .min(50.0)
                    .max(50.0) // Same as min
                    .label("Zero Range"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ZeroRangeView);
}

#[gpui::test]
async fn test_vertical_slider_negative_range(cx: &mut TestAppContext) {
    struct NegativeRangeView;

    impl Render for NegativeRangeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VerticalSlider::new("negative-range")
                    .value(-30.0)
                    .min(-60.0)
                    .max(12.0)
                    .unit("dB")
                    .label("Gain"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NegativeRangeView);
}

#[gpui::test]
async fn test_vertical_slider_large_range(cx: &mut TestAppContext) {
    struct LargeRangeView;

    impl Render for LargeRangeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VerticalSlider::new("large-range")
                    .value(5000.0)
                    .min(20.0)
                    .max(20000.0)
                    .scale(Scale::Logarithmic)
                    .with_ticks()
                    .unit("Hz")
                    .label("Wide Range"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LargeRangeView);
}

#[gpui::test]
async fn test_vertical_slider_all_features(cx: &mut TestAppContext) {
    struct AllFeaturesView;

    impl Render for AllFeaturesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VerticalSlider::new("all-features")
                    .value(1000.0)
                    .min(20.0)
                    .max(20000.0)
                    .scale(Scale::Logarithmic)
                    .with_ticks()
                    .unit("Hz")
                    .label("Frequency")
                    .shortcut_key('f')
                    .size(VerticalSliderSize::Lg)
                    .selected(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllFeaturesView);
}

// ============================================================================
// INTERACTION TESTS - Click to Select
// ============================================================================

/// View that tracks select events
struct SliderSelectInteractionView {
    selected: Rc<RefCell<bool>>,
    select_count: Arc<AtomicUsize>,
}

impl Render for SliderSelectInteractionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = *self.selected.borrow();
        let selected_rc = self.selected.clone();
        let select_count = self.select_count.clone();

        div().size_full().child(
            VerticalSlider::new("select-interaction-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Select Test")
                .selected(is_selected)
                .on_select(move |_window, _cx| {
                    *selected_rc.borrow_mut() = true;
                    select_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test clicking vertical slider triggers on_select callback
#[gpui::test]
async fn test_vertical_slider_click_to_select(cx: &mut TestAppContext) {
    let selected = Rc::new(RefCell::new(false));
    let select_count = Arc::new(AtomicUsize::new(0));

    let selected_clone = selected.clone();
    let select_count_clone = select_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderSelectInteractionView {
        selected: selected_clone,
        select_count: select_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initially not selected
    assert!(!*selected.borrow(), "Should not be selected initially");

    // Click the slider to select
    if let Some(bounds) = cx.debug_bounds("select-interaction-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            select_count.load(Ordering::SeqCst),
            1,
            "on_select should have been called once"
        );
        assert!(*selected.borrow(), "Should be selected after click");
    }
}

// ============================================================================
// INTERACTION TESTS - Click to Step Value (when no drag handler)
// ============================================================================

/// View that tracks value changes via click
struct SliderClickStepView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderClickStepView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            VerticalSlider::new("click-step-slider")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .label("Click Step")
                // No on_select or on_drag_start, so click should step value
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test clicking vertical slider steps value by 10% when no drag handler
#[gpui::test]
async fn test_vertical_slider_click_to_step_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderClickStepView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initial value should be 50
    assert!(
        ((*value.borrow()) - 50.0).abs() < 0.01,
        "Initial value should be 50"
    );

    // Click the slider - should step value by 10% (10 units)
    if let Some(bounds) = cx.debug_bounds("click-step-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            change_count.load(Ordering::SeqCst),
            1,
            "on_change should have been called once"
        );
        // Value should have increased by 10% (10 units on 0-100 range)
        let new_val = *value.borrow();
        assert!(
            (new_val - 60.0).abs() < 0.01,
            "Value should be 60 after click step, got {}",
            new_val
        );
    }
}

// ============================================================================
// INTERACTION TESTS - Drag Start Callback
// ============================================================================

/// View that tracks drag start events
struct SliderDragStartView {
    drag_started: Arc<AtomicBool>,
    drag_y: Rc<RefCell<Option<f32>>>,
    drag_value: Rc<RefCell<Option<f64>>>,
}

impl Render for SliderDragStartView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let drag_started = self.drag_started.clone();
        let drag_y = self.drag_y.clone();
        let drag_value = self.drag_value.clone();

        div().size_full().child(
            VerticalSlider::new("drag-start-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Drag Test")
                .on_drag_start(move |y, value, _window, _cx| {
                    drag_started.store(true, Ordering::SeqCst);
                    *drag_y.borrow_mut() = Some(y);
                    *drag_value.borrow_mut() = Some(value);
                }),
        )
    }
}

/// Test mouse down triggers on_drag_start callback
#[gpui::test]
async fn test_vertical_slider_drag_start_callback(cx: &mut TestAppContext) {
    let drag_started = Arc::new(AtomicBool::new(false));
    let drag_y: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));
    let drag_value: Rc<RefCell<Option<f64>>> = Rc::new(RefCell::new(None));

    let drag_started_clone = drag_started.clone();
    let drag_y_clone = drag_y.clone();
    let drag_value_clone = drag_value.clone();

    let window = cx.add_window(move |_window, _cx| SliderDragStartView {
        drag_started: drag_started_clone,
        drag_y: drag_y_clone,
        drag_value: drag_value_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Mouse down should trigger drag start
    if let Some(bounds) = cx.debug_bounds("drag-start-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        assert!(
            drag_started.load(Ordering::SeqCst),
            "Drag should have started"
        );
        assert!(drag_y.borrow().is_some(), "Drag Y position should be set");
        assert!(
            drag_value.borrow().is_some(),
            "Drag value should be captured"
        );
        assert!(
            (drag_value.borrow().unwrap() - 50.0).abs() < 0.01,
            "Drag value should be 50"
        );
    }
}

/// Test combined select and drag start callbacks
struct SliderSelectAndDragView {
    select_count: Arc<AtomicUsize>,
    drag_started: Arc<AtomicBool>,
}

impl Render for SliderSelectAndDragView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let select_count = self.select_count.clone();
        let drag_started = self.drag_started.clone();

        div().size_full().child(
            VerticalSlider::new("select-drag-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Select+Drag")
                .on_select(move |_window, _cx| {
                    select_count.fetch_add(1, Ordering::SeqCst);
                })
                .on_drag_start({
                    let drag_started = drag_started.clone();
                    move |_y, _value, _window, _cx| {
                        drag_started.store(true, Ordering::SeqCst);
                    }
                }),
        )
    }
}

/// Test both on_select and on_drag_start fire on mouse down
#[gpui::test]
async fn test_vertical_slider_select_and_drag_start_combined(cx: &mut TestAppContext) {
    let select_count = Arc::new(AtomicUsize::new(0));
    let drag_started = Arc::new(AtomicBool::new(false));

    let select_count_clone = select_count.clone();
    let drag_started_clone = drag_started.clone();

    let window = cx.add_window(move |_window, _cx| SliderSelectAndDragView {
        select_count: select_count_clone,
        drag_started: drag_started_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("select-drag-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            select_count.load(Ordering::SeqCst),
            1,
            "on_select should have been called"
        );
        assert!(
            drag_started.load(Ordering::SeqCst),
            "on_drag_start should have been called"
        );
    }
}

// ============================================================================
// INTERACTION TESTS - Double-Click Reset
// ============================================================================

/// View that tracks reset events
struct SliderResetView {
    value: Rc<RefCell<f64>>,
    reset_count: Arc<AtomicUsize>,
}

impl Render for SliderResetView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let reset_count = self.reset_count.clone();

        div().size_full().child(
            VerticalSlider::new("reset-slider")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .label("Reset Test")
                .on_reset(move |_window, _cx| {
                    *value_rc.borrow_mut() = 50.0; // Reset to default
                    reset_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test double-click triggers on_reset callback
#[gpui::test]
async fn test_vertical_slider_double_click_reset(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(75.0)); // Start at non-default
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderResetView {
        value: value_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initial value is 75
    assert!(
        ((*value.borrow()) - 75.0).abs() < 0.01,
        "Initial value should be 75"
    );

    // Double-click to reset
    if let Some(bounds) = cx.debug_bounds("reset-slider") {
        let center = bounds.center();

        // First click
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Second click (double-click)
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Note: The component uses on_click with click_count() == 2
        // GPUI test framework handles this via the click event simulation
    }
}

// ============================================================================
// INTERACTION TESTS - Keyboard Navigation
// ============================================================================

/// View for keyboard navigation tests
struct SliderKeyboardView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
    reset_count: Arc<AtomicUsize>,
}

impl Render for SliderKeyboardView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();
        let reset_count_rc = self.reset_count.clone();

        div().size_full().child(
            VerticalSlider::new("keyboard-slider")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .label("Keyboard Test")
                .selected(true) // Must be selected for keyboard navigation
                .on_change({
                    let value_rc = value_rc.clone();
                    let change_count = change_count.clone();
                    move |new_val, _window, _cx| {
                        *value_rc.borrow_mut() = new_val;
                        change_count.fetch_add(1, Ordering::SeqCst);
                    }
                })
                .on_reset(move |_window, _cx| {
                    *value_rc.borrow_mut() = 50.0;
                    reset_count_rc.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test Arrow Up key increases value when selected
#[gpui::test]
async fn test_vertical_slider_arrow_up_increases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Focus the slider first by clicking
    if let Some(bounds) = cx.debug_bounds("keyboard-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press Arrow Up - should increase value by 5%
        cx.simulate_keystrokes("up");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val > 50.0,
            "Value should increase after Arrow Up, got {}",
            new_val
        );
    }
}

/// Test Arrow Down key decreases value when selected
#[gpui::test]
async fn test_vertical_slider_arrow_down_decreases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press Arrow Down - should decrease value by 5%
        cx.simulate_keystrokes("down");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val < 50.0,
            "Value should decrease after Arrow Down, got {}",
            new_val
        );
    }
}

/// Test Home key sets value to minimum
#[gpui::test]
async fn test_vertical_slider_home_sets_minimum(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press Home - should set to min (0)
        cx.simulate_keystrokes("home");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            (new_val - 0.0).abs() < 0.01,
            "Value should be 0 after Home, got {}",
            new_val
        );
    }
}

/// Test End key sets value to maximum
#[gpui::test]
async fn test_vertical_slider_end_sets_maximum(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press End - should set to max (100)
        cx.simulate_keystrokes("end");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            (new_val - 100.0).abs() < 0.01,
            "Value should be 100 after End, got {}",
            new_val
        );
    }
}

/// Test Escape key triggers reset and resets value to default
#[gpui::test]
async fn test_vertical_slider_escape_resets_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(75.0)); // Start at non-default value
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initial value should be 75 (non-default)
    assert!(
        ((*value.borrow()) - 75.0).abs() < 0.01,
        "Initial value should be 75"
    );

    if let Some(bounds) = cx.debug_bounds("keyboard-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press Escape - should trigger reset to default (50.0)
        cx.simulate_keystrokes("escape");
        cx.run_until_parked();

        assert!(
            reset_count.load(Ordering::SeqCst) > 0,
            "Reset callback should have been triggered"
        );

        // Verify value was actually reset to default (50.0)
        let new_val = *value.borrow();
        assert!(
            (new_val - 50.0).abs() < 0.01,
            "Value should be reset to default (50.0), got {}",
            new_val
        );
    }
}

/// Test Arrow Right increases value (alternative to Up)
#[gpui::test]
async fn test_vertical_slider_arrow_right_increases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("right");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val > 50.0,
            "Value should increase after Arrow Right, got {}",
            new_val
        );
    }
}

/// Test Arrow Left decreases value (alternative to Down)
#[gpui::test]
async fn test_vertical_slider_arrow_left_decreases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("left");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val < 50.0,
            "Value should decrease after Arrow Left, got {}",
            new_val
        );
    }
}

// ============================================================================
// INTERACTION TESTS - Disabled State
// ============================================================================

/// View with disabled slider that tracks interactions
struct SliderDisabledView {
    change_count: Arc<AtomicUsize>,
    select_count: Arc<AtomicUsize>,
}

impl Render for SliderDisabledView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();
        let select_count = self.select_count.clone();

        div().size_full().child(
            VerticalSlider::new("disabled-interaction-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Disabled")
                .disabled(true)
                .on_change(move |_, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                })
                .on_select(move |_window, _cx| {
                    select_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test disabled slider ignores all interactions
#[gpui::test]
async fn test_vertical_slider_disabled_ignores_clicks(cx: &mut TestAppContext) {
    let change_count = Arc::new(AtomicUsize::new(0));
    let select_count = Arc::new(AtomicUsize::new(0));

    let change_count_clone = change_count.clone();
    let select_count_clone = select_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderDisabledView {
        change_count: change_count_clone,
        select_count: select_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Try clicking the disabled slider
    if let Some(bounds) = cx.debug_bounds("disabled-interaction-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // No callbacks should have been triggered
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            0,
            "Disabled slider should not trigger on_change"
        );
        assert_eq!(
            select_count.load(Ordering::SeqCst),
            0,
            "Disabled slider should not trigger on_select"
        );
    }
}

// ============================================================================
// INTERACTION TESTS - Logarithmic Scale
// ============================================================================

/// View for logarithmic scale tests
struct SliderLogScaleView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderLogScaleView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            VerticalSlider::new("log-scale-slider")
                .value(current_value)
                .min(20.0)
                .max(20000.0)
                .scale(Scale::Logarithmic)
                .unit("Hz")
                .label("Frequency")
                .selected(true)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test logarithmic scale keyboard navigation uses scale-aware stepping
#[gpui::test]
async fn test_vertical_slider_log_scale_keyboard_step(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(1000.0)); // 1kHz
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderLogScaleView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("log-scale-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        let initial_value = *value.borrow();

        // Press Up - should use logarithmic stepping
        cx.simulate_keystrokes("up");
        cx.run_until_parked();

        let new_value = *value.borrow();
        assert!(
            new_value > initial_value,
            "Log scale: value should increase, initial={}, new={}",
            initial_value,
            new_value
        );
        // Logarithmic stepping should give a larger absolute change
    }
}

/// Test logarithmic scale Home sets to minimum
#[gpui::test]
async fn test_vertical_slider_log_scale_home_sets_min(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(1000.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderLogScaleView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("log-scale-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("home");
        cx.run_until_parked();

        let new_value = *value.borrow();
        assert!(
            (new_value - 20.0).abs() < 0.01,
            "Log scale: Home should set to min (20Hz), got {}",
            new_value
        );
    }
}

/// Test logarithmic scale End sets to maximum
#[gpui::test]
async fn test_vertical_slider_log_scale_end_sets_max(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(1000.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderLogScaleView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("log-scale-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("end");
        cx.run_until_parked();

        let new_value = *value.borrow();
        assert!(
            (new_value - 20000.0).abs() < 0.01,
            "Log scale: End should set to max (20kHz), got {}",
            new_value
        );
    }
}

// ============================================================================
// INTERACTION TESTS - Multiple Sequential Interactions
// ============================================================================

/// Test multiple arrow key presses accumulate value changes
#[gpui::test]
async fn test_vertical_slider_multiple_arrow_key_presses(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press Up 3 times
        for _ in 0..3 {
            cx.simulate_keystrokes("up");
            cx.run_until_parked();
        }

        let new_val = *value.borrow();
        assert!(
            new_val > 60.0,
            "Value should increase significantly after 3 Up presses, got {}",
            new_val
        );
        assert!(
            change_count.load(Ordering::SeqCst) >= 3,
            "on_change should have been called at least 3 times"
        );
    }
}

/// Test value clamping at boundaries via keyboard
#[gpui::test]
async fn test_vertical_slider_keyboard_respects_bounds(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(95.0)); // Near max
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-slider") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press Up multiple times - should clamp at max (100)
        for _ in 0..5 {
            cx.simulate_keystrokes("up");
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

// ============================================================================
// INTERACTION TESTS - Scroll Wheel
// ============================================================================

/// View for scroll wheel tests
struct SliderScrollWheelView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderScrollWheelView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            VerticalSlider::new("scroll-wheel-slider")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .label("Scroll Test")
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test scroll wheel up (negative delta) increases value
#[gpui::test]
async fn test_vertical_slider_scroll_wheel_up_increases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initial value should be 50
    assert!(
        ((*value.borrow()) - 50.0).abs() < 0.01,
        "Initial value should be 50"
    );

    // Scroll wheel up (negative Y delta) should increase value
    if let Some(bounds) = cx.debug_bounds("scroll-wheel-slider") {
        let center = bounds.center();

        // Simulate scroll wheel up (negative delta = increase value)
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

/// Test scroll wheel down (positive delta) decreases value
#[gpui::test]
async fn test_vertical_slider_scroll_wheel_down_decreases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Scroll wheel down (positive Y delta) should decrease value
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

/// Test scroll wheel with Shift modifier for fine control
#[gpui::test]
async fn test_vertical_slider_scroll_wheel_shift_fine_control(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
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

        // Scroll with Shift held - should use smaller step (0.5% instead of 5%)
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
        // With shift, step is 0.5% of range (0.5 on 0-100 range)
        // Value should increase but by a smaller amount than normal scroll
        assert!(
            new_val > 50.0 && new_val < 51.0, // Fine control = small change
            "Shift+scroll should give fine control, got {}",
            new_val
        );
    }
}

/// Test multiple scroll wheel events accumulate
#[gpui::test]
async fn test_vertical_slider_scroll_wheel_multiple_events(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
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
        // Each scroll should add ~5% (5 units on 0-100 range)
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

/// Test scroll wheel respects min/max bounds
#[gpui::test]
async fn test_vertical_slider_scroll_wheel_respects_bounds(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(95.0)); // Near max
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

        // Scroll up many times - should clamp at max (100)
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

/// Test scroll wheel with pixel delta (trackpad)
#[gpui::test]
async fn test_vertical_slider_scroll_wheel_pixel_delta(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
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

        // Simulate trackpad scroll (pixel delta)
        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Pixels(point(gpui::px(0.0), gpui::px(-10.0))), // Negative = up
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val > 50.0,
            "Value should increase after pixel scroll up, got {}",
            new_val
        );
    }
}

/// View for disabled scroll wheel test
struct SliderScrollWheelDisabledView {
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderScrollWheelDisabledView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();

        div().size_full().child(
            VerticalSlider::new("scroll-wheel-disabled-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Disabled Scroll")
                .disabled(true)
                .on_change(move |_, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test scroll wheel is ignored on disabled slider
#[gpui::test]
async fn test_vertical_slider_scroll_wheel_disabled_ignored(cx: &mut TestAppContext) {
    let change_count = Arc::new(AtomicUsize::new(0));
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelDisabledView {
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-disabled-slider") {
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

/// View for logarithmic scroll wheel test
struct SliderScrollWheelLogView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderScrollWheelLogView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            VerticalSlider::new("scroll-wheel-log-slider")
                .value(current_value)
                .min(20.0)
                .max(20000.0)
                .scale(Scale::Logarithmic)
                .unit("Hz")
                .label("Frequency")
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test scroll wheel on logarithmic scale uses scale-aware stepping
#[gpui::test]
async fn test_vertical_slider_scroll_wheel_log_scale(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(1000.0)); // 1kHz
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderScrollWheelLogView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-log-slider") {
        let center = bounds.center();
        let initial_value = *value.borrow();

        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(0.0, -1.0)),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val > initial_value,
            "Log scale: value should increase after scroll up, initial={}, new={}",
            initial_value,
            new_val
        );
        // Logarithmic scale should give a proportional change
    }
}

// ============================================================================
// INTERACTION TESTS - Percentage Unit Bounds
// ============================================================================

/// View for percentage unit tests
struct SliderPercentageView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderPercentageView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            VerticalSlider::new("percentage-slider")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .unit("%")
                .label("Percentage")
                .selected(true)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test percentage unit value stays between 0 and 100 when scrolling up at max
#[gpui::test]
async fn test_vertical_slider_percentage_clamped_at_max(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(100.0)); // Start at max
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

        // Try to scroll up past 100%
        for _ in 0..5 {
            cx.simulate_event(ScrollWheelEvent {
                position: center,
                delta: ScrollDelta::Lines(point(0.0, -1.0)), // Scroll up
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            });
            cx.run_until_parked();
        }

        let final_val = *value.borrow();
        assert!(
            final_val <= 100.0,
            "Percentage value should not exceed 100%, got {}",
            final_val
        );
        assert!(
            (final_val - 100.0).abs() < 0.01,
            "Percentage value should be clamped at 100%, got {}",
            final_val
        );
    }
}

/// Test percentage unit value stays between 0 and 100 when scrolling down at min
#[gpui::test]
async fn test_vertical_slider_percentage_clamped_at_min(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(0.0)); // Start at min
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

        // Try to scroll down past 0%
        for _ in 0..5 {
            cx.simulate_event(ScrollWheelEvent {
                position: center,
                delta: ScrollDelta::Lines(point(0.0, 1.0)), // Scroll down
                modifiers: Modifiers::default(),
                touch_phase: TouchPhase::Moved,
            });
            cx.run_until_parked();
        }

        let final_val = *value.borrow();
        assert!(
            final_val >= 0.0,
            "Percentage value should not go below 0%, got {}",
            final_val
        );
        assert!(
            final_val.abs() < 0.01,
            "Percentage value should be clamped at 0%, got {}",
            final_val
        );
    }
}

/// Test percentage unit value stays between 0 and 100 with keyboard (End key)
#[gpui::test]
async fn test_vertical_slider_percentage_end_key_at_100(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
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
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press End - should set to exactly 100%
        cx.simulate_keystrokes("end");
        cx.run_until_parked();

        let final_val = *value.borrow();
        assert!(
            (final_val - 100.0).abs() < 0.01,
            "End key should set percentage to exactly 100%, got {}",
            final_val
        );
    }
}

/// Test percentage unit value stays between 0 and 100 with keyboard (Home key)
#[gpui::test]
async fn test_vertical_slider_percentage_home_key_at_0(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
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
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press Home - should set to exactly 0%
        cx.simulate_keystrokes("home");
        cx.run_until_parked();

        let final_val = *value.borrow();
        assert!(
            final_val.abs() < 0.01,
            "Home key should set percentage to exactly 0%, got {}",
            final_val
        );
    }
}

/// Test percentage unit value stays in range after multiple arrow key presses
#[gpui::test]
async fn test_vertical_slider_percentage_arrow_keys_respect_bounds(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(95.0)); // Start near max
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
        cx.simulate_mouse_down(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.simulate_mouse_up(center, gpui::MouseButton::Left, gpui::Modifiers::default());
        cx.run_until_parked();

        // Press Up many times - should clamp at 100%
        for _ in 0..10 {
            cx.simulate_keystrokes("up");
            cx.run_until_parked();
        }

        let final_val = *value.borrow();
        assert!(
            final_val >= 0.0 && final_val <= 100.0,
            "Percentage value should stay between 0% and 100%, got {}",
            final_val
        );
        assert!(
            (final_val - 100.0).abs() < 0.01,
            "Percentage value should be clamped at 100%, got {}",
            final_val
        );
    }
}

/// Test horizontal scroll (X delta) is handled when Y is zero (macOS Shift+scroll)
#[gpui::test]
async fn test_vertical_slider_scroll_wheel_horizontal_fallback(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
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

        // On macOS, Shift+scroll converts vertical to horizontal
        // The component should fall back to X delta when Y is 0
        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(-1.0, 0.0)), // X delta only
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        let new_val = *value.borrow();
        // Negative X delta = increase value (same as negative Y)
        assert!(
            new_val > 50.0,
            "Value should increase with X delta when Y is 0, got {}",
            new_val
        );
    }
}

// ============================================================================
// INTERACTION TESTS - Click-to-Position on Track
// ============================================================================

/// View for testing click-to-position on the track
struct SliderTrackClickView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for SliderTrackClickView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            VerticalSlider::new("track-click-slider")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .label("Track Click")
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

/// Test clicking on the track sets value based on click position
#[gpui::test]
async fn test_vertical_slider_track_click_sets_position(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderTrackClickView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Click on the track element specifically
    if let Some(track_bounds) = cx.debug_bounds("track-click-slider-track") {
        // Click near the top of the track (should give high value)
        let top_point = point(track_bounds.center().x, track_bounds.origin.y + px(5.0));
        cx.simulate_mouse_down(
            top_point,
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.run_until_parked();

        let high_val = *value.borrow();
        assert!(
            high_val > 70.0,
            "Clicking near top of track should give high value (>70), got {}",
            high_val
        );

        // Click near the bottom of the track (should give low value)
        let bottom_point = point(
            track_bounds.center().x,
            track_bounds.origin.y + track_bounds.size.height - px(5.0),
        );
        cx.simulate_mouse_down(
            bottom_point,
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.run_until_parked();

        let low_val = *value.borrow();
        assert!(
            low_val < 30.0,
            "Clicking near bottom of track should give low value (<30), got {}",
            low_val
        );

        assert!(
            change_count.load(Ordering::SeqCst) >= 2,
            "on_change should have been called at least twice"
        );
    }
}

/// Test dragging on the track continuously updates the value
#[gpui::test]
async fn test_vertical_slider_track_drag(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderTrackClickView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(track_bounds) = cx.debug_bounds("track-click-slider-track") {
        // Start drag at center
        let start_point = track_bounds.center();
        cx.simulate_mouse_down(
            start_point,
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.run_until_parked();

        let start_val = *value.borrow();

        // Move mouse up (should increase value)
        let drag_up_point = point(track_bounds.center().x, track_bounds.origin.y + px(10.0));
        cx.simulate_mouse_move(
            drag_up_point,
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        cx.run_until_parked();

        let after_drag_val = *value.borrow();
        assert!(
            after_drag_val > start_val,
            "Dragging up should increase value. Start: {}, After: {}",
            start_val,
            after_drag_val
        );
    }
}

/// Test scroll wheel on track element stops propagation
#[gpui::test]
async fn test_vertical_slider_track_scroll_wheel(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| SliderTrackClickView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(track_bounds) = cx.debug_bounds("track-click-slider-track") {
        let center = track_bounds.center();

        // Scroll wheel up on track (negative Y = increase value)
        cx.simulate_event(ScrollWheelEvent {
            position: center,
            delta: ScrollDelta::Lines(point(0.0, -1.0)),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val > 50.0,
            "Scroll wheel on track should increase value, got {}",
            new_val
        );
        assert!(
            change_count.load(Ordering::SeqCst) >= 1,
            "on_change should have been called"
        );
    }
}
