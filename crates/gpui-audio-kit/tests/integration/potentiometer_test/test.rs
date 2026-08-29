use super::disabled_pot_test_view::DisabledPotTestView;
use super::pot_drag_test_view::PotDragTestView;
use super::pot_keyboard_view::PotKeyboardView;
use super::pot_log_scale_view::PotLogScaleView;
use super::pot_percentage_view::PotPercentageView;
use super::pot_reset_test_view::PotResetTestView;
use super::pot_scroll_wheel_view::PotScrollWheelView;
use super::pot_select_test_view::PotSelectTestView;
use super::pot_value_change_test_view::PotValueChangeTestView;
use super::potentiometer_test_view::PotentiometerTestView;
use gpui::{
    Context, Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase,
    VisualTestContext, Window, div, point, prelude::*,
};
use gpui_audio_kit::audio::potentiometer::{Potentiometer, PotentiometerScale, PotentiometerSize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

struct OutsideReleasePotView {
    value: Rc<RefCell<f64>>,
    change_count: Arc<AtomicUsize>,
    commit_count: Arc<AtomicUsize>,
}

impl Render for OutsideReleasePotView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.value.clone();
        let change_count = self.change_count.clone();
        let commit_count = self.commit_count.clone();

        div().size_full().relative().child(
            div()
                .absolute()
                .left(gpui::px(100.0))
                .top(gpui::px(100.0))
                .child(
                    Potentiometer::new("outside-release-pot")
                        .value(*self.value.borrow())
                        .min(0.0)
                        .max(100.0)
                        .label("Outside release")
                        .on_change(move |next_value, _window, _cx| {
                            *value.borrow_mut() = next_value;
                            change_count.fetch_add(1, Ordering::SeqCst);
                        })
                        .on_commit(move |_value, _window, _cx| {
                            commit_count.fetch_add(1, Ordering::SeqCst);
                        }),
                ),
        )
    }
}

#[gpui::test]
async fn test_potentiometer_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| PotentiometerTestView);
}

