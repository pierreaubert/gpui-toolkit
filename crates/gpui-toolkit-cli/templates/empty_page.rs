// LAYOUT (V > Tx"Title")
//! Empty page template, emitted by `gpui-toolkit template empty-page`.
//!
//! Content-only: wrap in your own app shell and navigation. Hidden from
//! the default `template` list; pass `--all` to see it.

use gpui::div;
use gpui::prelude::*;
use gpui_ui_kit::VStack;

/// Empty page with a heading; add blocks below the title.
pub fn empty_page() -> impl IntoElement {
    VStack::new().child(div().child("Title"))
}
