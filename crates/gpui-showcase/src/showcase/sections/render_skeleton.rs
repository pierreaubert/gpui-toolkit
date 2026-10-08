use super::prelude::*;

impl Showcase {
    pub(crate) fn render_skeleton_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionSkeleton);

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Text lines
            .child(Text::new("Text lines:").weight(TextWeight::Semibold))
            .child(Skeleton::new("skeleton-text-sm").size(SkeletonSize::Sm))
            .child(Skeleton::new("skeleton-text-md"))
            .child(Skeleton::new("skeleton-text-lg").size(SkeletonSize::Lg))
            // Rectangle and circle
            .child(Text::new("Shapes:").weight(TextWeight::Semibold))
            .child(Skeleton::new("skeleton-rect").variant(SkeletonVariant::Rectangular))
            .child(Skeleton::new("skeleton-circle").variant(SkeletonVariant::Circular))
            // Fixed overrides
            .child(Text::new("Fixed size:").weight(TextWeight::Semibold))
            .child(
                Skeleton::new("skeleton-fixed")
                    .variant(SkeletonVariant::Rectangular)
                    .width(160.0)
                    .height(48.0),
            )
    }
}
