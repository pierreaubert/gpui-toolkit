use gpui::{
    ClipboardItem, Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};
use gpui_ui_kit::number_input::NumberInput;
use std::cell::RefCell;
use std::rc::Rc;

struct NumberInputDebugRegressionView {
    value: Rc<RefCell<f64>>,
    changes: Rc<RefCell<Vec<f64>>>,
}

impl Render for NumberInputDebugRegressionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let value = *self.value.borrow();
        let value_for_change = self.value.clone();
        let changes = self.changes.clone();

        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div().w(px(180.)).child(
                    NumberInput::new("number-debug-main")
                        .value(value)
                        .range(0.0, 100.0)
                        .step(1.0)
                        .label("Volume")
                        .on_change(move |next, _window, _cx| {
                            *value_for_change.borrow_mut() = next;
                            changes.borrow_mut().push(next);
                        }),
                ),
            )
            .child(
                div().w(px(180.)).child(
                    NumberInput::new("number-debug-blur-target")
                        .value(0.0)
                        .range(0.0, 100.0),
                ),
            )
    }
}

#[allow(clippy::type_complexity)]
fn setup(cx: &mut TestAppContext) -> (VisualTestContext, Rc<RefCell<f64>>, Rc<RefCell<Vec<f64>>>) {
    let value = Rc::new(RefCell::new(50.0));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window({
        let value = value.clone();
        let changes = changes.clone();

        move |_window, _cx| NumberInputDebugRegressionView { value, changes }
    });

    let cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    (cx, value, changes)
}

fn click(cx: &mut VisualTestContext, point: gpui::Point<gpui::Pixels>) {
    cx.simulate_mouse_down(point, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(point, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();
}

fn click_selector(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("missing selector {selector}"));
    click(cx, bounds.center());
}

fn click_increment(cx: &mut VisualTestContext) {
    click_selector(cx, "number-debug-main-inc");
}

fn click_decrement(cx: &mut VisualTestContext) {
    click_selector(cx, "number-debug-main-dec");
}

fn click_value(cx: &mut VisualTestContext) {
    click_selector(cx, "number-debug-main-value");
}

fn blur_to_other_number(cx: &mut VisualTestContext) {
    let bounds = cx
        .debug_bounds("number-debug-blur-target")
        .expect("blur target bounds should be available");
    click(cx, bounds.center());
}

#[gpui::test]
async fn test_number_input_debug_plus_changes_value(cx: &mut TestAppContext) {
    let (mut cx, value, changes) = setup(cx);

    click_increment(&mut cx);

    assert_eq!(*value.borrow(), 51.0);
    assert_eq!(changes.borrow().as_slice(), &[51.0]);
}

#[gpui::test]
async fn test_number_input_debug_minus_changes_value(cx: &mut TestAppContext) {
    let (mut cx, value, changes) = setup(cx);

    click_decrement(&mut cx);

    assert_eq!(*value.borrow(), 49.0);
    assert_eq!(changes.borrow().as_slice(), &[49.0]);
}

#[gpui::test]
async fn test_number_input_debug_entering_value_changes_value(cx: &mut TestAppContext) {
    let (mut cx, value, changes) = setup(cx);

    click_value(&mut cx);
    cx.simulate_input("73");
    cx.run_until_parked();
    blur_to_other_number(&mut cx);

    assert_eq!(
        *value.borrow(),
        73.0,
        "typed number should commit when focus leaves; changes: {:?}",
        changes.borrow().as_slice()
    );
}

#[gpui::test]
async fn test_number_input_debug_pasting_value_changes_value(cx: &mut TestAppContext) {
    let (mut cx, value, changes) = setup(cx);

    click_value(&mut cx);
    cx.write_to_clipboard(ClipboardItem::new_string("42".to_string()));
    #[cfg(target_os = "macos")]
    cx.simulate_keystrokes("cmd-v");
    #[cfg(not(target_os = "macos"))]
    cx.simulate_keystrokes("ctrl-v");
    cx.run_until_parked();
    blur_to_other_number(&mut cx);

    assert_eq!(
        *value.borrow(),
        42.0,
        "pasted number should commit when focus leaves; changes: {:?}",
        changes.borrow().as_slice()
    );
}
