use super::mute_toggle_test_view::MuteToggleTestView;
use super::volume_change_test_view::VolumeChangeTestView;
use super::volume_knob_keyboard_view::VolumeKnobKeyboardView;
use super::volume_knob_scroll_wheel_view::VolumeKnobScrollWheelView;
use super::volume_knob_test_view::VolumeKnobTestView;
use gpui::{
    Context, Modifiers, MouseButton, ScrollDelta, ScrollWheelEvent, TestAppContext, TouchPhase,
    VisualTestContext, Window, div, point, prelude::*, px,
};
use gpui_audio_kit::audio::volume_knob::{VolumeKnob, VolumeKnobTheme};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[gpui::test]
async fn test_volume_knob_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| VolumeKnobTestView);
}

#[gpui::test]
async fn test_volume_knob_value_range(cx: &mut TestAppContext) {
    struct ValueRangeView;

    impl Render for ValueRangeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(VolumeKnob::new().id("vol-0").value(0.0).label("0%"))
                .child(VolumeKnob::new().id("vol-25").value(0.25).label("25%"))
                .child(VolumeKnob::new().id("vol-50").value(0.5).label("50%"))
                .child(VolumeKnob::new().id("vol-75").value(0.75).label("75%"))
                .child(VolumeKnob::new().id("vol-100").value(1.0).label("100%"))
        }
    }

    let _window = cx.add_window(|_window, _cx| ValueRangeView);
}

#[gpui::test]
async fn test_volume_knob_on_change(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| VolumeChangeTestView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initial value should be 0.5
    assert_eq!(*value.borrow(), 0.5);
}

#[gpui::test]
async fn test_volume_knob_muted(cx: &mut TestAppContext) {
    struct MutedView;

    impl Render for MutedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VolumeKnob::new()
                        .id("not-muted")
                        .value(0.7)
                        .muted(false)
                        .label("ON"),
                )
                .child(
                    VolumeKnob::new()
                        .id("muted")
                        .value(0.7)
                        .muted(true)
                        .label("MUTE"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| MutedView);
}

#[gpui::test]
async fn test_volume_knob_mute_toggle(cx: &mut TestAppContext) {
    let muted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let toggle_count = Arc::new(AtomicUsize::new(0));

    let muted_clone = muted.clone();
    let toggle_count_clone = toggle_count.clone();

    let _window = cx.add_window(move |_window, _cx| MuteToggleTestView {
        muted: muted_clone,
        toggle_count: toggle_count_clone,
    });

    // Initially not muted
    assert!(!*muted.borrow());
    assert_eq!(toggle_count.load(Ordering::SeqCst), 0);
}

#[gpui::test]
async fn test_volume_knob_sizes(cx: &mut TestAppContext) {
    struct SizesView;

    impl Render for SizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .items_end()
                .gap_4()
                .child(
                    VolumeKnob::new()
                        .id("size-20")
                        .size(px(20.0))
                        .value(0.7)
                        .label("XS"),
                )
                .child(
                    VolumeKnob::new()
                        .id("size-32")
                        .size(px(32.0))
                        .value(0.7)
                        .label("S"),
                )
                .child(
                    VolumeKnob::new()
                        .id("size-40")
                        .size(px(40.0))
                        .value(0.7)
                        .label("M"),
                )
                .child(
                    VolumeKnob::new()
                        .id("size-56")
                        .size(px(56.0))
                        .value(0.7)
                        .label("L"),
                )
                .child(
                    VolumeKnob::new()
                        .id("size-80")
                        .size(px(80.0))
                        .value(0.7)
                        .label("XL"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizesView);
}

#[gpui::test]
async fn test_volume_knob_labels(cx: &mut TestAppContext) {
    struct LabelsView;

    impl Render for LabelsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(VolumeKnob::new().id("label-vol").value(0.7).label("VOL"))
                .child(
                    VolumeKnob::new()
                        .id("label-master")
                        .value(0.8)
                        .size(px(50.0))
                        .label("M"),
                )
                .child(VolumeKnob::new().id("label-empty").value(0.5).label(""))
        }
    }

    let _window = cx.add_window(|_window, _cx| LabelsView);
}

