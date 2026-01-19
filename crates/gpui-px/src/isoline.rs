//! Isoline chart (unfilled contour lines) - Plotly Express style API.

use crate::error::ChartError;
use crate::{
    DEFAULT_COLOR, DEFAULT_HEIGHT, DEFAULT_TITLE_FONT_SIZE, DEFAULT_WIDTH, ScaleType,
    TITLE_AREA_HEIGHT, extent_padded, validate_data_array, validate_dimensions,
    validate_grid_dimensions, validate_monotonic, validate_positive,
};
use d3rs::axis::{AxisConfig, DefaultAxisTheme, render_axis};
use d3rs::color::D3Color;
use d3rs::contour::ContourGenerator;
use d3rs::grid::{GridConfig, render_grid};
use d3rs::scale::{LinearScale, LogScale};
use d3rs::shape::{ContourConfig, render_contour};
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, div, hsla, px, rgb};

/// Isoline chart builder (unfilled contour lines).
#[derive(Debug, Clone)]
pub struct IsolineChart {
    z: Vec<f64>,
    grid_width: usize,
    grid_height: usize,
    x_values: Option<Vec<f64>>,
    y_values: Option<Vec<f64>>,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
    levels: Option<Vec<f64>>,
    color: u32,
    stroke_width: f32,
    opacity: f32,
    title: Option<String>,
    width: f32,
    height: f32,
    // Axis range overrides (for zoom support)
    x_range: Option<[f64; 2]>,
    y_range: Option<[f64; 2]>,
}

impl IsolineChart {
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

    /// Set level values for isolines.
    ///
    /// Each level generates one contour line.
    /// If not set, auto-generates 10 evenly spaced levels.
    pub fn levels(mut self, levels: Vec<f64>) -> Self {
        self.levels = Some(levels);
        self
    }

    /// Set line color as 24-bit RGB hex value (format: 0xRRGGBB).
    pub fn color(mut self, hex: u32) -> Self {
        self.color = hex;
        self
    }

    /// Set line stroke width in pixels.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }

    /// Set line opacity (0.0 - 1.0).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set chart dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
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
        // Validate inputs
        validate_data_array(&self.z, "z")?;
        validate_grid_dimensions(&self.z, self.grid_width, self.grid_height)?;
        validate_dimensions(self.width, self.height)?;

        // Generate or validate x values
        let x_values = match self.x_values {
            Some(ref v) => {
                if v.len() != self.grid_width {
                    return Err(ChartError::DataLengthMismatch {
                        x_field: "x",
                        y_field: "grid_width",
                        x_len: v.len(),
                        y_len: self.grid_width,
                    });
                }
                validate_data_array(v, "x")?;
                validate_monotonic(v, "x")?;
                if self.x_scale_type == ScaleType::Log {
                    validate_positive(v, "x")?;
                }
                v.clone()
            }
            None => (0..self.grid_width).map(|i| i as f64).collect(),
        };

        // Generate or validate y values
        let y_values = match self.y_values {
            Some(ref v) => {
                if v.len() != self.grid_height {
                    return Err(ChartError::DataLengthMismatch {
                        x_field: "y",
                        y_field: "grid_height",
                        x_len: v.len(),
                        y_len: self.grid_height,
                    });
                }
                validate_data_array(v, "y")?;
                validate_monotonic(v, "y")?;
                if self.y_scale_type == ScaleType::Log {
                    validate_positive(v, "y")?;
                }
                v.clone()
            }
            None => (0..self.grid_height).map(|i| i as f64).collect(),
        };

