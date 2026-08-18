//! Integration tests for accessibility tree registration
//!
//! Verifies that components register themselves in the AccessibilityTree
//! during render with correct roles, labels, and states.

use gpui::{Context, ElementId, IntoElement, ParentElement, Render, TestAppContext, Window, div};
use gpui_ui_kit::accessibility::{AccessibilityTree, AriaRole, AriaState};
use gpui_ui_kit::menu::{Menu, MenuItem};
use gpui_ui_kit::{Button, Checkbox, Slider, Toggle};

struct ButtonA11yView;

impl Render for ButtonA11yView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Button::new("ok-btn", "OK"))
    }
}

#[gpui::test]
async fn test_button_registers_in_accessibility_tree(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(AccessibilityTree::new());
    });

    let _window = cx.add_window(|_window, _cx| ButtonA11yView);

    cx.update(|cx| {
        let tree = cx.try_global::<AccessibilityTree>();
        assert!(tree.is_some(), "AccessibilityTree should be set as global");
        let tree = tree.unwrap();
        assert!(!tree.is_empty(), "Tree should have registered nodes");

        let node = tree.get(&ElementId::Name("ok-btn".into()));
        assert!(node.is_some(), "Button should be registered");
        let node = node.unwrap();
        assert_eq!(node.props.role, AriaRole::Button);
        assert_eq!(node.label.as_ref(), "OK");
    });
}

struct CheckboxA11yView;

impl Render for CheckboxA11yView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Checkbox::new("terms-cb")
                .checked(true)
                .label("Accept terms"),
        )
    }
}

#[gpui::test]
async fn test_checkbox_registers_checked_state(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(AccessibilityTree::new());
    });

    let _window = cx.add_window(|_window, _cx| CheckboxA11yView);

    cx.update(|cx| {
        let tree = cx.global::<AccessibilityTree>();
        let node = tree.get(&ElementId::Name("terms-cb".into()));
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.props.role, AriaRole::Checkbox);
        assert_eq!(node.label.as_ref(), "Accept terms");
        assert!(
            node.props.states.contains(&AriaState::Checked(true)),
            "Checkbox should have Checked(true) state"
        );
    });
}

struct SliderA11yView;

impl Render for SliderA11yView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Slider::new("volume").value(75.0).label("Volume"))
    }
}

#[gpui::test]
async fn test_slider_registers_value_range(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(AccessibilityTree::new());
    });

    let _window = cx.add_window(|_window, _cx| SliderA11yView);

    cx.update(|cx| {
        let tree = cx.global::<AccessibilityTree>();
        let node = tree.get(&ElementId::Name("volume".into()));
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.props.role, AriaRole::Slider);
        assert_eq!(node.label.as_ref(), "Volume");
        assert_eq!(node.props.value_now, Some(75.0));
        assert_eq!(node.props.value_min, Some(0.0));
        assert_eq!(node.props.value_max, Some(100.0));
    });
}

struct ToggleA11yView;

impl Render for ToggleA11yView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(Toggle::new("dark-mode").checked(false).label("Dark Mode"))
    }
}

#[gpui::test]
async fn test_toggle_registers_as_switch(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(AccessibilityTree::new());
    });

    let _window = cx.add_window(|_window, _cx| ToggleA11yView);

    cx.update(|cx| {
        let tree = cx.global::<AccessibilityTree>();
        let node = tree.get(&ElementId::Name("dark-mode".into()));
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(node.props.role, AriaRole::Switch);
        assert!(node.props.states.contains(&AriaState::Checked(false)));
    });
}

struct CustomAriaView;

impl Render for CustomAriaView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Button::new("custom-btn", "X")
                .aria_label("Close dialog")
                .aria_role(AriaRole::Button),
        )
    }
}

#[gpui::test]
async fn test_custom_aria_label_overrides_text(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(AccessibilityTree::new());
    });

    let _window = cx.add_window(|_window, _cx| CustomAriaView);

    cx.update(|cx| {
        let tree = cx.global::<AccessibilityTree>();
        let node = tree.get(&ElementId::Name("custom-btn".into()));
        assert!(node.is_some());
        let node = node.unwrap();
        assert_eq!(
            node.label.as_ref(),
            "Close dialog",
            "aria_label should override the button text"
        );
    });
}

struct MenuA11yView;

impl Render for MenuA11yView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().child(
            Menu::new(
                "actions-menu",
                vec![
                    MenuItem::new("open", "Open"),
                    MenuItem::checkbox("pin", "Pin", true).disabled(true),
                    MenuItem::separator(),
                ],
            )
            .aria_label("Actions"),
        )
    }
}

#[gpui::test]
async fn test_menu_registers_named_items_and_states(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_global(AccessibilityTree::new()));
    let _window = cx.add_window(|_window, _cx| MenuA11yView);

    cx.update(|cx| {
        let tree = cx.global::<AccessibilityTree>();
        let menu = tree
            .get(&ElementId::Name("actions-menu".into()))
            .expect("menu should be registered");
        assert_eq!(menu.props.role, AriaRole::Menu);
        assert_eq!(menu.label.as_ref(), "Actions");

        let open = tree
            .get(&ElementId::Name("menu-item-open".into()))
            .expect("menu item should be registered");
        assert_eq!(open.props.role, AriaRole::Menuitem);
        assert_eq!(open.label.as_ref(), "Open");

        let pin = tree
            .get(&ElementId::Name("menu-item-pin".into()))
            .expect("checkbox menu item should be registered");
        assert_eq!(pin.props.role, AriaRole::Checkbox);
        assert!(pin.props.states.contains(&AriaState::Checked(true)));
        assert!(pin.props.states.contains(&AriaState::Disabled));

        let snapshot = tree.to_bridge_snapshot();
        assert!(snapshot.all_nodes_named());
    });
}
