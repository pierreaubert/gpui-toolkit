//! Integration tests for `FocusGroup` sizing, initial focus, and public helpers.

use gpui::{
    Context, FocusHandle, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, px,
};
use gpui_ui_kit::button::Button;
use gpui_ui_kit::focus::{FocusDirection, FocusGroup, FocusMove};

struct StyledGroupView {
    targets: Vec<FocusHandle>,
    initial_focus: bool,
}

impl Render for StyledGroupView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().items_start().child(
            FocusGroup::new("styled-group")
                .style(|style| {
                    style
                        .debug_selector(|| "styled-group".to_string())
                        .w(px(120.))
                        .h(px(44.))
                })
                .initial_focus(self.initial_focus)
                .focus_targets(self.targets.clone())
                .child(Button::new("group-first", "First"))
                .child(Button::new("group-second", "Second")),
        )
    }
}

#[gpui::test]
async fn test_focus_group_style_sizes_container_and_initial_focus(cx: &mut TestAppContext) {
    let (first_handle, second_handle) = cx.update(|cx| {
        (
            Button::focus_handle_for("group-first", cx),
            Button::focus_handle_for("group-second", cx),
        )
    });
    let targets = vec![first_handle.clone(), second_handle.clone()];
    let window = cx.add_window(move |_window, _cx| StyledGroupView {
        targets,
        initial_focus: true,
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let bounds = cx
        .debug_bounds("styled-group")
        .expect("styled focus group must be laid out");
    assert_eq!(f32::from(bounds.size.width), 120.);
    assert_eq!(f32::from(bounds.size.height), 44.);
    assert!(
        cx.update(|window, _| first_handle.is_focused(window)),
        "initial_focus must focus the first target when nothing has focus"
    );
}

#[test]
fn focus_group_navigation_helpers_are_public() {
    assert_eq!(
        FocusGroup::navigation_delta(FocusDirection::Vertical, "down", false),
        Some(FocusMove::Next)
    );
    assert_eq!(
        FocusGroup::target_index(3, Some(2), FocusMove::Next, true),
        Some(0)
    );
    assert_eq!(
        FocusGroup::target_index(3, Some(0), FocusMove::Previous, false),
        None
    );
}
