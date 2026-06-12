impl Showcase {
    fn render_sidebar_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionSidebar);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Left sidebar preview (expanded)
            .child(Text::new("Left sidebar (expanded):").weight(TextWeight::Semibold))
            .child(
                div()
                    .flex()
                    .h(px(150.0))
                    .w_full()
                    .max_w(px(600.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        Sidebar::new("preview-left")
                            .side(SidebarSide::Left)
                            .width(px(180.0))
                            .collapsed(false)
                            .content(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("Navigation").weight(TextWeight::Semibold).size(TextSize::Sm))
                                    .child(Text::new("Home").size(TextSize::Sm))
                                    .child(Text::new("Library").size(TextSize::Sm))
                                    .child(Text::new("Settings").size(TextSize::Sm)),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Text::new("Main Content").muted(true)),
                    ),
            )
            // Right sidebar preview (collapsed)
            .child(Text::new("Right sidebar (collapsed):").weight(TextWeight::Semibold))
            .child(
                div()
                    .flex()
                    .h(px(150.0))
                    .w_full()
                    .max_w(px(600.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Text::new("Main Content").muted(true)),
                    )
                    .child(
                        Sidebar::new("preview-right")
                            .side(SidebarSide::Right)
                            .width(px(180.0))
                            .collapsed(true)
                            .content(Text::new("Details Panel")),
                    ),
            )
    }
}
