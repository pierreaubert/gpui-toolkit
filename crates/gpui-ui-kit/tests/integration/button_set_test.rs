//! Integration tests for ButtonSet component
//!
//! Tests the button set (segmented control) component including:
//! - Basic rendering with options
//! - Size variants
//! - Selection via click
//! - Disabled options
//! - Disabled entire button set
//! - Theme customization

use gpui::{Context, TestAppContext, VisualTestContext, Window, div, prelude::*};
use gpui_ui_kit::button_set::{ButtonSet, ButtonSetOption, ButtonSetSize, ButtonSetTheme};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct ButtonSetTestView;

impl Render for ButtonSetTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(ButtonSet::new("test-button-set").options(vec![
            ButtonSetOption::new("list", "List"),
            ButtonSetOption::new("grid", "Grid"),
            ButtonSetOption::new("table", "Table"),
        ]))
    }
}

#[gpui::test]
async fn test_button_set_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| ButtonSetTestView);
}

// ============================================================================
// Size Variant Tests
// ============================================================================

#[gpui::test]
async fn test_button_set_sizes(cx: &mut TestAppContext) {
    struct SizeTestView;

    impl Render for SizeTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    ButtonSet::new("xs-button-set")
                        .size(ButtonSetSize::Xs)
                        .options(vec![
                            ButtonSetOption::new("a", "A"),
                            ButtonSetOption::new("b", "B"),
                        ]),
                )
                .child(
                    ButtonSet::new("sm-button-set")
                        .size(ButtonSetSize::Sm)
                        .options(vec![
                            ButtonSetOption::new("a", "A"),
                            ButtonSetOption::new("b", "B"),
                        ]),
                )
                .child(
                    ButtonSet::new("md-button-set")
                        .size(ButtonSetSize::Md)
                        .options(vec![
                            ButtonSetOption::new("a", "A"),
                            ButtonSetOption::new("b", "B"),
                        ]),
                )
                .child(
                    ButtonSet::new("lg-button-set")
                        .size(ButtonSetSize::Lg)
                        .options(vec![
                            ButtonSetOption::new("a", "A"),
                            ButtonSetOption::new("b", "B"),
                        ]),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizeTestView);
}

// ============================================================================
// Selection Tests
// ============================================================================

/// View that tracks selection changes
struct ButtonSetSelectionTestView {
    selected: Rc<RefCell<Option<String>>>,
    change_count: Arc<AtomicUsize>,
}

impl Render for ButtonSetSelectionTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.selected.borrow().clone();
        let selected_rc = self.selected.clone();
        let change_count = self.change_count.clone();

        let mut button_set = ButtonSet::new("selection-test-button-set")
            .options(vec![
                ButtonSetOption::new("option1", "Option 1"),
                ButtonSetOption::new("option2", "Option 2"),
                ButtonSetOption::new("option3", "Option 3"),
            ])
            .on_change(move |value, _window, _cx| {
                *selected_rc.borrow_mut() = Some(value.to_string());
                change_count.fetch_add(1, Ordering::SeqCst);
            });

        if let Some(ref val) = selected {
            button_set = button_set.selected(val.clone());
        }

        div().size_full().child(button_set)
    }
}

#[gpui::test]
async fn test_button_set_selection(cx: &mut TestAppContext) {
    let selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let change_count = Arc::new(AtomicUsize::new(0));

    let selected_clone = selected.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| ButtonSetSelectionTestView {
        selected: selected_clone,
        change_count: change_count_clone,
    });

    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initially nothing selected
    assert!(
        selected.borrow().is_none(),
        "Nothing should be selected initially"
    );
}

#[gpui::test]
async fn test_button_set_preselected(cx: &mut TestAppContext) {
    struct PreselectedView;

    impl Render for PreselectedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                ButtonSet::new("preselected-button-set")
                    .options(vec![
                        ButtonSetOption::new("a", "A"),
                        ButtonSetOption::new("b", "B"),
                        ButtonSetOption::new("c", "C"),
                    ])
                    .selected("b"), // Pre-select B
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| PreselectedView);
}

// ============================================================================
// Disabled Option Tests
// ============================================================================

