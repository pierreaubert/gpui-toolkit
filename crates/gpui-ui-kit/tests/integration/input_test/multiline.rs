//! Composition must grow, preserve line breaks, and submit only on explicit Enter.

use gpui::{Context, FocusHandle, TestAppContext, VisualTestContext, Window, div, prelude::*, px};
use gpui_ui_kit::Input;
use std::{cell::RefCell, rc::Rc};

struct Composer {
    value: Rc<RefCell<String>>,
    submissions: Rc<RefCell<Vec<String>>>,
    focus: FocusHandle,
}

impl Render for Composer {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let value = self.value.borrow().clone();
        let draft = self.value.clone();
        let submissions = self.submissions.clone();
        div().w(px(240.0)).child(
            Input::new("multiline-composer-test")
                .multiline(true)
                .focus_handle(self.focus.clone())
                .value(value)
                .on_text_change(move |text, _, _| *draft.borrow_mut() = text.into())
                .on_change(move |text, _, _| submissions.borrow_mut().push(text.into())),
        )
    }
}

#[gpui::test]
async fn multiline_grows_and_blur_never_submits(cx: &mut TestAppContext) {
    let draft = Rc::new(RefCell::new(String::new()));
    let submissions = Rc::new(RefCell::new(Vec::new()));
    let window = cx.add_window(|window, cx| {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        Composer {
            value: draft.clone(),
            submissions: submissions.clone(),
            focus,
        }
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();
    let initial_height = cx
        .debug_bounds("multiline-composer-test")
        .expect("input rendered")
        .size
        .height;
    cx.simulate_input("first 🦀 line");
    cx.simulate_keystrokes("shift-enter");
    cx.simulate_input("second line");
    cx.simulate_keystrokes("shift-enter");
    cx.simulate_input("third line");
    cx.run_until_parked();
    assert_eq!(&*draft.borrow(), "first 🦀 line\nsecond line\nthird line");
    assert!(submissions.borrow().is_empty());
    assert!(
        cx.debug_bounds("multiline-composer-test")
            .unwrap()
            .size
            .height
            > initial_height
    );
    cx.update(|window, _| window.blur());
    cx.run_until_parked();
    assert!(
        submissions.borrow().is_empty(),
        "focus loss is never a send action"
    );
    window
        .update(&mut cx, |view, window, cx| view.focus.focus(window, cx))
        .unwrap();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert_eq!(submissions.borrow().as_slice(), &[draft.borrow().clone()]);
}
