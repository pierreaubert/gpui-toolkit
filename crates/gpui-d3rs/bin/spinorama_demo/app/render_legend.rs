impl SpinoramaApp {
    fn render_legend(&self, colors: &HashMap<&'static str, D3Color>) -> Div {
        div()
            .flex()
            .flex_wrap()
            .gap_4()
            .p_4()
            .bg(rgb(0xf5f5f5))
            .rounded_md()
            .children(CEA2034_CURVES.iter().map(|&name| {
                let color = colors
                    .get(name)
                    .cloned()
                    .unwrap_or(D3Color::rgb(128, 128, 128));
                let (r, g, b) = (
                    (color.r * 255.0) as u32,
                    (color.g * 255.0) as u32,
                    (color.b * 255.0) as u32,
                );
                let font_config = VectorFontConfig::horizontal(12.0, hsla(0.0, 0.0, 0.2, 1.0));

                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(16.0)).h(px(3.0)).bg(rgb(r << 16 | g << 8 | b)))
                    .child(render_vector_text(name, &font_config))
            }))
    }
}

