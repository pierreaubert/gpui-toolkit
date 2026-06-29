use super::prelude::*;

impl Showcase {
    pub(crate) fn render_command_palette_section(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionCommandPalette);
        let _theme = cx.theme();

        let commands = vec![
            CommandItem::new("cmd-open", "Open File")
                .shortcut("Cmd+O")
                .category("File"),
            CommandItem::new("cmd-save", "Save")
                .shortcut("Cmd+S")
                .category("File"),
            CommandItem::new("cmd-settings", "Open Settings")
                .shortcut("Cmd+,")
                .category("Preferences"),
            CommandItem::new("cmd-theme", "Toggle Theme")
                .shortcut("Cmd+T")
                .category("Preferences"),
            CommandItem::new("cmd-palette", "Command Palette")
                .shortcut("Cmd+K")
                .category("Navigation"),
        ];

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                Text::new("Inline preview (normally rendered as overlay):")
                    .weight(TextWeight::Semibold),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .h(px(380.0))
                    .relative()
                    .child(
                        CommandPalette::new("cmd-demo", commands)
                            .placeholder("Type a command...")
                            .selected_index(0),
                    ),
            )
    }
}
