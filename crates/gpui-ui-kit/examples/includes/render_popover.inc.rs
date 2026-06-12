impl Showcase {
    fn render_popover_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionPopover);
        let theme = cx.theme();
        let entity = self.entity.clone();
        let open = self.popover_open;

        let placements: &[(&str, &str, PopoverPlacement)] = &[
            ("top", "Top", PopoverPlacement::Top),
            ("bottom", "Bottom", PopoverPlacement::Bottom),
            ("left", "Left", PopoverPlacement::Left),
            ("right", "Right", PopoverPlacement::Right),
            ("top-start", "Top Start", PopoverPlacement::TopStart),
            ("top-end", "Top End", PopoverPlacement::TopEnd),
            ("bottom-start", "Bottom Start", PopoverPlacement::BottomStart),
            ("bottom-end", "Bottom End", PopoverPlacement::BottomEnd),
        ];

        let mut row1 = HStack::new().spacing(StackSpacing::Md);
        let mut row2 = HStack::new().spacing(StackSpacing::Md);
        let mut row3 = HStack::new().spacing(StackSpacing::Md);

        for &(id, label, placement) in placements {
            let is_open = open == Some(id);
            let entity_open = entity.clone();
            let entity_close = entity.clone();

            let mut trigger = div()
                .id(SharedString::from(format!("popover-trigger-{}", id)))
                .relative()
                .px_4()
                .py_2()
                .bg(if is_open { theme.accent } else { theme.surface })
                .border_1()
                .border_color(if is_open { theme.accent } else { theme.border })
                .rounded_md()
                .cursor_pointer()
                .text_sm()
                .text_color(if is_open {
                    rgba(0xffffffff)
                } else {
                    theme.text_primary
                })
                .hover(|s| s.bg(theme.surface_hover))
                .child(label)
                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                    entity_open.update(cx, |this, cx| {
                        if this.popover_open == Some(id) {
                            this.popover_open = None;
                        } else {
                            this.popover_open = Some(id);
                        }
                        cx.notify();
                    });
                });

            if is_open {
                let popover_content = div()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_primary)
                            .child(format!("Popover ({})", label)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child("This popover floats relative to its trigger button. Click outside to dismiss."),
                    );

                trigger = trigger.child(
                    Popover::new(format!("popover-{}", id))
                        .placement(placement)
                        .width(px(220.0))
                        .content(popover_content)
                        .on_close(move |_window, cx| {
                            entity_close.update(cx, |this, cx| {
                                this.popover_open = None;
                                cx.notify();
                            });
                        }),
                );
            }

            match placement {
                PopoverPlacement::Top | PopoverPlacement::Bottom | PopoverPlacement::Left | PopoverPlacement::Right => {
                    row1 = row1.child(trigger);
                }
                PopoverPlacement::TopStart | PopoverPlacement::TopEnd => {
                    row2 = row2.child(trigger);
                }
                PopoverPlacement::BottomStart | PopoverPlacement::BottomEnd => {
                    row3 = row3.child(trigger);
                }
            }
        }

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            .child(
                Text::new("Click any button to open a popover at that placement. Click outside to dismiss:")
                    .muted(true),
            )
            .child(
                VStack::new()
                    .spacing(StackSpacing::Md)
                    .child(row1)
                    .child(row2)
                    .child(row3),
            )
    }
}
