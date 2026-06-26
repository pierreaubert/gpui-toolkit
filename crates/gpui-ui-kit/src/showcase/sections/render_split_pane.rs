use super::prelude::*;

impl Showcase {
    pub(crate) fn render_split_pane_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionSplitPane);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Horizontal split
            .child(Text::new("Horizontal split (30/70):").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .h(px(200.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        SplitPane::new("split-h")
                            .direction(SplitDirection::Horizontal)
                            .ratio(0.3)
                            .first(
                                div()
                                    .p_4()
                                    .bg(theme.surface)
                                    .size_full()
                                    .child(Text::new("Left Panel").weight(TextWeight::Semibold)),
                            )
                            .second(
                                div()
                                    .p_4()
                                    .size_full()
                                    .child(Text::new("Right Panel (main content)")),
                            ),
                    ),
            )
            // Vertical split
            .child(Text::new("Vertical split (50/50):").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(600.0))
                    .h(px(300.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        SplitPane::new("split-v")
                            .direction(SplitDirection::Vertical)
                            .ratio(0.5)
                            .first(
                                div()
                                    .p_4()
                                    .bg(theme.surface)
                                    .size_full()
                                    .child(Text::new("Top Panel").weight(TextWeight::Semibold)),
                            )
                            .second(
                                div()
                                    .p_4()
                                    .size_full()
                                    .child(Text::new("Bottom Panel")),
                            ),
                    ),
            )
    }
}
