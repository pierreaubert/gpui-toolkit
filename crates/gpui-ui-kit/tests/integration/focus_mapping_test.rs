use gpui::prelude::{InteractiveElement, IntoElement, ParentElement, Styled};
use gpui::{
    AnyWindowHandle, AppContext as _, Context, ElementId, FocusHandle, Render, TestAppContext,
    Window, div,
};
use gpui_ui_kit::accessibility::{AccessibilityNode, AccessibilityTree, AriaProps, AriaRole};
use gpui_ui_kit::{Button, Input, TabItem, Tabs};
use std::cell::Cell;
use std::rc::Rc;

struct FocusMappingView {
    focus_a: FocusHandle,
    focus_b: FocusHandle,
    show_a: Rc<Cell<bool>>,
    include_sibling_b: bool,
    include_nested: bool,
}

impl Render for FocusMappingView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div().size_full();
        if self.include_sibling_b {
            root = root.child(
                div()
                    .id("b")
                    .track_focus_element(&self.focus_b)
                    .w(gpui::px(100.))
                    .h(gpui::px(40.)),
            );
        }

        if self.show_a.get() {
            root = root.child(
                div()
                    .id("a")
                    .track_focus_element(&self.focus_a)
                    .w(gpui::px(100.))
                    .h(gpui::px(40.)),
            );
        }

        if self.include_nested {
            root = root.child(
                div().id("parent").track_focus_element(&self.focus_a).child(
                    div()
                        .id("child")
                        .track_focus_element(&self.focus_b)
                        .w(gpui::px(100.))
                        .h(gpui::px(40.)),
                ),
            );
        }

        root
    }
}

fn draw(cx: &mut TestAppContext, window: AnyWindowHandle) {
    cx.update_window(window, |_, window, cx| window.draw(cx).clear())
        .unwrap();
}

fn focus_and_draw(cx: &mut TestAppContext, window: AnyWindowHandle, handle: &FocusHandle) {
    cx.update_window(window, |_, window, cx| window.focus(handle, cx))
        .unwrap();
    cx.run_until_parked();
    draw(cx, window);
}

fn focused_id(cx: &mut TestAppContext, window: AnyWindowHandle) -> Option<ElementId> {
    cx.update_window(window, |_, window, cx| window.focused_element_id(cx))
        .unwrap()
}

#[gpui::test]
async fn focus_mapping_is_stable_and_frame_scoped(cx: &mut TestAppContext) {
    let (focus_a, focus_b, unregistered) =
        cx.update(|cx| (cx.focus_handle(), cx.focus_handle(), cx.focus_handle()));
    let show_a = Rc::new(Cell::new(true));
    let window = cx.add_window({
        let show_a = show_a.clone();
        let focus_a = focus_a.clone();
        let focus_b = focus_b.clone();
        move |_window, _cx| FocusMappingView {
            focus_a,
            focus_b,
            show_a,
            include_sibling_b: true,
            include_nested: false,
        }
    });
    let window: AnyWindowHandle = window.into();

    draw(cx, window);
    focus_and_draw(cx, window, &unregistered);
    assert_eq!(focused_id(cx, window), None);

    focus_and_draw(cx, window, &focus_a);
    assert_eq!(focused_id(cx, window), Some(ElementId::from("a")));

    // Re-rendering the same ID/handle retains the mapping.
    draw(cx, window);
    assert_eq!(focused_id(cx, window), Some(ElementId::from("a")));

    focus_and_draw(cx, window, &focus_b);
    assert_eq!(focused_id(cx, window), Some(ElementId::from("b")));

    // An unregistered focused handle must not inherit the previous element.
    focus_and_draw(cx, window, &unregistered);
    assert_eq!(focused_id(cx, window), None);

    // Omitting the focused element clears its mapping on the next frame.
    show_a.set(false);
    cx.refresh().unwrap();
    cx.run_until_parked();
    draw(cx, window);
    focus_and_draw(cx, window, &focus_a);
    show_a.set(false);
    cx.refresh().unwrap();
    cx.run_until_parked();
    draw(cx, window);
    assert_eq!(focused_id(cx, window), None);
}

struct SharedElementView {
    focus: FocusHandle,
}

impl Render for SharedElementView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(div().id("shared").track_focus_element(&self.focus))
    }
}

#[gpui::test]
async fn focus_mapping_is_window_local_even_for_overlapping_element_ids(cx: &mut TestAppContext) {
    let (focus_a, focus_b) = cx.update(|cx| (cx.focus_handle(), cx.focus_handle()));
    let window_a = cx.add_window({
        let focus_a = focus_a.clone();
        move |_window, _cx| SharedElementView { focus: focus_a }
    });
    let window_b = cx.add_window({
        let focus_b = focus_b.clone();
        move |_window, _cx| SharedElementView { focus: focus_b }
    });
    let window_a: AnyWindowHandle = window_a.into();
    let window_b: AnyWindowHandle = window_b.into();

    draw(cx, window_a);
    draw(cx, window_b);
    focus_and_draw(cx, window_a, &focus_a);
    assert_eq!(focused_id(cx, window_a), Some(ElementId::from("shared")));
    assert_eq!(focused_id(cx, window_b), None);

    focus_and_draw(cx, window_b, &focus_b);
    assert_eq!(focused_id(cx, window_a), Some(ElementId::from("shared")));
    assert_eq!(focused_id(cx, window_b), Some(ElementId::from("shared")));
}

