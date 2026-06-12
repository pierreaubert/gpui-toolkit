//! SearchBar component tests

use gpui_ui_kit::search_bar::{SearchBar, SearchBarSize};

#[test]
fn test_search_bar_creation() {
    let bar = SearchBar::new("library-search");
    drop(bar);
}

#[test]
fn test_search_bar_value() {
    let bar = SearchBar::new("test").value("hello");
    drop(bar);
}

#[test]
fn test_search_bar_placeholder() {
    let bar = SearchBar::new("test").placeholder("Search albums...");
    drop(bar);
}

#[test]
fn test_search_bar_all_sizes() {
    let sizes = [SearchBarSize::Sm, SearchBarSize::Md, SearchBarSize::Lg];

    for size in &sizes {
        let bar = SearchBar::new("test").size(*size);
        drop(bar);
    }
}

#[test]
fn test_search_bar_show_icon() {
    let bar = SearchBar::new("test").show_icon(true);
    drop(bar);

    let bar = SearchBar::new("test").show_icon(false);
    drop(bar);
}

#[test]
fn test_search_bar_show_clear() {
    let bar = SearchBar::new("test").show_clear(true);
    drop(bar);

    let bar = SearchBar::new("test").show_clear(false);
    drop(bar);
}

#[test]
fn test_search_bar_on_change() {
    let bar = SearchBar::new("test").on_change(|_text, _window, _cx| {});
    drop(bar);
}

#[test]
fn test_search_bar_on_submit() {
    let bar = SearchBar::new("test").on_submit(|_text, _window, _cx| {});
    drop(bar);
}

#[test]
fn test_search_bar_on_escape() {
    let bar = SearchBar::new("test").on_escape(|_window, _cx| {});
    drop(bar);
}

#[test]
fn test_search_bar_full_configuration() {
    let bar = SearchBar::new("library-search")
        .value("beethoven")
        .placeholder("Search albums...")
        .size(SearchBarSize::Md)
        .show_icon(true)
        .show_clear(true)
        .on_change(|_text, _window, _cx| {})
        .on_submit(|_text, _window, _cx| {})
        .on_escape(|_window, _cx| {});
    drop(bar);
}
