impl Showcase {
    fn render_spinners_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionSpinners);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Spinners
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Spinners").weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .align(StackAlign::End)
                            .child(Spinner::new().size(SpinnerSize::Xs))
                            .child(Spinner::new().size(SpinnerSize::Sm))
                            .child(Spinner::new().size(SpinnerSize::Md))
                            .child(Spinner::new().size(SpinnerSize::Lg))
                            .child(Spinner::new().size(SpinnerSize::Xl))
                            .child(Spinner::new().size(SpinnerSize::Md).label("Loading...")),
                    ),
            )
            // Loading Dots
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Loading Dots").weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .align(StackAlign::End)
                            .child(LoadingDots::new().size(SpinnerSize::Sm))
                            .child(LoadingDots::new().size(SpinnerSize::Md))
                            .child(LoadingDots::new().size(SpinnerSize::Lg))
                            .child(
                                LoadingDots::new()
                                    .size(SpinnerSize::Md)
                                    .color(rgb(0x2da44e)),
                            ),
                    ),
            )
    }
}
