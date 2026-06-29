use crate::color_scale::ColorScale;
use crate::error::ChartError;
use crate::{
    ChartSize, DEFAULT_HEIGHT, DEFAULT_TITLE_FONT_SIZE, DEFAULT_WIDTH, ScaleType,
    TITLE_AREA_HEIGHT, apply_chart_size, default_design, extent_padded, resolved_chart_dimensions,
    validate_data_array, validate_dimensions, validate_grid_dimensions, validate_monotonic,
    validate_positive, validate_range, validate_range_log,
};
use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
#[cfg(feature = "gpu-2d")]
use d3rs::gpu2d::render_heatmap;
use d3rs::grid::{GridConfig, render_grid};
use d3rs::scale::{LinearScale, LogScale};
#[cfg(not(feature = "gpu-2d"))]
use d3rs::shape::render_heatmap;
use d3rs::shape::{ContourConfig, HeatmapData};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, div, hsla, px, rgb};
use gpui_design::DesignSystem;
use std::sync::Arc;

/// Heatmap chart builder.
#[derive(Clone)]
pub struct HeatmapChart {
    pub(super) z: Vec<f64>,
    pub(super) grid_width: usize,
    pub(super) grid_height: usize,
    pub(super) x_values: Option<Vec<f64>>,
    pub(super) y_values: Option<Vec<f64>>,
    pub(super) x_scale_type: ScaleType,
    pub(super) y_scale_type: ScaleType,
    pub(super) color_scale: ColorScale,
    pub(super) title: Option<String>,
    pub(super) opacity: f32,
    pub(super) width: f32,
    pub(super) height: f32,
    pub(super) chart_size: ChartSize,
    // Axis range overrides (for zoom support)
    pub(super) x_range: Option<[f64; 2]>,
    pub(super) y_range: Option<[f64; 2]>,
    pub(super) design: Option<Arc<DesignSystem>>,
}

