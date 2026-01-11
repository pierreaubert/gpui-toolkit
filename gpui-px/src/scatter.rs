//! Scatter chart - Plotly Express style API.

use crate::error::ChartError;
use crate::line::LegendPosition;
use crate::{
    DEFAULT_COLOR, DEFAULT_HEIGHT, DEFAULT_PADDING_FRACTION, DEFAULT_TITLE_FONT_SIZE,
    DEFAULT_WIDTH, ScaleType, TITLE_AREA_HEIGHT, extent_padded, validate_data_array,
    validate_data_length, validate_dimensions, validate_positive,
};
use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::D3Color;
use d3rs::grid::{GridConfig, render_grid};
use d3rs::scale::{LinearScale, LogScale};
use d3rs::shape::{ScatterConfig, ScatterPoint, render_scatter};
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, Rgba, div, hsla, px, rgb};

/// A single series in a scatter chart
#[derive(Debug, Clone)]
struct ScatterSeries {
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: u32,
    point_radius: f32,
    opacity: f32,
}

/// Theme for scatter chart styling
#[derive(Debug, Clone)]
pub struct ScatterTheme {
    /// Background color for plot area
    pub plot_background: Rgba,
    /// Title text color
    pub title_color: Rgba,
    /// Legend text color
    pub legend_text_color: Rgba,
}

impl Default for ScatterTheme {
    fn default() -> Self {
        Self {
            plot_background: rgb(0xf8f8f8),
            title_color: hsla(0.0, 0.0, 0.2, 1.0).into(),
            legend_text_color: Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.6,
            },
        }
    }
}

/// Scatter chart builder.
#[derive(Debug, Clone)]
pub struct ScatterChart {
    // Primary series
    x: Vec<f64>,
    y: Vec<f64>,
    label: Option<String>,
    color: u32,
    point_radius: f32,
    opacity: f32,
    // Additional series
    series: Vec<ScatterSeries>,
    // Common settings
    title: Option<String>,
    width: f32,
    height: f32,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
    // Axis range overrides (for zoom support)
    x_range: Option<[f64; 2]>,
    y_range: Option<[f64; 2]>,
    // Legend settings
    show_legend: bool,
    legend_position: LegendPosition,
    legend_position_explicit: bool,
    graph_ratio: f32,
    theme: ScatterTheme,
}

impl ScatterChart {
    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set point color as 24-bit RGB hex value (format: 0xRRGGBB).
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::scatter;
    /// let chart = scatter(&[1.0], &[1.0])
    ///     .color(0x1f77b4)  // Plotly blue
    ///     .build();
    /// ```
    pub fn color(mut self, hex: u32) -> Self {
        self.color = hex;
        self
    }

    /// Set point radius in pixels.
    pub fn point_radius(mut self, radius: f32) -> Self {
        self.point_radius = radius;
        self
    }

    /// Set point opacity (0.0 - 1.0).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set chart dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set X-axis scale type (linear or log).
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::{scatter, ScaleType};
    /// let chart = scatter(&[10.0, 100.0, 1000.0], &[1.0, 2.0, 3.0])
    ///     .x_scale(ScaleType::Log)
    ///     .build();
    /// ```
    pub fn x_scale(mut self, scale: ScaleType) -> Self {
        self.x_scale_type = scale;
        self
    }

    /// Set Y-axis scale type (linear or log).
    pub fn y_scale(mut self, scale: ScaleType) -> Self {
        self.y_scale_type = scale;
        self
    }

    /// Set explicit X-axis range (for zoom support).
    pub fn x_range(mut self, min: f64, max: f64) -> Self {
        self.x_range = Some([min, max]);
        self
    }

