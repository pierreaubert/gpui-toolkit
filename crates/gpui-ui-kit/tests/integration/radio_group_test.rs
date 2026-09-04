//! Integration tests for RadioGroup component
//!
//! Tests radio group rendering, click selection, disabled state, sizes,
//! orientation, and keyboard navigation using VisualTestContext.

use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div,
};
use gpui_ui_kit::radio_group::{RadioGroup, RadioGroupOrientation, RadioGroupSize, RadioOption};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn test_options() -> Vec<RadioOption> {
    vec![
        RadioOption::new("a", "Alpha"),
        RadioOption::new("b", "Beta"),
        RadioOption::new("c", "Gamma").disabled(true),
    ]
}

/// View that tracks radio group selection changes
struct RadioGroupTestView {
    selected: Rc<RefCell<Option<String>>>,
    change_count: Arc<AtomicUsize>,
    disabled: bool,
}

impl Render for RadioGroupTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let selected_rc = self.selected.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            RadioGroup::new("test-radio-group")
                .options(test_options())
                .selected(self.selected.borrow().clone().map(gpui::SharedString::from))
                .disabled(self.disabled)
                .on_change(move |value, _window, _cx| {
                    *selected_rc.borrow_mut() = Some(value.to_string());
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}

#[gpui::test]
async fn test_radio_group_click_selects_option(cx: &mut TestAppContext) {
    let selected = Rc::new(RefCell::new(None));
    let change_count = Arc::new(AtomicUsize::new(0));
    let selected_clone = selected.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| RadioGroupTestView {
        selected: selected_clone,
        change_count: change_count_clone,
        disabled: false,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(selected.borrow().is_none(), "Should start unselected");

    // Option rows register `Name("test-radio-group")-option-<index>`
    // selectors (see `RadioGroup::build_with_theme_and_design`). Fail closed:
    // a missing selector means the row never rendered.
    let bounds = cx
        .debug_bounds("Name(\"test-radio-group\")-option-1")
        .expect("option 1 row rendered");
    let center = bounds.center();
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    assert_eq!(
        selected.borrow().as_deref(),
        Some("b"),
        "Should select option b after click"
    );
    assert_eq!(
        change_count.load(Ordering::SeqCst),
        1,
        "on_change should have been called once"
    );
}

#[gpui::test]
async fn test_radio_group_disabled_ignores_click(cx: &mut TestAppContext) {
    let selected = Rc::new(RefCell::new(None));
    let change_count = Arc::new(AtomicUsize::new(0));
    let selected_clone = selected.clone();
    let change_count_clone = change_count.clone();

    let window = cx.add_window(move |_window, _cx| RadioGroupTestView {
        selected: selected_clone,
        change_count: change_count_clone,
        disabled: true,
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let bounds = cx
        .debug_bounds("Name(\"test-radio-group\")-option-0")
        .expect("option 0 row rendered");
    let center = bounds.center();
    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    assert!(
        selected.borrow().is_none(),
        "Disabled group should stay unselected"
    );
    assert_eq!(
        change_count.load(Ordering::SeqCst),
        0,
        "Disabled group should not trigger on_change"
    );
}

#[gpui::test]
async fn test_radio_group_all_sizes(cx: &mut TestAppContext) {
    struct SizesView;

    impl Render for SizesView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    RadioGroup::new("rg-sm")
                        .options(test_options())
                        .size(RadioGroupSize::Sm),
                )
                .child(
                    RadioGroup::new("rg-md")
                        .options(test_options())
                        .size(RadioGroupSize::Md),
                )
                .child(
                    RadioGroup::new("rg-lg")
                        .options(test_options())
                        .size(RadioGroupSize::Lg),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| SizesView);
}

#[gpui::test]
async fn test_radio_group_orientations(cx: &mut TestAppContext) {
    struct OrientationView;

    impl Render for OrientationView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    RadioGroup::new("rg-v")
                        .options(test_options())
                        .orientation(RadioGroupOrientation::Vertical),
                )
                .child(
                    RadioGroup::new("rg-h")
                        .options(test_options())
                        .orientation(RadioGroupOrientation::Horizontal),
                )
        }
    }

    let _window = cx.add_window(|_window, _cx| OrientationView);
}
