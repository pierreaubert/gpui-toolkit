impl Showcase {
    fn render_tooltip_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTooltips);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(Text::new("Tooltip placements (shown inline for showcase):").muted(true))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Xl)
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Top").weight(TextWeight::Medium))
                            .child(Tooltip::new("Tooltip on top").placement(TooltipPlacement::Top)),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Bottom").weight(TextWeight::Medium))
                            .child(
                                Tooltip::new("Tooltip on bottom")
                                    .placement(TooltipPlacement::Bottom),
                            ),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Left").weight(TextWeight::Medium))
                            .child(Tooltip::new("Left tooltip").placement(TooltipPlacement::Left)),
                    )
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Sm)
                            .child(Text::new("Right").weight(TextWeight::Medium))
                            .child(
                                Tooltip::new("Right tooltip").placement(TooltipPlacement::Right),
                            ),
                    ),
            )
    }
}
