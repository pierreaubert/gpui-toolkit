//! Bar chart - Plotly Express style API.

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
use d3rs::shape::{
    BarConfig, BarDatum, GroupedBarConfig, GroupedBarDatum, GroupedBarMeta, analyze_grouped_data,
    render_bars, render_grouped_bars,
};
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, Rgba, div, hsla, px, rgb};

/// A single series in a bar chart (for grouped/stacked bars)
#[derive(Debug, Clone)]
struct BarSeries {
    values: Vec<f64>,
    label: Option<String>,
    color: u32,
    #[allow(dead_code)] // Reserved for future per-series opacity support
    opacity: f32,
}

/// Theme for bar chart styling
#[derive(Debug, Clone)]
pub struct BarTheme {
    /// Background color for plot area
    pub plot_background: Rgba,
    /// Title text color
    pub title_color: Rgba,
    /// Legend text color
    pub legend_text_color: Rgba,
}

impl Default for BarTheme {
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

/// Bar chart builder.
#[derive(Debug, Clone)]
pub struct BarChart {
    // Primary series
    categories: Vec<String>,
    values: Vec<f64>,
    label: Option<String>,
    color: u32,
    opacity: f32,
    // Additional series
    series: Vec<BarSeries>,
    // Common settings
    title: Option<String>,
    bar_gap: f32,
    border_radius: f32,
    width: f32,
    height: f32,
    y_scale_type: ScaleType,
    // Legend settings
    show_legend: bool,
    legend_position: LegendPosition,
    legend_position_explicit: bool,
    graph_ratio: f32,
    theme: BarTheme,
}

impl BarChart {
    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set bar color as 24-bit RGB hex value (format: 0xRRGGBB).
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::bar;
    /// let chart = bar(&["A"], &[1.0])
    ///     .color(0x2ca02c)  // Plotly green
    ///     .build();
    /// ```
    pub fn color(mut self, hex: u32) -> Self {
        self.color = hex;
        self
    }

    /// Set bar opacity (0.0 - 1.0).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set gap between bars in pixels.
    pub fn bar_gap(mut self, gap: f32) -> Self {
        self.bar_gap = gap;
        self
    }

    /// Set bar corner radius.
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    /// Set chart dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set Y-axis scale type (linear or log).
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::{bar, ScaleType};
    /// let chart = bar(&["A", "B", "C"], &[10.0, 100.0, 1000.0])
    ///     .y_scale(ScaleType::Log)
    ///     .build();
    /// ```
    pub fn y_scale(mut self, scale: ScaleType) -> Self {
        self.y_scale_type = scale;
        self
    }

    /// Set label for legend entry.
    ///
    /// When a label is set, the legend will automatically be shown.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::bar;
    /// let chart = bar(&["A", "B", "C"], &[1.0, 2.0, 3.0])
    ///     .label("Sales 2024")
    ///     .build();
    /// ```
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.show_legend = true;
        self
    }

    /// Add an additional data series to the chart (for grouped bars).
    ///
    /// All series must have the same number of values as the primary series.
    ///
    /// # Example
    /// ```rust,no_run
    /// use gpui_px::bar;
    /// let categories = vec!["Q1", "Q2", "Q3", "Q4"];
    /// let sales_2023 = vec![100.0, 120.0, 90.0, 150.0];
    /// let sales_2024 = vec![110.0, 140.0, 100.0, 170.0];
    /// let chart = bar(&categories, &sales_2023)
    ///     .label("2023")
    ///     .color(0x3b82f6)
    ///     .add_series(&sales_2024, Some("2024"), 0xff7f0e, 0.8)
    ///     .build();
    /// ```
    pub fn add_series(
        mut self,
        values: &[f64],
        label: Option<impl Into<String>>,
        color: u32,
        opacity: f32,
    ) -> Self {
        self.series.push(BarSeries {
            values: values.to_vec(),
            label: label.map(|l| l.into()),
            color,
            opacity,
        });
        // Auto-enable legend if any series has a label
        if self.series.iter().any(|s| s.label.is_some()) {
            self.show_legend = true;
        }
        self
    }

