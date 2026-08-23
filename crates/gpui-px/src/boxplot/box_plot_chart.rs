use super::box_stats::BoxStats;
use crate::error::ChartError;
use crate::{
    ChartAccessibilitySummary, ChartSize, DEFAULT_COLOR, DEFAULT_HEIGHT, DEFAULT_PADDING_FRACTION,
    DEFAULT_TITLE_FONT_SIZE, DEFAULT_WIDTH, ScaleType, TITLE_AREA_HEIGHT, apply_chart_size,
    default_design, extent_padded, finite_range, format_range, format_scale,
    resolved_chart_dimensions, validate_data_array, validate_data_length, validate_dimensions,
    validate_positive,
};
use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::D3Color;
use d3rs::grid::{GridConfig, render_grid};
use d3rs::render2d::{Renderer2D, VelloBackend};
use d3rs::scale::{LinearScale, LogScale, Scale};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, PathBuilder, canvas, div, hsla, point, px, rgb};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Box plot builder.
#[derive(Debug, Clone)]
pub struct BoxPlotChart {
    pub(super) x: Arc<[f64]>,
    pub(super) y: Arc<[f64]>,
    pub(super) title: Option<String>,
    pub(super) box_color: u32,
    pub(super) median_color: u32,
    pub(super) whisker_color: u32,
    pub(super) outlier_color: u32,
    pub(super) box_opacity: f32,
    pub(super) renderer_2d: Renderer2D,
    pub(super) vello_backend: VelloBackend,
    pub(super) box_width: f32,
    pub(super) stroke_width: f32,
    pub(super) outlier_radius: f32,
    pub(super) num_bins: Option<usize>,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) chart_size: ChartSize,
    pub(super) x_scale_type: ScaleType,
    pub(super) y_scale_type: ScaleType,
    pub(super) design: Option<Arc<DesignSystem>>,
}

#[derive(Clone, Debug)]
struct BoxDrawData {
    x_px: f32,
    half_width: f32,
    box_top: f32,
    box_height: f32,
    whisker_low_px: f32,
    whisker_high_px: f32,
    q2_px: f32,
    outliers_low: Vec<f32>,
    outliers_high: Vec<f32>,
}

impl BoxPlotChart {
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

    /// Export this box plot as deterministic SVG.
    pub fn to_svg(&self) -> Result<String, ChartError> {
        self.to_svg_with_options(crate::StaticSvgOptions::new(self.width, self.height))
    }

    /// Export this box plot as deterministic SVG with explicit export options.
    pub fn to_svg_with_options(
        &self,
        options: crate::StaticSvgOptions,
    ) -> Result<String, ChartError> {
        crate::static_export::render_boxplot_svg(
            self.title.as_deref(),
            crate::static_export::StaticBoxPlotSeries {
                x: &self.x,
                y: &self.y,
                x_scale_type: self.x_scale_type,
                y_scale_type: self.y_scale_type,
                num_bins: self.num_bins,
                box_color: self.box_color,
                median_color: self.median_color,
                whisker_color: self.whisker_color,
                outlier_color: self.outlier_color,
                box_opacity: self.box_opacity,
                box_width: self.box_width,
                stroke_width: self.stroke_width,
                outlier_radius: self.outlier_radius,
            },
            options,
        )
    }

    /// Return structured accessibility metadata for this chart.
    pub fn accessibility_summary(&self) -> ChartAccessibilitySummary {
        let x_range = finite_range(self.x.iter());
        let y_range = finite_range(self.y.iter());
        let title = self.title.clone();
        let name = title.as_deref().unwrap_or("Box plot");
        let bins = self.num_bins.map_or_else(
            || "automatic bins".to_string(),
            |count| format!("{count} bins"),
        );
        let description = format!(
            "{name}: box plot with {} observations grouped into {bins}. {}, {}. X scale {}, Y scale {}.",
            self.y.len(),
            format_range("X", x_range),
            format_range("Y", y_range),
            format_scale(self.x_scale_type),
            format_scale(self.y_scale_type)
        );

        ChartAccessibilitySummary {
            chart_type: "boxplot",
            title,
            series_count: 1,
            datum_count: self.y.len(),
            x_range,
            y_range,
            value_range: y_range,
            x_scale: Some(self.x_scale_type),
            y_scale: Some(self.y_scale_type),
            series_labels: vec!["Box plot values".to_string()],
            description,
        }
    }

    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set box fill color as 24-bit RGB hex value (format: 0xRRGGBB).
    pub fn box_color(mut self, hex: u32) -> Self {
        self.box_color = hex;
        self
    }

