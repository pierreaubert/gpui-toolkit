use super::prelude::*;

impl Showcase {
    pub(crate) fn render_hover_card_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionHoverCard);
        let theme = cx.theme();

        let trigger = |id: &str, label: &str| {
            div()
                .px_4()
                .py_2()
                .bg(theme.surface)
                .border_1()
                .border_color(theme.border)
                .rounded_md()
                .text_sm()
                .text_color(theme.text_primary)
                .child(label.to_string())
                .id(SharedString::from(format!("hover-card-trigger-{id}")))
        };

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Placements
            .child(Text::new("Placements:").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Xl)
                    .child(
                        div().relative().child(trigger("top", "Top")).child(
                            HoverCard::new("showcase-hover-top", "Above the target")
                                .description("Placed on top.")
                                .placement(HoverCardPlacement::Top)
                                .open(true),
                        ),
                    )
                    .child(
                        div().relative().child(trigger("bottom", "Bottom")).child(
                            HoverCard::new("showcase-hover-bottom", "Below the target")
                                .description("Placed below.")
                                .placement(HoverCardPlacement::Bottom)
                                .open(true),
                        ),
                    )
                    .child(
                        div().relative().child(trigger("left", "Left")).child(
                            HoverCard::new("showcase-hover-left", "Left of the target")
                                .placement(HoverCardPlacement::Left)
                                .open(true),
                        ),
                    )
                    .child(
                        div().relative().child(trigger("right", "Right")).child(
                            HoverCard::new("showcase-hover-right", "Right of the target")
                                .placement(HoverCardPlacement::Right)
                                .open(true),
                        ),
                    ),
            )
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                div().relative().child(trigger("sizes", "Sizes")).child(
                    HoverCard::new("showcase-hover-lg", "Ada Lovelace")
                        .description("First programmer, Analytical Engine.")
                        .size(HoverCardSize::Lg)
                        .open(true),
                ),
            )
    }
}
