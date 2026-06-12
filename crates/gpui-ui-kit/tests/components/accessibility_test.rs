//! Accessibility module unit tests

use gpui_ui_kit::accessibility::{
    AccessibilityNode, AccessibilityTree, AriaLive, AriaProps, AriaRole, AriaState,
};

#[test]
fn test_aria_role_default_is_none() {
    assert_eq!(AriaRole::default(), AriaRole::None);
}

#[test]
fn test_aria_props_with_role() {
    let props = AriaProps::with_role(AriaRole::Button);
    assert_eq!(props.role, AriaRole::Button);
    assert!(props.states.is_empty());
}

#[test]
fn test_aria_props_builder_chain() {
    let props = AriaProps::with_role(AriaRole::Slider)
        .description("Adjust volume level")
        .value_range(50.0, 0.0, 100.0)
        .state(AriaState::Disabled);

    assert_eq!(props.role, AriaRole::Slider);
    assert_eq!(
        props.description.as_ref().map(|s| s.as_ref()),
        Some("Adjust volume level")
    );
    assert_eq!(props.value_now, Some(50.0));
    assert_eq!(props.value_min, Some(0.0));
    assert_eq!(props.value_max, Some(100.0));
    assert_eq!(props.states.len(), 1);
    assert_eq!(props.states[0], AriaState::Disabled);
}

#[test]
fn test_aria_props_maybe_state() {
    let props = AriaProps::with_role(AriaRole::Button)
        .maybe_state(true, AriaState::Disabled)
        .maybe_state(false, AriaState::Pressed(true));

    assert_eq!(props.states.len(), 1);
    assert_eq!(props.states[0], AriaState::Disabled);
}

#[test]
fn test_aria_props_live() {
    let props = AriaProps::with_role(AriaRole::Status).live(AriaLive::Polite);
    assert_eq!(props.live, Some(AriaLive::Polite));
}

#[test]
fn test_aria_props_level() {
    let props = AriaProps::with_role(AriaRole::Heading).level(2);
    assert_eq!(props.level, Some(2));
}

#[test]
fn test_aria_props_value_text() {
    let props = AriaProps::with_role(AriaRole::Slider)
        .value_range(50.0, 0.0, 100.0)
        .value_text("50%");
    assert_eq!(props.value_text.as_ref().map(|s| s.as_ref()), Some("50%"));
}

#[test]
fn test_accessibility_tree_operations() {
    let mut tree = AccessibilityTree::new();
    assert!(tree.is_empty());
    assert_eq!(tree.len(), 0);

    let node = AccessibilityNode {
        element_id: gpui::ElementId::Name("btn-ok".into()),
        label: "OK".into(),
        props: AriaProps::with_role(AriaRole::Button),
    };
    tree.register(node);

    assert_eq!(tree.len(), 1);
    assert!(!tree.is_empty());

    let retrieved = tree.get(&gpui::ElementId::Name("btn-ok".into()));
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.label.as_ref(), "OK");
    assert_eq!(retrieved.props.role, AriaRole::Button);
}

#[test]
fn test_accessibility_tree_order() {
    let mut tree = AccessibilityTree::new();

    tree.register(AccessibilityNode {
        element_id: gpui::ElementId::Name("first".into()),
        label: "First".into(),
        props: AriaProps::with_role(AriaRole::Button),
    });
    tree.register(AccessibilityNode {
        element_id: gpui::ElementId::Name("second".into()),
        label: "Second".into(),
        props: AriaProps::with_role(AriaRole::Checkbox),
    });

    let ordered = tree.nodes_in_order();
    assert_eq!(ordered.len(), 2);
    assert_eq!(ordered[0].label.as_ref(), "First");
    assert_eq!(ordered[1].label.as_ref(), "Second");
}

#[test]
fn test_accessibility_tree_clear() {
    let mut tree = AccessibilityTree::new();
    tree.register(AccessibilityNode {
        element_id: gpui::ElementId::Name("btn".into()),
        label: "Click".into(),
        props: AriaProps::with_role(AriaRole::Button),
    });
    assert_eq!(tree.len(), 1);

    tree.clear();
    assert!(tree.is_empty());
    assert!(tree.get(&gpui::ElementId::Name("btn".into())).is_none());
}

#[test]
fn test_button_aria_label_compiles() {
    let _ = gpui_ui_kit::Button::new("ok", "OK").aria_label("Confirm and close");
    let _ = gpui_ui_kit::Button::new("cancel", "Cancel").aria_role(AriaRole::Button);
}

#[test]
fn test_checkbox_aria_compiles() {
    let _ = gpui_ui_kit::Checkbox::new("cb")
        .checked(true)
        .aria_label("Accept terms");
}

#[test]
fn test_toggle_aria_compiles() {
    let _ = gpui_ui_kit::Toggle::new("tgl")
        .aria_label("Dark mode")
        .aria_role(AriaRole::Switch);
}

#[test]
fn test_slider_aria_compiles() {
    let _ = gpui_ui_kit::Slider::new("vol")
        .value(50.0)
        .aria_label("Volume");
}

#[test]
fn test_input_aria_compiles() {
    let _ = gpui_ui_kit::Input::new("name")
        .aria_label("Full name")
        .aria_role(AriaRole::Textbox);
}

#[test]
fn test_select_aria_compiles() {
    let _ = gpui_ui_kit::Select::new("country").aria_label("Select country");
}

#[test]
fn test_dialog_aria_compiles() {
    let _ = gpui_ui_kit::Dialog::new("dlg").aria_label("Settings dialog");
}

#[test]
fn test_aria_state_equality() {
    assert_eq!(AriaState::Checked(true), AriaState::Checked(true));
    assert_ne!(AriaState::Checked(true), AriaState::Checked(false));
    assert_ne!(AriaState::Checked(true), AriaState::Pressed(true));
    assert_eq!(AriaState::Mixed, AriaState::Mixed);
}