impl std::fmt::Debug for HeatmapChart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeatmapChart")
            .field("grid_width", &self.grid_width)
            .field("grid_height", &self.grid_height)
            .field("x_scale_type", &self.x_scale_type)
            .field("y_scale_type", &self.y_scale_type)
            .field("color_scale", &self.color_scale)
            .field("title", &self.title)
            .field("opacity", &self.opacity)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl HeatmapChart {
    /// Set custom x axis values.
    ///
    /// Values must be strictly monotonically increasing.
    /// Length must match grid_width.
    pub fn x(mut self, values: &[f64]) -> Self {
        self.x_values = Some(values.to_vec());
        self
    }

    /// Set custom y axis values.
    ///
    /// Values must be strictly monotonically increasing.
    /// Length must match grid_height.
    pub fn y(mut self, values: &[f64]) -> Self {
        self.y_values = Some(values.to_vec());
        self
    }

    /// Set x-axis scale type.
    pub fn x_scale(mut self, scale: ScaleType) -> Self {
        self.x_scale_type = scale;
        self
    }

    /// Set y-axis scale type.
    pub fn y_scale(mut self, scale: ScaleType) -> Self {
        self.y_scale_type = scale;
        self
    }

    /// Set color scale.
    pub fn color_scale(mut self, scale: ColorScale) -> Self {
        self.color_scale = scale;
        self
    }

    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set fill opacity (0.0 - 1.0).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set chart dimensions.
    pub fn chart_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self.chart_size = ChartSize::fixed(width, height);
        self
    }

    /// Set chart dimensions (alias for `chart_size`).
    pub fn size(self, width: f32, height: f32) -> Self {
        self.chart_size(width, height)
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

    /// Build and validate the chart, returning renderable element.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        let design = self.design.clone().unwrap_or_else(default_design);
        let (layout_width, layout_height) = resolved_chart_dimensions(self.chart_size);

        // Validate inputs
        validate_data_array(&self.z, "z")?;
        validate_grid_dimensions(&self.z, self.grid_width, self.grid_height)?;
        validate_dimensions(layout_width, layout_height)?;

        // Generate or validate x values. Take ownership of explicit values
        // instead of cloning them; only allocate when falling back to defaults.
        let x_values = match self.x_values {
            Some(v) => {
                if v.len() != self.grid_width {
                    return Err(ChartError::DataLengthMismatch {
                        x_field: "x",
                        y_field: "grid_width",
                        x_len: v.len(),
                        y_len: self.grid_width,
                    });
                }
                validate_data_array(&v, "x")?;
                validate_monotonic(&v, "x")?;
                if self.x_scale_type == ScaleType::Log {
                    validate_positive(&v, "x")?;
                }
                v
            }
            None => {
                if self.x_scale_type == ScaleType::Log {
                    return Err(ChartError::InvalidData {
                        field: "x",
                        reason: "log scale requires explicit positive x values",
                    });
                }
                (0..self.grid_width).map(|i| i as f64).collect()
            }
        };

        // Generate or validate y values
        let y_values = match self.y_values {
            Some(v) => {
                if v.len() != self.grid_height {
                    return Err(ChartError::DataLengthMismatch {
                        x_field: "y",
                        y_field: "grid_height",
                        x_len: v.len(),
                        y_len: self.grid_height,
                    });
                }
                validate_data_array(&v, "y")?;
                validate_monotonic(&v, "y")?;
                if self.y_scale_type == ScaleType::Log {
                    validate_positive(&v, "y")?;
                }
                v
            }
            None => {
                if self.y_scale_type == ScaleType::Log {
                    return Err(ChartError::InvalidData {
                        field: "y",
                        reason: "log scale requires explicit positive y values",
                    });
                }
                (0..self.grid_height).map(|i| i as f64).collect()
            }
        };

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

        let plot_width = (layout_width as f64 - margin_left - margin_right).max(0.0);
        let plot_height =
            (layout_height as f64 - title_height as f64 - margin_top - margin_bottom).max(0.0);

        // Validate explicit ranges
        if let Some([min, max]) = self.x_range {
            if self.x_scale_type == ScaleType::Log {
                validate_range_log(min, max, "x_range")?;
            } else {
                validate_range(min, max, "x_range")?;
            }
        }
        if let Some([min, max]) = self.y_range {
            if self.y_scale_type == ScaleType::Log {
                validate_range_log(min, max, "y_range")?;
            } else {
                validate_range(min, max, "y_range")?;
            }
        }

        // Calculate domains with padding, or use explicit ranges if set
        let (x_min, x_max) = if let Some([min, max]) = self.x_range {
            (min, max)
        } else {
            extent_padded(&x_values, 0.0)
        };
        let (y_min, y_max) = if let Some([min, max]) = self.y_range {
            (min, max)
        } else {
            extent_padded(&y_values, 0.0)
        };

        // Create HeatmapData
        let heatmap_data = HeatmapData::new(x_values, y_values, self.z);

        // Build config with color scale
        let color_fn = self.color_scale.to_fn();
        let config = ContourConfig::from_design(&design)
            .fill(true)
            .fill_opacity(self.opacity)
            .color_scale(color_fn);

        let theme = DefaultAxisTheme;
        let x_axis_config = AxisConfig::bottom().with_design(&design);
        let y_axis_config = AxisConfig::left().with_design(&design);
        let grid_config = GridConfig::default().with_design(&design);

        // Build the element based on scale types
        let chart_content: AnyElement = match (self.x_scale_type, self.y_scale_type) {
            (ScaleType::Linear, ScaleType::Linear) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                div()
                    .flex()
                    .child(render_axis(
                        &y_scale,
                        &y_axis_config,
                        plot_height as f32,
                        &theme,
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
                                    .overflow_hidden()
                                    .bg(rgb(0xf8f8f8))
                                    .child(render_grid(
                                        &x_scale,
                                        &y_scale,
                                        &grid_config,
                                        plot_width as f32,
                                        plot_height as f32,
                                        &theme,
                                    ))
                                    .child(div().absolute().inset_0().size_full().child({
                                        let el = render_heatmap(
                                            heatmap_data,
                                            &x_scale,
                                            &y_scale,
                                            &config,
                                        );
                                        #[cfg(not(feature = "gpu-2d"))]
                                        let el = el.height(px(plot_height as f32));
                                        el
                                    })),
                            )
                            .child(render_axis(
                                &x_scale,
                                &x_axis_config,
                                plot_width as f32,
                                &theme,
                            )),
                    )
                    .into_any_element()
            }
            (ScaleType::Log, ScaleType::Linear) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, plot_width);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height, 0.0);

                div()
                    .flex()
                    .child(render_axis(
                        &y_scale,
                        &y_axis_config,
                        plot_height as f32,
                        &theme,
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
                                    .overflow_hidden()
                                    .bg(rgb(0xf8f8f8))
                                    .child(render_grid(
                                        &x_scale,
                                        &y_scale,
                                        &grid_config,
                                        plot_width as f32,
                                        plot_height as f32,
                                        &theme,
                                    ))
                                    .child(div().absolute().inset_0().size_full().child({
                                        let el = render_heatmap(
                                            heatmap_data,
                                            &x_scale,
                                            &y_scale,
                                            &config,
                                        );
                                        #[cfg(not(feature = "gpu-2d"))]
                                        let el = el.height(px(plot_height as f32));
                                        el
                                    })),
                            )
                            .child(render_axis(
                                &x_scale,
                                &x_axis_config,
                                plot_width as f32,
                                &theme,
                            )),
                    )
                    .into_any_element()
            }
            (ScaleType::Linear, ScaleType::Log) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, plot_width);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height, 0.0);

                div()
                    .flex()
                    .child(render_axis(
                        &y_scale,
                        &y_axis_config,
                        plot_height as f32,
                        &theme,
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
                                    .overflow_hidden()
                                    .bg(rgb(0xf8f8f8))
                                    .child(render_grid(
                                        &x_scale,
                                        &y_scale,
                                        &grid_config,
                                        plot_width as f32,
                                        plot_height as f32,
                                        &theme,
                                    ))
                                    .child(div().absolute().inset_0().size_full().child({
                                        let el = render_heatmap(
                                            heatmap_data,
                                            &x_scale,
                                            &y_scale,
                                            &config,
                                        );
                                        #[cfg(not(feature = "gpu-2d"))]
                                        let el = el.height(px(plot_height as f32));
                                        el
                                    })),
                            )
                            .child(render_axis(
                                &x_scale,
                                &x_axis_config,
                                plot_width as f32,
                                &theme,
                            )),
                    )
                    .into_any_element()
            }
            (ScaleType::Log, ScaleType::Log) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, plot_width);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height, 0.0);

                div()
                    .flex()
                    .child(render_axis(
                        &y_scale,
                        &y_axis_config,
                        plot_height as f32,
                        &theme,
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
                                    .overflow_hidden()
                                    .bg(rgb(0xf8f8f8))
                                    .child(render_grid(
                                        &x_scale,
                                        &y_scale,
                                        &grid_config,
                                        plot_width as f32,
                                        plot_height as f32,
                                        &theme,
                                    ))
                                    .child(div().absolute().inset_0().size_full().child({
                                        let el = render_heatmap(
                                            heatmap_data,
                                            &x_scale,
                                            &y_scale,
                                            &config,
                                        );
                                        #[cfg(not(feature = "gpu-2d"))]
                                        let el = el.height(px(plot_height as f32));
                                        el
                                    })),
                            )
                            .child(render_axis(
                                &x_scale,
                                &x_axis_config,
                                plot_width as f32,
                                &theme,
                            )),
                    )
                    .into_any_element()
            }
        };

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
}

