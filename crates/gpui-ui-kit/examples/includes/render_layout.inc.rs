impl Showcase {
    fn render_layout_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionLayout);
        let theme = cx.theme();

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // HStack and VStack
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("HStack & VStack").weight(TextWeight::Medium))
                    .child(
                        HStack::new()
                            .spacing(StackSpacing::Lg)
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(
                                        div()
                                            .p_2()
                                            .bg(theme.surface_hover)
                                            .rounded_md()
                                            .child("VStack Item 1"),
                                    )
                                    .child(
                                        div()
                                            .p_2()
                                            .bg(theme.surface_hover)
                                            .rounded_md()
                                            .child("VStack Item 2"),
                                    )
                                    .child(
                                        div()
                                            .p_2()
                                            .bg(theme.surface_hover)
                                            .rounded_md()
                                            .child("VStack Item 3"),
                                    ),
                            )
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Xs)
                                    .child(
                                        div()
                                            .p_2()
                                            .bg(theme.surface_hover)
                                            .rounded_md()
                                            .child("H1"),
                                    )
                                    .child(
                                        div()
                                            .p_2()
                                            .bg(theme.surface_hover)
                                            .rounded_md()
                                            .child("H2"),
                                    )
                                    .child(
                                        div()
                                            .p_2()
                                            .bg(theme.surface_hover)
                                            .rounded_md()
                                            .child("H3"),
                                    ),
                            ),
                    ),
            )
            // Flex grow demonstration
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Flex Grow (flex_1)").weight(TextWeight::Medium))
                    .child(Text::new("Panels expand to fill available space equally").size(TextSize::Sm).color(theme.text_muted))
                    .child(
                        HStack::new()
                            .width(StackSize::Fixed(px(500.0)))
                            .height(StackSize::Fixed(px(80.0)))
                            .spacing(StackSpacing::Sm)
                            .child(
                                VStack::new()
                                    .flex_1()
                                    .height(StackSize::Full)
                                    .justify(StackJustify::Center)
                                    .align(StackAlign::Center)
                                    .child(div().p_3().bg(theme.accent).rounded_md().child(
                                        Text::new("flex_1").color(theme.background),
                                    )),
                            )
                            .child(
                                VStack::new()
                                    .flex_1()
                                    .height(StackSize::Full)
                                    .justify(StackJustify::Center)
                                    .align(StackAlign::Center)
                                    .child(div().p_3().bg(theme.success).rounded_md().child(
                                        Text::new("flex_1").color(theme.background),
                                    )),
                            )
                            .child(
                                VStack::new()
                                    .flex_1()
                                    .height(StackSize::Full)
                                    .justify(StackJustify::Center)
                                    .align(StackAlign::Center)
                                    .child(div().p_3().bg(theme.warning).rounded_md().child(
                                        Text::new("flex_1").color(theme.background),
                                    )),
                            ),
                    ),
            )
            // Size options demonstration
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Size Options").weight(TextWeight::Medium))
                    .child(Text::new("Fixed, Fraction, and Full sizing").size(TextSize::Sm).color(theme.text_muted))
                    .child(
                        VStack::new()
                            .width(StackSize::Fixed(px(500.0)))
                            .spacing(StackSpacing::Xs)
                            .child(
                                HStack::new()
                                    .width(StackSize::Full)
                                    .child(
                                        div()
                                            .p_2()
                                            .bg(theme.surface_hover)
                                            .rounded_md()
                                            .child(Text::new("width: Full (100%)").size(TextSize::Sm)),
                                    ),
                            )
                            .child(
                                HStack::new()
                                    .width(StackSize::Fraction(0.5))
                                    .child(
                                        div()
                                            .p_2()
                                            .bg(theme.accent)
                                            .rounded_md()
                                            .child(Text::new("width: 50%").size(TextSize::Sm).color(theme.background)),
                                    ),
                            )
                            .child(
                                HStack::new()
                                    .width(StackSize::Fixed(px(150.0)))
                                    .child(
                                        div()
                                            .p_2()
                                            .bg(theme.success)
                                            .rounded_md()
                                            .child(Text::new("width: 150px").size(TextSize::Sm).color(theme.background)),
                                    ),
                            ),
                    ),
            )
            // Wrap demonstration
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Flex Wrap").weight(TextWeight::Medium))
                    .child(Text::new("Items wrap to next line when they don't fit").size(TextSize::Sm).color(theme.text_muted))
                    .child(
                        HStack::new()
                            .width(StackSize::Fixed(px(300.0)))
                            .wrap(true)
                            .spacing(StackSpacing::Xs)
                            .child(div().px_3().py_1().bg(theme.accent).rounded_md().child(Text::new("Tag 1").size(TextSize::Sm).color(theme.background)))
                            .child(div().px_3().py_1().bg(theme.success).rounded_md().child(Text::new("Tag 2").size(TextSize::Sm).color(theme.background)))
                            .child(div().px_3().py_1().bg(theme.warning).rounded_md().child(Text::new("Tag 3").size(TextSize::Sm).color(theme.background)))
                            .child(div().px_3().py_1().bg(theme.error).rounded_md().child(Text::new("Tag 4").size(TextSize::Sm).color(theme.background)))
                            .child(div().px_3().py_1().bg(theme.accent).rounded_md().child(Text::new("Tag 5").size(TextSize::Sm).color(theme.background)))
                            .child(div().px_3().py_1().bg(theme.success).rounded_md().child(Text::new("Tag 6").size(TextSize::Sm).color(theme.background))),
                    ),
            )
            // Justify options demonstration
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Justify Options").weight(TextWeight::Medium))
                    .child(
                        VStack::new()
                            .width(StackSize::Fixed(px(400.0)))
                            .spacing(StackSpacing::Xs)
                            .child(
                                HStack::new()
                                    .width(StackSize::Full)
                                    .justify(StackJustify::Start)
                                    .spacing(StackSpacing::Xs)
                                    .child(div().px_2().py_1().bg(theme.surface_hover).rounded_sm().child(Text::new("Start").size(TextSize::Xs)))
                                    .child(div().px_2().py_1().bg(theme.surface_hover).rounded_sm().child(Text::new("A").size(TextSize::Xs)))
                                    .child(div().px_2().py_1().bg(theme.surface_hover).rounded_sm().child(Text::new("B").size(TextSize::Xs))),
                            )
                            .child(
                                HStack::new()
                                    .width(StackSize::Full)
                                    .justify(StackJustify::Center)
                                    .spacing(StackSpacing::Xs)
                                    .child(div().px_2().py_1().bg(theme.surface_hover).rounded_sm().child(Text::new("Center").size(TextSize::Xs)))
                                    .child(div().px_2().py_1().bg(theme.surface_hover).rounded_sm().child(Text::new("A").size(TextSize::Xs)))
                                    .child(div().px_2().py_1().bg(theme.surface_hover).rounded_sm().child(Text::new("B").size(TextSize::Xs))),
                            )
                            .child(
                                HStack::new()
                                    .width(StackSize::Full)
                                    .justify(StackJustify::End)
                                    .spacing(StackSpacing::Xs)
                                    .child(div().px_2().py_1().bg(theme.surface_hover).rounded_sm().child(Text::new("End").size(TextSize::Xs)))
                                    .child(div().px_2().py_1().bg(theme.surface_hover).rounded_sm().child(Text::new("A").size(TextSize::Xs)))
                                    .child(div().px_2().py_1().bg(theme.surface_hover).rounded_sm().child(Text::new("B").size(TextSize::Xs))),
                            )
                            .child(
                                HStack::new()
                                    .width(StackSize::Full)
                                    .justify(StackJustify::SpaceBetween)
                                    .child(div().px_2().py_1().bg(theme.accent).rounded_sm().child(Text::new("Between").size(TextSize::Xs).color(theme.background)))
                                    .child(div().px_2().py_1().bg(theme.accent).rounded_sm().child(Text::new("A").size(TextSize::Xs).color(theme.background)))
                                    .child(div().px_2().py_1().bg(theme.accent).rounded_sm().child(Text::new("B").size(TextSize::Xs).color(theme.background))),
                            ),
                    ),
            )
            // Spacer demonstration
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Spacer").weight(TextWeight::Medium))
                    .child(
                        HStack::new().spacing(StackSpacing::Md).child(
                            div()
                                .w(px(400.0))
                                .p_3()
                                .bg(theme.surface)
                                .rounded_md()
                                .flex()
                                .items_center()
                                .child(Text::new("Left"))
                                .child(Spacer::new())
                                .child(Text::new("Right")),
                        ),
                    ),
            )
            // Dividers
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Dividers").weight(TextWeight::Medium))
                    .child(
                        VStack::new()
                            .spacing(StackSpacing::Md)
                            .child(
                                div()
                                    .w(px(300.0))
                                    .child(Divider::new().color(theme.border_hover).build()),
                            )
                            .child(
                                div().w(px(300.0)).child(
                                    Divider::new()
                                        .thickness(px(2.0))
                                        .color(theme.accent)
                                        .build(),
                                ),
                            )
                            .child(
                                HStack::new()
                                    .spacing(StackSpacing::Md)
                                    .child(Text::new("Left"))
                                    .child(div().h(px(20.0)).child(
                                        Divider::vertical().color(theme.border_hover).build(),
                                    ))
                                    .child(Text::new("Right")),
                            ),
                    ),
            )
            // Pane Dividers (interactive collapsible/resizable)
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Pane Dividers").weight(TextWeight::Medium))
                    .child(Text::new("Interactive dividers with collapse/expand and drag-to-resize support").size(TextSize::Sm).color(theme.text_muted))
                    .child(self.render_pane_divider_demo(cx)),
            )
    }

    fn render_pane_divider_demo(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let entity = self.entity.clone();
        let left_collapsed = self.pane_left_collapsed;
        let left_width = self.pane_left_width;
        let dragging = self.pane_dragging_left;

        // Build the demo layout
        let mut container = div()
            .id("pane-divider-demo-container")
            .w(px(600.0))
            .h(px(200.0))
            .flex()
            .bg(theme.surface)
            .rounded_lg()
            .border_1()
            .border_color(theme.border)
            .overflow_hidden();

        // Left panel (collapsible)
        if !left_collapsed {
            container = container.child(
                div()
                    .w(px(left_width))
                    .h_full()
                    .bg(theme.muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        VStack::new()
                            .align(StackAlign::Center)
                            .child(Text::new("Left Panel").weight(TextWeight::Medium))
                            .child(Text::new(format!("Width: {:.0}px", left_width)).size(TextSize::Sm).color(theme.text_muted)),
                    ),
            );
        }

        // Divider
        let entity_toggle = entity.clone();
        let entity_drag = entity.clone();
        container = container.child(
            PaneDivider::vertical("left-divider", CollapseDirection::Left)
                .label("Left")
                .collapsed(left_collapsed)
                .on_toggle(move |new_collapsed, _window, cx| {
                    entity_toggle.update(cx, |state, cx| {
                        state.pane_left_collapsed = new_collapsed;
                        cx.notify();
                    });
                })
                .on_drag_start(move |pos, _window, cx| {
                    entity_drag.update(cx, |state, cx| {
                        state.pane_dragging_left = true;
                        state.pane_drag_start_x = pos;
                        state.pane_drag_start_width = state.pane_left_width;
                        cx.notify();
                    });
                }),
        );

        // Right panel (main content)
        container = container.child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    VStack::new()
                        .align(StackAlign::Center)
                        .child(Text::new("Main Content").weight(TextWeight::Medium))
                        .child(Text::new("Double-click divider to collapse").size(TextSize::Sm).color(theme.text_muted))
                        .child(Text::new("Drag divider to resize").size(TextSize::Sm).color(theme.text_muted)),
                ),
        );

        // Add mouse tracking for drag
        if dragging {
            let entity_move = entity.clone();
            let entity_up = entity.clone();
            let start_x = self.pane_drag_start_x;
            let start_width = self.pane_drag_start_width;

            container = container
                .on_mouse_move(move |event, _window, cx| {
                    let current_x: f32 = event.position.x.into();
                    let delta = current_x - start_x;
                    let new_width = (start_width + delta).clamp(100.0, 400.0);
                    entity_move.update(cx, |state, cx| {
                        state.pane_left_width = new_width;
                        cx.notify();
                    });
                })
                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                    entity_up.update(cx, |state, cx| {
                        state.pane_dragging_left = false;
                        cx.notify();
                    });
                });
        }

        container
    }
}
