use super::prelude::*;

impl Showcase {
    pub(crate) fn render_image_view_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionImageView);
        let _theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Different sizes
            .child(Text::new("Various sizes:").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        ImageView::new("img-sm")
                            .size(px(64.0))
                            .rounded(px(8.0))
                            .placeholder_icon("S"),
                    )
                    .child(
                        ImageView::new("img-md")
                            .size(px(128.0))
                            .rounded(px(12.0))
                            .placeholder_icon("M"),
                    )
                    .child(
                        ImageView::new("img-lg")
                            .size(px(200.0))
                            .rounded(px(16.0))
                            .placeholder_icon("L"),
                    )
                    .build(),
            )
            // With border
            .child(Text::new("With border:").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        ImageView::new("img-border")
                            .size(px(128.0))
                            .rounded(px(8.0))
                            .show_border(true)
                            .placeholder_icon("B"),
                    )
                    .child(
                        ImageView::new("img-circle")
                            .size(px(128.0))
                            .rounded(px(64.0))
                            .show_border(true)
                            .placeholder_icon("C"),
                    )
                    .build(),
            )
            // Fit modes
            .child(Text::new("Fit modes (Cover, Contain, Fill):").weight(TextWeight::Semibold))
            .child(
                HStack::new()
                    .spacing(StackSpacing::Md)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Text::new("Cover").size(TextSize::Xs))
                            .child(
                                ImageView::new("img-cover")
                                    .width(px(120.0))
                                    .height(px(80.0))
                                    .fit(ImageFit::Cover)
                                    .show_border(true),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Text::new("Contain").size(TextSize::Xs))
                            .child(
                                ImageView::new("img-contain")
                                    .width(px(120.0))
                                    .height(px(80.0))
                                    .fit(ImageFit::Contain)
                                    .show_border(true),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(Text::new("Fill").size(TextSize::Xs))
                            .child(
                                ImageView::new("img-fill")
                                    .width(px(120.0))
                                    .height(px(80.0))
                                    .fit(ImageFit::Fill)
                                    .show_border(true),
                            ),
                    )
                    .build(),
            )
    }
}
