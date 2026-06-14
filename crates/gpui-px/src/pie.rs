//! Pie chart - Plotly Express style API.

use crate::error::ChartError;
use crate::{
    ChartSize, DEFAULT_HEIGHT, DEFAULT_TITLE_FONT_SIZE, DEFAULT_WIDTH, TITLE_AREA_HEIGHT,
    apply_chart_size, default_design, resolved_chart_dimensions, validate_data_array,
    validate_data_length, validate_dimensions,
};
use d3rs::color::D3Color;
use d3rs::shape::{Arc, Pie};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{IntoElement, PathBuilder, canvas, div, hsla, point, px};
use gpui_design::DesignSystem;
use std::sync::Arc as StdArc;

/// Default color palette (Plotly)
const DEFAULT_PALETTE: [u32; 10] = [
    0x1f77b4, 0xff7f0e, 0x2ca02c, 0xd62728, 0x9467bd, 0x8c564b, 0xe377c2, 0x7f7f7f, 0xbcbd22,
    0x17becf,
];

/// Pie chart builder.
#[derive(Clone)]
pub struct PieChart {
    labels: Option<Vec<String>>,
    values: Vec<f64>,
    title: Option<String>,
    inner_radius_fraction: f64, // 0.0 to 1.0 of outer radius
    pad_angle: f64,
    corner_radius: f64,
    colors: Option<Vec<u32>>,
    width: f32,
    height: f32,
    chart_size: ChartSize,
    sort: bool,
    design: Option<StdArc<DesignSystem>>,
}

impl PieChart {
    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set custom colors for slices.
    pub fn colors(mut self, colors: &[u32]) -> Self {
        self.colors = Some(colors.to_vec());
        self
    }

    /// Set hole size fraction (0.0 to 1.0).
    /// 0.0 = full pie, 0.5 = donut with hole half the radius.
    pub fn hole(mut self, fraction: f64) -> Self {
        self.inner_radius_fraction = fraction.clamp(0.0, 0.99);
        self
    }

    /// Set padding angle between slices (in radians).
    pub fn pad_angle(mut self, angle: f64) -> Self {
        self.pad_angle = angle;
        self
    }

    /// Set corner radius for slices.
    pub fn corner_radius(mut self, radius: f64) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Sort slices by value (descending). Default is true.
    pub fn sort(mut self, sort: bool) -> Self {
        self.sort = sort;
        self
    }

