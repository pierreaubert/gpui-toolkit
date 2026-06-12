impl SpinoramaApp {
    fn render_legend(
        &self,
        colors: &HashMap<&'static str, D3Color>,
        ds: &DesignSystem,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Div {
        let font_size = (12.0 * self.font_scale()).round();
        let row_height = (font_size * 1.25).ceil().max(18.0);
        let marker_width = 16.0;
        let marker_height = 3.0;
        let marker_top = ((row_height - marker_height) * 0.5).round();

        div()
            .flex()
            .flex_wrap()
            .justify_center()
            .items_center()
            .gap(px(ds.spacing.section_gap))
            .p(px(ds.spacing.card_padding))
            .bg(theme.muted)
            .rounded(px(ds.corners.md))
            .children(CEA2034_CURVES.iter().map(|&name| {
                let key = name.to_string();
                let is_hidden = self.hidden_cea2034_curves.contains(name);
                let color = colors
                    .get(name)
                    .cloned()
                    .unwrap_or(D3Color::rgb(128, 128, 128));
                let (r, g, b) = (
                    (color.r * 255.0) as u32,
                    (color.g * 255.0) as u32,
                    (color.b * 255.0) as u32,
                );

                div()
                    .flex()
                    .h(px(row_height))
                    .gap(px(ds.spacing.control_gap))
                    .rounded(px(ds.corners.sm))
                    .cursor_pointer()
                    .opacity(if is_hidden { 0.35 } else { 1.0 })
                    .hover(|el| el.bg(theme.surface_hover))
                    .child(
                        div()
                            .relative()
                            .flex_none()
                            .w(px(marker_width))
                            .h(px(row_height))
                            .child(
                                div()
                                    .absolute()
                                    .left_0()
                                    .top(px(marker_top))
                                    .w(px(marker_width))
                                    .h(px(marker_height))
                                    .bg(rgb(r << 16 | g << 8 | b)),
                            ),
                    )
                    .child(
                        div()
                            .h(px(row_height))
                            .line_height(px(row_height))
                            .text_size(px(font_size))
                            .text_color(theme.text_primary)
                            .child(name),
                    )
                    .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _window, cx| {
                        if !this.hidden_cea2034_curves.insert(key.clone()) {
                            this.hidden_cea2034_curves.remove(&key);
                        }
                        cx.notify();
                    }))
            }))
    }
}