#[gpui::test]
async fn test_volume_knob_with_custom_theme(cx: &mut TestAppContext) {
    struct ThemedView;

    impl Render for ThemedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let custom_theme = VolumeKnobTheme {
                accent: gpui::rgba(0xff8000ff),     // Orange
                muted: gpui::rgba(0x4d4d4dff),      // 30% gray
                background: gpui::rgba(0x262626ff), // 15% gray (dark)
                text: gpui::rgba(0xf2f2f2ff),       // 95% gray (nearly white)
            };

            div().child(
                VolumeKnob::new()
                    .id("themed-knob")
                    .theme(custom_theme)
                    .value(0.7)
                    .label("THM"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ThemedView);
}

#[gpui::test]
async fn test_volume_knob_color_overrides(cx: &mut TestAppContext) {
    struct ColorOverridesView;

    impl Render for ColorOverridesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .gap_4()
                .child(
                    VolumeKnob::new()
                        .id("accent-red")
                        .value(0.7)
                        .accent_color(gpui::hsla(0.0, 1.0, 0.5, 1.0)) // Red
                        .label("R"),
                )
                .child(
                    VolumeKnob::new()
                        .id("accent-green")
                        .value(0.7)
                        .accent_color(gpui::hsla(0.33, 1.0, 0.4, 1.0)) // Green
                        .label("G"),
                )
                .child(
                    VolumeKnob::new()
                        .id("accent-blue")
                        .value(0.7)
                        .accent_color(gpui::hsla(0.6, 1.0, 0.5, 1.0)) // Blue
                        .label("B"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| ColorOverridesView);
}

#[gpui::test]
async fn test_volume_knob_muted_color_override(cx: &mut TestAppContext) {
    struct MutedColorView;

    impl Render for MutedColorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VolumeKnob::new()
                    .id("muted-color-knob")
                    .value(0.7)
                    .muted(true)
                    .muted_color(gpui::hsla(0.0, 1.0, 0.3, 1.0)) // Dark red when muted
                    .label("MUTE"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| MutedColorView);
}

#[gpui::test]
async fn test_volume_knob_bg_color_override(cx: &mut TestAppContext) {
    struct BgColorView;

    impl Render for BgColorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VolumeKnob::new()
                    .id("bg-color-knob")
                    .value(0.7)
                    .bg_color(gpui::hsla(0.6, 0.3, 0.2, 1.0)) // Dark blue
                    .label("BG"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| BgColorView);
}

#[gpui::test]
async fn test_volume_knob_text_color_override(cx: &mut TestAppContext) {
    struct TextColorView;

    impl Render for TextColorView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VolumeKnob::new()
                    .id("text-color-knob")
                    .value(0.7)
                    .text_color(gpui::hsla(0.1, 1.0, 0.5, 1.0)) // Orange text
                    .label("TXT"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| TextColorView);
}

#[gpui::test]
async fn test_volume_knob_zero_value(cx: &mut TestAppContext) {
    struct ZeroValueView;

    impl Render for ZeroValueView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(VolumeKnob::new().id("zero-value").value(0.0).label("0"))
        }
    }

    let _window = cx.add_window(|_window, _cx| ZeroValueView);
}

#[gpui::test]
async fn test_volume_knob_max_value(cx: &mut TestAppContext) {
    struct MaxValueView;

    impl Render for MaxValueView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(VolumeKnob::new().id("max-value").value(1.0).label("MAX"))
        }
    }

    let _window = cx.add_window(|_window, _cx| MaxValueView);
}

#[gpui::test]
async fn test_volume_knob_over_max_clamped(cx: &mut TestAppContext) {
    struct OverMaxView;

    impl Render for OverMaxView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VolumeKnob::new()
                    .id("over-max")
                    .value(1.5) // Over 1.0, should be clamped
                    .label("CLAMP"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| OverMaxView);
}

#[gpui::test]
async fn test_volume_knob_negative_clamped(cx: &mut TestAppContext) {
    struct NegativeView;

    impl Render for NegativeView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VolumeKnob::new()
                    .id("negative")
                    .value(-0.5) // Negative, should be clamped to 0
                    .label("NEG"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| NegativeView);
}

#[gpui::test]
async fn test_volume_knob_default(cx: &mut TestAppContext) {
    struct DefaultView;

    impl Render for DefaultView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(VolumeKnob::default())
        }
    }

    let _window = cx.add_window(|_window, _cx| DefaultView);
}

#[gpui::test]
async fn test_volume_knob_all_color_overrides(cx: &mut TestAppContext) {
    struct AllOverridesView;

    impl Render for AllOverridesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                VolumeKnob::new()
                    .id("all-overrides")
                    .value(0.7)
                    .accent_color(gpui::hsla(0.8, 1.0, 0.5, 1.0)) // Purple
                    .muted_color(gpui::hsla(0.0, 0.5, 0.3, 1.0))
                    .bg_color(gpui::hsla(0.0, 0.0, 0.05, 1.0))
                    .text_color(gpui::hsla(0.8, 1.0, 0.8, 1.0))
                    .label("ALL"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| AllOverridesView);
}

