impl Showcase {
    fn render_text_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTypography);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Headings
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Heading::h1("Heading 1 (h1)"))
                    .child(Heading::h2("Heading 2 (h2)"))
                    .child(Heading::h3("Heading 3 (h3)"))
                    .child(Heading::h4("Heading 4 (h4)")),
            )
            // Text variants
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Regular text with default styling"))
                    .child(Text::new("Bold text").weight(TextWeight::Bold))
                    .child(Text::new("Medium weight text").weight(TextWeight::Medium))
                    .child(Text::new("Light weight text").weight(TextWeight::Light))
                    .child(Text::new("Small text").size(TextSize::Sm))
                    .child(Text::new("Extra small text").size(TextSize::Xs)),
            )
            // Code and Links
            .child(
                HStack::new()
                    .spacing(StackSpacing::Lg)
                    .child(Code::new("inline_code()"))
                    .child(Link::new("link-1", "Clickable Link")),
            )
    }
}
