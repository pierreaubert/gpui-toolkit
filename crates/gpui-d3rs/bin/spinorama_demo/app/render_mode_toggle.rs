impl SpinoramaApp {
    /// Render a toggle button for switching between isoline and surface modes
    fn render_mode_toggle<T: Fn(&mut Self, &mut Context<Self>) + 'static>(
        &self,
        mode: ContourRenderMode,
        id: &'static str,
        on_click: T,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<T> {
        let theme = cx.theme();
        let ds = cx.design();
        let s = self.font_scale();
        let entity = cx.entity().clone();
        let entity_for_colormap = cx.entity().clone();
        let colormap = self.contour_colormap;

        div()
            .id(id)
            .flex()
            .items_center()
            .gap(px(ds.spacing.section_gap))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(ds.spacing.control_gap))
                    .child(div().text_size(px(ds.typography.small_size * s)).text_color(theme.text_secondary).child("Render:"))
                    .child(
                        div()
                            .id(ElementId::Name(format!("{}-btn", id).into()))
                            .flex()
                            .items_center()
                            .px(px(ds.spacing.control_padding_x))
                            .py(px(ds.spacing.control_padding_y * 0.5))
                            .bg(theme.muted)
                            .border_1()
                            .border_color(theme.text_secondary)
                            .rounded(px(ds.corners.md))
                            .cursor_pointer()
                            .text_size(px(ds.typography.small_size * s))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_primary)
                            .hover(|s| s.bg(theme.surface_hover))
                            .child(mode.label())
                            .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                entity.update(cx, |this, cx| {
                                    on_click(this, cx);
                                    cx.notify();
                                });
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(ds.spacing.control_gap))
                    .child(div().text_size(px(ds.typography.small_size * s)).text_color(theme.text_secondary).child("Colormap:"))
                    .child(
                        div()
                            .id(ElementId::Name(format!("{}-colormap-btn", id).into()))
                            .flex()
                            .items_center()
                            .px(px(ds.spacing.control_padding_x))
                            .py(px(ds.spacing.control_padding_y * 0.5))
                            .bg(theme.muted)
                            .border_1()
                            .border_color(theme.text_secondary)
                            .rounded(px(ds.corners.md))
                            .cursor_pointer()
                            .text_size(px(ds.typography.small_size * s))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_primary)
                            .hover(|s| s.bg(theme.surface_hover))
                            .child(colormap.label())
                            .on_mouse_down(MouseButton::Left, move |_, _window, cx| {
                                entity_for_colormap.update(cx, |this, cx| {
                                    this.contour_colormap = this.contour_colormap.next();
                                    cx.notify();
                                });
                            }),
                    ),
            )
    }
}
