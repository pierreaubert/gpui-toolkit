impl Showcase {
    fn render_badges_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionBadges);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Badge Variants
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(Badge::new("Default").variant(BadgeVariant::Default))
                    .child(Badge::new("Primary").variant(BadgeVariant::Primary))
                    .child(Badge::new("Success").variant(BadgeVariant::Success))
                    .child(Badge::new("Warning").variant(BadgeVariant::Warning))
                    .child(Badge::new("Error").variant(BadgeVariant::Error))
                    .child(Badge::new("Info").variant(BadgeVariant::Info)),
            )
            // Badge Sizes and Styles
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(Badge::new("Small").size(BadgeSize::Sm))
                    .child(Badge::new("Medium").size(BadgeSize::Md))
                    .child(Badge::new("Large").size(BadgeSize::Lg))
                    .child(Badge::new("Rounded").rounded(true))
                    .child(Badge::new("With Icon").icon("*")),
            )
            // Badge Dots
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(BadgeDot::new().variant(BadgeVariant::Default))
                    .child(BadgeDot::new().variant(BadgeVariant::Primary))
                    .child(BadgeDot::new().variant(BadgeVariant::Success))
                    .child(BadgeDot::new().variant(BadgeVariant::Warning))
                    .child(BadgeDot::new().variant(BadgeVariant::Error)),
            )
    }
}
