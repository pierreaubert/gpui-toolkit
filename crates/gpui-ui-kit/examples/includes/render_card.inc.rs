impl Showcase {
    fn render_card_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionCards);
        let cancel = cx.t(TranslationKey::ButtonCancel);
        let save = cx.t(TranslationKey::ButtonSave);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .wrap(true)
                    .child(
                        Card::new()
                            .header(
                                div()
                                    .child(Heading::h3("Card Title"))
                                    .child(Text::new("Card subtitle").size(TextSize::Sm)),
                            )
                            .content(
                                Text::new("This is the card content. Cards can contain any content including text, images, and other components."),
                            )
                            .footer(
                                HStack::new()
                                    .justify(StackJustify::End)
                                    .spacing(StackSpacing::Sm)
                                    .child(Button::new("card-cancel", cancel).variant(ButtonVariant::Ghost))
                                    .child(Button::new("card-save", save).variant(ButtonVariant::Primary)),
                            )
                            .style(|d| d.w(px(300.0))),
                    )
                    .child(
                        Card::new()
                            .header(Heading::h3("Simple Card"))
                            .content(
                                VStack::new()
                                    .spacing(StackSpacing::Sm)
                                    .child(Text::new("* Feature one"))
                                    .child(Text::new("* Feature two"))
                                    .child(Text::new("* Feature three")),
                            )
                            .style(|d| d.w(px(250.0))),
                    ),
            )
    }
}
