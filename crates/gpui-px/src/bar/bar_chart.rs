use super::bar_theme::BarTheme;
use super::types::BarSeries;
use crate::error::ChartError;
use crate::line::LegendPosition;
use crate::{
    ChartAccessibilitySummary, ChartAnnotation, ChartAnnotationSummary, ChartLegendItem,
    ChartLegendMarker, ChartLegendSummary, ChartSize, DEFAULT_COLOR, DEFAULT_HEIGHT,
    DEFAULT_PADDING_FRACTION, DEFAULT_TITLE_FONT_SIZE, DEFAULT_WIDTH, ScaleType, TITLE_AREA_HEIGHT,
    apply_chart_size, default_design, extent_padded_iter, finite_range_owned, format_range,
    format_scale, indexed_label, resolved_chart_dimensions, validate_data_array,
    validate_data_length, validate_dimensions, validate_positive, validate_range,
    validate_range_log,
};
use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::D3Color;
use d3rs::grid::{GridConfig, render_grid};
use d3rs::render2d::{Renderer2D, VelloBackend};
use d3rs::scale::{LinearScale, LogScale, Scale};
use d3rs::shape::{BarConfig, BarDatum};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, PathBuilder, canvas, div, point, px, rgb};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Bar chart builder.
#[derive(Debug, Clone)]
pub struct BarChart {
    // Primary series
    pub(super) categories: Vec<String>,
    pub(super) values: Arc<[f64]>,
    pub(super) label: Option<String>,
    pub(super) color: u32,
    pub(super) opacity: f32,
    pub(super) renderer_2d: Renderer2D,
    pub(super) vello_backend: VelloBackend,
    // Additional series
    pub(super) series: Vec<BarSeries>,
    // Common settings
    pub(super) title: Option<String>,
    pub(super) bar_gap: f32,
    pub(super) border_radius: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) chart_size: ChartSize,
    pub(super) y_scale_type: ScaleType,
    // Axis range overrides (for zoom support)
    pub(super) y_range: Option<[f64; 2]>,
    // Legend settings
    pub(super) show_legend: bool,
    pub(super) legend_position: LegendPosition,
    pub(super) legend_position_explicit: bool,
    pub(super) graph_ratio: f32,
    pub(super) theme: BarTheme,
    pub(super) design: Option<Arc<DesignSystem>>,
    /// Non-rendering annotation metadata for QA and host integrations.
    pub(super) annotations: Vec<ChartAnnotation>,
}

impl BarChart {
    /// Select the high-level 2D renderer. Vello is the default when enabled.
    pub fn renderer_2d(mut self, renderer: Renderer2D) -> Self {
        self.renderer_2d = renderer;
        self
    }

    /// Select the Vello WGPU/CPU backend.
    pub fn vello_backend(mut self, backend: VelloBackend) -> Self {
        self.vello_backend = backend;
        self
    }

    /// Export this bar chart as deterministic SVG.
    pub fn to_svg(&self) -> Result<String, ChartError> {
        self.to_svg_with_options(crate::StaticSvgOptions::new(self.width, self.height))
    }

    /// Export this bar chart as deterministic SVG with explicit export options.
    pub fn to_svg_with_options(
        &self,
        options: crate::StaticSvgOptions,
    ) -> Result<String, ChartError> {
        let mut series = Vec::with_capacity(1 + self.series.len());
        series.push(crate::static_export::StaticBarSeries {
            values: &self.values,
            label: self.label.as_deref(),
            color: self.color,
            opacity: self.opacity,
        });

        for bar_series in &self.series {
            series.push(crate::static_export::StaticBarSeries {
                values: &bar_series.values,
                label: bar_series.label.as_deref(),
                color: bar_series.color,
                opacity: bar_series.opacity,
            });
        }

        crate::static_export::render_bar_svg(
            self.title.as_deref(),
            &self.categories,
            self.y_scale_type,
            self.y_range,
            &series,
            options,
        )
    }

