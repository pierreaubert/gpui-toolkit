impl SpinoramaApp {
    fn render_contour_plot(&mut self, cx: &mut Context<Self>) -> Div {
        let has_contour_data = self.contour_data.is_some();
        let has_directivity_data = self
            .directivity_data
            .as_ref()
            .is_some_and(|d| !d.horizontal.is_empty());

        if !has_contour_data && !has_directivity_data {
            return div().flex().items_center().justify_center().h_full().child(
                div()
                    .text_base()
                    .text_color(rgb(0x666666))
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
        );
        let directivity_contour = self.render_contour_from_directivity(
            "Directivity Contour (SPL Horizontal)",
            directivity_mode,
            colormap,
        );

        div()
            .flex()
            .flex_col()
            .gap_8()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(0x333333))
                    .child(format!("Horizontal Contour Plots - {}", speaker_name)),
            )
            // SPL Horizontal Contour (new format, -180 to +180) with toggle
            .when_some(spl_contour, |el, contour_div| {
                el.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
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
                        .gap_2()
                        .child(directivity_toggle)
                        .child(contour_div),
                )
            })
    }
}

