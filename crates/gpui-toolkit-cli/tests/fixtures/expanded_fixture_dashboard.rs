// Expanded by `gpui-toolkit layout expand`; do not hand-edit.
use gpui::IntoElement;
use gpui::div;
use gpui::prelude::*;
use gpui_ui_kit::{Button, ButtonVariant, HStack, Input, VStack};

pub fn fixture_dashboard() -> impl IntoElement {
    VStack::new().child(div().child("Dashboard")).child(HStack::new().child(Input::new("name").label("Name")).child(Button::new("go", "Go").variant(ButtonVariant::Primary)))
}