    /// Set explicit Y-axis range (for zoom support).
    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.y_range = Some([min, max]);
        self
    }

    /// Set label for legend entry.
    ///
    /// When a label is set, the legend will automatically be shown.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::scatter;
    /// let chart = scatter(&[1.0, 2.0], &[1.0, 2.0])
    ///     .label("Series A")
    ///     .build();
    /// ```
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.show_legend = true;
        self
    }

    /// Add an additional data series to the chart.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::scatter;
    /// let x1 = vec![1.0, 2.0, 3.0];
    /// let y1 = vec![1.0, 2.0, 3.0];
    /// let x2 = vec![1.5, 2.5, 3.5];
    /// let y2 = vec![3.0, 2.0, 1.0];
    /// let chart = scatter(&x1, &y1)
    ///     .label("Series 1")
    ///     .color(0x3b82f6)
    ///     .add_series(&x2, &y2, Some("Series 2"), 0xff7f0e, 5.0, 0.7)
    ///     .build();
    /// ```
    pub fn add_series(
        mut self,
        x: &[f64],
        y: &[f64],
        label: Option<impl Into<String>>,
        color: u32,
        point_radius: f32,
        opacity: f32,
    ) -> Self {
        self.series.push(ScatterSeries {
            x: x.to_vec(),
            y: y.to_vec(),
            label: label.map(|l| l.into()),
            color,
            point_radius,
            opacity,
        });
        // Auto-enable legend if any series has a label
        if self.series.iter().any(|s| s.label.is_some()) {
            self.show_legend = true;
        }
        self
    }

    /// Set the chart theme.
    pub fn theme(mut self, theme: ScatterTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the legend position.
    ///
    /// Controls where the legend is displayed relative to the chart area.
    /// Available positions: `Right` (default), `Left`, `Top`, `Bottom`.
    ///
    /// When not explicitly set, the legend position is automatically chosen
    /// to achieve a graph aspect ratio closest to `graph_ratio`.
    pub fn legend_position(mut self, position: LegendPosition) -> Self {
        self.legend_position = position;
        self.legend_position_explicit = true;
        self
    }

    /// Set the target aspect ratio for the graph area.
    ///
    /// The ratio is defined as `height / width`. Default is `1.414` (≈ √2, similar to A4 paper).
    ///
    /// When a legend is shown and `legend_position` is not explicitly set,
    /// the legend position is automatically chosen to achieve an aspect ratio
    /// closest to this target ratio.
    pub fn graph_ratio(mut self, ratio: f32) -> Self {
        self.graph_ratio = ratio;
        self
    }

    /// Build and validate the chart, returning renderable element.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        // Validate inputs
        validate_data_array(&self.x, "x")?;
        validate_data_array(&self.y, "y")?;
        validate_data_length(self.x.len(), self.y.len(), "x", "y")?;
        validate_dimensions(self.width, self.height)?;

        // Validate positive values for log scales
        if self.x_scale_type == ScaleType::Log {
            validate_positive(&self.x, "x")?;
        }
        if self.y_scale_type == ScaleType::Log {
            validate_positive(&self.y, "y")?;
        }

        // Validate all additional series
        for series in &self.series {
            validate_data_array(&series.x, "series.x")?;
            validate_data_array(&series.y, "series.y")?;
            validate_data_length(series.x.len(), series.y.len(), "series.x", "series.y")?;
            if self.x_scale_type == ScaleType::Log {
                validate_positive(&series.x, "series.x")?;
            }
            if self.y_scale_type == ScaleType::Log {
                validate_positive(&series.y, "series.y")?;
            }
        }

        // Define margins
        let margin_left = 50.0;
        let margin_bottom = 30.0;
        let margin_top = 10.0;
        let margin_right = 20.0;

        // Calculate plot area (reserve space for title if present)
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };

        // Calculate legend dimensions based on position
        let legend_gap = 20.0;

        // Count legend items and calculate max label length
        let mut legend_item_count = 0;
        let mut max_label_len = 0;

        if self.show_legend {
            if let Some(ref label) = self.label {
                legend_item_count += 1;
                max_label_len = max_label_len.max(label.len());
            }

            for series in &self.series {
                if let Some(ref label) = series.label {
                    legend_item_count += 1;
                    max_label_len = max_label_len.max(label.len());
                }
            }
        }

        let has_legend_items = legend_item_count > 0;

        // Calculate base legend dimensions for each orientation
        let estimated_text_width = (max_label_len as f32) * 7.0;
        let single_item_width = 16.0 + 8.0 + estimated_text_width + 16.0;
        let single_item_height = 24.0;

        // Vertical legend dimensions (for Left/Right)
        let vertical_legend_width = single_item_width;
        let vertical_legend_height = (legend_item_count as f32) * single_item_height + 16.0;

        // Horizontal legend dimensions (for Top/Bottom)
        let horizontal_legend_width = (legend_item_count as f32) * (single_item_width + 16.0);
        let horizontal_legend_height = single_item_height + 8.0;

        // Base available dimensions (without legend)
        let base_available_width = self.width as f64 - margin_left - margin_right;
        let base_available_height =
            self.height as f64 - title_height as f64 - margin_top - margin_bottom;

        // Determine legend position (auto-select if not explicit)
        let legend_position = if has_legend_items && !self.legend_position_explicit {
            let target_ratio = self.graph_ratio as f64;

            let ratio_distance = |plot_w: f64, plot_h: f64| -> f64 {
                if plot_w <= 0.0 || plot_h <= 0.0 {
                    return f64::MAX;
                }
                let ratio = plot_h / plot_w;
                (ratio - target_ratio).abs()
            };

            let lr_plot_width = base_available_width - (vertical_legend_width + legend_gap) as f64;
            let lr_plot_height = base_available_height;
            let lr_distance = ratio_distance(lr_plot_width, lr_plot_height);

            let tb_plot_width = base_available_width;
            let tb_plot_height =
                base_available_height - (horizontal_legend_height + legend_gap) as f64;
            let tb_distance = ratio_distance(tb_plot_width, tb_plot_height);

            if lr_distance <= tb_distance {
                LegendPosition::Right
            } else {
                LegendPosition::Bottom
            }
        } else {
            self.legend_position
        };

        // Calculate final legend dimensions based on chosen position
        let (legend_width, legend_height) = if has_legend_items {
            match legend_position {
                LegendPosition::Left | LegendPosition::Right => {
                    (vertical_legend_width, vertical_legend_height)
                }
                LegendPosition::Top | LegendPosition::Bottom => {
                    (horizontal_legend_width, horizontal_legend_height)
                }
                LegendPosition::Hidden => (0.0, 0.0),
            }
        } else {
            (0.0, 0.0)
        };

        // Calculate plot dimensions, accounting for legend position
        let width_for_legend = match legend_position {
            LegendPosition::Left | LegendPosition::Right if has_legend_items => {
                legend_width + legend_gap
            }
            _ => 0.0,
        };
        let height_for_legend = match legend_position {
            LegendPosition::Top | LegendPosition::Bottom if has_legend_items => {
                legend_height + legend_gap
            }
            _ => 0.0,
        };

        let plot_width =
            (self.width as f64 - margin_left - margin_right - width_for_legend as f64).max(0.0);
        let plot_height = (self.height as f64
            - title_height as f64
            - margin_top
            - margin_bottom
            - height_for_legend as f64)
            .max(0.0);

        // Calculate domains with padding - include all series, or use explicit ranges if set
        let (x_min, x_max) = if let Some([min, max]) = self.x_range {
            (min, max)
        } else {
            let mut all_x: Vec<f64> = self.x.clone();
            for series in &self.series {
                all_x.extend_from_slice(&series.x);
            }
            extent_padded(&all_x, DEFAULT_PADDING_FRACTION)
        };
        let (y_min, y_max) = if let Some([min, max]) = self.y_range {
            (min, max)
        } else {
            let mut all_y: Vec<f64> = self.y.clone();
            for series in &self.series {
                all_y.extend_from_slice(&series.y);
            }
            extent_padded(&all_y, DEFAULT_PADDING_FRACTION)
        };

        // Create data points for primary series
        let primary_data: Vec<ScatterPoint> = self
            .x
            .iter()
            .zip(self.y.iter())
            .map(|(&x, &y)| ScatterPoint::new(x, y))
            .collect();

        let primary_config = ScatterConfig::new()
            .fill_color(D3Color::from_hex(self.color))
            .point_radius(self.point_radius)
            .opacity(self.opacity);

        // Prepare additional series data and configs
        let series_data_configs: Vec<(Vec<ScatterPoint>, ScatterConfig)> = self
            .series
            .iter()
            .map(|s| {
                let points: Vec<ScatterPoint> =
                    s.x.iter()
                        .zip(s.y.iter())
                        .map(|(&x, &y)| ScatterPoint::new(x, y))
                        .collect();
                let config = ScatterConfig::new()
                    .fill_color(D3Color::from_hex(s.color))
                    .point_radius(s.point_radius)
                    .opacity(s.opacity);
                (points, config)
            })
            .collect();

        let axis_theme = DefaultAxisTheme;

        // Helper macro to build plot area with all series
        macro_rules! build_plot_area {
            ($x_scale:expr, $y_scale:expr) => {{
                let mut plot_area = div()
                    .w(px(plot_width as f32))
                    .h(px(plot_height as f32))
                    .relative()
                    .overflow_hidden()
                    .bg(self.theme.plot_background)
                    .child(render_grid(
                        &$x_scale,
                        &$y_scale,
                        &GridConfig::default(),
                        plot_width as f32,
                        plot_height as f32,
                        &axis_theme,
                    ));

                // Render additional series first
                for (series_data, series_config) in &series_data_configs {
                    plot_area = plot_area.child(render_scatter(
                        &$x_scale,
                        &$y_scale,
                        series_data,
                        series_config,
                    ));
                }

                // Render primary series on top
                plot_area = plot_area.child(render_scatter(
                    &$x_scale,
                    &$y_scale,
                    &primary_data,
                    &primary_config,
                ));

                plot_area
            }};
        }

        // Build the element based on scale types
        let chart_content: AnyElement = match (self.x_scale_type, self.y_scale_type) {
            (ScaleType::Linear, ScaleType::Linear) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                let plot_area = build_plot_area!(x_scale, y_scale);

                div()
                    .flex()
                    .child(render_axis(
                        &y_scale,
                        &AxisConfig::left(),
                        plot_height as f32,
                        &axis_theme,
                    ))
                    .child(div().flex().flex_col().child(plot_area).child(render_axis(
                        &x_scale,
                        &AxisConfig::bottom(),
                        plot_width as f32,
                        &axis_theme,
                    )))
                    .into_any_element()
            }
            (ScaleType::Log, ScaleType::Linear) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                let plot_area = build_plot_area!(x_scale, y_scale);

                div()
                    .flex()
                    .child(render_axis(
                        &y_scale,
                        &AxisConfig::left(),
                        plot_height as f32,
                        &axis_theme,
                    ))
                    .child(div().flex().flex_col().child(plot_area).child(render_axis(
                        &x_scale,
                        &AxisConfig::bottom(),
                        plot_width as f32,
                        &axis_theme,
                    )))
                    .into_any_element()
            }
            (ScaleType::Linear, ScaleType::Log) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height, 0.0);

                let plot_area = build_plot_area!(x_scale, y_scale);

                div()
                    .flex()
                    .child(render_axis(
                        &y_scale,
                        &AxisConfig::left(),
                        plot_height as f32,
                        &axis_theme,
                    ))
                    .child(div().flex().flex_col().child(plot_area).child(render_axis(
                        &x_scale,
                        &AxisConfig::bottom(),
                        plot_width as f32,
                        &axis_theme,
                    )))
                    .into_any_element()
            }
            (ScaleType::Log, ScaleType::Log) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, plot_width);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height, 0.0);

                let plot_area = build_plot_area!(x_scale, y_scale);

                div()
                    .flex()
                    .child(render_axis(
                        &y_scale,
                        &AxisConfig::left(),
                        plot_height as f32,
                        &axis_theme,
                    ))
                    .child(div().flex().flex_col().child(plot_area).child(render_axis(
                        &x_scale,
                        &AxisConfig::bottom(),
                        plot_width as f32,
                        &axis_theme,
                    )))
                    .into_any_element()
            }
        };

        // Collect legend items if enabled
        let mut legend_items = Vec::new();
        if has_legend_items {
            if let Some(label) = &self.label {
                legend_items.push((self.color, label.clone()));
            }
            for series in &self.series {
                if let Some(label) = &series.label {
                    legend_items.push((series.color, label.clone()));
                }
            }
        }

        // Build container with optional title
        let mut container = div()
            .w(px(self.width))
            .h(px(self.height))
            .relative()
            .flex()
            .flex_col();

        // Add title if present
        if let Some(title) = &self.title {
            let font_config = VectorFontConfig::horizontal(
                DEFAULT_TITLE_FONT_SIZE,
                self.theme.title_color.into(),
            );
            container = container.child(
                div()
                    .w_full()
                    .h(px(title_height))
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(render_vector_text(title, &font_config)),
            );
        }

        // Add chart content and legend based on position
        if !legend_items.is_empty() {
            // Build legend element (individual item for each series)
            // Use circle indicator for scatter plots
            let legend_item = |color: u32, label: String| {
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(10.0))
                            .h(px(10.0))
                            .rounded(px(5.0))
                            .bg(rgb(color)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(self.theme.legend_text_color)
                            .child(label),
                    )
            };

            match legend_position {
                LegendPosition::Right => {
                    let mut legend_column = div().flex().flex_col().gap_2().p_2();
                    for (color, label) in legend_items {
                        legend_column = legend_column.child(legend_item(color, label));
                    }

                    container = container.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(legend_gap))
                            .child(chart_content)
                            .child(div().w(px(legend_width)).child(legend_column)),
                    );
                }
                LegendPosition::Left => {
                    let mut legend_column = div().flex().flex_col().gap_2().p_2();
                    for (color, label) in legend_items {
                        legend_column = legend_column.child(legend_item(color, label));
                    }

                    container = container.child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(legend_gap))
                            .child(div().w(px(legend_width)).child(legend_column))
                            .child(chart_content),
                    );
                }
                LegendPosition::Top => {
                    let mut legend_row = div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap_4()
                        .p_2()
                        .justify_center();
                    for (color, label) in legend_items {
                        legend_row = legend_row.child(legend_item(color, label));
                    }

                    container = container.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(legend_gap))
                            .child(div().h(px(legend_height)).child(legend_row))
                            .child(chart_content),
                    );
                }
                LegendPosition::Bottom => {
                    let mut legend_row = div()
                        .flex()
                        .flex_row()
                        .flex_wrap()
                        .gap_4()
                        .p_2()
                        .justify_center();
                    for (color, label) in legend_items {
                        legend_row = legend_row.child(legend_item(color, label));
                    }

                    container = container.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(legend_gap))
                            .child(chart_content)
                            .child(div().h(px(legend_height)).child(legend_row)),
                    );
                }
                LegendPosition::Hidden => {
                    container = container.child(div().relative().child(chart_content));
                }
            }
        } else {
            container = container.child(div().relative().child(chart_content));
        }

        Ok(container)
    }
}