    /// Set median line color.
    pub fn median_color(mut self, hex: u32) -> Self {
        self.median_color = hex;
        self
    }

    /// Set whisker line color.
    pub fn whisker_color(mut self, hex: u32) -> Self {
        self.whisker_color = hex;
        self
    }

    /// Set outlier point color.
    pub fn outlier_color(mut self, hex: u32) -> Self {
        self.outlier_color = hex;
        self
    }

    /// Set box opacity (0.0 - 1.0).
    pub fn box_opacity(mut self, opacity: f32) -> Self {
        self.box_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set box width in pixels.
    pub fn box_width(mut self, width: f32) -> Self {
        self.box_width = width;
        self
    }

    /// Set stroke width for median and whisker lines.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Set outlier point radius.
    pub fn outlier_radius(mut self, radius: f32) -> Self {
        self.outlier_radius = radius;
        self
    }

    /// Set the number of bins for grouping data.
    /// If not set, automatically calculated based on chart width.
    pub fn bins(mut self, n: usize) -> Self {
        self.num_bins = Some(n);
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
    pub fn design(mut self, design: impl Into<Arc<DesignSystem>>) -> Self {
        self.design = Some(design.into());
        self
    }

    /// Set X-axis scale type (linear or log).
    pub fn x_scale(mut self, scale: ScaleType) -> Self {
        self.x_scale_type = scale;
        self
    }

    /// Set Y-axis scale type (linear or log).
    pub fn y_scale(mut self, scale: ScaleType) -> Self {
        self.y_scale_type = scale;
        self
    }

    /// Build and validate the chart, returning renderable element.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        let design = self.design.clone().unwrap_or_else(default_design);
        let (layout_width, layout_height) = resolved_chart_dimensions(self.chart_size);

        // Validate inputs
        validate_data_array(&self.x, "x")?;
        validate_data_array(&self.y, "y")?;
        validate_data_length(self.x.len(), self.y.len(), "x", "y")?;
        validate_dimensions(layout_width, layout_height)?;

        // Validate positive values for log scale
        if self.x_scale_type == ScaleType::Log {
            validate_positive(&self.x, "x")?;
        }
        if self.y_scale_type == ScaleType::Log {
            validate_positive(&self.y, "y")?;
        }

        // Define margins
        let margin_left = 60.0;
        let margin_bottom = 30.0;
        let margin_top = 10.0;
        let margin_right = 20.0;

        // Calculate plot area
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };

        let plot_width = (layout_width as f64 - margin_left - margin_right).max(0.0);
        let plot_height =
            (layout_height as f64 - title_height as f64 - margin_top - margin_bottom).max(0.0);

        // Calculate domains
        let (x_min, x_max) = extent_padded(&self.x, DEFAULT_PADDING_FRACTION);
        let (y_min, y_max) = extent_padded(&self.y, DEFAULT_PADDING_FRACTION);

        // Calculate number of bins. Reject zero up-front so the
        // `num_bins - 1` underflow and division by `num_bins` in
        // calculate_boxes cannot fire even when callers pass `.bins(0)`.
        let num_bins = self
            .num_bins
            .unwrap_or_else(|| (plot_width / 40.0).max(3.0) as usize);
        if num_bins == 0 {
            return Err(ChartError::InvalidData {
                field: "bins",
                reason: "boxplot bin count must be at least 1",
            });
        }

        // Bin the data
        let boxes = self.calculate_boxes(x_min, x_max, num_bins);

        // Build based on scale types
        let chart_content = self.render_chart(
            &boxes,
            x_min,
            x_max,
            y_min,
            y_max,
            plot_width,
            plot_height,
            &design,
        );

        // Build container with optional title
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

        // Add chart content
        container = container.child(div().relative().child(chart_content));

        Ok(container)
    }

