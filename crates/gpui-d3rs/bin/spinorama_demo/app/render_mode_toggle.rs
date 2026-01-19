impl SpinoramaApp {
    /// Render a toggle button for switching between isoline and surface modes
    fn render_mode_toggle<T: Fn(&mut Self, &mut Context<Self>) + 'static>(
        &self,
        mode: ContourRenderMode,
        id: &'static str,
        on_click: T,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<T> {
        let entity = cx.entity().clone();
        let entity_for_colormap = cx.entity().clone();
        let colormap = self.contour_colormap;

        div()
            .id(id)
            .flex()
            .items_center()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_sm().text_color(rgb(0x666666)).child("Render:"))
                    .child(
                        div()
                            .id(ElementId::Name(format!("{}-btn", id).into()))
                            .flex()
                            .items_center()
                            .px_3()
                            .py_1()
                            .bg(rgb(0xe0e0e0))
                            .border_1()
                            .border_color(rgb(0xcccccc))
                            .rounded_md()
                            .cursor_pointer()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0x333333))
                            .hover(|s| s.bg(rgb(0xd0d0d0)))
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
                    .gap_2()
                    .child(div().text_sm().text_color(rgb(0x666666)).child("Colormap:"))
                    .child(
                        div()
                            .id(ElementId::Name(format!("{}-colormap-btn", id).into()))
                            .flex()
                            .items_center()
                            .px_3()
                            .py_1()
                            .bg(rgb(0xe0e0e0))
                            .border_1()
                            .border_color(rgb(0xcccccc))
                            .rounded_md()
                            .cursor_pointer()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0x333333))
                            .hover(|s| s.bg(rgb(0xd0d0d0)))
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

