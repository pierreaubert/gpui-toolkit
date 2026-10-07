// LAYOUT (V > (Tx"Dashboard" + (H > (V > Tx"$42k" + Tx"Revenue") + (V > Tx"128" + Tx"Orders")) + B.primary"New report"#dashboard-new))
//! Dashboard page template, emitted by `gpui-toolkit template dashboard-page`.
//!
//! Content-only: wrap in your own app shell and navigation.

use gpui::prelude::*;
use gpui::div;
use gpui_ui_kit::{Button, ButtonVariant, HStack, VStack};

/// Dashboard page with two stat blocks and a primary action.
pub fn dashboard_page() -> impl IntoElement {
    VStack::new()
        .child(div().child("Dashboard"))
        .child(
            HStack::new()
                .child(stat_block("Revenue", "$42k"))
                .child(stat_block("Orders", "128")),
        )
        .child(Button::new("dashboard-new", "New report").variant(ButtonVariant::Primary))
}

/// One stat block: prominent value over a muted label.
fn stat_block(label: &str, value: &str) -> impl IntoElement {
    VStack::new()
        .child(div().child(value))
        .child(div().child(label))
}