    /// Calculate box statistics for each bin
    pub(super) fn calculate_boxes(&self, x_min: f64, x_max: f64, num_bins: usize) -> Vec<BoxStats> {
        let bin_width = (x_max - x_min) / num_bins as f64;

        // Group data points by bin
        let mut bins: Vec<Vec<f64>> = vec![Vec::new(); num_bins];

        for (&x, &y) in self.x.iter().zip(self.y.iter()) {
            let bin_idx = ((x - x_min) / bin_width).floor() as usize;
            let bin_idx = bin_idx.min(num_bins - 1);
            bins[bin_idx].push(y);
        }

        // Sort each bin once, then compute statistics from the sorted slice.
        bins.into_iter()
            .enumerate()
            .filter_map(|(i, mut bin)| {
                if bin.is_empty() {
                    return None;
                }
                bin.sort_by(|a, b| a.total_cmp(b));
                let x_center = x_min + (i as f64 + 0.5) * bin_width;
                BoxStats::from_values(x_center, &bin)
            })
            .collect()
    }

    /// Render the chart content
    pub(super) fn render_chart(
        &self,
        boxes: &[BoxStats],
        x_min: f64,
        x_max: f64,
        y_min: f64,
        y_max: f64,
        plot_width: f64,
        plot_height: f64,
        design: &DesignSystem,
    ) -> AnyElement {
        let theme = DefaultAxisTheme;

        match (self.x_scale_type, self.y_scale_type) {
            (ScaleType::Linear, ScaleType::Linear) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                self.render_with_scales(
                    &x_scale,
                    &y_scale,
                    boxes,
                    plot_width,
                    plot_height,
                    &theme,
                    design,
                )
            }
            (ScaleType::Log, ScaleType::Linear) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                self.render_with_scales(
                    &x_scale,
                    &y_scale,
                    boxes,
                    plot_width,
                    plot_height,
                    &theme,
                    design,
                )
            }
            (ScaleType::Linear, ScaleType::Log) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height, 0.0);

                self.render_with_scales(
                    &x_scale,
                    &y_scale,
                    boxes,
                    plot_width,
                    plot_height,
                    &theme,
                    design,
                )
            }
            (ScaleType::Log, ScaleType::Log) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, plot_width);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height, 0.0);

                self.render_with_scales(
                    &x_scale,
                    &y_scale,
                    boxes,
                    plot_width,
                    plot_height,
                    &theme,
                    design,
                )
            }
        }
    }

    /// Render with specific scale types using a single batched canvas layer.
    pub(super) fn render_with_scales<XS, YS>(
        &self,
        x_scale: &XS,
        y_scale: &YS,
        boxes: &[BoxStats],
        plot_width: f64,
        plot_height: f64,
        theme: &DefaultAxisTheme,
        design: &DesignSystem,
    ) -> AnyElement
    where
        XS: Scale<f64, f64>,
        YS: Scale<f64, f64>,
    {
        let box_color = D3Color::from_hex(self.box_color).to_rgba();
        let median_color = D3Color::from_hex(self.median_color).to_rgba();
        let whisker_color = D3Color::from_hex(self.whisker_color).to_rgba();
        let outlier_color = D3Color::from_hex(self.outlier_color).to_rgba();
        let x_axis_config = AxisConfig::bottom().with_design(design);
        let y_axis_config = AxisConfig::left().with_design(design);
        let grid_config = GridConfig::default().with_design(design);

        let box_width = self.box_width;

        let draw_data: Vec<BoxDrawData> = boxes
            .iter()
            .map(|stats| {
                let x_px = x_scale.scale(stats.x) as f32;
                let half_width = box_width / 2.0;

                let q1_px = y_scale.scale(stats.q1) as f32;
                let q2_px = y_scale.scale(stats.q2) as f32;
                let q3_px = y_scale.scale(stats.q3) as f32;
                let whisker_low_px = y_scale.scale(stats.whisker_low) as f32;
                let whisker_high_px = y_scale.scale(stats.whisker_high) as f32;

                let box_top = q3_px.min(q1_px);
                let box_bottom = q3_px.max(q1_px);
                let box_height = (box_bottom - box_top).max(1.0);

                BoxDrawData {
                    x_px,
                    half_width,
                    box_top,
                    box_height,
                    whisker_low_px,
                    whisker_high_px,
                    q2_px,
                    outliers_low: stats
                        .outliers_low
                        .iter()
                        .map(|&o| y_scale.scale(o) as f32)
                        .collect(),
                    outliers_high: stats
                        .outliers_high
                        .iter()
                        .map(|&o| y_scale.scale(o) as f32)
                        .collect(),
                }
            })
            .collect();

        let stroke_width = self.stroke_width;
        let box_opacity = self.box_opacity;
        let outlier_radius = self.outlier_radius;
        let renderer_2d = self.renderer_2d;
        let vello_backend = self.vello_backend;
        #[cfg(feature = "vello")]
        let vello_draw_data = draw_data.clone();

        let legacy_plot = canvas(
            move |_bounds, _window, _cx| draw_data.clone(),
            move |bounds, draw_data, window, _cx| {
                let origin_x: f32 = bounds.origin.x.into();
                let origin_y: f32 = bounds.origin.y.into();

                let mut whisker_builder = PathBuilder::stroke(px(stroke_width));
                let mut box_builder = PathBuilder::fill();
                let mut median_builder = PathBuilder::stroke(px(stroke_width * 2.0));
                let mut outlier_builder = PathBuilder::fill();

                for box_data in &draw_data {
                    let x = origin_x + box_data.x_px;

                    // Whisker line (vertical from low to high)
                    let whisker_top =
                        origin_y + box_data.whisker_high_px.min(box_data.whisker_low_px);
                    let whisker_bottom =
                        origin_y + box_data.whisker_high_px.max(box_data.whisker_low_px);
                    whisker_builder.move_to(point(px(x), px(whisker_top)));
                    whisker_builder.line_to(point(px(x), px(whisker_bottom)));

                    // Lower cap
                    let cap_y_low = origin_y + box_data.whisker_low_px;
                    whisker_builder
                        .move_to(point(px(x - box_data.half_width * 0.5), px(cap_y_low)));
                    whisker_builder
                        .line_to(point(px(x + box_data.half_width * 0.5), px(cap_y_low)));

                    // Upper cap
                    let cap_y_high = origin_y + box_data.whisker_high_px;
                    whisker_builder
                        .move_to(point(px(x - box_data.half_width * 0.5), px(cap_y_high)));
                    whisker_builder
                        .line_to(point(px(x + box_data.half_width * 0.5), px(cap_y_high)));

                    // Box (IQR)
                    add_rect_to_path(
                        &mut box_builder,
                        x - box_data.half_width,
                        origin_y + box_data.box_top,
                        box_width,
                        box_data.box_height,
                    );

                    // Median line
                    let median_y = origin_y + box_data.q2_px;
                    median_builder.move_to(point(px(x - box_data.half_width), px(median_y)));
                    median_builder.line_to(point(px(x + box_data.half_width), px(median_y)));

                    // Outliers as small circles (drawn as filled rounded quads)
                    let r = outlier_radius;
                    let diameter = r * 2.0;
                    for &y_px in &box_data.outliers_low {
                        let cy = origin_y + y_px;
                        add_rect_to_path(&mut outlier_builder, x - r, cy - r, diameter, diameter);
                    }
                    for &y_px in &box_data.outliers_high {
                        let cy = origin_y + y_px;
                        add_rect_to_path(&mut outlier_builder, x - r, cy - r, diameter, diameter);
                    }
                }

                if let Ok(path) = whisker_builder.build() {
                    window.paint_path(path, whisker_color);
                }
                if let Ok(path) = box_builder.build() {
                    let mut fill = box_color;
                    fill.a *= box_opacity;
                    window.paint_path(path, fill);
                }
                if let Ok(path) = median_builder.build() {
                    window.paint_path(path, median_color);
                }
                if let Ok(path) = outlier_builder.build() {
                    let mut fill = outlier_color;
                    fill.a *= 0.7;
                    window.paint_path(path, fill);
                }
            },
        )
        .size_full()
        .absolute()
        .inset_0();

        let plot: AnyElement = {
            #[cfg(feature = "vello")]
            if renderer_2d == Renderer2D::Vello {
                let draw_data = vello_draw_data;
                return_vello_boxplot(
                    draw_data,
                    plot_width as f32,
                    plot_height as f32,
                    box_color,
                    median_color,
                    whisker_color,
                    outlier_color,
                    box_opacity,
                    outlier_radius,
                    stroke_width,
                    vello_backend,
                )
            } else {
                legacy_plot.into_any_element()
            }
            #[cfg(not(feature = "vello"))]
            {
                let _ = (renderer_2d, vello_backend);
                legacy_plot.into_any_element()
            }
        };

        div()
            .flex()
            .child(render_axis(
                y_scale,
                &y_axis_config,
                plot_height as f32,
                theme,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .w(px(plot_width as f32))
                            .h(px(plot_height as f32))
                            .relative()
                            .bg(rgb(0xf8f8f8))
                            .child(render_grid(
                                x_scale,
                                y_scale,
                                &grid_config,
                                plot_width as f32,
                                plot_height as f32,
                                theme,
                            ))
                            .child(plot),
                    )
                    .child(render_axis(
                        x_scale,
                        &x_axis_config,
                        plot_width as f32,
                        theme,
                    )),
            )
            .into_any_element()
    }
}

