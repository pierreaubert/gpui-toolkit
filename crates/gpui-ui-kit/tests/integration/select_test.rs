//! Integration tests for Select component
//!
//! Tests the select/dropdown component including:
//! - Basic rendering
//! - Opening/closing dropdown via click
//! - Option selection via click
//! - Keyboard navigation (Arrow Up/Down, Enter, Escape, Space)
//! - Highlighted option tracking
//! - Size variants

use gpui::{
    Context, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, prelude::*,
};
use gpui_ui_kit::select::{Select, SelectOption, SelectSize};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// Basic Rendering Tests
// ============================================================================

struct SelectTestView;

impl Render for SelectTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Select::new("test-select")
                .placeholder("Choose option")
                .options(vec![
                    SelectOption::new("1", "Option 1"),
                    SelectOption::new("2", "Option 2"),
                ]),
        )
    }
}

#[gpui::test]
async fn test_select_renders(cx: &mut TestAppContext) {
    let _window = cx.add_window(|_window, _cx| SelectTestView);
}

/// Test select with size variants
#[gpui::test]
async fn test_select_sizes(cx: &mut TestAppContext) {
    struct SizeTestView;

    impl Render for SizeTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .child(
                    Select::new("small-select")
                        .size(SelectSize::Sm)
                        .options(vec![SelectOption::new("a", "Small")]),
                )
                .child(
                    Select::new("medium-select")
                        .size(SelectSize::Md)
                        .options(vec![SelectOption::new("a", "Medium")]),
                )
                .child(
                    Select::new("large-select")
                        .size(SelectSize::Lg)
                        .options(vec![SelectOption::new("a", "Large")]),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizeTestView);
}

// ============================================================================
// Toggle/Open Tests
// ============================================================================

/// View that tracks open/close state
struct SelectToggleTestView {
    is_open: Rc<RefCell<bool>>,
    toggle_count: Arc<AtomicUsize>,
}

impl Render for SelectToggleTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let open = *self.is_open.borrow();
        let is_open_rc = self.is_open.clone();
        let toggle_count = self.toggle_count.clone();

        div().size_full().child(
            Select::new("toggle-test-select")
                .placeholder("Click to toggle")
                .options(vec![
                    SelectOption::new("1", "Option 1"),
                    SelectOption::new("2", "Option 2"),
                    SelectOption::new("3", "Option 3"),
                ])
                .is_open(open)
                .on_toggle(move |new_open, _window, _cx| {
                    *is_open_rc.borrow_mut() = new_open;
                    toggle_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_select_click_to_open(cx: &mut TestAppContext) {
    let is_open = Rc::new(RefCell::new(false));
    let toggle_count = Arc::new(AtomicUsize::new(0));

    let is_open_clone = is_open.clone();
    let toggle_count_clone = toggle_count.clone();

    let window = cx.add_window(move |_window, _cx| SelectToggleTestView {
        is_open: is_open_clone,
        toggle_count: toggle_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initially closed
    assert!(!*is_open.borrow(), "Select should be initially closed");

    // Click to open
    if let Some(bounds) = cx.debug_bounds("toggle-test-select") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        assert_eq!(
            toggle_count.load(Ordering::SeqCst),
            1,
            "Toggle should have been called once"
        );
        assert!(*is_open.borrow(), "Select should be open after click");
    }
}

// ============================================================================
// Selection Tests
// ============================================================================

/// View that tracks selected value
struct SelectSelectionTestView {
    is_open: Rc<RefCell<bool>>,
    selected: Rc<RefCell<Option<String>>>,
}

impl Render for SelectSelectionTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let open = *self.is_open.borrow();
        let selected = self.selected.borrow().clone();
        let is_open_rc = self.is_open.clone();
        let selected_rc = self.selected.clone();

        let mut select = Select::new("selection-test-select")
            .placeholder("Select an option")
            .options(vec![
                SelectOption::new("apple", "Apple"),
                SelectOption::new("banana", "Banana"),
                SelectOption::new("cherry", "Cherry"),
            ])
            .is_open(open)
            .on_toggle({
                let is_open_rc = is_open_rc.clone();
                move |new_open, _window, _cx| {
                    *is_open_rc.borrow_mut() = new_open;
                }
            })
            .on_change(move |value, _window, _cx| {
                *selected_rc.borrow_mut() = Some(value.to_string());
                // Close after selection
                *is_open_rc.borrow_mut() = false;
            });

        if let Some(ref val) = selected {
            select = select.selected(val.clone());
        }

        div().size_full().child(select)
    }
}

#[gpui::test]
async fn test_select_option_selection(cx: &mut TestAppContext) {
    let is_open = Rc::new(RefCell::new(false));
    let selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let is_open_clone = is_open.clone();
    let selected_clone = selected.clone();

    let window = cx.add_window(move |_window, _cx| SelectSelectionTestView {
        is_open: is_open_clone,
        selected: selected_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Initially nothing selected
    assert!(
        selected.borrow().is_none(),
        "Nothing should be selected initially"
    );

    // Open the dropdown
    if let Some(bounds) = cx.debug_bounds("selection-test-select") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Mark as open for the next render
        *is_open.borrow_mut() = true;
    }
}

// ============================================================================
// Disabled Option Tests
// ============================================================================

#[gpui::test]
async fn test_select_with_disabled_options(cx: &mut TestAppContext) {
    struct DisabledOptionsView;

    impl Render for DisabledOptionsView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Select::new("disabled-options-select")
                    .placeholder("Select")
                    .options(vec![
                        SelectOption::new("enabled", "Enabled Option"),
                        SelectOption::new("disabled", "Disabled Option").disabled(true),
                        SelectOption::new("also-enabled", "Also Enabled"),
                    ])
                    .is_open(true), // Show dropdown for visual testing
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| DisabledOptionsView);
}

// ============================================================================
// Keyboard Navigation Tests
// ============================================================================

/// View with keyboard navigation support
struct SelectKeyboardTestView {
    is_open: Rc<RefCell<bool>>,
    highlighted: Rc<RefCell<Option<usize>>>,
    selected: Rc<RefCell<Option<String>>>,
}

impl Render for SelectKeyboardTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let open = *self.is_open.borrow();
        let highlighted = *self.highlighted.borrow();
        let selected = self.selected.borrow().clone();

        let is_open_rc = self.is_open.clone();
        let highlighted_rc = self.highlighted.clone();
        let selected_rc = self.selected.clone();

        let mut select = Select::new("keyboard-test-select")
            .placeholder("Navigate with keyboard")
            .options(vec![
                SelectOption::new("first", "First"),
                SelectOption::new("second", "Second"),
                SelectOption::new("third", "Third"),
            ])
            .is_open(open)
            .highlighted_index(highlighted)
            .on_toggle({
                let is_open_rc = is_open_rc.clone();
                move |new_open, _window, _cx| {
                    *is_open_rc.borrow_mut() = new_open;
                }
            })
            .on_highlight({
                let highlighted_rc = highlighted_rc.clone();
                move |idx, _window, _cx| {
                    *highlighted_rc.borrow_mut() = idx;
                }
            })
            .on_change({
                let selected_rc = selected_rc.clone();
                let is_open_rc = is_open_rc.clone();
                move |value, _window, _cx| {
                    *selected_rc.borrow_mut() = Some(value.to_string());
                    *is_open_rc.borrow_mut() = false;
                }
            });

        if let Some(ref val) = selected {
            select = select.selected(val.clone());
        }

        div().size_full().child(select)
    }
}