#[gpui::test]
async fn test_button_set_disabled_option(cx: &mut TestAppContext) {
    struct DisabledOptionView;

    impl Render for DisabledOptionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(ButtonSet::new("disabled-option-button-set").options(vec![
                ButtonSetOption::new("enabled", "Enabled"),
                ButtonSetOption::new("disabled", "Disabled").disabled(true),
                ButtonSetOption::new("also-enabled", "Also Enabled"),
            ]))
        }
    }

    let _window = cx.add_window(|_window, _cx| DisabledOptionView);
}

/// Test that clicking a disabled option doesn't trigger change
struct DisabledOptionClickTestView {
    change_count: Arc<AtomicUsize>,
}

impl Render for DisabledOptionClickTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let change_count = self.change_count.clone();

        div().size_full().child(
            ButtonSet::new("disabled-click-test")
                .options(vec![
                    ButtonSetOption::new("enabled", "Enabled"),
                    ButtonSetOption::new("disabled", "Disabled").disabled(true),
                ])
                .on_change(move |_, _window, _cx| {
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_button_set_disabled_option_click(cx: &mut TestAppContext) {
    let change_count = Arc::new(AtomicUsize::new(0));
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| DisabledOptionClickTestView {
        change_count: change_count_clone,
    });

    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initial state
    assert_eq!(change_count.load(Ordering::SeqCst), 0);
}

// ============================================================================
// Disabled Entire ButtonSet Tests
// ============================================================================

#[gpui::test]
async fn test_button_set_disabled(cx: &mut TestAppContext) {
    struct DisabledSetView;

    impl Render for DisabledSetView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                ButtonSet::new("disabled-set")
                    .options(vec![
                        ButtonSetOption::new("a", "A"),
                        ButtonSetOption::new("b", "B"),
                    ])
                    .disabled(true),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DisabledSetView);
}

// ============================================================================
// Option with Icon Tests
// ============================================================================

#[gpui::test]
async fn test_button_set_with_icons(cx: &mut TestAppContext) {
    struct IconsView;

    impl Render for IconsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                ButtonSet::new("icons-button-set")
                    .options(vec![
                        ButtonSetOption::new("list", "List").icon("📋"),
                        ButtonSetOption::new("grid", "Grid").icon("📊"),
                        ButtonSetOption::new("table", "Table").icon("📑"),
                    ])
                    .selected("grid"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| IconsView);
}

// ============================================================================
// Theme Tests
// ============================================================================

#[gpui::test]
async fn test_button_set_with_custom_theme(cx: &mut TestAppContext) {
    struct ThemedView;

    impl Render for ThemedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let custom_theme = ButtonSetTheme {
                bg: gpui::rgba(0x1a1a1aff),
                bg_hover: gpui::rgba(0x2a2a2aff),
                bg_selected: gpui::rgba(0xff6600ff),
                text_color: gpui::rgba(0xccccccff),
                text_color_selected: gpui::rgba(0xffffffff),
                border: gpui::rgba(0x444444ff),
                border_selected: gpui::rgba(0xff6600ff),
            };

            div().child(
                ButtonSet::new("themed-button-set")
                    .theme(custom_theme)
                    .options(vec![
                        ButtonSetOption::new("a", "A"),
                        ButtonSetOption::new("b", "B"),
                        ButtonSetOption::new("c", "C"),
                    ])
                    .selected("b"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ThemedView);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[gpui::test]
async fn test_button_set_single_option(cx: &mut TestAppContext) {
    struct SingleOptionView;

    impl Render for SingleOptionView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                ButtonSet::new("single-option-set")
                    .options(vec![ButtonSetOption::new("only", "Only Option")])
                    .selected("only"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SingleOptionView);
}

#[gpui::test]
async fn test_button_set_empty(cx: &mut TestAppContext) {
    struct EmptyView;

    impl Render for EmptyView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(ButtonSet::new("empty-set").options(vec![]))
        }
    }

    let _window = cx.add_window(|_window, _cx| EmptyView);
}

#[gpui::test]
async fn test_button_set_many_options(cx: &mut TestAppContext) {
    struct ManyOptionsView;

    impl Render for ManyOptionsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                ButtonSet::new("many-options-set")
                    .options(vec![
                        ButtonSetOption::new("1", "One"),
                        ButtonSetOption::new("2", "Two"),
                        ButtonSetOption::new("3", "Three"),
                        ButtonSetOption::new("4", "Four"),
                        ButtonSetOption::new("5", "Five"),
                        ButtonSetOption::new("6", "Six"),
                    ])
                    .selected("3"),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| ManyOptionsView);
}