    /// Return structured native-legend metadata for this chart.
    pub fn legend_summary(&self) -> ChartLegendSummary {
        let mut items = Vec::new();

        if self.show_legend {
            if let Some(label) = &self.label {
                items.push(ChartLegendItem {
                    series_index: 0,
                    label: label.clone(),
                    color: self.color,
                    marker: ChartLegendMarker::Square,
                    hidden: false,
                    uses_secondary_axis: false,
                });
            }

            items.extend(
                self.series
                    .iter()
                    .enumerate()
                    .filter_map(|(index, series)| {
                        series.label.as_ref().map(|label| ChartLegendItem {
                            series_index: index + 1,
                            label: label.clone(),
                            color: series.color,
                            marker: ChartLegendMarker::Square,
                            hidden: false,
                            uses_secondary_axis: false,
                        })
                    }),
            );
        }

        ChartLegendSummary::new(
            "bar",
            self.show_legend,
            self.legend_position,
            self.legend_position_explicit,
            items,
        )
    }

    /// Return structured annotation metadata for this chart.
    pub fn annotation_summary(&self) -> ChartAnnotationSummary {
        ChartAnnotationSummary::new("bar", self.annotations.clone())
    }

    /// Return structured accessibility metadata for this chart.
    pub fn accessibility_summary(&self) -> ChartAccessibilitySummary {
        let series_count = 1 + self.series.len();
        let datum_count = self.categories.len() * series_count;
        let value_range = finite_range_owned(
            self.values.iter().copied().chain(
                self.series
                    .iter()
                    .flat_map(|series| series.values.iter().copied()),
            ),
        );
        let mut series_labels = vec![indexed_label(&self.label, "Series", 0)];
        series_labels.extend(
            self.series
                .iter()
                .enumerate()
                .map(|(index, series)| indexed_label(&series.label, "Series", index + 1)),
        );
        let title = self.title.clone();
        let name = title.as_deref().unwrap_or("Bar chart");
        let description = format!(
            "{name}: bar chart with {series_count} series across {} categories and {datum_count} bars. {}. Y scale {}.",
            self.categories.len(),
            format_range("Value", value_range),
            format_scale(self.y_scale_type)
        );

        ChartAccessibilitySummary {
            chart_type: "bar",
            title,
            series_count,
            datum_count,
            x_range: None,
            y_range: value_range,
            value_range,
            x_scale: None,
            y_scale: Some(self.y_scale_type),
            series_labels,
            description,
        }
    }

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

    /// Add non-rendering annotation metadata for host rendering or release QA.
    pub fn annotation(mut self, annotation: ChartAnnotation) -> Self {
        self.annotations.push(annotation);
        self
    }