/// Create a scatter chart from x and y data.
///
/// # Example
///
/// ```rust,no_run
/// use gpui_px::scatter;
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];
///
/// let chart = scatter(&x, &y)
///     .title("My Scatter Plot")
///     .color(0x1f77b4)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn scatter(x: &[f64], y: &[f64]) -> ScatterChart {
    ScatterChart {
        x: x.to_vec(),
        y: y.to_vec(),
        label: None,
        color: DEFAULT_COLOR,
        point_radius: 5.0,
        opacity: 0.7,
        series: Vec::new(),
        title: None,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        x_scale_type: ScaleType::Linear,
        y_scale_type: ScaleType::Linear,
        x_range: None,
        y_range: None,
        show_legend: false,
        legend_position: LegendPosition::default(),
        legend_position_explicit: false,
        graph_ratio: 1.414,
        theme: ScatterTheme::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scatter_empty_x_data() {
        let result = scatter(&[], &[1.0, 2.0, 3.0]).build();
        assert!(matches!(result, Err(ChartError::EmptyData { field: "x" })));
    }

    #[test]
    fn test_scatter_empty_y_data() {
        let result = scatter(&[1.0, 2.0, 3.0], &[]).build();
        assert!(matches!(result, Err(ChartError::EmptyData { field: "y" })));
    }

    #[test]
    fn test_scatter_data_length_mismatch() {
        let result = scatter(&[1.0, 2.0], &[1.0, 2.0, 3.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "x",
                y_field: "y",
                x_len: 2,
                y_len: 3,
            })
        ));
    }

    #[test]
    fn test_scatter_nan_in_x() {
        let result = scatter(&[1.0, f64::NAN, 3.0], &[1.0, 2.0, 3.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_scatter_infinity_in_y() {
        let result = scatter(&[1.0, 2.0, 3.0], &[1.0, f64::INFINITY, 3.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "y",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_scatter_zero_width() {
        let result = scatter(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0])
            .size(0.0, 400.0)
            .build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidDimension {
                field: "width",
                value: 0.0
            })
        ));
    }

    #[test]
    fn test_scatter_negative_height() {
        let result = scatter(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0])
            .size(600.0, -100.0)
            .build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidDimension {
                field: "height",
                value: -100.0
            })
        ));
    }

    #[test]
    fn test_scatter_successful_build() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];
        let result = scatter(&x, &y).title("Test Chart").color(0x1f77b4).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_scatter_builder_chain() {
        let result = scatter(&[1.0, 2.0], &[3.0, 4.0])
            .title("My Plot")
            .color(0xff0000)
            .point_radius(10.0)
            .opacity(0.5)
            .size(800.0, 600.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_scatter_log_x_scale() {
        let x = vec![10.0, 100.0, 1000.0, 10000.0];
        let y = vec![1.0, 2.0, 3.0, 4.0];
        let result = scatter(&x, &y).x_scale(ScaleType::Log).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_scatter_log_y_scale() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![10.0, 100.0, 1000.0, 10000.0];
        let result = scatter(&x, &y).y_scale(ScaleType::Log).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_scatter_log_xy_scale() {
        let x = vec![10.0, 100.0, 1000.0];
        let y = vec![20.0, 200.0, 2000.0];
        let result = scatter(&x, &y)
            .x_scale(ScaleType::Log)
            .y_scale(ScaleType::Log)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_scatter_log_x_negative_values() {
        let x = vec![-10.0, -5.0, 5.0, 10.0];
        let y = vec![1.0, 2.0, 3.0, 4.0];
        let result = scatter(&x, &y).x_scale(ScaleType::Log).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_scatter_log_y_zero_value() {
        let x = vec![1.0, 2.0, 3.0, 4.0];
        let y = vec![0.0, 1.0, 2.0, 3.0];
        let result = scatter(&x, &y).y_scale(ScaleType::Log).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "y",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_scatter_log_scale_with_title() {
        let x = vec![10.0, 100.0, 1000.0];
        let y = vec![1.0, 2.0, 3.0];
        let result = scatter(&x, &y)
            .title("Log Scale Plot")
            .x_scale(ScaleType::Log)
            .color(0x1f77b4)
            .build();
        assert!(result.is_ok());
    }
}
