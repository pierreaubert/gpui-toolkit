use super::prelude::*;

impl Showcase {
    pub(crate) fn render_toolbar_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionToolbar);
        let _theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Basic toolbar
            .child(Text::new("Basic toolbar with separator:").weight(TextWeight::Semibold))
            .child(
                Toolbar::new("toolbar-basic")
                    .item(ToolbarItem::button("tb-bold", "B"))
                    .item(ToolbarItem::button("tb-italic", "I"))
                    .item(ToolbarItem::button("tb-underline", "U"))
                    .separator()
                    .item(ToolbarItem::button("tb-left", "<"))
                    .item(ToolbarItem::button("tb-center", "=").active(true))
                    .item(ToolbarItem::button("tb-right", ">")),
            )
            // With active and disabled states
            .child(Text::new("Active and disabled buttons:").weight(TextWeight::Semibold))
            .child(
                Toolbar::new("toolbar-states")
                    .item(ToolbarItem::button("tb-play", "Play").active(true))
                    .item(ToolbarItem::button("tb-pause", "Pause"))
                    .item(ToolbarItem::button("tb-stop", "Stop"))
                    .separator()
                    .item(ToolbarItem::button("tb-record", "Record").disabled(true)),
            )
    }
}