        // Calculate plot area (reserve space for title and axes)
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };

        // Reserve space for axes
        let left_margin = 60.0_f64;
        let bottom_margin = 40.0_f64;
        let plot_width = (self.width as f64) - left_margin;
        let plot_height = (self.height as f64) - title_height as f64 - bottom_margin;

        let theme = DefaultAxisTheme;

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

        // Calculate z extent for auto-levels
        let (z_min, z_max) = extent_padded(&self.z, 0.0);

        // Generate levels if not provided
        let levels = match self.levels {
            Some(l) => l,
            None => {
                // Auto-generate 10 evenly spaced levels
                let n = 10;
                (0..=n)
                    .map(|i| z_min + (z_max - z_min) * (i as f64) / (n as f64))
                    .collect()
            }
        };

        // Generate contours
        let generator = ContourGenerator::new(self.grid_width, self.grid_height)
            .x_values(x_values)
            .y_values(y_values);
        let contours = generator.contours(&self.z, &levels);

        // Build config with fixed color (no fill for isolines)
        let config = ContourConfig::new()
            .fill(false)
            .stroke_color(D3Color::from_hex(self.color))
            .stroke_width(self.stroke_width)
            .stroke_opacity(self.opacity);

        // Build the element based on scale types
        let isoline_element: AnyElement = match (self.x_scale_type, self.y_scale_type) {
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
                        &AxisConfig::left(),
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
                                        &GridConfig::default(),
                                        plot_width as f32,
                                        plot_height as f32,
                                        &theme,
                                    ))
                                    .child(div().absolute().inset_0().child(render_contour(
                                        contours, &x_scale, &y_scale, &config,
                                    ))),
                            )
                            .child(render_axis(
                                &x_scale,
                                &AxisConfig::bottom(),
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
                        &AxisConfig::left(),
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
                                        &GridConfig::default(),
                                        plot_width as f32,
                                        plot_height as f32,
                                        &theme,
                                    ))
                                    .child(div().absolute().inset_0().child(render_contour(
                                        contours, &x_scale, &y_scale, &config,
                                    ))),
                            )
                            .child(render_axis(
                                &x_scale,
                                &AxisConfig::bottom(),
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
                        &AxisConfig::left(),
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
                                        &GridConfig::default(),
                                        plot_width as f32,
                                        plot_height as f32,
                                        &theme,
                                    ))
                                    .child(div().absolute().inset_0().child(render_contour(
                                        contours, &x_scale, &y_scale, &config,
                                    ))),
                            )
                            .child(render_axis(
                                &x_scale,
                                &AxisConfig::bottom(),
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
                        &AxisConfig::left(),
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
                                        &GridConfig::default(),
                                        plot_width as f32,
                                        plot_height as f32,
                                        &theme,
                                    ))
                                    .child(div().absolute().inset_0().child(render_contour(
                                        contours, &x_scale, &y_scale, &config,
                                    ))),
                            )
                            .child(render_axis(
                                &x_scale,
                                &AxisConfig::bottom(),
                                plot_width as f32,
                                &theme,
                            )),
                    )
                    .into_any_element()
            }
        };

        // Build container with optional title
        let mut container = div()
            .w(px(self.width))
            .h(px(self.height))
            .relative()
            .flex()
            .flex_col();

        // Add title if present
        if let Some(title) = &self.title {
            let font_config =
                VectorFontConfig::horizontal(DEFAULT_TITLE_FONT_SIZE, hsla(0.0, 0.0, 0.2, 1.0));
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

        // Add plot area with axes
        container = container.child(isoline_element);

        Ok(container)
    }
}

/// Create an isoline chart (unfilled contour lines) from z data with grid dimensions.
///
/// Data is in row-major order: `z[row * width + col]` where row 0 is at the bottom.
///
/// # Example
///
/// ```rust,no_run
/// use gpui_px::isoline;
///
/// // 3x3 grid
/// let z = vec![
///     1.0, 2.0, 3.0,  // row 0 (bottom)
///     4.0, 5.0, 6.0,  // row 1
///     7.0, 8.0, 9.0,  // row 2 (top)
/// ];
///
/// let chart = isoline(&z, 3, 3)
///     .title("My Isolines")
///     .levels(vec![2.0, 4.0, 6.0, 8.0])
///     .color(0x333333)
///     .stroke_width(1.5)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn isoline(z: &[f64], grid_width: usize, grid_height: usize) -> IsolineChart {
    IsolineChart {
        z: z.to_vec(),
        grid_width,
        grid_height,
        x_values: None,
        y_values: None,
        x_scale_type: ScaleType::Linear,
        y_scale_type: ScaleType::Linear,
        levels: None,
        color: DEFAULT_COLOR,
        stroke_width: 1.5,
        opacity: 1.0,
        title: None,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        x_range: None,
        y_range: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isoline_empty_z() {
        let result = isoline(&[], 0, 0).build();
        assert!(matches!(result, Err(ChartError::EmptyData { field: "z" })));
    }

    #[test]
    fn test_isoline_grid_mismatch() {
        let z = vec![1.0, 2.0, 3.0, 4.0, 5.0]; // 5 values
        let result = isoline(&z, 2, 3).build(); // expects 6
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
    fn test_isoline_successful_build() {
        let z = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]; // 3x3 grid
        let result = isoline(&z, 3, 3)
            .title("Test Isolines")
            .color(0x333333)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_isoline_with_custom_levels() {
        let z = vec![1.0; 9]; // 3x3 grid
        let result = isoline(&z, 3, 3).levels(vec![0.5, 1.0, 1.5]).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_isoline_with_custom_axes() {
        let z = vec![1.0; 6]; // 2x3 grid
        let x = vec![10.0, 100.0];
        let y = vec![0.0, 1.0, 2.0];
        let result = isoline(&z, 2, 3).x(&x).y(&y).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_isoline_log_scale() {
        let z = vec![1.0; 4]; // 2x2 grid
        let x = vec![10.0, 100.0];
        let y = vec![1.0, 10.0];
        let result = isoline(&z, 2, 2)
            .x(&x)
            .y(&y)
            .x_scale(ScaleType::Log)
            .y_scale(ScaleType::Log)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_isoline_builder_chain() {
        let z = vec![1.0; 9]; // 3x3 grid
        let result = isoline(&z, 3, 3)
            .title("My Isolines")
            .color(0xff0000)
            .stroke_width(2.0)
            .opacity(0.8)
            .levels(vec![0.5, 1.0, 1.5])
            .size(800.0, 600.0)
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_isoline_with_explicit_ranges() {
        let z = vec![1.0; 9]; // 3x3 grid
        let result = isoline(&z, 3, 3)
            .x_range(0.0, 10.0)
            .y_range(-5.0, 5.0)
            .build();
        assert!(result.is_ok());
    }
}
