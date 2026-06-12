impl Showcase {
    fn render_drag_list_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionDragList);
        let _theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Vertical drag list
            .child(Text::new("Vertical:").weight(TextWeight::Semibold))
            .child(
                div().w_full().max_w(px(400.0)).child(
                    DragList::new(
                        "drag-vertical",
                        vec![
                            DragItem::new("eq", div().child("Parametric EQ")),
                            DragItem::new("comp", div().child("Compressor")),
                            DragItem::new("limiter", div().child("Limiter")),
                            DragItem::new("upmixer", div().child("Upmixer")),
                        ],
                    ),
                ),
            )
            // Horizontal drag list
            .child(Text::new("Horizontal:").weight(TextWeight::Semibold))
            .child(
                DragList::new(
                    "drag-horizontal",
                    vec![
                        DragItem::new("h-1", div().child("Track 1")),
                        DragItem::new("h-2", div().child("Track 2")),
                        DragItem::new("h-3", div().child("Track 3")),
                    ],
                )
                .orientation(DragListOrientation::Horizontal),
            )
    }
}
