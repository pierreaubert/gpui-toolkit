//! Sidebar component tests

use gpui::div;
use gpui::prelude::{IntoElement, ParentElement};
use gpui_ui_kit::sidebar::{Sidebar, SidebarSide};

#[test]
fn test_sidebar_creation() {
    let sidebar = Sidebar::new("nav-sidebar");
    drop(sidebar);
}

#[test]
fn test_sidebar_both_sides() {
    let sides = [SidebarSide::Left, SidebarSide::Right];

    for side in &sides {
        let sidebar = Sidebar::new("test").side(*side);
        drop(sidebar);
    }
}

#[test]
fn test_sidebar_width() {
    let sidebar = Sidebar::new("test").width(gpui::px(300.0));
    drop(sidebar);
}

#[test]
fn test_sidebar_collapsed() {
    let sidebar = Sidebar::new("test").collapsed(true);
    drop(sidebar);

    let sidebar = Sidebar::new("test").collapsed(false);
    drop(sidebar);
}

#[test]
fn test_sidebar_content() {
    let sidebar = Sidebar::new("test").content(div().child("Navigation items"));
    drop(sidebar);
}

#[test]
fn test_sidebar_content_with_factory() {
    let sidebar = Sidebar::new("test")
        .content_with(|_theme| div().child("Themed navigation").into_any_element());
    drop(sidebar);
}

#[test]
fn test_sidebar_header() {
    let sidebar = Sidebar::new("test").header(div().child("App Logo"));
    drop(sidebar);
}

#[test]
fn test_sidebar_footer() {
    let sidebar = Sidebar::new("test").footer(div().child("Settings"));
    drop(sidebar);
}

#[test]
fn test_sidebar_show_border() {
    let sidebar = Sidebar::new("test").show_border(true);
    drop(sidebar);

    let sidebar = Sidebar::new("test").show_border(false);
    drop(sidebar);
}

#[test]
fn test_sidebar_full_configuration() {
    let sidebar = Sidebar::new("nav-sidebar")
        .side(SidebarSide::Left)
        .width(gpui::px(260.0))
        .collapsed(false)
        .show_border(true)
        .header(div().child("Header"))
        .content(div().child("Content"))
        .footer(div().child("Footer"));
    drop(sidebar);
}