#[gpui::test]
async fn test_potentiometer_commits_drag_released_outside(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let commit_count = Arc::new(AtomicUsize::new(0));
    let window = cx.add_window({
        let value = value.clone();
        let change_count = change_count.clone();
        let commit_count = commit_count.clone();
        move |_window, _cx| OutsideReleasePotView {
            value,
            change_count,
            commit_count,
        }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let start = point(gpui::px(120.0), gpui::px(120.0));
    let release_outside = point(gpui::px(300.0), gpui::px(300.0));

    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
    cx.simulate_mouse_up(release_outside, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    assert_ne!(
        *value.borrow(),
        50.0,
        "outside release must apply the final drag delta"
    );
    assert_eq!(change_count.load(Ordering::SeqCst), 1);
    assert_eq!(commit_count.load(Ordering::SeqCst), 1);
}

/// Test different size variants
#[gpui::test]
async fn test_potentiometer_sizes(cx: &mut TestAppContext) {
    struct SizeTestView;

    impl Render for SizeTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    Potentiometer::new("pot-sm")
                        .size(PotentiometerSize::Sm)
                        .value(25.0)
                        .label("Small"),
                )
                .child(
                    Potentiometer::new("pot-md")
                        .size(PotentiometerSize::Md)
                        .value(50.0)
                        .label("Medium"),
                )
                .child(
                    Potentiometer::new("pot-lg")
                        .size(PotentiometerSize::Lg)
                        .value(75.0)
                        .label("Large"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizeTestView);
}

#[gpui::test]
async fn test_potentiometer_click_to_increment(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotValueChangeTestView {
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

    // Click the potentiometer to increment (10% step = +10 on 0-100 range)
    if let Some(bounds) = cx.debug_bounds("change-test-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            change_count.load(Ordering::SeqCst),
            1,
            "on_change should have been called"
        );
        // Value should have increased by 10% (10 units on 0-100 range)
    }
}

#[gpui::test]
async fn test_potentiometer_on_select(cx: &mut TestAppContext) {
    let selected = Rc::new(RefCell::new(false));
    let select_count = Arc::new(AtomicUsize::new(0));

    let selected_clone = selected.clone();
    let select_count_clone = select_count.clone();

    let window = cx.add_window(move |_window, _cx| PotSelectTestView {
        selected: selected_clone,
        select_count: select_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initially not selected
    assert!(!*selected.borrow(), "Should not be selected initially");

    // Click to select
    if let Some(bounds) = cx.debug_bounds("select-test-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            select_count.load(Ordering::SeqCst),
            1,
            "on_select should have been called"
        );
        assert!(*selected.borrow(), "Should be selected after click");
    }
}

#[gpui::test]
async fn test_potentiometer_double_click_reset(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(75.0)); // Start not at default
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| PotResetTestView {
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

    // Double-click to reset (simulated via click with count=2 handled by component)
    // Note: Testing double-click requires the component's on_click handler to check click_count
    if let Some(bounds) = cx.debug_bounds("reset-test-pot") {
        let center = bounds.center();
        // Simulate two quick clicks
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
    }
}

#[gpui::test]
async fn test_potentiometer_linear_scale(cx: &mut TestAppContext) {
    struct LinearScaleView;

    impl Render for LinearScaleView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Potentiometer::new("linear-pot")
                    .value(50.0)
                    .min(0.0)
                    .max(100.0)
                    .scale(PotentiometerScale::Linear)
                    .label("Linear"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LinearScaleView);
}

#[gpui::test]
async fn test_potentiometer_logarithmic_scale(cx: &mut TestAppContext) {
    struct LogScaleView;

    impl Render for LogScaleView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Potentiometer::new("log-pot")
                    .value(1000.0)
                    .min(20.0) // Must be > 0 for log scale
                    .max(20000.0)
                    .scale(PotentiometerScale::Logarithmic)
                    .unit("Hz")
                    .label("Frequency"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LogScaleView);
}

#[gpui::test]
async fn test_potentiometer_selected_appearance(cx: &mut TestAppContext) {
    struct SelectedView;

    impl Render for SelectedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    Potentiometer::new("unselected-pot")
                        .value(50.0)
                        .selected(false)
                        .label("Unselected"),
                )
                .child(
                    Potentiometer::new("selected-pot")
                        .value(50.0)
                        .selected(true)
                        .label("Selected"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SelectedView);
}

#[gpui::test]
async fn test_potentiometer_disabled_state(cx: &mut TestAppContext) {
    let change_count = Arc::new(AtomicUsize::new(0));
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| DisabledPotTestView {
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Try clicking the disabled potentiometer
    if let Some(bounds) = cx.debug_bounds("disabled-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // on_change should NOT have been called
        assert_eq!(
            change_count.load(Ordering::SeqCst),
            0,
            "Disabled potentiometer should not trigger on_change"
        );
    }
}

#[gpui::test]
async fn test_potentiometer_with_units(cx: &mut TestAppContext) {
    struct UnitsView;

    impl Render for UnitsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    Potentiometer::new("pot-percent")
                        .value(50.0)
                        .min(0.0)
                        .max(100.0)
                        .unit("%")
                        .label("Percent"),
                )
                .child(
                    Potentiometer::new("pot-db")
                        .value(0.0)
                        .min(-60.0)
                        .max(12.0)
                        .unit("dB")
                        .label("Gain"),
                )
                .child(
                    Potentiometer::new("pot-hz")
                        .value(1000.0)
                        .min(20.0)
                        .max(20000.0)
                        .unit("Hz")
                        .label("Frequency"),
                )
                .child(
                    Potentiometer::new("pot-ratio")
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

#[gpui::test]
async fn test_potentiometer_with_shortcut_key(cx: &mut TestAppContext) {
    struct ShortcutView;

    impl Render for ShortcutView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    Potentiometer::new("pot-a")
                        .value(50.0)
                        .label("Attack")
                        .shortcut_key('a'),
                )
                .child(
                    Potentiometer::new("pot-r")
                        .value(50.0)
                        .label("Release")
                        .shortcut_key('r'),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| ShortcutView);
}

#[gpui::test]
async fn test_potentiometer_drag_start(cx: &mut TestAppContext) {
    let drag_started = Arc::new(AtomicBool::new(false));
    let drag_y: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));

    let drag_started_clone = drag_started.clone();
    let drag_y_clone = drag_y.clone();

    let window = cx.add_window(move |_window, _cx| PotDragTestView {
        drag_started: drag_started_clone,
        drag_y: drag_y_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Mouse down should trigger drag start
    if let Some(bounds) = cx.debug_bounds("drag-test-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(
            drag_started.load(Ordering::SeqCst),
            "Drag should have started"
        );
        assert!(drag_y.borrow().is_some(), "Drag Y position should be set");
    }
}

#[gpui::test]
async fn test_potentiometer_with_custom_theme(cx: &mut TestAppContext) {
    use gpui_audio_kit::audio::potentiometer::PotentiometerTheme;

    struct ThemedPotView;

    impl Render for ThemedPotView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let custom_theme = PotentiometerTheme {
                surface: gpui::rgba(0x1a1a1aff),
                surface_hover: gpui::rgba(0x2a2a2aff),
                knob_bg: gpui::rgba(0x333333ff),
                accent: gpui::rgba(0xff6600ff), // Orange accent
                accent_muted: gpui::rgba(0xff660033),
                border: gpui::rgba(0x444444ff),
                text_secondary: gpui::rgba(0xaaaaaaff),
                text_primary: gpui::rgba(0xffffffff),
                text_muted: gpui::rgba(0x888888ff),
                text_on_accent: gpui::rgba(0xffffffff),
                background_secondary: gpui::rgba(0x222222ff),
            };

            div().child(
                Potentiometer::new("themed-pot")
                    .value(50.0)
                    .min(0.0)
                    .max(100.0)
                    .label("Themed")
                    .theme(custom_theme),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ThemedPotView);
}

#[gpui::test]
async fn test_potentiometer_value_clamping(cx: &mut TestAppContext) {
    struct ClampingView;

    impl Render for ClampingView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .child(
                    Potentiometer::new("pot-below-min")
                        .value(-10.0) // Below min
                        .min(0.0)
                        .max(100.0)
                        .label("Below Min"),
                )
                .child(
                    Potentiometer::new("pot-above-max")
                        .value(200.0) // Above max
                        .min(0.0)
                        .max(100.0)
                        .label("Above Max"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| ClampingView);
}

#[gpui::test]
async fn test_potentiometer_negative_range(cx: &mut TestAppContext) {
    struct NegativeRangeView;

    impl Render for NegativeRangeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Potentiometer::new("pot-negative")
                    .value(0.0)
                    .min(-60.0)
                    .max(12.0)
                    .unit("dB")
                    .label("Gain"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NegativeRangeView);
}

/// Test scroll wheel up increases value
#[gpui::test]
async fn test_potentiometer_scroll_wheel_up_increases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(
        ((*value.borrow()) - 50.0).abs() < 0.01,
        "Initial value should be 50"
    );

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-pot") {
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
async fn test_potentiometer_scroll_wheel_down_decreases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-pot") {
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
async fn test_potentiometer_scroll_wheel_shift_fine_control(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-pot") {
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

/// Test scroll wheel respects bounds
#[gpui::test]
async fn test_potentiometer_scroll_wheel_respects_bounds(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(95.0)); // Near max
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-pot") {
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

/// Test scroll wheel on disabled potentiometer is ignored
#[gpui::test]
async fn test_potentiometer_scroll_wheel_disabled_ignored(cx: &mut TestAppContext) {
    struct DisabledScrollView {
        change_count: Arc<AtomicUsize>,
    }

    impl Render for DisabledScrollView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let change_count = self.change_count.clone();

            div().size_full().child(
                Potentiometer::new("scroll-disabled-pot")
                    .value(50.0)
                    .min(0.0)
                    .max(100.0)
                    .disabled(true)
                    .label("Disabled")
                    .on_change(move |_, _window, _cx| {
                        change_count.fetch_add(1, Ordering::SeqCst);
                    }),
            )
        }
    }

    let change_count = Arc::new(AtomicUsize::new(0));
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| DisabledScrollView {
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-disabled-pot") {
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
            "Disabled potentiometer should not respond to scroll wheel"
        );
    }
}

/// Test Arrow Up increases value when selected
#[gpui::test]
async fn test_potentiometer_arrow_up_increases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| PotKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

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

/// Test Arrow Down decreases value when selected
#[gpui::test]
async fn test_potentiometer_arrow_down_decreases_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| PotKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

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
async fn test_potentiometer_home_sets_minimum(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| PotKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("home");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val.abs() < 0.01,
            "Home should set value to min (0), got {}",
            new_val
        );
    }
}

/// Test End key sets value to maximum
#[gpui::test]
async fn test_potentiometer_end_sets_maximum(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| PotKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("end");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            (new_val - 100.0).abs() < 0.01,
            "End should set value to max (100), got {}",
            new_val
        );
    }
}