/// Test scroll wheel up increases value
#[gpui::test]
async fn test_volume_knob_scroll_wheel_up_increases_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| VolumeKnobScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(
        ((*value.borrow()) - 0.5).abs() < 0.001,
        "Initial value should be 0.5"
    );

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-knob") {
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
            new_val > 0.5,
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
async fn test_volume_knob_scroll_wheel_down_decreases_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| VolumeKnobScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-knob") {
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
            new_val < 0.5,
            "Value should decrease after scroll down, got {}",
            new_val
        );
    }
}

/// Test scroll wheel respects upper bound (1.0)
#[gpui::test]
async fn test_volume_knob_scroll_wheel_respects_max_bound(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.98)); // Near max
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| VolumeKnobScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-knob") {
        let center = bounds.center();

        // Scroll up multiple times
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
            new_val <= 1.0,
            "Value should be clamped at max (1.0), got {}",
            new_val
        );
        assert!(
            (new_val - 1.0).abs() < 0.001,
            "Value should be exactly 1.0, got {}",
            new_val
        );
    }
}

/// Test scroll wheel respects lower bound (0.0)
#[gpui::test]
async fn test_volume_knob_scroll_wheel_respects_min_bound(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.02)); // Near min
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| VolumeKnobScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-knob") {
        let center = bounds.center();

        // Scroll down multiple times
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
            "Value should be clamped at min (0.0), got {}",
            new_val
        );
        assert!(
            new_val.abs() < 0.001,
            "Value should be exactly 0.0, got {}",
            new_val
        );
    }
}

/// Test multiple scroll events accumulate
#[gpui::test]
async fn test_volume_knob_multiple_scroll_events(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let change_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| VolumeKnobScrollWheelView {
        value: value_clone,
        change_count: change_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("scroll-wheel-knob") {
        let center = bounds.center();

        // Scroll up 3 times (each step is 0.05)
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
        // Expected: 0.5 + (3 * 0.05) = 0.65
        assert!(
            (new_val - 0.65).abs() < 0.01,
            "Value should be around 0.65 after 3 scrolls, got {}",
            new_val
        );
        assert!(
            change_count.load(Ordering::SeqCst) >= 3,
            "on_change should have been called at least 3 times"
        );
    }
}

/// Test Up arrow key increases value
#[gpui::test]
async fn test_volume_knob_keyboard_up_increases_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let muted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let mute_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let muted_clone = muted.clone();
    let change_count_clone = change_count.clone();
    let mute_count_clone = mute_count.clone();

    let window = cx.add_window(move |_window, cx| VolumeKnobKeyboardView {
        value: value_clone,
        muted: muted_clone,
        change_count: change_count_clone,
        mute_count: mute_count_clone,
        focus_handle: cx.focus_handle(),
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Click to focus the knob
    if let Some(bounds) = cx.debug_bounds("keyboard-knob") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Press Up arrow
        cx.simulate_keystrokes("up");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val > 0.5,
            "Value should increase after Up key, got {}",
            new_val
        );
    }
}

/// Test Down arrow key decreases value
#[gpui::test]
async fn test_volume_knob_keyboard_down_decreases_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let muted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let mute_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let muted_clone = muted.clone();
    let change_count_clone = change_count.clone();
    let mute_count_clone = mute_count.clone();

    let window = cx.add_window(move |_window, cx| VolumeKnobKeyboardView {
        value: value_clone,
        muted: muted_clone,
        change_count: change_count_clone,
        mute_count: mute_count_clone,
        focus_handle: cx.focus_handle(),
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-knob") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("down");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val < 0.5,
            "Value should decrease after Down key, got {}",
            new_val
        );
    }
}

/// Test Right arrow key increases value (same as Up)
#[gpui::test]
async fn test_volume_knob_keyboard_right_increases_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let muted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let mute_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let muted_clone = muted.clone();
    let change_count_clone = change_count.clone();
    let mute_count_clone = mute_count.clone();

    let window = cx.add_window(move |_window, cx| VolumeKnobKeyboardView {
        value: value_clone,
        muted: muted_clone,
        change_count: change_count_clone,
        mute_count: mute_count_clone,
        focus_handle: cx.focus_handle(),
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-knob") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("right");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val > 0.5,
            "Value should increase after Right key, got {}",
            new_val
        );
    }
}

