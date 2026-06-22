impl Showcase {
    fn render_drag_list_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionDragList);
        let _theme = cx.theme();
        let entity = self.entity.clone();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Vertical drag list
            .child(Text::new("Vertical:").weight(TextWeight::Semibold))
            .child(
                div().w_full().max_w(px(400.0)).child(
                    DragList::new(
                        "drag-vertical",
                        self.drag_vertical_items
                            .iter()
                            .map(|id| {
                                let label = match id.as_ref() {
                                    "eq" => "Parametric EQ",
                                    "comp" => "Compressor",
                                    "limiter" => "Limiter",
                                    "upmixer" => "Upmixer",
                                    _ => "Unknown",
                                };
                                DragItem::new(id.clone(), div().child(label))
                            })
                            .collect(),
                    )
                    .on_reorder({
                        let entity = entity.clone();
                        move |from, to, _window, cx| {
                            entity.update(cx, |this, _cx| {
                                if from < this.drag_vertical_items.len()
                                    && to < this.drag_vertical_items.len()
                                    && from != to
                                {
                                    let item = this.drag_vertical_items.remove(from);
                                    this.drag_vertical_items.insert(to, item);
                                }
                            });
                        }
                    }),
                ),
            )
            // Horizontal drag list
            .child(Text::new("Horizontal:").weight(TextWeight::Semibold))
            .child(
                DragList::new(
                    "drag-horizontal",
                    self.drag_horizontal_items
                        .iter()
                        .map(|id| DragItem::new(id.clone(), div().child(id.to_string())))
                        .collect(),
                )
                .orientation(DragListOrientation::Horizontal)
                .on_reorder(move |from, to, _window, cx| {
                    entity.update(cx, |this, _cx| {
                        if from < this.drag_horizontal_items.len()
                            && to < this.drag_horizontal_items.len()
                            && from != to
                        {
                            let item = this.drag_horizontal_items.remove(from);
                            this.drag_horizontal_items.insert(to, item);
                        }
                    });
                }),
            )
    }
}
