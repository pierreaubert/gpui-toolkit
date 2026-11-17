//! Integration tests for `FocusTrap` keyboard containment.

use gpui::{
    Context, FocusHandle, IntoElement, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div,
};
use gpui_ui_kit::button::Button;
use gpui_ui_kit::focus::FocusTrap;

struct FocusTrapView {
    first: FocusHandle,
    middle: FocusHandle,
    last: FocusHandle,
}

impl Render for FocusTrapView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(
                FocusTrap::new("button-trap")
                    .focus_targets([self.first.clone(), self.middle.clone(), self.last.clone()])
                    .children([
                        Button::new("trap-first", "First").into_any_element(),
                        Button::new("trap-middle", "Middle").into_any_element(),
                        Button::new("trap-last", "Last").into_any_element(),
                    ]),
            )
            .child(Button::new("trap-outside", "Outside"))
    }
}

#[gpui::test]
async fn test_focus_trap_wraps_from_last_button(cx: &mut TestAppContext) {
    let (first, middle, last, outside_handle) = cx.update(|cx| {
        (
            Button::focus_handle_for("trap-first", cx),
            Button::focus_handle_for("trap-middle", cx),
            Button::focus_handle_for("trap-last", cx),
            Button::focus_handle_for("trap-outside", cx),
        )
    });
    let view_handles = (first.clone(), middle.clone(), last.clone());
    let window = cx.add_window(move |_window, _cx| FocusTrapView {
        first: view_handles.0,
        middle: view_handles.1,
        last: view_handles.2,
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    cx.update(|window, app| window.focus(&last, app));
    cx.run_until_parked();
    assert!(cx.update(|window, _| last.is_focused(window)));

    cx.simulate_keystrokes("tab");
    cx.run_until_parked();

    assert!(
        cx.update(|window, _| first.is_focused(window)),
        "Tab from the last trap target should wrap to the first"
    );
    assert!(
        !cx.update(|window, _| outside_handle.is_focused(window)),
        "Tab must not move focus outside the trap"
    );

    cx.simulate_keystrokes("shift-tab");
    cx.run_until_parked();
    assert!(
        cx.update(|window, _| last.is_focused(window)),
        "Shift+Tab from the first trap target should wrap to the last"
    );
    assert!(
        cx.update(|window, _| !middle.is_focused(window)),
        "wrap must not stop at the middle target"
    );
}
