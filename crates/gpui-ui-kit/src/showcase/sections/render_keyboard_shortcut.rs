use super::prelude::*;

impl Showcase {
    pub(crate) fn render_keyboard_shortcut_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionKeyboardShortcut);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Small size
            .child(Text::new("Small:").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .child(KeyboardShortcutLabel::new("Cmd+K").size(KeyboardShortcutSize::Sm))
                    .child(KeyboardShortcutLabel::new("Ctrl+Shift+P").size(KeyboardShortcutSize::Sm))
                    .child(KeyboardShortcutLabel::new("Alt+F4").size(KeyboardShortcutSize::Sm)),
            )
            // Medium size
            .child(Text::new("Medium (default):").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .child(KeyboardShortcutLabel::new("Cmd+K").size(KeyboardShortcutSize::Md))
                    .child(KeyboardShortcutLabel::new("Ctrl+Shift+P").size(KeyboardShortcutSize::Md))
                    .child(KeyboardShortcutLabel::new("Alt+F4").size(KeyboardShortcutSize::Md)),
            )
            // Large size
            .child(Text::new("Large:").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .child(KeyboardShortcutLabel::new("Cmd+K").size(KeyboardShortcutSize::Lg))
                    .child(KeyboardShortcutLabel::new("Ctrl+Shift+P").size(KeyboardShortcutSize::Lg))
                    .child(KeyboardShortcutLabel::new("Alt+F4").size(KeyboardShortcutSize::Lg)),
            )
    }
}
