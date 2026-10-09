// Expanded by `gpui-toolkit layout expand`; do not hand-edit.
use gpui::IntoElement;
use gpui_ui_kit::{Heading, HStack, StackSpacing, Text, TextSize, TextWeight, VStack};

pub fn fixture_styled() -> impl IntoElement {
    VStack::new().spacing(StackSpacing::Lg).child(Heading::h2("Analytics")).child(HStack::new().spacing(StackSpacing::Md).child(Text::new("$42k").size(TextSize::Lg)).child(Text::new("Revenue").weight(TextWeight::Bold).size(TextSize::Sm).muted(true)))
}
