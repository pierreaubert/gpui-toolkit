//! Integration tests for Button component
//!
//! Tests button rendering, click handling, disabled state, variants, sizes,
//! selected state, full width, icons, and custom theming using VisualTestContext.

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div,
};
use gpui_ui_kit::button::{Button, ButtonSize, ButtonTheme, ButtonVariant};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Click Callback Tests
// ============================================================================

/// View that tracks click callbacks
struct ButtonClickTestView {
    click_count: Arc<AtomicUsize>,
    disabled: bool,
}

impl Render for ButtonClickTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let click_count = self.click_count.clone();
        div().size_full().child(
            Button::new("test-button", "Click Me")
                .disabled(self.disabled)
                .on_click(move |_window, _cx| {
                    click_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_button_click_triggers_callback(cx: &mut TestAppContext) {
    let click_count = Arc::new(AtomicUsize::new(0));
    let click_count_clone = click_count.clone();

    let window = cx.add_window(move |_window, _cx| ButtonClickTestView {
        click_count: click_count_clone,
        disabled: false,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("test-button") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            click_count.load(Ordering::SeqCst),
            1,
            "Click callback should have been called once"
        );
    }
}

#[gpui::test]
async fn test_button_disabled_ignores_click(cx: &mut TestAppContext) {
    let click_count = Arc::new(AtomicUsize::new(0));
    let click_count_clone = click_count.clone();

    let window = cx.add_window(move |_window, _cx| ButtonClickTestView {
        click_count: click_count_clone,
        disabled: true,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("test-button") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();
    }

    assert_eq!(
        click_count.load(Ordering::SeqCst),
        0,
        "Disabled button should not trigger callback"
    );
}

#[gpui::test]
async fn test_button_multiple_clicks(cx: &mut TestAppContext) {
    let click_count = Arc::new(AtomicUsize::new(0));
    let click_count_clone = click_count.clone();

    let window = cx.add_window(move |_window, _cx| ButtonClickTestView {
        click_count: click_count_clone,
        disabled: false,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    if let Some(bounds) = cx.debug_bounds("test-button") {
        let center = bounds.center();
        for _ in 0..3 {
            cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
            cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
            cx.run_until_parked();
        }

        assert_eq!(
            click_count.load(Ordering::SeqCst),
            3,
            "Click callback should have been called 3 times"
        );
    }
}

// ============================================================================
// Variant Rendering Tests
// ============================================================================

#[gpui::test]
async fn test_button_all_variants_render(cx: &mut TestAppContext) {
    struct VariantsView;

    impl Render for VariantsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Button::new("btn-primary", "Primary").variant(ButtonVariant::Primary))
                .child(Button::new("btn-secondary", "Secondary").variant(ButtonVariant::Secondary))
                .child(
                    Button::new("btn-destructive", "Destructive")
                        .variant(ButtonVariant::Destructive),
                )
                .child(Button::new("btn-ghost", "Ghost").variant(ButtonVariant::Ghost))
                .child(Button::new("btn-outline", "Outline").variant(ButtonVariant::Outline))
        }
    }

    let _window = cx.add_window(|_window, _cx| VariantsView);
}

// ============================================================================
// Size Rendering Tests
// ============================================================================

#[gpui::test]
async fn test_button_all_sizes_render(cx: &mut TestAppContext) {
    struct SizesView;

    impl Render for SizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Button::new("btn-xs", "Xs").size(ButtonSize::Xs))
                .child(Button::new("btn-sm", "Sm").size(ButtonSize::Sm))
                .child(Button::new("btn-md", "Md").size(ButtonSize::Md))
                .child(Button::new("btn-lg", "Lg").size(ButtonSize::Lg))
        }
    }

    let _window = cx.add_window(|_window, _cx| SizesView);
}

// ============================================================================
// Selected State Test
// ============================================================================

#[gpui::test]
async fn test_button_selected_state(cx: &mut TestAppContext) {
    struct SelectedView {
        selected: Rc<RefCell<bool>>,
        click_count: Arc<AtomicUsize>,
    }

    impl Render for SelectedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let is_selected = *self.selected.borrow();
            let selected_rc = self.selected.clone();
            let click_count = self.click_count.clone();

            div().size_full().child(
                Button::new("selected-btn", "Toggle")
                    .selected(is_selected)
                    .on_click(move |_window, _cx| {
                        let mut s = selected_rc.borrow_mut();
                        *s = !*s;
                        click_count.fetch_add(1, Ordering::SeqCst);
                    }),
            )
        }
    }

    let selected = Rc::new(RefCell::new(false));
    let click_count = Arc::new(AtomicUsize::new(0));
    let selected_clone = selected.clone();
    let click_count_clone = click_count.clone();

    let window = cx.add_window(move |_window, _cx| SelectedView {
        selected: selected_clone,
        click_count: click_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(!*selected.borrow(), "Should start unselected");

    if let Some(bounds) = cx.debug_bounds("selected-btn") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert!(*selected.borrow(), "Should be selected after click");
        assert_eq!(click_count.load(Ordering::SeqCst), 1);
    }
}

// ============================================================================
// Full Width Test
// ============================================================================

#[gpui::test]
async fn test_button_full_width(cx: &mut TestAppContext) {
    struct FullWidthView;

    impl Render for FullWidthView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .w(gpui::px(400.0))
                .child(Button::new("full-width-btn", "Full Width").full_width(true))
        }
    }

    let _window = cx.add_window(|_window, _cx| FullWidthView);
}

// ============================================================================
// Icon Tests
// ============================================================================

#[gpui::test]
async fn test_button_with_icons(cx: &mut TestAppContext) {
    struct IconView;

    impl Render for IconView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(Button::new("icon-left", "Save").icon_left("+"))
                .child(Button::new("icon-right", "Next").icon_right(">"))
                .child(
                    Button::new("icon-both", "Download")
                        .icon_left("<")
                        .icon_right(">"),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| IconView);
}

// ============================================================================
// Custom Theme Test
// ============================================================================

#[gpui::test]
async fn test_button_custom_theme(cx: &mut TestAppContext) {
    struct ThemedView;

    impl Render for ThemedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let custom_theme = ButtonTheme {
                accent: gpui::rgba(0xff6600ff),
                accent_hover: gpui::rgba(0xff8800ff),
                surface: gpui::rgba(0x2a2a2aff),
                surface_hover: gpui::rgba(0x3a3a3aff),
                text_primary: gpui::rgba(0xffffffff),
                text_secondary: gpui::rgba(0xccccccff),
                text_on_accent: gpui::rgba(0xffffffff),
                error: gpui::rgba(0xcc3333ff),
                error_hover: gpui::rgba(0xe64545ff),
                border: gpui::rgba(0x555555ff),
                transparent: gpui::rgba(0x00000000),
            };

            div().child(
                Button::new("themed-btn", "Custom Theme")
                    .theme(custom_theme)
                    .variant(ButtonVariant::Primary),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ThemedView);
}