    /// Set chart dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self.chart_size = ChartSize::fixed(width, height);
        self
    }

    /// Fill the parent using the current minimum chart dimensions.
    pub fn fill(mut self) -> Self {
        self.chart_size = ChartSize::fill().min_size(self.width, self.height);
        self
    }

    /// Set minimum dimensions for responsive fill sizing.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self.chart_size = self.chart_size.min_size(width, height);
        self
    }

    /// Set preferred fill-layout aspect ratio.
    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.chart_size = self.chart_size.aspect_ratio(ratio);
        self
    }

    /// Override the design system used for chart defaults.
    pub fn design(mut self, design: impl Into<StdArc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Build and validate the chart, returning renderable element.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        let design = self.design.clone().unwrap_or_else(default_design);
        let (layout_width, layout_height) = resolved_chart_dimensions(self.chart_size);

        // Validate inputs
        validate_data_array(&self.values, "values")?;
        validate_dimensions(layout_width, layout_height)?;

        if let Some(ref labels) = self.labels {
            validate_data_length(labels.len(), self.values.len(), "labels", "values")?;
        }

        // Reject negative values
        if self.values.iter().any(|&v| v < 0.0) {
            return Err(ChartError::InvalidData {
                field: "values",
                reason: "pie chart values must be non-negative",
            });
        }

        // Ensure sum is strictly positive
        let total: f64 = self.values.iter().sum();
        if total <= 0.0 {
            return Err(ChartError::InvalidData {
                field: "values",
                reason: "pie chart values must sum to a positive number",
            });
        }

        // Calculate plot area
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };
        let plot_height = layout_height - title_height;
        let plot_width = layout_width;

        // Calculate radius
        let radius = (plot_width.min(plot_height) / 2.0) as f64 * 0.9; // 90% fit
        let inner_radius = radius * self.inner_radius_fraction;

        // Prepare pie generator
        let pie = Pie::new()
            .pad_angle(self.pad_angle)
            .corner_radius(self.corner_radius)
            .inner_radius(inner_radius)
            .outer_radius(radius)
            .sort(self.sort);

        // Generate slices
        let slices = pie.generate(&self.values, |v| *v);

        // Determine colors. Keep any custom palette as an owned
        // `Option<Vec<u32>>` so it can be moved into the paint closure. The
        // default palette is a `static` slice, so no allocation is needed when
        // no custom colors are supplied. An empty custom palette is treated as
        // `None` to avoid division by zero.
        let custom_palette: Option<Vec<u32>> =
            self.colors.filter(|c| !c.is_empty()).map(|c| c.to_vec());

        // Pre-build flattened paths for every slice. The points are relative to
        // the plot-area origin, with the pie centered at (plot_width/2,
        // plot_height/2); bounds.origin is applied in the paint closure.
        let center_x = plot_width as f64 / 2.0;
        let center_y = plot_height as f64 / 2.0;
        let arc_gen = Arc::new().center(center_x, center_y);
        let slice_paths: Vec<Vec<gpui::Point<gpui::Pixels>>> = slices
            .iter()
            .map(|slice| {
                arc_gen
                    .generate(&slice.arc)
                    .flatten(0.5)
                    .into_iter()
                    .map(|p| point(px(p.x as f32), px(p.y as f32)))
                    .collect()
            })
            .collect();

        // Render function
        let render_element = canvas(
            move |bounds, _, _| (slice_paths, custom_palette, bounds),
            move |_, (slice_paths, custom_palette, bounds), window, _| {
                let palette: &[u32] = custom_palette.as_deref().unwrap_or(&DEFAULT_PALETTE);
                let origin_x: f32 = bounds.origin.x.into();
                let origin_y: f32 = bounds.origin.y.into();

                for (i, path_points) in slice_paths.iter().enumerate() {
                    if path_points.is_empty() {
                        continue;
                    }

                    let color = D3Color::from_hex(palette[i % palette.len()]);
                    let fill_color = color.to_rgba();

                    let mut builder = PathBuilder::fill();

                    let first = path_points[0];
                    builder.move_to(point(
                        px(origin_x + f32::from(first.x)),
                        px(origin_y + f32::from(first.y)),
                    ));
                    for p in path_points.iter().skip(1) {
                        builder.line_to(point(
                            px(origin_x + f32::from(p.x)),
                            px(origin_y + f32::from(p.y)),
                        ));
                    }

                    builder.close();

                    if let Ok(gpui_path) = builder.build() {
                        window.paint_path(gpui_path, fill_color);
                    }
                }
            },
        );

        // Build container
        let mut container = apply_chart_size(div(), self.chart_size)
            .relative()
            .flex()
            .flex_col();

        // Add title if present
        if let Some(title) = &self.title {
            let font_config = GlyphTextConfig::horizontal(
                design.typography.large_size.max(DEFAULT_TITLE_FONT_SIZE),
                hsla(0.0, 0.0, 0.2, 1.0),
            );
            container = container.child(
                div()
                    .w_full()
                    .h(px(title_height))
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(render_glyph_text(title, &font_config)),
            );
        }

        // Add plot area
        container = container.child(
            div()
                .w(px(layout_width))
                .h(px(plot_height))
                .relative()
                .child(render_element),
        );

        Ok(container)
    }
}

/// Create a pie chart from values.
///
/// # Example
///
/// ```rust,no_run
/// use gpui_px::pie;
///
/// let values = vec![10.0, 20.0, 30.0, 40.0];
/// let labels = vec!["A", "B", "C", "D"];
///
/// let chart = pie(&values)
///     .labels(&labels)
///     .title("My Pie Chart")
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn pie(values: &[f64]) -> PieChart {
    PieChart {
        labels: None,
        values: values.to_vec(),
        title: None,
        inner_radius_fraction: 0.0,
        pad_angle: 0.0,
        corner_radius: 0.0,
        colors: None,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        chart_size: ChartSize::default(),
        sort: true,
        design: None,
    }
}

impl PieChart {
    /// Set labels for slices (used for tooltips/legend - currently unused).
    pub fn labels(mut self, labels: &[impl ToString]) -> Self {
        self.labels = Some(labels.iter().map(|l| l.to_string()).collect());
        self
    }
}

/// Create a donut chart from values (shorthand for pie with hole).
///
/// # Example
///
/// ```rust,no_run
/// use gpui_px::donut;
///
/// let values = vec![10.0, 20.0, 30.0];
/// let chart = donut(&values).title("My Donut").build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn donut(values: &[f64]) -> PieChart {
    pie(values).hole(0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pie_negative_values_rejected() {
        let values = vec![10.0, -5.0, 30.0];
        let result = pie(&values).build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_pie_all_zero_values_rejected() {
        let values = vec![0.0, 0.0, 0.0];
        let result = pie(&values).build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_pie_valid_values() {
        let values = vec![10.0, 20.0, 30.0];
        let result = pie(&values).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_pie_responsive_size_defaults_and_fixed_opt_in() {
        let values = vec![10.0, 20.0, 30.0];

        crate::assert_default_chart_size(pie(&values).chart_size);
        crate::assert_default_chart_size(donut(&values).chart_size);
        crate::assert_fixed_chart_size(pie(&values).size(260.0, 260.0).chart_size, 260.0, 260.0);
        crate::assert_fill_chart_size(
            pie(&values)
                .size(260.0, 260.0)
                .fill()
                .min_size(220.0, 220.0)
                .aspect_ratio(1.0)
                .chart_size,
            220.0,
            220.0,
            Some(1.0),
        );
    }
}
