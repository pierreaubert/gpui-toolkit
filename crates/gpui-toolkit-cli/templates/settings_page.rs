// LAYOUT (V > (Tx"Settings" + I#settings-name[label="Display name"] + I#settings-email[label="Email"] + (H > B.primary"Save"#settings-save + B"Cancel"#settings-cancel)))
//! Settings page template, emitted by `gpui-toolkit template settings-page`.
//!
//! Content-only: wrap in your own app shell and navigation.

use gpui::prelude::*;
use gpui::div;
use gpui_ui_kit::{Button, ButtonVariant, HStack, Input, VStack};

/// Settings page with two fields and save/cancel actions.
pub fn settings_page() -> impl IntoElement {
    VStack::new()
        .child(div().child("Settings"))
        .child(
            Input::new("settings-name")
                .label("Display name")
                .placeholder("Ada"),
        )
        .child(
            Input::new("settings-email")
                .label("Email")
                .placeholder("ada@example.com"),
        )
        .child(
            HStack::new()
                .child(Button::new("settings-save", "Save").variant(ButtonVariant::Primary))
                .child(Button::new("settings-cancel", "Cancel")),
        )
}
