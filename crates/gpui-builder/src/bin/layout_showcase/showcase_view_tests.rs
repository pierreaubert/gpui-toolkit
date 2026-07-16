use super::*;
use gpui::{Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px};
use gpui_design::DesignSystemState;

fn install_design_system(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_global(DesignSystemState::new()));
}

#[gpui::test]
async fn showcase_renders_and_tree_rows_are_selectable(cx: &mut TestAppContext) {
    install_design_system(cx);
    let window = cx.add_window(|_, _| ShowcaseView::new(None));
    let state_window = window;
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    assert!(cx.debug_bounds("showcase-root").is_some());
    assert!(cx.debug_bounds("content-area").is_some());
    assert!(cx.debug_bounds("div-v-sidebar").is_some());

    let main_row = cx
        .debug_bounds("tree-row-main")
        .expect("main solved-tree row should be rendered");
    cx.simulate_click(main_row.center(), Modifiers::default());
    cx.run_until_parked();

    state_window
        .update(&mut cx, |view, _, _| {
            assert_eq!(view.selected_node.as_deref(), Some("main"));
        })
        .unwrap();
}

#[gpui::test]
async fn divider_click_collapses_sidebar(cx: &mut TestAppContext) {
    install_design_system(cx);
    let window = cx.add_window(|_, _| ShowcaseView::new(None));
    let state_window = window;
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let divider = cx
        .debug_bounds("div-v-sidebar")
        .expect("horizontal layout should render sidebar divider");
    cx.simulate_click(divider.center(), Modifiers::default());
    cx.run_until_parked();

    state_window
        .update(&mut cx, |view, _, _| {
            assert!(view.sidebar_collapsed);
        })
        .unwrap();
}

#[gpui::test]
async fn divider_drag_resizes_without_collapsing_sidebar(cx: &mut TestAppContext) {
    install_design_system(cx);
    let window = cx.add_window(|_, _| ShowcaseView::new(None));
    let state_window = window;
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    let divider = cx
        .debug_bounds("div-v-sidebar")
        .expect("horizontal layout should render sidebar divider");
    let start = divider.center();
    let end = point(start.x + px(80.0), start.y);
    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::default());
    cx.simulate_mouse_move(end, Some(MouseButton::Left), Modifiers::default());
    cx.simulate_mouse_up(end, MouseButton::Left, Modifiers::default());
    cx.run_until_parked();

    state_window
        .update(&mut cx, |view, _, _| {
            assert!(view.sidebar_ratio_h > 0.22);
            assert!(!view.sidebar_collapsed);
            assert!(view.dragging.is_none());
        })
        .unwrap();
}