    /// Replace annotation metadata for this chart.
    pub fn annotations(mut self, annotations: impl Into<Vec<ChartAnnotation>>) -> Self {
        self.annotations = annotations.into();
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
            values: Arc::from(values),
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

    /// Set an explicit design system for chart spacing and typography defaults.
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
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
        let design = self.design.clone().unwrap_or_else(default_design);
        let (layout_width, layout_height) = resolved_chart_dimensions(self.chart_size);
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
        validate_dimensions(layout_width, layout_height)?;

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
        let base_available_width = layout_width as f64 - margin_left - margin_right;
        let base_available_height =
            layout_height as f64 - title_height as f64 - margin_top - margin_bottom;

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
            (layout_width as f64 - margin_left - margin_right - width_for_legend as f64).max(0.0);
        let plot_height = (layout_height as f64
            - title_height as f64
            - margin_top
            - margin_bottom
            - height_for_legend as f64)
            .max(0.0);

        // Validate explicit y_range
        if let Some([min, max]) = self.y_range {
            if self.y_scale_type == ScaleType::Log {
                validate_range_log(min, max, "y_range")?;
            } else {
                validate_range(min, max, "y_range")?;
            }
        }

        // Calculate y domain with padding - include all series without cloning.
        // Use explicit y_range if set, otherwise calculate from data.
        let (mut y_min, mut y_max) = if let Some([min, max]) = self.y_range {
            (min, max)
        } else {
            extent_padded_iter(
                self.values
                    .iter()
                    .chain(self.series.iter().flat_map(|s| s.values.iter()))
                    .copied(),
                DEFAULT_PADDING_FRACTION,
            )
        };

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
        let grid_config = GridConfig::default().with_design(&design);
        let x_axis_config = AxisConfig::bottom().with_design(&design);
        let y_axis_config = AxisConfig::left().with_design(&design);

        // Determine if we're using grouped bars (multiple series) or simple bars
        let use_grouped_bars = !self.series.is_empty();

        // Prepare data for single-series bars
        let primary_data: Vec<BarDatum>;
        let primary_config: BarConfig;

        if use_grouped_bars {
            // Grouped bars are rendered directly from the original category and
            // series slices; no per-datum string clones are performed.
            primary_data = Vec::new();
            primary_config = BarConfig::from_design(&design);
        } else {
            // Single series - use simple bars
            primary_data = self
                .categories
                .iter()
                .zip(self.values.iter())
                .map(|(cat, &val)| BarDatum::new(cat.clone(), val))
                .collect();

            primary_config = BarConfig::from_design(&design)
                .fill_color(D3Color::from_hex(self.color))
                .opacity(self.opacity)
                .bar_gap(self.bar_gap)
                .border_radius(self.border_radius);
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
                        &grid_config,
                        plot_width as f32,
                        plot_height as f32,
                        &axis_theme,
                    ));