/// Create a box plot from x and y data.
///
/// The data is binned by x values, and for each bin, box-and-whisker statistics
/// are calculated from the y values.
///
/// # Example
///
/// ```rust,ignore
/// use gpui_px::boxplot;
///
/// // Generate some sample data
/// let x: Vec<f64> = (0..100).map(|i| (i / 10) as f64).collect();
/// let y: Vec<f64> = x.iter().map(|&xi| xi * 2.0 + rand::random::<f64>() * 10.0).collect();
///
/// let chart = boxplot(&x, &y)
///     .title("Distribution by Group")
///     .box_color(0xdddddd)
///     .median_color(0x000000)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn boxplot(x: &[f64], y: &[f64]) -> BoxPlotChart {
    BoxPlotChart {
        x: Arc::from(x),
        y: Arc::from(y),
        title: None,
        box_color: 0xdddddd,
        median_color: 0x000000,
        whisker_color: 0x333333,
        outlier_color: DEFAULT_COLOR,
        box_opacity: 1.0,
        renderer_2d: Renderer2D::default(),
        vello_backend: VelloBackend::default(),
        box_width: 20.0,
        stroke_width: 2.0,
        outlier_radius: 3.0,
        num_bins: None,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        chart_size: ChartSize::default(),
        x_scale_type: ScaleType::Linear,
        y_scale_type: ScaleType::Linear,
        design: None,
    }
}

