use super::prelude::*;

impl Showcase {
    pub(crate) fn render_carousel_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionCarousel);
        let entity = self.weak_entity_handle();
        let index = self.carousel_index;

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Interactive rotation
            .child(Text::new("Interactive rotation:").weight(TextWeight::Semibold))
            .child({
                let entity_clone = entity.clone();
                Carousel::new("tour")
                    .slide(CarouselSlide::new(
                        "Fast startup",
                        "First paint in milliseconds, even on large projects.",
                    ))
                    .slide(CarouselSlide::new(
                        "Portable themes",
                        "One theme definition across desktop, mobile, and web.",
                    ))
                    .slide(CarouselSlide::new(
                        "Typed props",
                        "Every story declares editable props for the lab.",
                    ))
                    .index(index)
                    .on_change(move |next, _window, cx| {
                        entity_clone.update(cx, |this, cx| {
                            this.carousel_index = next;
                            this.notify_content(cx);
                        });
                    })
            })
            // Sizes
            .child(Text::new("Sizes:").weight(TextWeight::Semibold))
            .child(
                Carousel::new("tour-sm")
                    .slide(CarouselSlide::new("Compact", "Small slide text."))
                    .size(CarouselSize::Sm),
            )
            .child(
                Carousel::new("tour-lg")
                    .slide(CarouselSlide::new("Large", "Large slide text."))
                    .size(CarouselSize::Lg),
            )
    }
}