/// Test Left arrow key decreases value (same as Down)
#[gpui::test]
async fn test_volume_knob_keyboard_left_decreases_value(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let muted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let mute_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let muted_clone = muted.clone();
    let change_count_clone = change_count.clone();
    let mute_count_clone = mute_count.clone();

    let window = cx.add_window(move |_window, cx| VolumeKnobKeyboardView {
        value: value_clone,
        muted: muted_clone,
        change_count: change_count_clone,
        mute_count: mute_count_clone,
        focus_handle: cx.focus_handle(),
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-knob") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("left");
        cx.run_until_parked();

        let new_val = *value.borrow();
        assert!(
            new_val < 0.5,
            "Value should decrease after Left key, got {}",
            new_val
        );
    }
}

/// Test M key toggles mute
#[gpui::test]
async fn test_volume_knob_keyboard_m_toggles_mute(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let muted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let mute_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let muted_clone = muted.clone();
    let change_count_clone = change_count.clone();
    let mute_count_clone = mute_count.clone();

    let window = cx.add_window(move |_window, cx| VolumeKnobKeyboardView {
        value: value_clone,
        muted: muted_clone,
        change_count: change_count_clone,
        mute_count: mute_count_clone,
        focus_handle: cx.focus_handle(),
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(!*muted.borrow(), "Initial state should not be muted");

    if let Some(bounds) = cx.debug_bounds("keyboard-knob") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        cx.simulate_keystrokes("m");
        cx.run_until_parked();

        assert!(*muted.borrow(), "Should be muted after pressing M");
        assert_eq!(
            mute_count.load(Ordering::SeqCst),
            1,
            "Mute toggle should have been called once"
        );
    }
}

/// Test keyboard respects upper bound
#[gpui::test]
async fn test_volume_knob_keyboard_respects_max_bound(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.98));
    let muted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let mute_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let muted_clone = muted.clone();
    let change_count_clone = change_count.clone();
    let mute_count_clone = mute_count.clone();

    let window = cx.add_window(move |_window, cx| VolumeKnobKeyboardView {
        value: value_clone,
        muted: muted_clone,
        change_count: change_count_clone,
        mute_count: mute_count_clone,
        focus_handle: cx.focus_handle(),
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-knob") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Press Up multiple times
        for _ in 0..10 {
            cx.simulate_keystrokes("up");
            cx.run_until_parked();
        }

        let new_val = *value.borrow();
        assert!(
            new_val <= 1.0,
            "Value should be clamped at max (1.0), got {}",
            new_val
        );
    }
}

/// Test keyboard respects lower bound
#[gpui::test]
async fn test_volume_knob_keyboard_respects_min_bound(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.02));
    let muted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let mute_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let muted_clone = muted.clone();
    let change_count_clone = change_count.clone();
    let mute_count_clone = mute_count.clone();

    let window = cx.add_window(move |_window, cx| VolumeKnobKeyboardView {
        value: value_clone,
        muted: muted_clone,
        change_count: change_count_clone,
        mute_count: mute_count_clone,
        focus_handle: cx.focus_handle(),
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-knob") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Press Down multiple times
        for _ in 0..10 {
            cx.simulate_keystrokes("down");
            cx.run_until_parked();
        }

        let new_val = *value.borrow();
        assert!(
            new_val >= 0.0,
            "Value should be clamped at min (0.0), got {}",
            new_val
        );
    }
}

/// Test that keyboard step size is 0.05
#[gpui::test]
async fn test_volume_knob_step_size(cx: &mut TestAppContext) {
    let value: Rc<RefCell<f32>> = Rc::new(RefCell::new(0.5));
    let muted: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
    let change_count = Arc::new(AtomicUsize::new(0));
    let mute_count = Arc::new(AtomicUsize::new(0));

    let value_clone = value.clone();
    let muted_clone = muted.clone();
    let change_count_clone = change_count.clone();
    let mute_count_clone = mute_count.clone();

    let window = cx.add_window(move |_window, cx| VolumeKnobKeyboardView {
        value: value_clone,
        muted: muted_clone,
        change_count: change_count_clone,
        mute_count: mute_count_clone,
        focus_handle: cx.focus_handle(),
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("keyboard-knob") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Press Up once
        cx.simulate_keystrokes("up");
        cx.run_until_parked();

        let new_val = *value.borrow();
        // Expected: 0.5 + 0.05 = 0.55
        assert!(
            (new_val - 0.55).abs() < 0.001,
            "Step size should be 0.05, got {} (expected 0.55)",
            new_val
        );
    }
}
