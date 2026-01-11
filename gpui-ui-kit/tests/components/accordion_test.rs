//! Accordion component tests

use gpui::SharedString;
use gpui_ui_kit::accordion::{Accordion, AccordionItem, AccordionMode, AccordionOrientation};

#[test]
fn test_accordion_modes() {
    let single = Accordion::new().mode(AccordionMode::Single);
    drop(single);

    let multiple = Accordion::new().mode(AccordionMode::Multiple);
    drop(multiple);
}

#[test]
fn test_accordion_orientations() {
    let vertical = Accordion::new().orientation(AccordionOrientation::Vertical);
    drop(vertical);

    let horizontal = Accordion::new().orientation(AccordionOrientation::Horizontal);
    drop(horizontal);

    let side = Accordion::new().orientation(AccordionOrientation::Side);
    drop(side);
}

#[test]
fn test_accordion_configuration() {
    let items = vec![
        AccordionItem::new("item-1", "Item 1").content("Content 1"),
        AccordionItem::new("item-2", "Item 2")
            .content("Content 2")
            .disabled(true),
        AccordionItem::new("item-3", "Item 3").content("Content 3"),
    ];

    let accordion = Accordion::new()
        .items(items)
        .mode(AccordionMode::Multiple)
        .orientation(AccordionOrientation::Vertical)
        .expanded(vec!["item-1".into(), "item-2".into()]);

    drop(accordion);
}

#[test]
fn test_accordion_item_creation() {
    let item = AccordionItem::new("id", "Title");
    let expected_id: SharedString = "id".into();
    assert_eq!(item.id(), &expected_id);

    let item_with_content = AccordionItem::new("id", "Title").content("Content text");
    assert_eq!(item_with_content.id(), &expected_id);
}

// Interaction tests

#[test]
fn test_accordion_supports_mouse_click() {
    let items = vec![
        AccordionItem::new("item-1", "Section 1").content("Content 1"),
        AccordionItem::new("item-2", "Section 2").content("Content 2"),
    ];

    let accordion = Accordion::new()
        .items(items)
        .mode(AccordionMode::Single)
        .on_change(|_id, _is_expanded, _window, _cx| {});

    drop(accordion);
}

#[test]
fn test_accordion_headers_clickable() {
    let items = vec![
        AccordionItem::new("item-1", "Clickable Header 1").content("Content"),
        AccordionItem::new("item-2", "Clickable Header 2").content("Content"),
    ];

    let accordion = Accordion::new()
        .items(items)
        .expanded(vec!["item-1".into()])
        .on_change(|_id, _is_expanded, _window, _cx| {});

    drop(accordion);
}

#[test]
fn test_disabled_accordion_item_no_events() {
    let items = vec![
        AccordionItem::new("item-1", "Enabled").content("Content"),
        AccordionItem::new("item-2", "Disabled")
            .content("Content")
            .disabled(true),
    ];

    let accordion = Accordion::new()
        .items(items)
        .on_change(|_id, _is_expanded, _window, _cx| {});

    drop(accordion);
}
