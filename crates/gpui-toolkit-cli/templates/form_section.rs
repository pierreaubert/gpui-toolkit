// LAYOUT (V > (I#form-name[label="Name"] + I#form-email[label="Email"]))
//! Form block template, emitted by `gpui-toolkit template form-section`.
//!
//! Reusable block: embed in any page layout.

use gpui::prelude::*;
use gpui_ui_kit::{Input, VStack};

/// Two labeled inputs stacked vertically.
pub fn form_section() -> impl IntoElement {
    VStack::new()
        .child(Input::new("form-name").label("Name").placeholder("Ada"))
        .child(
            Input::new("form-email")
                .label("Email")
                .placeholder("ada@example.com"),
        )
}