/// Test Escape key triggers reset and resets value to default
#[gpui::test]
async fn test_potentiometer_escape_resets_value(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(75.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let reset_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();
    let reset_count_clone = reset_count.clone();

    let window = cx.add_window(move |_window, _cx| PotKeyboardView {
        value: value_clone,
        change_count: change_count_clone,
        reset_count: reset_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(
        ((*value.borrow()) - 75.0).abs() < 0.01,
        "Initial value should be 75"
    );

    if let Some(bounds) = cx.debug_bounds("keyboard-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("escape");
        cx.run_until_parked();

        assert!(
            reset_count.load(Ordering::SeqCst) > 0,
            "Reset callback should have been triggered"
        );

        let new_val = *value.borrow();
        assert!(
            (new_val - 50.0).abs() < 0.01,
            "Value should be reset to default (50.0), got {}",
            new_val
        );
    }
}

/// Test percentage stays clamped at 100% max
#[gpui::test]
async fn test_potentiometer_percentage_clamped_at_max(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(100.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotPercentageView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("percentage-pot") {
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
async fn test_potentiometer_percentage_clamped_at_min(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(0.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotPercentageView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("percentage-pot") {
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

/// Test click increments value by 10%
#[gpui::test]
async fn test_potentiometer_click_increments_by_10_percent(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(50.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotValueChangeTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(
        ((*value.borrow()) - 50.0).abs() < 0.01,
        "Initial value should be 50"
    );

    if let Some(bounds) = cx.debug_bounds("change-test-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        let new_val = *value.borrow();
        // Value should increment by 10% (10 units on 0-100 range)
        assert!(
            (new_val - 60.0).abs() < 0.01,
            "Value should be 60 after click (50 + 10%), got {}",
            new_val
        );
    }
}

/// Even sub-threshold pointer movement must not turn into a click-step on release.
#[gpui::test]
async fn test_potentiometer_pointer_movement_does_not_click_step(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(0.0));
    let change_count = Arc::new(AtomicUsize::new(0));
    let window = cx.add_window({
        let value = value.clone();
        let change_count = change_count.clone();
        move |_window, _cx| PotValueChangeTestView {
            value,
            change_count,
        }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("change-test-pot") {
        let start = bounds.center();
        let moved_down = point(start.x, start.y + gpui::px(1.0));
        cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
        cx.simulate_mouse_move(moved_down, Some(MouseButton::Left), Modifiers::default());
        cx.run_until_parked();
        cx.simulate_mouse_up(start, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
    }

    assert!(
        value.borrow().abs() < 0.01,
        "pointer movement must not apply the 10% click step"
    );
    assert_eq!(change_count.load(Ordering::SeqCst), 0);
}

/// Test logarithmic scale scroll wheel uses scale-aware stepping
#[gpui::test]
async fn test_potentiometer_log_scale_scroll(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(1000.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotLogScaleView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("log-scale-pot") {
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
            "Log scale: value should increase, initial={}, new={}",
            initial_value,
            new_val
        );
    }
}

/// Test logarithmic scale Home sets to min
#[gpui::test]
async fn test_potentiometer_log_scale_home_sets_min(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(1000.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotLogScaleView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("log-scale-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("home");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            (new_val - 20.0).abs() < 0.01,
            "Home should set to min (20Hz), got {}",
            new_val
        );
    }
}

/// Test logarithmic scale End sets to max
#[gpui::test]
async fn test_potentiometer_log_scale_end_sets_max(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new(1000.0));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| PotLogScaleView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("log-scale-pot") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("end");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            (new_val - 20000.0).abs() < 0.01,
            "End should set to max (20kHz), got {}",
            new_val
        );
    }
}