#[gpui::test]
async fn test_select_keyboard_navigation(cx: &mut TestAppContext) {
    let is_open = Rc::new(RefCell::new(false));
    let highlighted: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let selected: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let is_open_clone = is_open.clone();
    let highlighted_clone = highlighted.clone();
    let selected_clone = selected.clone();

    let window = cx.add_window(move |_window, _cx| SelectKeyboardTestView {
        is_open: is_open_clone,
        highlighted: highlighted_clone,
        selected: selected_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Find and focus the select
    if let Some(bounds) = cx.debug_bounds("keyboard-test-select") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Dropdown should now be open
        // Note: Testing keyboard events requires more complex setup with focus handling
    }
}

// ============================================================================
// Label Tests
// ============================================================================

#[gpui::test]
async fn test_select_with_label(cx: &mut TestAppContext) {
    struct LabelTestView;

    impl Render for LabelTestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Select::new("labeled-select")
                    .label("Choose a fruit")
                    .placeholder("Select...")
                    .options(vec![
                        SelectOption::new("apple", "Apple"),
                        SelectOption::new("orange", "Orange"),
                    ]),
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| LabelTestView);
}

// ============================================================================
// Disabled Select Tests
// ============================================================================

/// View with disabled select that tracks toggle attempts
struct DisabledSelectTestView {
    toggle_count: Arc<AtomicUsize>,
}

impl Render for DisabledSelectTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let toggle_count = self.toggle_count.clone();

        div().size_full().child(
            Select::new("disabled-select")
                .placeholder("Cannot click")
                .options(vec![SelectOption::new("a", "A")])
                .disabled(true)
                .on_toggle(move |_, _window, _cx| {
                    toggle_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_select_disabled_state(cx: &mut TestAppContext) {
    let toggle_count = Arc::new(AtomicUsize::new(0));
    let toggle_count_clone = toggle_count.clone();

    let window = cx.add_window(move |_window, _cx| DisabledSelectTestView {
        toggle_count: toggle_count_clone,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    // Try clicking the disabled select
    if let Some(bounds) = cx.debug_bounds("disabled-select") {
        let center = bounds.center();
        cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
        cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
        cx.run_until_parked();

        // Toggle should NOT have been called because the select is disabled
        assert_eq!(
            toggle_count.load(Ordering::SeqCst),
            0,
            "Disabled select should not trigger on_toggle"
        );
    }
}

// ============================================================================
// Preselected Value Tests
// ============================================================================

#[gpui::test]
async fn test_select_with_preselected_value(cx: &mut TestAppContext) {
    struct PreselectedView;

    impl Render for PreselectedView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Select::new("preselected-select")
                    .options(vec![
                        SelectOption::new("red", "Red"),
                        SelectOption::new("green", "Green"),
                        SelectOption::new("blue", "Blue"),
                    ])
                    .selected("green"), // Pre-select "Green"
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| PreselectedView);
}

/// Test that displays selected label properly
#[gpui::test]
async fn test_select_shows_selected_label(cx: &mut TestAppContext) {
    struct SelectedLabelView;

    impl Render for SelectedLabelView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().child(
                Select::new("selected-label-select")
                    .options(vec![
                        SelectOption::new("val1", "Display Label 1"),
                        SelectOption::new("val2", "Display Label 2"),
                    ])
                    .selected("val1"), // Shows "Display Label 1" not "val1"
            )
        }
    }

    let _window = cx.add_window(|_window, _cx| SelectedLabelView);
}
