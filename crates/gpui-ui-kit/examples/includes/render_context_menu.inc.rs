impl Showcase {
    fn render_context_menu_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionContextMenu);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                Text::new("Context menu items preview (right-click triggered, shown inline for showcase):")
                    .muted(true),
            )
            // Show menu items as a static preview
            .child(
                div()
                    .w(px(220.0))
                    .bg(theme.surface)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .shadow_md()
                    .py_1()
                    .flex()
                    .flex_col()
                    .child(
                        div().px_3().py_1().text_sm().text_color(theme.text_secondary).child("Cut"),
                    )
                    .child(
                        div().px_3().py_1().text_sm().text_color(theme.text_secondary).child("Copy"),
                    )
                    .child(
                        div().px_3().py_1().text_sm().text_color(theme.text_secondary).child("Paste"),
                    )
                    .child(
                        div().h(px(1.0)).mx_2().my_1().bg(theme.border),
                    )
                    .child(
                        div().px_3().py_1().text_sm().text_color(theme.text_muted).child("Disabled Item"),
                    )
                    .child(
                        div().h(px(1.0)).mx_2().my_1().bg(theme.border),
                    )
                    .child(
                        div().px_3().py_1().text_sm().text_color(theme.error).child("Delete"),
                    ),
            )
            // Item types
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(Badge::new("Normal").variant(BadgeVariant::Default))
                    .child(Badge::new("Separator").variant(BadgeVariant::Info))
                    .child(Badge::new("Disabled").variant(BadgeVariant::Warning))
                    .child(Badge::new("Danger").variant(BadgeVariant::Error)),
            )
    }
}
