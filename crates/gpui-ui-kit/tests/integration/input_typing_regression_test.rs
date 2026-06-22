use gpui::{
    Context, IntoElement, Modifiers, MouseButton, ParentElement, Render, Styled, TestAppContext,
    VisualTestContext, Window, div, point, px,
};
use gpui_ui_kit::input::Input;
use std::cell::RefCell;
use std::rc::Rc;

struct InputTypingRegressionView {
    live_text: Rc<RefCell<String>>,
    text_changes: Rc<RefCell<Vec<String>>>,
}

struct InputCommitOnBlurRegressionView {
    value: Rc<RefCell<String>>,
    changes: Rc<RefCell<Vec<String>>>,
}

impl Render for InputCommitOnBlurRegressionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let value = self.value.borrow().clone();
        let value_for_change = self.value.clone();
        let changes = self.changes.clone();

        div()
            .size_full()
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div().w(px(300.)).child(
                    Input::new("input-blur-commit")
                        .value(value)
                        .placeholder("Type something here...")
                        .on_change(move |text, _window, _cx| {
                            *value_for_change.borrow_mut() = text.to_string();
                            changes.borrow_mut().push(text.to_string());
                        }),
                ),
            )
            .child(
                div()
                    .w(px(160.))
                    .child(Input::new("blur-target").value("").placeholder("outside")),
            )
    }
}

struct InputEmacsRegressionView {
    live_text: Rc<RefCell<String>>,
}

impl Render for InputEmacsRegressionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let live_text = self.live_text.clone();

        div().size_full().p_4().child(
            div().w(px(300.)).child(
                Input::new("input-emacs")
                    .value("")
                    .placeholder("Type something here...")
                    .on_text_change(move |text, _window, _cx| {
                        *live_text.borrow_mut() = text;
                    }),
            ),
        )
    }
}

struct InputCaretRegressionView;

impl Render for InputCaretRegressionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().p_4().child(
            div().w(px(600.)).child(
                Input::new("input-caret-gap")
                    .value("")
                    .placeholder("Type something here..."),
            ),
        )
    }
}

impl Render for InputTypingRegressionView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let live_text = self.live_text.clone();
        let text_changes = self.text_changes.clone();

        div().size_full().p_4().child(
            div().w(px(300.)).child(
                Input::new("input-debug-empty")
                    .value("")
                    .placeholder("Type something here...")
                    .on_text_change(move |text, _window, _cx| {
                        *live_text.borrow_mut() = text.clone();
                        text_changes.borrow_mut().push(text);
                    }),
            ),
        )
    }
}

#[gpui::test]
async fn test_input_debug_repeated_typing_builds_text(cx: &mut TestAppContext) {
    let live_text = Rc::new(RefCell::new(String::new()));
    let text_changes = Rc::new(RefCell::new(Vec::new()));

    let window = cx.add_window({
        let live_text = live_text.clone();
        let text_changes = text_changes.clone();

        move |_window, _cx| InputTypingRegressionView {
            live_text,
            text_changes,
        }
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let center = cx
        .debug_bounds("input-debug-empty")
        .map(|bounds| bounds.center())
        .unwrap_or_else(|| point(px(150.), px(24.)));

    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    cx.simulate_input("eee");
    cx.run_until_parked();

    assert_eq!(
        live_text.borrow().as_str(),
        "eee",
        "Typing e e e into an empty input should build the visible live text; changes: {:?}",
        text_changes.borrow().as_slice()
    );
    assert!(
        !text_changes.borrow().is_empty(),
        "on_text_change should be called while typing"
    );
}

#[gpui::test]
async fn test_input_debug_commits_live_text_on_blur(cx: &mut TestAppContext) {
    let value = Rc::new(RefCell::new("Hello World".to_string()));
    let changes = Rc::new(RefCell::new(Vec::new()));

    let window = cx.add_window({
        let value = value.clone();
        let changes = changes.clone();

        move |_window, _cx| InputCommitOnBlurRegressionView { value, changes }
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let input_center = cx
        .debug_bounds("input-blur-commit")
        .map(|bounds| bounds.center())
        .unwrap_or_else(|| point(px(150.), px(24.)));

    cx.simulate_mouse_down(input_center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(input_center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    cx.simulate_keystrokes("cmd-a");
    cx.simulate_input("test");
    cx.run_until_parked();

    let blur_center = cx
        .debug_bounds("blur-target")
        .map(|bounds| bounds.center())
        .unwrap_or_else(|| point(px(40.), px(80.)));
    cx.simulate_mouse_down(blur_center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(blur_center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    assert_eq!(
        value.borrow().as_str(),
        "test",
        "Clicking outside the input should commit the current live edit; changes: {:?}",
        changes.borrow().as_slice()
    );
}

#[gpui::test]
async fn test_input_debug_emacs_keybindings_edit_text(cx: &mut TestAppContext) {
    let live_text = Rc::new(RefCell::new(String::new()));

    let window = cx.add_window({
        let live_text = live_text.clone();
        move |_window, _cx| InputEmacsRegressionView { live_text }
    });

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let center = cx
        .debug_bounds("input-emacs")
        .map(|bounds| bounds.center())
        .unwrap_or_else(|| point(px(150.), px(24.)));

    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    cx.simulate_input("hello");
    cx.simulate_keystrokes("ctrl-a");
    cx.simulate_input("X");
    cx.simulate_keystrokes("ctrl-e ctrl-h ctrl-k");
    cx.run_until_parked();

    assert_eq!(
        live_text.borrow().as_str(),
        "Xhell",
        "Emacs keybindings should move/delete within the focused input"
    );
}

#[gpui::test]
async fn test_input_debug_caret_stays_near_rendered_text(cx: &mut TestAppContext) {
    let window = cx.add_window(move |_window, _cx| InputCaretRegressionView);

    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let center = cx
        .debug_bounds("input-caret-gap")
        .map(|bounds| bounds.center())
        .unwrap_or_else(|| point(px(150.), px(24.)));

    cx.simulate_mouse_down(center, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_up(center, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    cx.simulate_input("test input te");
    cx.run_until_parked();

    let input_bounds = cx
        .debug_bounds("input-caret-gap")
        .expect("input bounds should be available");
    let cursor_bounds = cx
        .debug_bounds("input-caret-gap-cursor")
        .expect("cursor bounds should be available");

    let cursor_offset: f32 = (cursor_bounds.origin.x - input_bounds.origin.x).into();
    assert!(
        cursor_offset < 170.0,
        "Caret should stay near the rendered text end; observed offset {cursor_offset}"
    );
}
