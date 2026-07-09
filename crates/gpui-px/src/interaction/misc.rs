/// Clamp log-scale domain bounds to a small positive epsilon and ensure min < max.
pub(super) fn clamp_log_domain(min: f64, max: f64) -> (f64, f64) {
    let epsilon = 1e-10;
    let min = if min.is_finite() && min > epsilon {
        min
    } else {
        epsilon
    };
    let max = if max.is_finite() && max > epsilon {
        max
    } else {
        epsilon
    };
    if min >= max {
        (min, min + epsilon)
    } else {
        (min, max)
    }
}

#[cfg(feature = "gpui")]
pub(super) mod gpui_render {
    use d3rs::brush::{BrushConfig, BrushSelection};
    use d3rs::zoom::ZoomState;
    use gpui::prelude::*;
    use gpui::{IntoElement, div, hsla, px};

    /// Render a brush selection overlay.
    ///
    /// This renders a semi-transparent rectangle showing the current brush selection.
    pub fn render_brush_overlay(
        selection: &BrushSelection,
        config: &BrushConfig,
    ) -> impl IntoElement {
        let x = selection.x0 as f32;
        let y = selection.y0 as f32;
        let width = selection.width() as f32;
        let height = selection.height() as f32;

        let (_r, _g, _b, a) = config.fill_color;

        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(width))
            .h(px(height))
            .bg(hsla(210.0 / 360.0, 0.5, 0.6, a as f32 / 255.0))
            .border_1()
            .border_color(hsla(210.0 / 360.0, 0.5, 0.4, 1.0))
    }

    /// Render a zoom indicator showing the current zoom level.
    pub fn render_zoom_indicator(zoom: &ZoomState, x: f32, y: f32) -> impl IntoElement {
        let level = zoom.zoom_level();
        if level == 0 {
            return div().into_any_element();
        }

        let text = format!("Zoom: {}x", level);

        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .px_2()
            .py_1()
            .bg(hsla(0.0, 0.0, 0.2, 0.8))
            .rounded_md()
            .child(text)
            .text_color(hsla(0.0, 0.0, 1.0, 1.0))
            .text_xs()
            .into_any_element()
    }

    /// Render a reset button for zoom.
    pub fn render_reset_button<F>(x: f32, y: f32, on_click: F) -> impl IntoElement
    where
        F: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    {
        div()
            .id("reset-button")
            .absolute()
            .left(px(x))
            .top(px(y))
            .px_2()
            .py_1()
            .bg(hsla(0.0, 0.0, 0.3, 0.9))
            .rounded_md()
            .child("Reset")
            .text_color(hsla(0.0, 0.0, 1.0, 1.0))
            .text_xs()
            .cursor_pointer()
            .hover(|s| s.bg(hsla(0.0, 0.0, 0.4, 0.9)))
            .on_click(on_click)
    }

    /// Render crosshairs at the mouse position.
    pub fn render_crosshairs(x: f32, y: f32, width: f32, height: f32) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .size_full()
            .child(
                // Vertical line
                div()
                    .absolute()
                    .left(px(x))
                    .top_0()
                    .w_px()
                    .h(px(height))
                    .bg(hsla(0.0, 0.0, 0.5, 0.5)),
            )
            .child(
                // Horizontal line
                div()
                    .absolute()
                    .left_0()
                    .top(px(y))
                    .w(px(width))
                    .h_px()
                    .bg(hsla(0.0, 0.0, 0.5, 0.5)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_log_domain_valid() {
        let (min, max) = clamp_log_domain(1.0, 100.0);
        assert!((min - 1.0).abs() < 1e-10);
        assert!((max - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_clamp_log_domain_reversed() {
        let (min, max) = clamp_log_domain(100.0, 1.0);
        assert!((min - 100.0).abs() < 1e-10);
        assert!((max - (100.0 + 1e-10)).abs() < 1e-10);
        assert!(min < max);
    }

    #[test]
    fn test_clamp_log_domain_zero() {
        let (min, max) = clamp_log_domain(0.0, 10.0);
        assert!((min - 1e-10).abs() < 1e-10);
        assert!((max - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_clamp_log_domain_negative() {
        let (min, max) = clamp_log_domain(-10.0, -1.0);
        assert!((min - 1e-10).abs() < 1e-10);
        assert!((max - 2e-10).abs() < 1e-10);
        assert!(min < max);
    }

    #[test]
    fn test_clamp_log_domain_nan() {
        let (min, max) = clamp_log_domain(f64::NAN, 10.0);
        assert!((min - 1e-10).abs() < 1e-10);
        assert!((max - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_clamp_log_domain_infinite() {
        let (min, max) = clamp_log_domain(f64::INFINITY, 10.0);
        assert!((min - 1e-10).abs() < 1e-10);
        assert!((max - 10.0).abs() < 1e-10);
    }
}
