impl Showcase {
    fn render_tabs_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let section_title = cx.t(TranslationKey::SectionTabs);
        let entity = self.entity.clone();
        let theme = cx.theme();
        let selected_tab = self.selected_tab;

        // Build custom tab buttons to handle clicks properly with entity context
        let tab_data = [("📊", "Overview", "This is the overview panel showing a summary of all activities."),
            ("📈", "Analytics", "Here you can see detailed analytics and statistics."),
            ("📋", "Reports", "View and generate reports from your data."),
            ("⚙️", "Settings", "Configure your preferences and settings here.")];

        VStack::new()
            .spacing(StackSpacing::Lg)
            .child(self.section_header(section_title))
            // Interactive Underline Tabs with Content
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Interactive Underline Tabs").weight(TextWeight::Medium))
                    .child({
                        let mut tab_bar = div().flex().items_center();

                        for (idx, (icon, label, _)) in tab_data.iter().enumerate() {
                            let is_selected = idx == selected_tab;
                            let entity_clone = entity.clone();
                            let accent = theme.accent;
                            let border = theme.border;
                            let text_selected = theme.text_primary;
                            let text_unselected = theme.text_muted;
                            let text_hover = theme.text_secondary;

                            let tab_content = {
                                let mut content = div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_4()
                                    .py_2()
                                    .text_sm()
                                    .text_color(if is_selected { text_selected } else { text_unselected })
                                    .child(div().child(*icon))
                                    .child(div().child(*label));

                                if is_selected {
                                    content = content.font_weight(FontWeight::SEMIBOLD);
                                } else {
                                    content = content.hover(move |s| s.text_color(text_hover));
                                }

                                content
                            };

                            let tab = div()
                                .id(SharedString::from(format!("custom-tab-{}", idx)))
                                .flex()
                                .flex_col()
                                .cursor_pointer()
                                .child(tab_content)
                                .child(
                                    div()
                                        .h(if is_selected { px(2.0) } else { px(1.0) })
                                        .w_full()
                                        .bg(if is_selected { accent } else { border })
                                )
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    entity_clone.update(cx, |this, cx| {
                                        this.selected_tab = idx;
                                        cx.notify();
                                    });
                                });

                            tab_bar = tab_bar.child(tab);
                        }

                        tab_bar
                    })
                    // Content panel
                    .child(
                        div()
                            .p_4()
                            .bg(theme.surface)
                            .rounded_lg()
                            .border_1()
                            .border_color(theme.border)
                            .child(
                                VStack::new()
                                    .spacing(StackSpacing::Md)
                                    .child(Text::new(tab_data[selected_tab].1).weight(TextWeight::Semibold))
                                    .child(Text::new(tab_data[selected_tab].2))
                            ),
                    ),
            )
            // Pills Tabs (static - for visual comparison)
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Pills Variant (Static)").weight(TextWeight::Medium))
                    .child(
                        Tabs::new("tabs")
                            .variant(TabVariant::Pills)
                            .selected_index(1)
                            .tabs(vec![
                                TabItem::new("pill-1", "All"),
                                TabItem::new("pill-2", "Active"),
                                TabItem::new("pill-3", "Completed"),
                            ]),
                    )
                    .child(
                        div()
                            .p_4()
                            .bg(theme.surface)
                            .rounded_lg()
                            .border_1()
                            .border_color(theme.border)
                            .child(
                                Text::new("Active tasks are displayed here. This shows the Pills variant styling."),
                            ),
                    ),
            )
            // Enclosed Tabs (static - for visual comparison)
            .child(
                VStack::new()
                    .spacing(StackSpacing::Sm)
                    .child(Text::new("Enclosed Variant (Static)").weight(TextWeight::Medium))
                    .child(
                        Tabs::new("tabs")
                            .variant(TabVariant::Enclosed)
                            .selected_index(0)
                            .tabs(vec![
                                TabItem::new("enc-1", "Files"),
                                TabItem::new("enc-2", "Folders"),
                                TabItem::new("enc-3", "Trash").badge("3"),
                            ]),
                    )
                    .child(
                        div()
                            .p_4()
                            .bg(theme.surface)
                            .rounded_lg()
                            .border_1()
                            .border_color(theme.border)
                            .child(Text::new("Your files are displayed here. This shows the Enclosed variant styling.")),
                    ),
            )
    }
}