/// Create a heatmap chart from z data with grid dimensions.
///
/// Data is in row-major order: `z[row * width + col]` where row 0 is at the bottom.
///
/// # Example
///
/// ```rust,no_run
/// use gpui_px::{heatmap, ColorScale, ScaleType};
///
/// // 3x3 grid
/// let z = vec![
///     1.0, 2.0, 3.0,  // row 0 (bottom)
///     4.0, 5.0, 6.0,  // row 1
///     7.0, 8.0, 9.0,  // row 2 (top)
/// ];
///
/// let chart = heatmap(&z, 3, 3)
///     .title("My Heatmap")
///     .color_scale(ColorScale::Inferno)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
///
/// # With custom axes
///
/// ```rust,no_run
/// use gpui_px::{heatmap, ColorScale, ScaleType};
///
/// let freq_bins = vec![20.0, 100.0, 1000.0, 10000.0, 20000.0];
/// let time_bins = vec![0.0, 1.0, 2.0, 3.0];
/// let z = vec![0.0; 20]; // 5x4 grid
///
/// let chart = heatmap(&z, 5, 4)
///     .x(&freq_bins)
///     .y(&time_bins)
///     .x_scale(ScaleType::Log)
///     .color_scale(ColorScale::Viridis)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn heatmap(z: &[f64], grid_width: usize, grid_height: usize) -> HeatmapChart {
    HeatmapChart {
        z: z.to_vec(),
        grid_width,
        grid_height,
        x_values: None,
        y_values: None,
        x_scale_type: ScaleType::Linear,
        y_scale_type: ScaleType::Linear,
        color_scale: ColorScale::default(),
        title: None,
        opacity: 1.0,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        chart_size: ChartSize::default(),
        x_range: None,
        y_range: None,
        design: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::any::Any;

    fn assert_heatmap_builds(chart: HeatmapChart) {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| chart.build().map(|_| ())));

        match result {
            Ok(result) => assert!(
                result.is_ok(),
                "expected heatmap build to succeed, got {result:?}"
            ),
            Err(payload) if is_missing_gpu_adapter_panic(payload.as_ref()) => {}
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    fn is_missing_gpu_adapter_panic(payload: &(dyn Any + Send)) -> bool {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&'static str>().copied())
            .unwrap_or_default();

        message.contains("Failed to find suitable GPU adapter")
    }

    #[test]
    fn test_heatmap_empty_z() {
        let result = heatmap(&[], 0, 0).build();
        assert!(matches!(result, Err(ChartError::EmptyData { field: "z" })));
    }

    #[test]
    fn test_heatmap_grid_mismatch() {
        let z = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // 5 values
        let result = heatmap(&z, 2, 3).build(); // expects 6
        assert!(matches!(
            result,
            Err(ChartError::GridDimensionMismatch {
                z_len: 5,
                width: 2,
                height: 3,
                expected: 6,
            })
        ));
    }

    #[test]
    fn test_heatmap_x_length_mismatch() {
        let z = vec![1.0; 6]; // 2x3 grid
        let x = vec![0.0, 1.0, 2.0]; // 3 values, expects 2
        let result = heatmap(&z, 2, 3).x(&x).build();
        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "x",
                y_field: "grid_width",
                x_len: 3,
                y_len: 2,
            })
        ));
    }

    #[test]
    fn test_heatmap_y_length_mismatch() {
        let z = vec![1.0; 6]; // 2x3 grid
        let y = vec![0.0, 1.0]; // 2 values, expects 3
        let result = heatmap(&z, 2, 3).y(&y).build();
        assert!(matches!(
            result,
            Err(ChartError::DataLengthMismatch {
                x_field: "y",
                y_field: "grid_height",
                x_len: 2,
                y_len: 3,
            })
        ));
    }

    #[test]
    fn test_heatmap_non_monotonic_x() {
        let z = vec![1.0; 4]; // 2x2 grid
        let x = vec![1.0, 0.0]; // not monotonic
        let result = heatmap(&z, 2, 2).x(&x).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "must be strictly monotonically increasing"
            })
        ));
    }

    #[test]
    fn test_heatmap_log_scale_negative() {
        let z = vec![1.0; 4]; // 2x2 grid
        let x = vec![-1.0, 1.0]; // negative values
        let result = heatmap(&z, 2, 2).x(&x).x_scale(ScaleType::Log).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_heatmap_successful_build() {
        let z = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 2x3 grid
        assert_heatmap_builds(
            heatmap(&z, 2, 3)
                .title("Test Heatmap")
                .color_scale(ColorScale::Viridis),
        );
    }

    #[test]
    fn test_heatmap_with_custom_axes() {
        let z = vec![1.0; 6]; // 2x3 grid
        let x = vec![10.0, 100.0];
        let y = vec![0.0, 1.0, 2.0];
        assert_heatmap_builds(heatmap(&z, 2, 3).x(&x).y(&y));
    }

    #[test]
    fn test_heatmap_log_scale() {
        let z = vec![1.0; 4]; // 2x2 grid
        let x = vec![10.0, 100.0];
        let y = vec![1.0, 10.0];
        assert_heatmap_builds(
            heatmap(&z, 2, 2)
                .x(&x)
                .y(&y)
                .x_scale(ScaleType::Log)
                .y_scale(ScaleType::Log),
        );
    }

    #[test]
    fn test_heatmap_log_scale_auto_axes() {
        let z = vec![1.0; 4]; // 2x2 grid
        let result = heatmap(&z, 2, 2).x_scale(ScaleType::Log).build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_heatmap_builder_chain() {
        let z = vec![1.0; 9]; // 3x3 grid
        assert_heatmap_builds(
            heatmap(&z, 3, 3)
                .title("My Heatmap")
                .color_scale(ColorScale::Plasma)
                .opacity(0.8)
                .size(800.0, 600.0),
        );
    }

    #[test]
    fn test_heatmap_with_explicit_ranges() {
        let z = vec![1.0; 9]; // 3x3 grid
        assert_heatmap_builds(heatmap(&z, 3, 3).x_range(0.0, 10.0).y_range(-5.0, 5.0));
    }

    #[test]
    fn test_heatmap_range_reversal_rejected() {
        let z = vec![1.0; 9];
        let result = heatmap(&z, 3, 3).x_range(10.0, 0.0).build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_heatmap_log_scale_negative_range_rejected() {
        let z = vec![1.0; 9];
        let x = vec![1.0, 2.0, 3.0];
        let result = heatmap(&z, 3, 3)
            .x(&x)
            .x_scale(ScaleType::Log)
            .x_range(-1.0, 10.0)
            .build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_heatmap_responsive_size_defaults_and_fixed_opt_in() {
        let z = vec![1.0; 9];

        crate::assert_default_chart_size(heatmap(&z, 3, 3).chart_size);
        crate::assert_fixed_chart_size(
            heatmap(&z, 3, 3).size(360.0, 240.0).chart_size,
            360.0,
            240.0,
        );
        crate::assert_fill_chart_size(
            heatmap(&z, 3, 3)
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

    #[test]
    fn test_heatmap_invalid_dimensions() {
        let z = vec![1.0; 9];
        let result = heatmap(&z, 3, 3).size(0.0, 400.0).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidDimension {
                field: "width",
                value: 0.0
            })
        ));
    }

    #[test]
    fn test_heatmap_nan_in_z() {
        let z = vec![1.0, f64::NAN, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
        let result = heatmap(&z, 3, 3).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "z",
                reason: "contains NaN or Infinity"
            })
        ));
    }

    #[test]
    fn test_heatmap_y_not_monotonic() {
        let z = vec![1.0; 6];
        let y = vec![2.0, 1.0, 0.0];
        let result = heatmap(&z, 2, 3).y(&y).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "y",
                reason: "must be strictly monotonically increasing"
            })
        ));
    }

    #[test]
    fn test_heatmap_x_log_negative() {
        let z = vec![1.0; 4];
        let x = vec![-1.0, 1.0];
        let result = heatmap(&z, 2, 2).x(&x).x_scale(ScaleType::Log).build();
        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "contains non-positive values for log scale"
            })
        ));
    }

    #[test]
    fn test_heatmap_log_scale_requires_explicit_axes() {
        let z = vec![1.0; 9];
        let result = heatmap(&z, 3, 3).y_scale(ScaleType::Log).build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_heatmap_opacity_clamping() {
        let z = vec![1.0; 9];
        let chart = heatmap(&z, 3, 3).opacity(2.0);
        assert_eq!(chart.opacity, 1.0);
        let chart = heatmap(&z, 3, 3).opacity(-1.0);
        assert_eq!(chart.opacity, 0.0);
    }

    #[test]
    fn test_heatmap_all_scale_combinations_build() {
        let z = vec![1.0; 9];
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![1.0, 2.0, 3.0];
        for (x_scale, y_scale) in [
            (ScaleType::Linear, ScaleType::Linear),
            (ScaleType::Log, ScaleType::Linear),
            (ScaleType::Linear, ScaleType::Log),
            (ScaleType::Log, ScaleType::Log),
        ] {
            let result = heatmap(&z, 3, 3)
                .x(&x)
                .y(&y)
                .x_scale(x_scale)
                .y_scale(y_scale)
                .build();
            assert!(result.is_ok(), "failed for x={x_scale:?}, y={y_scale:?}");
        }
    }
}
