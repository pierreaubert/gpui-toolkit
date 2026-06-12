impl SpinoramaApp {
    fn render_contour_plot(&mut self, cx: &mut Context<Self>) -> Div {
        let theme = cx.theme();
        let ds = cx.design();
        let s = self.font_scale();
        let has_contour_data = self.contour_data.is_some();
        let has_directivity_data = self
            .directivity_data
            .as_ref()
            .is_some_and(|d| !d.horizontal.is_empty());

        if !has_contour_data && !has_directivity_data {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_size(px(ds.typography.base_size * s))
                    .text_color(theme.text_secondary)
                    .child("No contour data available for this speaker."),
            );
        }

        let speaker_name = self.selected_speaker.as_deref().unwrap_or("Unknown");
        let spl_mode = self.contour_mode_spl;
        let directivity_mode = self.contour_mode_directivity;

        // Render toggle buttons with the contour plots
        let spl_toggle = self.render_mode_toggle(
            spl_mode,
            "spl-contour-toggle",
            |app, _cx| {
                app.contour_mode_spl = app.contour_mode_spl.next();
            },
            cx,
        );

        let directivity_toggle = self.render_mode_toggle(
            directivity_mode,
            "directivity-contour-toggle",
            |app, _cx| {
                app.contour_mode_directivity = app.contour_mode_directivity.next();
            },
            cx,
        );

        // Pre-render the contour plots
        let colormap = self.contour_colormap;
        let spl_contour = self.render_contour_from_contour_data(
            "SPL Horizontal Contour (Full 360°)",
            spl_mode,
            colormap,
            &ds,
            &theme,
        );
        let directivity_contour = self.render_contour_from_directivity(
            "Directivity Contour (SPL Horizontal)",
            directivity_mode,
            colormap,
            &ds,
            &theme,
        );

        div()
            .flex()
            .flex_col()
            .gap(px(ds.spacing.section_gap * 2.0))
            .child(
                div()
                    .text_size(px(ds.typography.large_size * s))
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_primary)
                    .child(format!("Horizontal Contour Plots - {}", speaker_name)),
            )
            // SPL Horizontal Contour (new format, -180 to +180) with toggle
            .when_some(spl_contour, |el, contour_div| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(ds.spacing.control_gap))
                        .child(spl_toggle)
                        .child(contour_div),
                )
            })
            // Directivity-based contour (old format, typically -60 to +60) with toggle
            .when_some(directivity_contour, |el, contour_div| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(ds.spacing.control_gap))
                        .child(directivity_toggle)
                        .child(contour_div),
                )
            })
    }
}
