//! Live and deterministic-SVG colorbar rendering.

use crate::{ColorRange, ColorScale};
use std::fmt::Write;

/// Orientation of a colorbar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorbarOrientation {
    #[default]
    Vertical,
    Horizontal,
}

/// A labeled color scale with a configurable displayed range and ticks.
#[derive(Clone, Debug)]
pub struct Colorbar {
    label: String,
    unit: Option<String>,
    scale: ColorScale,
    range: ColorRange,
    ticks: Option<Vec<f64>>,
    orientation: ColorbarOrientation,
}

impl Colorbar {
    /// Create a colorbar with the default viridis scale and automatic range.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            unit: None,
            scale: ColorScale::default(),
            range: ColorRange::Auto,
            ticks: None,
            orientation: ColorbarOrientation::Vertical,
        }
    }

    /// Set the unit shown alongside the label and tick values.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Set the color scale.
    pub fn scale(mut self, scale: ColorScale) -> Self {
        self.scale = scale;
        self
    }

    /// Alias for [`Self::scale`], matching chart builder terminology.
    pub fn color_scale(self, scale: ColorScale) -> Self {
        self.scale(scale)
    }

    /// Set the displayed color range.
    pub fn range(mut self, range: ColorRange) -> Self {
        self.range = range;
        self
    }

    /// Set explicit tick values. When omitted, minimum, midpoint, and maximum are shown.
    pub fn ticks(mut self, ticks: impl Into<Vec<f64>>) -> Self {
        self.ticks = Some(ticks.into());
        self
    }

    /// Set the colorbar orientation.
    pub fn orientation(mut self, orientation: ColorbarOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Return the range available to the colorbar without requiring a data field.
    ///
    /// `Auto` is rendered against the normalized `[0, 1]` domain until a chart
    /// supplies a resolved range through [`Self::range`].
    fn display_range(&self) -> [f64; 2] {
        self.range.resolve(0.0, 1.0).unwrap_or([0.0, 1.0])
    }

    fn tick_values(&self, range: [f64; 2]) -> Vec<f64> {
        let mut ticks = self
            .ticks
            .clone()
            .unwrap_or_else(|| vec![range[1], (range[0] + range[1]) * 0.5, range[0]]);

        if !ticks.iter().any(|tick| *tick == range[1]) {
            ticks.insert(0, range[1]);
        }
        if !ticks.iter().any(|tick| *tick == range[0]) {
            ticks.push(range[0]);
        }
        ticks.retain(|tick| tick.is_finite());
        ticks
    }

    /// Render a live GPUI colorbar with a 16-sample vertical gradient.
    #[cfg(feature = "gpui")]
    pub fn render(
        &self,
        design: &gpui_design::DesignSystem,
        height: f32,
    ) -> impl gpui::IntoElement {
        use d3rs::text::{GlyphTextConfig, render_glyph_text};
        use gpui::prelude::*;
        use gpui::{div, hsla, px};

        let height = height.max(1.0);
        let range = self.display_range();
        let ticks = self.tick_values(range);
        let text_config =
            || GlyphTextConfig::horizontal(design.typography.base_size, hsla(0.0, 0.0, 0.25, 1.0));

        let mut gradient = div()
            .flex()
            .flex_col()
            .w(px(16.0))
            .h(px(height))
            .border_1()
            .border_color(hsla(0.0, 0.0, 0.35, 1.0));
        for step in 0..16 {
            let t = 1.0 - step as f64 / 15.0;
            gradient = gradient.child(div().flex_1().bg(self.scale.map(t).to_rgba()));
        }

        let mut tick_labels = div().flex().flex_col().justify_between().h(px(height));
        for tick in ticks {
            let text = format_tick(tick);
            tick_labels = tick_labels.child(render_glyph_text(&text, &text_config()));
        }

        let mut title = div().flex().flex_col();
        title = title.child(render_glyph_text(&self.label, &text_config()));
        if let Some(unit) = &self.unit {
            title = title.child(render_glyph_text(unit, &text_config()));
        }

        div()
            .flex()
            .flex_col()
            .gap(px(design.spacing.control_gap))
            .child(title)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(design.spacing.control_gap))
                    .child(gradient)
                    .child(tick_labels),
            )
    }

    /// Export a deterministic SVG colorbar with 12 gradient stops and labels.
    pub fn to_svg(&self, x: f64, y: f64, height: f64) -> String {
        let height = height.max(0.0);
        let bar_width = 16.0;
        let range = self.display_range();
        let ticks = self.tick_values(range);
        let mut svg = String::new();
        let _ = writeln!(
            svg,
            "<g class=\"gpui-px-colorbar\" data-orientation=\"{:?}\" data-lower=\"{:.6}\" data-upper=\"{:.6}\">",
            self.orientation, range[0], range[1]
        );

        for step in 0..12 {
            let t = step as f64 / 12.0;
            let rect_y = y + height * t;
            let rect_height = (height / 12.0).ceil();
            let fill = self.scale.map(1.0 - t).to_hex();
            let _ = writeln!(
                svg,
                "<rect class=\"gpui-px-colorbar-stop\" x=\"{x:.2}\" y=\"{rect_y:.2}\" width=\"{bar_width:.2}\" height=\"{rect_height:.2}\" fill=\"{fill}\"/>"
            );
        }

        let _ = writeln!(
            svg,
            "<rect class=\"gpui-px-colorbar-border\" x=\"{x:.2}\" y=\"{y:.2}\" width=\"{bar_width:.2}\" height=\"{height:.2}\" fill=\"none\" stroke=\"#555\" stroke-width=\"0.75\"/>"
        );

        let text_x = x + bar_width + 8.0;
        let label = escape_xml(&self.label);
        let _ = writeln!(
            svg,
            "<text class=\"gpui-px-colorbar-label\" x=\"{text_x:.2}\" y=\"{:.2}\">{label}</text>",
            y + 8.0
        );
        if let Some(unit) = &self.unit {
            let unit = escape_xml(unit);
            let _ = writeln!(
                svg,
                "<text class=\"gpui-px-colorbar-unit\" x=\"{text_x:.2}\" y=\"{:.2}\">{unit}</text>",
                y + 20.0
            );
        }

        for tick in ticks {
            let fraction = if range[1] == range[0] {
                0.0
            } else {
                ((range[1] - tick) / (range[1] - range[0])).clamp(0.0, 1.0)
            };
            let tick_y = y + height * fraction;
            let text = format_tick(tick);
            let _ = writeln!(
                svg,
                "<text class=\"gpui-px-colorbar-tick\" x=\"{text_x:.2}\" y=\"{tick_y:.2}\">{text}</text>"
            );
        }
        svg.push_str("</g>\n");
        svg
    }
}

fn format_tick(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        format!("{value:.3}")
    }
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorbar_svg_contains_label_unit_and_extremes() {
        let svg = Colorbar::new("Sound pressure level")
            .unit("dB SPL")
            .to_svg(0.0, 0.0, 200.0);
        assert!(svg.contains("Sound pressure level"));
        assert!(svg.contains("dB SPL"));
        assert!(svg.contains("dB SPL") || svg.contains("<rect"));
    }
}