    /// Set the chart theme.
    pub fn theme(mut self, theme: BarTheme) -> Self {
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
        if self.categories.is_empty() {
            return Err(ChartError::EmptyData {
                field: "categories",
            });
        }
        validate_data_array(&self.values, "values")?;
        validate_data_length(
            self.categories.len(),
            self.values.len(),
            "categories",
            "values",
        )?;
        validate_dimensions(self.width, self.height)?;

        // Validate positive values for log scale
        if self.y_scale_type == ScaleType::Log {
            validate_positive(&self.values, "values")?;
        }

        // Validate all additional series
        for series in &self.series {
            validate_data_array(&series.values, "series.values")?;
            validate_data_length(
                self.categories.len(),
                series.values.len(),
                "categories",
                "series.values",
            )?;
            if self.y_scale_type == ScaleType::Log {
                validate_positive(&series.values, "series.values")?;
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

        // Calculate y domain with padding - include all series
        let mut all_values = self.values.clone();
        for series in &self.series {
            all_values.extend_from_slice(&series.values);
        }
        let (mut y_min, mut y_max) = extent_padded(&all_values, DEFAULT_PADDING_FRACTION);

        // For linear scale, always include zero baseline for bar charts
        // For log scale, we can't include zero
        if self.y_scale_type == ScaleType::Linear {
            y_min = y_min.min(0.0);
            y_max = y_max.max(0.0);
        }

        // Create X scale (always linear for categories)
        let x_scale = LinearScale::new()
            .domain(0.0, self.categories.len() as f64)
            .range(0.0, plot_width);

        let axis_theme = DefaultAxisTheme;

        // Determine if we're using grouped bars (multiple series) or simple bars
        let use_grouped_bars = !self.series.is_empty();

        // Prepare data for grouped bars
        let grouped_data: Vec<GroupedBarDatum>;
        let grouped_meta: GroupedBarMeta;
        let grouped_config: GroupedBarConfig;

        // Prepare data for single-series bars
        let primary_data: Vec<BarDatum>;
        let primary_config: BarConfig;

        if use_grouped_bars {
            // Build grouped bar data from all series
            let mut all_data = Vec::new();

            // Primary series
            let primary_label = self.label.clone().unwrap_or_else(|| "Series 1".to_string());
            for (cat, &val) in self.categories.iter().zip(self.values.iter()) {
                all_data.push(GroupedBarDatum::new(
                    cat.clone(),
                    primary_label.clone(),
                    val,
                ));
            }

            // Additional series
            for (i, s) in self.series.iter().enumerate() {
                let series_label = s
                    .label
                    .clone()
                    .unwrap_or_else(|| format!("Series {}", i + 2));
                for (cat, &val) in self.categories.iter().zip(s.values.iter()) {
                    all_data.push(GroupedBarDatum::new(cat.clone(), series_label.clone(), val));
                }
            }

            grouped_data = all_data;
            grouped_meta = analyze_grouped_data(&grouped_data);

            // Collect colors for all series
            let mut series_colors = vec![D3Color::from_hex(self.color)];
            for s in &self.series {
                series_colors.push(D3Color::from_hex(s.color));
            }

            grouped_config = GroupedBarConfig::new()
                .series_colors(series_colors)
                .opacity(self.opacity)
                .group_gap(self.bar_gap * 3.0) // Gap between category groups
                .bar_gap(self.bar_gap * 0.5) // Gap between bars within group
                .border_radius(self.border_radius);

            // Dummy values for single-series (won't be used)
            primary_data = Vec::new();
            primary_config = BarConfig::new();
        } else {
            // Single series - use simple bars
            primary_data = self
                .categories
                .iter()
                .zip(self.values.iter())
                .map(|(cat, &val)| BarDatum::new(cat.clone(), val))
                .collect();

            primary_config = BarConfig::new()
                .fill_color(D3Color::from_hex(self.color))
                .opacity(self.opacity)
                .bar_gap(self.bar_gap)
                .border_radius(self.border_radius);

            // Dummy values for grouped bars (won't be used)
            grouped_data = Vec::new();
            grouped_meta = GroupedBarMeta {
                categories: Vec::new(),
                series: Vec::new(),
                min_value: 0.0,
                max_value: 0.0,
            };
            grouped_config = GroupedBarConfig::new();
        }

        // Helper macro to build plot area with appropriate bar rendering
        macro_rules! build_plot_area {
            ($y_scale:expr) => {{
                let plot_area = div()
                    .w(px(plot_width as f32))
                    .h(px(plot_height as f32))
                    .relative()
                    .bg(self.theme.plot_background)
                    .child(render_grid(
                        &x_scale,
                        &$y_scale,
                        &GridConfig::default(),
                        plot_width as f32,
                        plot_height as f32,
                        &axis_theme,
                    ));

                if use_grouped_bars {
                    // Use grouped bar rendering
                    plot_area.child(render_grouped_bars(
                        &$y_scale,
                        &grouped_data,
                        &grouped_meta,
                        plot_width as f32,
                        plot_height as f32,
                        &grouped_config,
                    ))
                } else {
                    // Use simple bar rendering
                    plot_area.child(render_bars(
                        &x_scale,
                        &$y_scale,
                        &primary_data,
                        plot_width as f32,
                        plot_height as f32,
                        &primary_config,
                    ))
                }
            }};
        }

        // Build the element based on Y scale type
        let chart_content: AnyElement = match self.y_scale_type {
            ScaleType::Linear => {
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                let plot_area = build_plot_area!(y_scale);

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
            ScaleType::Log => {
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height, 0.0);

                let plot_area = build_plot_area!(y_scale);

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
            // Build legend element (use square indicator for bars)
            let legend_item = |color: u32, label: String| {
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().w(px(12.0)).h(px(12.0)).bg(rgb(color)))
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

/// Create a bar chart from categories and values.
///
/// # Example
///
/// ```rust,no_run
/// use gpui_px::bar;
///
/// let categories = vec!["A", "B", "C", "D"];
/// let values = vec![10.0, 25.0, 15.0, 30.0];
///
/// let chart = bar(&categories, &values)
///     .title("My Bar Chart")
///     .color(0x2ca02c)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn bar<S: AsRef<str>>(categories: &[S], values: &[f64]) -> BarChart {
    BarChart {
        categories: categories.iter().map(|s| s.as_ref().to_string()).collect(),
        values: values.to_vec(),
        label: None,
        color: DEFAULT_COLOR,
        opacity: 0.8,
        series: Vec::new(),
        title: None,
        bar_gap: 2.0,
        border_radius: 2.0,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        y_scale_type: ScaleType::Linear,
        show_legend: false,
        legend_position: LegendPosition::default(),
        legend_position_explicit: false,
        graph_ratio: 1.414,
        theme: BarTheme::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_empty_categories() {
        let empty_categories: Vec<&str> = vec![];
        let result = bar(&empty_categories, &[1.0, 2.0, 3.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::EmptyData {
                field: "categories"
            })
        ));
    }

    #[test]
    fn test_bar_empty_values() {
        let result = bar(&["A", "B", "C"], &[]).build();
        assert!(matches!(
            result,
            Err(ChartError::EmptyData { field: "values" })
        ));
    }

    #[test]
    fn test_bar_data_length_mismatch() {
        let result = bar(&["A", "B"], &[1.0, 2.0, 3.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "categories",
                y_field: "values",
                x_len: 2,
                y_len: 3,
            })
        ));
    }

    #[test]
    fn test_bar_invalid_value_nan() {
        let result = bar(&["A", "B", "C"], &[1.0, f64::NAN, 3.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "values",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_bar_successful_build() {
        let categories = vec!["A", "B", "C", "D"];
        let values = vec![10.0, 25.0, 15.0, 30.0];
        let result = bar(&categories, &values)
            .title("Test Bar Chart")
            .color(0x2ca02c)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_bar_negative_values() {
        let categories = vec!["A", "B", "C"];
        let values = vec![-5.0, 10.0, -3.0];
        let result = bar(&categories, &values).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_bar_builder_chain() {
        let result = bar(&["X", "Y", "Z"], &[1.0, 2.0, 3.0])
            .title("My Bar Chart")
            .color(0xff0000)
            .opacity(0.9)
            .bar_gap(5.0)
            .border_radius(4.0)
            .size(800.0, 600.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_bar_log_y_scale() {
        let categories = vec!["A", "B", "C", "D"];
        let values = vec![10.0, 100.0, 1000.0, 10000.0];
        let result = bar(&categories, &values).y_scale(ScaleType::Log).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_bar_log_y_scale_zero_value() {
        let categories = vec!["A", "B", "C"];
        let values = vec![0.0, 10.0, 100.0];
        let result = bar(&categories, &values).y_scale(ScaleType::Log).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "values",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_bar_log_y_scale_negative_value() {
        let categories = vec!["A", "B", "C"];
        let values = vec![-5.0, 10.0, 100.0];
        let result = bar(&categories, &values).y_scale(ScaleType::Log).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "values",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_bar_log_scale_with_title() {
        let categories = vec!["Low", "Medium", "High"];
        let values = vec![10.0, 100.0, 1000.0];
        let result = bar(&categories, &values)
            .title("Log Scale Bar Chart")
            .y_scale(ScaleType::Log)
            .color(0x2ca02c)
            .build();
        assert!(result.is_ok());
    }
}
