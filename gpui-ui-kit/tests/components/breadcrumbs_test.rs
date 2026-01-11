//! Breadcrumbs component tests

use gpui_ui_kit::breadcrumbs::{BreadcrumbItem, BreadcrumbSeparator, Breadcrumbs};

#[test]
fn test_breadcrumbs_creation() {
    let items = vec![
        BreadcrumbItem::new("home", "Home").icon("🏠"),
        BreadcrumbItem::new("library", "Library").href("/library"),
        BreadcrumbItem::new("album", "Album Title"),
    ];

    let breadcrumbs = Breadcrumbs::new()
        .items(items)
        .separator(BreadcrumbSeparator::Chevron)
        .on_click(|id, _window, _cx| {
            println!("Clicked: {}", id);
        });

    drop(breadcrumbs);
}

#[test]
fn test_breadcrumb_separators() {
    let separators = [
        BreadcrumbSeparator::Slash,
        BreadcrumbSeparator::Chevron,
        BreadcrumbSeparator::Dot,
    ];

    for sep in &separators {
        let bc = Breadcrumbs::new().separator(*sep);
        drop(bc);
    }
}
