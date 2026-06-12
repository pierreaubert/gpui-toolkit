//! ConfirmDialog component tests

use gpui_ui_kit::confirm_dialog::{ConfirmDialog, ConfirmDialogVariant};

#[test]
fn test_confirm_dialog_creation() {
    let dialog = ConfirmDialog::new("delete-confirm");
    drop(dialog);
}

#[test]
fn test_confirm_dialog_title() {
    let dialog = ConfirmDialog::new("test").title("Delete Album");
    drop(dialog);
}

#[test]
fn test_confirm_dialog_message() {
    let dialog = ConfirmDialog::new("test").message("Are you sure you want to delete this album?");
    drop(dialog);
}

#[test]
fn test_confirm_dialog_all_variants() {
    let variants = [
        ConfirmDialogVariant::Default,
        ConfirmDialogVariant::Destructive,
        ConfirmDialogVariant::Warning,
    ];

    for variant in &variants {
        let dialog = ConfirmDialog::new("test").variant(*variant);
        drop(dialog);
    }
}

#[test]
fn test_confirm_dialog_confirm_label() {
    let dialog = ConfirmDialog::new("test").confirm_label("Delete");
    drop(dialog);
}

#[test]
fn test_confirm_dialog_cancel_label() {
    let dialog = ConfirmDialog::new("test").cancel_label("Go Back");
    drop(dialog);
}

#[test]
fn test_confirm_dialog_on_confirm() {
    let dialog = ConfirmDialog::new("test").on_confirm(|_window, _cx| {});
    drop(dialog);
}

#[test]
fn test_confirm_dialog_on_cancel() {
    let dialog = ConfirmDialog::new("test").on_cancel(|_window, _cx| {});
    drop(dialog);
}

#[test]
fn test_confirm_dialog_full_configuration() {
    let dialog = ConfirmDialog::new("delete-confirm")
        .title("Delete Album")
        .message("Are you sure you want to delete this album? This action cannot be undone.")
        .variant(ConfirmDialogVariant::Destructive)
        .confirm_label("Delete")
        .cancel_label("Cancel")
        .on_confirm(|_window, _cx| {})
        .on_cancel(|_window, _cx| {});
    drop(dialog);
}
