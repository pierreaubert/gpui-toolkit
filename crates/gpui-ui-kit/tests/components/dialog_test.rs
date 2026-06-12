//! Dialog component tests

use gpui::div;
use gpui::prelude::{IntoElement, ParentElement};
use gpui_ui_kit::dialog::{Dialog, DialogSize};

#[test]
fn test_dialog_configuration() {
    let dialog = Dialog::new("my-dialog")
        .title("Dialog Title")
        .size(DialogSize::Md)
        .content(div().child("Body content"))
        .footer(div().child("Footer buttons"))
        .show_close_button(true)
        .close_on_backdrop(true)
        .on_close(|_window, _cx| {});

    drop(dialog);
}

#[test]
fn test_dialog_sizes() {
    let sizes = [
        DialogSize::Sm,
        DialogSize::Md,
        DialogSize::Lg,
        DialogSize::Xl,
        DialogSize::Full,
    ];

    for size in &sizes {
        let dialog = Dialog::new("test").size(*size);
        drop(dialog);
    }
}

#[test]
fn test_dialog_content_with_factory() {
    let dialog =
        Dialog::new("test").content_with(|_theme| div().child("Themed content").into_any_element());
    drop(dialog);
}

#[test]
fn test_dialog_footer_with_factory() {
    let dialog =
        Dialog::new("test").footer_with(|_theme| div().child("Themed footer").into_any_element());
    drop(dialog);
}

#[test]
fn test_dialog_child_alias() {
    let dialog = Dialog::new("test").child(div().child("Content via child()"));
    drop(dialog);
}

#[test]
fn test_dialog_close_on_backdrop() {
    let dialog = Dialog::new("test")
        .close_on_backdrop(true)
        .on_close(|_window, _cx| {});
    drop(dialog);

    let dialog = Dialog::new("test").close_on_backdrop(false);
    drop(dialog);
}

#[test]
fn test_dialog_no_close_button() {
    let dialog = Dialog::new("test")
        .show_close_button(false)
        .content(div().child("No close button"));
    drop(dialog);
}