#[cfg(feature = "vello")]
fn return_vello_boxplot(
    draw_data: Vec<BoxDrawData>,
    plot_width: f32,
    plot_height: f32,
    box_color: gpui::Rgba,
    median_color: gpui::Rgba,
    whisker_color: gpui::Rgba,
    outlier_color: gpui::Rgba,
    box_opacity: f32,
    outlier_radius: f32,
    stroke_width: f32,
    backend: VelloBackend,
) -> AnyElement {
    let mut cache_key = d3rs::vello2d::SceneCacheKey::new();
    cache_key
        .add_f32(plot_width)
        .add_f32(plot_height)
        .add_f32(box_opacity)
        .add_f32(outlier_radius)
        .add_f32(stroke_width);
    for color in [box_color, median_color, whisker_color, outlier_color] {
        cache_key
            .add_f32(color.r)
            .add_f32(color.g)
            .add_f32(color.b)
            .add_f32(color.a);
    }
    for data in &draw_data {
        cache_key
            .add_f32(data.x_px)
            .add_f32(data.half_width)
            .add_f32(data.box_top)
            .add_f32(data.box_height)
            .add_f32(data.whisker_low_px)
            .add_f32(data.whisker_high_px)
            .add_f32(data.q2_px);
        for value in data.outliers_low.iter().chain(&data.outliers_high) {
            cache_key.add_f32(*value);
        }
    }
    let cache_key = cache_key.finish();
    d3rs::vello2d::VelloChartElement::with_builder(move |width, height| {
        box_plot_chart_scene(
            &draw_data,
            plot_width,
            plot_height,
            width,
            height,
            box_color,
            median_color,
            whisker_color,
            outlier_color,
            box_opacity,
            outlier_radius,
            stroke_width,
        )
    })
    .cache_key(cache_key)
    .backend(backend)
    .absolute()
    .into_any_element()
}