                if use_grouped_bars {
                    // Render grouped bars directly from original slices.
                    plot_area.child(render_grouped_bars_view(
                        &$y_scale,
                        &self.categories,
                        &self.values,
                        &self.series,
                        self.color,
                        self.opacity,
                        self.bar_gap,
                        self.border_radius,
                        plot_width as f32,
                        plot_height as f32,
                        self.renderer_2d,
                        self.vello_backend,
                    ))
                } else {
                    // Use simple bar rendering
                    plot_area.child(render_bars_selected(
                        &x_scale,
                        &$y_scale,
                        &primary_data,
                        plot_width as f32,
                        plot_height as f32,
                        &primary_config,
                        self.renderer_2d,
                        self.vello_backend,
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
                        &y_axis_config,
                        plot_height as f32,
                        &axis_theme,
                    ))
                    .child(div().flex().flex_col().child(plot_area).child(render_axis(
                        &x_scale,
                        &x_axis_config,
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
                        &y_axis_config,
                        plot_height as f32,
                        &axis_theme,
                    ))
                    .child(div().flex().flex_col().child(plot_area).child(render_axis(
                        &x_scale,
                        &x_axis_config,
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
        let mut container = apply_chart_size(div(), self.chart_size)
            .relative()
            .flex()
            .flex_col();

        // Add title if present
        if let Some(title) = &self.title {
            let font_config =
                GlyphTextConfig::horizontal(DEFAULT_TITLE_FONT_SIZE, self.theme.title_color);
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
        values: Arc::from(values),
        label: None,
        color: DEFAULT_COLOR,
        opacity: 0.8,
        renderer_2d: d3rs::render2d::Renderer2D::default(),
        vello_backend: d3rs::render2d::VelloBackend::default(),
        series: Vec::new(),
        title: None,
        bar_gap: 2.0,
        border_radius: 2.0,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        chart_size: ChartSize::default(),
        y_scale_type: ScaleType::Linear,
        y_range: None,
        show_legend: false,
        legend_position: LegendPosition::default(),
        legend_position_explicit: false,
        graph_ratio: 1.414,
        theme: BarTheme::default(),
        design: None,
        annotations: Vec::new(),
    }
}

fn render_bars_selected<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[BarDatum],
    width: f32,
    height: f32,
    config: &BarConfig,
    renderer: Renderer2D,
    backend: VelloBackend,
) -> AnyElement
where
    XS: Scale<f64, f64> + 'static,
    YS: Scale<f64, f64> + 'static,
{
    let config = config.clone().renderer_2d(renderer).vello_backend(backend);
    d3rs::shape::render_bars_selected(x_scale, y_scale, data, width, height, &config)
}

#[derive(Clone, Debug)]
struct GroupedBarQuad {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: D3Color,
}

/// Render grouped bars without cloning category/series strings.
///
/// Bar positions are computed directly from the original `categories`, `values`
/// and `series` slices, using numeric category/series indices instead of owned
/// label strings.
fn render_grouped_bars_view<YS>(
    y_scale: &YS,
    categories: &[String],
    values: &[f64],
    series: &[BarSeries],
    primary_color: u32,
    opacity: f32,
    bar_gap: f32,
    border_radius: f32,
    plot_width: f32,
    plot_height: f32,
    renderer: Renderer2D,
    backend: VelloBackend,
) -> AnyElement
where
    YS: Scale<f64, f64>,
{
    let num_categories = categories.len() as f32;
    let num_series = (series.len() + 1) as f32;

    let group_gap = bar_gap * 3.0;
    let inner_bar_gap = bar_gap * 0.5;
    let total_group_gaps = group_gap * (num_categories - 1.0).max(0.0);
    let available_width = plot_width - total_group_gaps;
    let group_width = available_width / num_categories;
    let total_bar_gaps = inner_bar_gap * (num_series - 1.0).max(0.0);
    let available_bar_width = group_width - total_bar_gaps;
    let bar_width = available_bar_width / num_series;

    let (y_min, y_max) = y_scale.range();
    let y_range_span = y_max - y_min;
    let (y_domain_min, y_domain_max) = y_scale.domain();
    let baseline = if y_domain_min <= 0.0 && y_domain_max >= 0.0 {
        y_scale.scale(0.0)
    } else {
        y_scale.scale(y_domain_min)
    };
    let baseline_pos = if y_range_span == 0.0 {
        0.5
    } else {
        1.0 - ((baseline - y_min) / y_range_span) as f32
    };

    let mut series_colors = vec![D3Color::from_hex(primary_color)];
    for s in series {
        series_colors.push(D3Color::from_hex(s.color));
    }

    let mut quads: Vec<GroupedBarQuad> = Vec::with_capacity(categories.len() * series_colors.len());
    for (cat_idx, value_ref) in values.iter().enumerate().take(categories.len()) {
        let group_start = cat_idx as f32 * (group_width + group_gap);
        for ser_idx in 0..series_colors.len() {
            let value = if ser_idx == 0 {
                *value_ref
            } else {
                series[ser_idx - 1].values[cat_idx]
            };
            let bar_offset = ser_idx as f32 * (bar_width + inner_bar_gap);
            let x_pos = group_start + bar_offset;

            let y_range = y_scale.scale(value);
            let y_pos = if y_range_span == 0.0 {
                0.5
            } else {
                1.0 - ((y_range - y_min) / y_range_span) as f32
            };
            let bar_height_rel = (baseline_pos - y_pos).abs();
            let bar_height_px = bar_height_rel * plot_height;
            let bar_top = if value >= 0.0 { y_pos } else { baseline_pos };
            let bar_top_px = bar_top * plot_height;

            quads.push(GroupedBarQuad {
                x: x_pos,
                y: bar_top_px,
                width: bar_width,
                height: bar_height_px,
                color: series_colors[ser_idx],
            });
        }
    }

    quads.sort_by(|a, b| {
        let a_key = (
            a.color.r.to_bits(),
            a.color.g.to_bits(),
            a.color.b.to_bits(),
            a.color.a.to_bits(),
        );
        let b_key = (
            b.color.r.to_bits(),
            b.color.g.to_bits(),
            b.color.b.to_bits(),
            b.color.a.to_bits(),
        );
        a_key.cmp(&b_key)
    });

    #[cfg(feature = "vello")]
    if renderer == Renderer2D::Vello {
        let cache_key =
            grouped_bar_scene_cache_key(&quads, plot_width, plot_height, opacity, border_radius);
        let vello_quads = quads.clone();
        return d3rs::vello2d::VelloChartElement::with_builder(move |width, height| {
            let quads: Vec<_> = vello_quads
                .iter()
                .map(|quad| (quad.x, quad.y, quad.width, quad.height, quad.color))
                .collect();
            if border_radius <= 0.0 {
                bar_chart_scene(&quads, plot_width, plot_height, width, height, opacity)
            } else {
                bar_chart_scene_with_radius(
                    &quads,
                    plot_width,
                    plot_height,
                    width,
                    height,
                    opacity,
                    border_radius,
                )
            }
        })
        .cache_key(cache_key)
        .backend(backend)
        .absolute()
        .into_any_element();
    }
    let _ = (renderer, backend);

    canvas(
        move |_bounds, _window, _cx| quads,
        move |bounds, quads, window, _cx| {
            let origin_x: f32 = bounds.origin.x.into();
            let origin_y: f32 = bounds.origin.y.into();

            let mut i = 0;
            while i < quads.len() {
                let color = quads[i].color;
                let mut group_end = i + 1;
                while group_end < quads.len() && quads[group_end].color == color {
                    group_end += 1;
                }

                let mut fill_builder = PathBuilder::fill();
                for quad in &quads[i..group_end] {
                    add_rounded_rect_to_path(
                        &mut fill_builder,
                        origin_x + quad.x,
                        origin_y + quad.y,
                        quad.width,
                        quad.height,
                        border_radius,
                    );
                }
                if let Ok(path) = fill_builder.build() {
                    let mut fill_color = color.to_rgba();
                    fill_color.a *= opacity;
                    window.paint_path(path, fill_color);
                }

                i = group_end;
            }
        },
    )
    .size_full()
    .absolute()
    .inset_0()
    .into_any_element()
}

/// Build a backend-neutral Vello scene for prepared grouped-bar quads.
#[cfg(feature = "vello")]
pub fn bar_chart_scene(
    quads: &[(f32, f32, f32, f32, D3Color)],
    source_width: f32,
    source_height: f32,
    width: f32,
    height: f32,
    opacity: f32,
) -> d3rs::vello2d::ChartScene {
    bar_chart_scene_with_radius(
        quads,
        source_width,
        source_height,
        width,
        height,
        opacity,
        0.0,
    )
}

#[cfg(feature = "vello")]
fn bar_chart_scene_with_radius(
    quads: &[(f32, f32, f32, f32, D3Color)],
    source_width: f32,
    source_height: f32,
    width: f32,
    height: f32,
    opacity: f32,
    border_radius: f32,
) -> d3rs::vello2d::ChartScene {
    use d3rs::vello2d::kurbo::Rect;
    use d3rs::vello2d::peniko::{Brush, Color};

    let sx = if source_width.abs() > f32::EPSILON {
        width / source_width
    } else {
        1.0
    };
    let sy = if source_height.abs() > f32::EPSILON {
        height / source_height
    } else {
        1.0
    };
    let radius = border_radius.max(0.0) as f64 * sx.abs().min(sy.abs()) as f64;
    let mut scene = d3rs::vello2d::ChartScene::new();
    let mut start = 0usize;
    while start < quads.len() {
        let color = quads[start].4;
        let mut end = start + 1;
        while end < quads.len() && quads[end].4 == color {
            end += 1;
        }
        let rgba = color.to_rgba();
        for &(x, y, quad_width, quad_height, _) in &quads[start..end] {
            scene.fill_rounded_rect(
                Rect::new(
                    (x * sx) as f64,
                    (y * sy) as f64,
                    ((x + quad_width) * sx) as f64,
                    ((y + quad_height) * sy) as f64,
                ),
                radius,
                Brush::Solid(Color::new([rgba.r, rgba.g, rgba.b, rgba.a * opacity])),
            );
        }
        start = end;
    }
    scene
}

#[cfg(feature = "vello")]
fn grouped_bar_scene_cache_key(
    quads: &[GroupedBarQuad],
    plot_width: f32,
    plot_height: f32,
    opacity: f32,
    border_radius: f32,
) -> u64 {
    let mut cache_key = d3rs::vello2d::SceneCacheKey::new();
    cache_key
        .add_f32(plot_width)
        .add_f32(plot_height)
        .add_f32(opacity)
        .add_f32(border_radius);
    for quad in quads {
        cache_key
            .add_f32(quad.x)
            .add_f32(quad.y)
            .add_f32(quad.width)
            .add_f32(quad.height)
            .add_f32(quad.color.r)
            .add_f32(quad.color.g)
            .add_f32(quad.color.b)
            .add_f32(quad.color.a);
    }
    cache_key.finish()
}

/// Append a rounded rectangle outline to a GPUI path builder.
pub(crate) fn add_rounded_rect_to_path(
    builder: &mut PathBuilder,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    radius: f32,
) {
    if radius <= 0.0 || width <= 0.0 || height <= 0.0 {
        builder.move_to(point(px(x), px(y)));
        builder.line_to(point(px(x + width), px(y)));
        builder.line_to(point(px(x + width), px(y + height)));
        builder.line_to(point(px(x), px(y + height)));
        builder.line_to(point(px(x), px(y)));
        return;
    }

    let r = radius.min(width / 2.0).min(height / 2.0);
    builder.move_to(point(px(x + r), px(y)));
    builder.line_to(point(px(x + width - r), px(y)));
    builder.curve_to(point(px(x + width), px(y + r)), point(px(x + width), px(y)));
    builder.line_to(point(px(x + width), px(y + height - r)));
    builder.curve_to(
        point(px(x + width - r), px(y + height)),
        point(px(x + width), px(y + height)),
    );
    builder.line_to(point(px(x + r), px(y + height)));
    builder.curve_to(
        point(px(x), px(y + height - r)),
        point(px(x), px(y + height)),
    );
    builder.line_to(point(px(x), px(y + r)));
    builder.curve_to(point(px(x + r), px(y)), point(px(x), px(y)));
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

    #[test]
    fn test_bar_with_explicit_y_range() {
        let categories = vec!["A", "B", "C"];
        let values = vec![10.0, 25.0, 15.0];
        let result = bar(&categories, &values).y_range(0.0, 50.0).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_bar_y_range_reversal_rejected() {
        let categories = vec!["A", "B", "C"];
        let values = vec![10.0, 25.0, 15.0];
        let result = bar(&categories, &values).y_range(50.0, 0.0).build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_bar_log_scale_negative_y_range_rejected() {
        let categories = vec!["A", "B", "C"];
        let values = vec![10.0, 25.0, 15.0];
        let result = bar(&categories, &values)
            .y_scale(ScaleType::Log)
            .y_range(-1.0, 100.0)
            .build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_bar_responsive_size_defaults_and_fixed_opt_in() {
        let categories = vec!["A", "B", "C"];
        let values = vec![10.0, 20.0, 30.0];

        crate::assert_default_chart_size(bar(&categories, &values).chart_size);
        crate::assert_fixed_chart_size(
            bar(&categories, &values).size(360.0, 220.0).chart_size,
            360.0,
            220.0,
        );
        crate::assert_fill_chart_size(
            bar(&categories, &values)
                .size(360.0, 220.0)
                .fill()
                .min_size(260.0, 180.0)
                .aspect_ratio(1.6)
                .chart_size,
            260.0,
            180.0,
            Some(1.6),
        );
    }

    #[test]
    fn test_bar_data_shared_via_arc_on_clone() {
        let categories = vec!["A", "B", "C"];
        let values = vec![10.0, 20.0, 30.0];
        let values2 = vec![5.0, 15.0, 25.0];
        let chart = bar(&categories, &values).add_series(&values2, Some("2024"), 0xff7f0e, 0.8);
        let cloned = chart.clone();

        assert!(Arc::ptr_eq(&chart.values, &cloned.values));
        assert!(Arc::ptr_eq(
            &chart.series[0].values,
            &cloned.series[0].values
        ));
    }

    #[test]
    fn test_bar_invalid_value_infinity() {
        let result = bar(&["A", "B", "C"], &[1.0, f64::INFINITY, 3.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "values",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_bar_series_length_mismatch() {
        let result = bar(&["A", "B", "C"], &[1.0, 2.0, 3.0])
            .add_series(&[1.0, 2.0], Some("Short"), 0xff0000, 0.8)
            .build();
        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "categories",
                y_field: "series.values",
                ..
            })
        ));
    }

    #[test]
    fn test_bar_series_nan() {
        let result = bar(&["A", "B", "C"], &[1.0, 2.0, 3.0])
            .add_series(&[1.0, f64::NAN, 3.0], Some("Bad"), 0xff0000, 0.8)
            .build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "series.values",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_bar_series_log_scale_zero() {
        let result = bar(&["A", "B", "C"], &[1.0, 2.0, 3.0])
            .add_series(&[0.0, 1.0, 2.0], Some("Zero"), 0xff0000, 0.8)
            .y_scale(ScaleType::Log)
            .build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "series.values",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_bar_grouped_builds() {
        let categories = vec!["A", "B", "C"];
        let values = vec![10.0, 20.0, 30.0];
        let values2 = vec![5.0, 15.0, 25.0];
        let result = bar(&categories, &values)
            .add_series(&values2, Some("2024"), 0xff7f0e, 0.8)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_bar_legend_positions_build() {
        let categories = vec!["A", "B"];
        let values = vec![10.0, 20.0];
        for position in [
            LegendPosition::Left,
            LegendPosition::Right,
            LegendPosition::Top,
            LegendPosition::Bottom,
            LegendPosition::Hidden,
        ] {
            let result = bar(&categories, &values)
                .label("Primary")
                .legend_position(position)
                .build();
            assert!(result.is_ok(), "failed for {position:?}");
        }
    }

    #[cfg(feature = "vello")]
    #[test]
    fn vello_grouped_bars_honor_border_radius_and_cache_it() {
        let scene_quads = [(0.0, 0.0, 10.0, 20.0, D3Color::from_hex(0x123456))];
        let square = bar_chart_scene_with_radius(&scene_quads, 10.0, 20.0, 10.0, 20.0, 1.0, 0.0);
        let rounded = bar_chart_scene_with_radius(&scene_quads, 10.0, 20.0, 10.0, 20.0, 1.0, 4.0);
        let path_element_count = |scene: &d3rs::vello2d::ChartScene| match &scene.commands()[0] {
            d3rs::vello2d::ChartCmd::Fill { path, .. } => path.elements().len(),
            d3rs::vello2d::ChartCmd::Stroke { .. } => 0,
        };
        assert!(path_element_count(&rounded) > path_element_count(&square));

        let cache_quads = [GroupedBarQuad {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 20.0,
            color: D3Color::from_hex(0x123456),
        }];
        assert_ne!(
            grouped_bar_scene_cache_key(&cache_quads, 10.0, 20.0, 1.0, 0.0),
            grouped_bar_scene_cache_key(&cache_quads, 10.0, 20.0, 1.0, 4.0),
        );
    }

    #[test]
    fn test_add_rounded_rect_to_path_zero_radius() {
        let mut builder = PathBuilder::fill();
        add_rounded_rect_to_path(&mut builder, 0.0, 0.0, 10.0, 20.0, 0.0);
        let _ = builder.build();
    }

    #[test]
    fn test_add_rounded_rect_to_path_large_radius() {
        // Radius larger than half width/height should be clamped
        let mut builder = PathBuilder::fill();
        add_rounded_rect_to_path(&mut builder, 0.0, 0.0, 10.0, 20.0, 100.0);
        let _ = builder.build();
    }
}
