use super::prelude::*;

impl Showcase {
    pub(crate) fn render_empty_state_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionEmptyState);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                div()
                    .w_full()
                    .max_w(px(500.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        EmptyState::new("No albums found")
                            .description("Try adjusting your search filters or add new music to your library.")
                            .icon("O")
                            .action(
                                Button::new("empty-action", "Add Music")
                                    .variant(ButtonVariant::Primary),
                            ),
                    ),
            )
            // Minimal empty state
            .child(Text::new("Minimal (no icon, no action):").weight(TextWeight::Semibold))
            .child(
                div()
                    .w_full()
                    .max_w(px(500.0))
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .overflow_hidden()
                    .child(
                        EmptyState::new("No results")
                            .description("Your queue is empty."),
                    ),
            )
    }
}