#[cfg(feature = "vello")]
fn box_plot_chart_scene(
    draw_data: &[BoxDrawData],
    plot_width: f32,
    plot_height: f32,
    width: f32,
    height: f32,
    box_color: gpui::Rgba,
    median_color: gpui::Rgba,
    whisker_color: gpui::Rgba,
    outlier_color: gpui::Rgba,
    box_opacity: f32,
    outlier_radius: f32,
    stroke_width: f32,
) -> d3rs::vello2d::ChartScene {
    use d3rs::vello2d::kurbo::{BezPath, PathEl, Stroke};
    use d3rs::vello2d::peniko::{Brush, Color};

    let sx = if plot_width > 0.0 {
        width / plot_width
    } else {
        1.0
    };
    let sy = if plot_height > 0.0 {
        height / plot_height
    } else {
        1.0
    };
    let brush = |color: gpui::Rgba, alpha: f32| {
        Brush::Solid(Color::new([color.r, color.g, color.b, color.a * alpha]))
    };
    let mut scene = d3rs::vello2d::ChartScene::new();
    let mut whiskers = BezPath::new();
    let mut boxes = BezPath::new();
    let mut medians = BezPath::new();
    let mut outliers = Vec::new();
    for data in draw_data {
        let x = data.x_px * sx;
        let half = data.half_width * sx;
        let low = data.whisker_low_px * sy;
        let high = data.whisker_high_px * sy;
        whiskers.push(PathEl::MoveTo((x as f64, low as f64).into()));
        whiskers.push(PathEl::LineTo((x as f64, high as f64).into()));
        for y in [low, high] {
            whiskers.push(PathEl::MoveTo(((x - half * 0.5) as f64, y as f64).into()));
            whiskers.push(PathEl::LineTo(((x + half * 0.5) as f64, y as f64).into()));
        }
        let top = data.box_top * sy;
        let bottom = (data.box_top + data.box_height) * sy;
        boxes.push(PathEl::MoveTo(((x - half) as f64, top as f64).into()));
        boxes.push(PathEl::LineTo(((x + half) as f64, top as f64).into()));
        boxes.push(PathEl::LineTo(((x + half) as f64, bottom as f64).into()));
        boxes.push(PathEl::LineTo(((x - half) as f64, bottom as f64).into()));
        boxes.push(PathEl::ClosePath);
        let median = data.q2_px * sy;
        medians.push(PathEl::MoveTo(((x - half) as f64, median as f64).into()));
        medians.push(PathEl::LineTo(((x + half) as f64, median as f64).into()));
        for y in data.outliers_low.iter().chain(data.outliers_high.iter()) {
            outliers.push((x, *y as f32 * sy));
        }
    }
    if !whiskers.is_empty() {
        scene.stroke_path(
            whiskers,
            Stroke::new(stroke_width as f64),
            brush(whisker_color, 1.0),
        );
    }
    if !boxes.is_empty() {
        scene.fill_path(boxes, brush(box_color, box_opacity));
    }
    if !medians.is_empty() {
        scene.stroke_path(
            medians,
            Stroke::new(stroke_width as f64 * 2.0),
            brush(median_color, 1.0),
        );
    }
    if !outliers.is_empty() {
        let radius = outlier_radius * sx.min(sy);
        let outlier_brush = brush(outlier_color, 0.7);
        for (x, y) in outliers {
            scene.fill_circle(x as f64, y as f64, radius as f64, outlier_brush.clone());
        }
    }
    scene
}