#[gpui::test]
async fn nested_focus_mapping_reports_the_exact_focused_owner(cx: &mut TestAppContext) {
    let (focus_a, focus_b) = cx.update(|cx| (cx.focus_handle(), cx.focus_handle()));
    let window = cx.add_window({
        let focus_a = focus_a.clone();
        let focus_b = focus_b.clone();
        move |_window, _cx| FocusMappingView {
            focus_a,
            focus_b,
            show_a: Rc::new(Cell::new(true)),
            include_sibling_b: false,
            include_nested: true,
        }
    });
    let window: AnyWindowHandle = window.into();

    focus_and_draw(cx, window, &focus_b);
    assert_eq!(focused_id(cx, window), Some(ElementId::from("child")));

    // The parent uses the other handle, so the child remains the exact owner
    // instead of being approximated through containment.
    cx.update_window(window, |_, window, cx| window.focus(&focus_b, cx))
        .unwrap();
    assert!(
        cx.update_window(window, |_, window, cx| {
            window.is_element_focused(&ElementId::from("child"), cx)
        })
        .unwrap()
    );
}

#[cfg(debug_assertions)]
struct DuplicateElementView {
    focus_a: FocusHandle,
    focus_b: FocusHandle,
}

#[cfg(debug_assertions)]
impl Render for DuplicateElementView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .child(div().id("duplicate").track_focus_element(&self.focus_a))
            .child(div().id("duplicate").track_focus_element(&self.focus_b))
    }
}

#[cfg(debug_assertions)]
#[gpui::test]
#[should_panic(expected = "was registered")]
async fn duplicate_element_ids_with_distinct_handles_are_diagnosed(cx: &mut TestAppContext) {
    let (focus_a, focus_b) = cx.update(|cx| (cx.focus_handle(), cx.focus_handle()));
    let window = cx.add_window(move |_window, _cx| DuplicateElementView { focus_a, focus_b });
    let window: AnyWindowHandle = window.into();
    draw(cx, window);
}

struct FocusMappingComponentsView;

impl Render for FocusMappingComponentsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(Button::new("button", "Button"))
            .child(Input::new("input"))
            .child(Tabs::new("tabs").tabs(vec![
                TabItem::new("tab-one", "One"),
                TabItem::new("tab-two", "Two"),
            ]))
    }
}

#[gpui::test]
async fn components_register_their_stable_element_ids(cx: &mut TestAppContext) {
    let window = cx.add_window(|_window, _cx| FocusMappingComponentsView);
    let window: AnyWindowHandle = window.into();
    draw(cx, window);

    let mapping = cx
        .update_window(window, |_, window, cx| window.focus_element_mapping(cx))
        .unwrap();
    let ids: Vec<_> = mapping.into_iter().map(|(id, _)| id).collect();
    assert!(ids.contains(&ElementId::from("button")));
    assert!(ids.contains(&ElementId::from("input")));
    assert!(ids.contains(&ElementId::from("tabs")));
}

#[gpui::test]
async fn bridge_snapshot_marks_at_most_one_registered_focused_node(cx: &mut TestAppContext) {
    let (focus_a, focus_b) = cx.update(|cx| (cx.focus_handle(), cx.focus_handle()));
    cx.update(|cx| {
        let mut tree = AccessibilityTree::new();
        tree.register(AccessibilityNode {
            element_id: ElementId::from("a"),
            label: "A".into(),
            props: AriaProps::with_role(AriaRole::Button),
        });
        tree.register(AccessibilityNode {
            element_id: ElementId::from("b"),
            label: "B".into(),
            props: AriaProps::with_role(AriaRole::Button),
        });
        cx.set_global(tree);
    });
    let window = cx.add_window({
        let focus_a = focus_a.clone();
        let focus_b = focus_b.clone();
        move |_window, _cx| FocusMappingView {
            focus_a,
            focus_b,
            show_a: Rc::new(Cell::new(true)),
            include_sibling_b: true,
            include_nested: false,
        }
    });
    let window: AnyWindowHandle = window.into();

    focus_and_draw(cx, window, &focus_a);
    let snapshot = cx
        .update_window(window, |_, window, cx| {
            cx.global::<AccessibilityTree>()
                .to_bridge_snapshot_for_window(window, cx)
        })
        .unwrap();
    let focused: Vec<_> = snapshot.nodes.iter().filter(|node| node.focused).collect();
    assert_eq!(focused.len(), 1);
    assert_eq!(focused[0].element_id, ElementId::from("a"));

    focus_and_draw(cx, window, &focus_b);
    let snapshot = cx
        .update_window(window, |_, window, cx| {
            cx.global::<AccessibilityTree>()
                .to_bridge_snapshot_for_window(window, cx)
        })
        .unwrap();
    assert_eq!(snapshot.nodes.iter().filter(|node| node.focused).count(), 1);
}
