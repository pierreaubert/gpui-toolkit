//! Dialog component tests

use gpui::div;
use gpui::prelude::*;
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