/// Append a rectangle outline to a GPUI path builder.
pub(crate) fn add_rect_to_path(builder: &mut PathBuilder, x: f32, y: f32, width: f32, height: f32) {
    builder.move_to(point(px(x), px(y)));
    builder.line_to(point(px(x + width), px(y)));
    builder.line_to(point(px(x + width), px(y + height)));
    builder.line_to(point(px(x), px(y + height)));
    builder.line_to(point(px(x), px(y)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boxplot_data_shared_via_arc_on_clone() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let chart = boxplot(&x, &y);
        let cloned = chart.clone();

        assert!(Arc::ptr_eq(&chart.x, &cloned.x));
        assert!(Arc::ptr_eq(&chart.y, &cloned.y));
    }

    #[test]
    fn test_boxplot_empty_x() {
        let result = boxplot(&[], &[1.0, 2.0]).build();
        assert!(matches!(result, Err(ChartError::EmptyData { field: "x" })));
    }

    #[test]
    fn test_boxplot_empty_y() {
        let result = boxplot(&[1.0, 2.0], &[]).build();
        assert!(matches!(result, Err(ChartError::EmptyData { field: "y" })));
    }

    #[test]
    fn test_boxplot_length_mismatch() {
        let result = boxplot(&[1.0, 2.0, 3.0], &[1.0, 2.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "x",
                y_field: "y",
                x_len: 3,
                y_len: 2,
            })
        ));
    }

    #[test]
    fn test_boxplot_nan_in_data() {
        let result = boxplot(&[1.0, f64::NAN], &[1.0, 2.0]).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_boxplot_invalid_dimensions() {
        let result = boxplot(&[1.0, 2.0], &[1.0, 2.0]).size(0.0, 400.0).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidDimension {
                field: "width",
                value: 0.0
            })
        ));
    }

    #[test]
    fn test_boxplot_log_scale_negative_x() {
        let result = boxplot(&[-1.0, 1.0], &[1.0, 2.0])
            .x_scale(ScaleType::Log)
            .build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_boxplot_log_scale_negative_y() {
        let result = boxplot(&[1.0, 2.0], [-1.0, 1.0].as_slice())
            .y_scale(ScaleType::Log)
            .build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "y",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_boxplot_zero_bins_rejected() {
        let result = boxplot(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]).bins(0).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "bins",
                reason: "boxplot bin count must be at least 1"
            })
        ));
    }

    #[test]
    fn test_boxplot_successful_build() {
        let x: Vec<f64> = (0..30).map(|i| i as f64).collect();
        let y: Vec<f64> = x.iter().map(|&xi| xi + 1.0).collect();
        let result = boxplot(&x, &y).title("Box Plot").build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_boxplot_all_scale_combinations_build() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        for (x_scale, y_scale) in [
            (ScaleType::Linear, ScaleType::Linear),
            (ScaleType::Log, ScaleType::Linear),
            (ScaleType::Linear, ScaleType::Log),
            (ScaleType::Log, ScaleType::Log),
        ] {
            let result = boxplot(&x, &y).x_scale(x_scale).y_scale(y_scale).build();
            assert!(result.is_ok(), "failed for x={x_scale:?}, y={y_scale:?}");
        }
    }

    #[test]
    fn test_calculate_boxes_single_bin() {
        let chart = boxplot(&[1.0, 2.0, 3.0], &[10.0, 20.0, 30.0]).bins(1);
        let boxes = chart.calculate_boxes(0.0, 4.0, 1);
        assert_eq!(boxes.len(), 1);
        // Linear interpolation on sorted [10,20,30]
        assert_eq!(boxes[0].q1, 15.0);
        assert_eq!(boxes[0].q2, 20.0);
        assert_eq!(boxes[0].q3, 25.0);
    }

    #[test]
    fn test_calculate_boxes_multiple_bins() {
        let x = vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let y = vec![1.0; 10];
        let chart = boxplot(&x, &y).bins(2);
        let boxes = chart.calculate_boxes(-1.0, 10.0, 2);
        assert!(!boxes.is_empty());
    }

    #[test]
    fn test_calculate_boxes_empty_bins_filtered() {
        // Sparse data so some bins are empty
        let x = vec![0.0, 9.0];
        let y = vec![1.0, 2.0];
        let chart = boxplot(&x, &y).bins(5);
        let boxes = chart.calculate_boxes(0.0, 10.0, 5);
        assert_eq!(boxes.len(), 2);
    }

    #[test]
    fn test_add_rect_to_path_builds_rectangle() {
        use gpui::PathBuilder;
        let mut builder = PathBuilder::fill();
        add_rect_to_path(&mut builder, 0.0, 0.0, 10.0, 20.0);
        // Building should succeed for a closed rectangle
        let _ = builder.build();
    }

    #[test]
    fn test_boxplot_builder_chain() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = boxplot(&x, &y)
            .title("My Box Plot")
            .box_color(0xdddddd)
            .median_color(0x000000)
            .whisker_color(0x333333)
            .outlier_color(0xff0000)
            .box_opacity(0.8)
            .box_width(15.0)
            .stroke_width(1.5)
            .outlier_radius(4.0)
            .bins(3)
            .size(800.0, 600.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_boxplot_responsive_size_defaults_and_fixed_opt_in() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        crate::assert_default_chart_size(boxplot(&x, &y).chart_size);
        crate::assert_fixed_chart_size(boxplot(&x, &y).size(360.0, 240.0).chart_size, 360.0, 240.0);
        crate::assert_fill_chart_size(
            boxplot(&x, &y)
                .size(360.0, 240.0)
                .fill()
                .min_size(300.0, 220.0)
                .aspect_ratio(1.2)
                .chart_size,
            300.0,
            220.0,
            Some(1.2),
        );
    }
}
