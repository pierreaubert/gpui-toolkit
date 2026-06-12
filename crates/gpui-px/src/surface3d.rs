//! 3D Surface chart.

use crate::error::ChartError;
use crate::{
    ChartSize, DEFAULT_HEIGHT, DEFAULT_TITLE_FONT_SIZE, DEFAULT_WIDTH, TITLE_AREA_HEIGHT,
    apply_chart_size, default_design, resolved_chart_dimensions, validate_data_array,
    validate_dimensions, validate_grid_dimensions, validate_monotonic, validate_positive,
};
use d3rs::gpu3d::{Colormap, Surface3DConfig, Surface3DElement, Surface3DState, SurfaceData};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{IntoElement, div, hsla, px};
use gpui_design::DesignSystem;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

/// Surface 3D chart builder.
#[derive(Clone)]
pub struct Surface3DChart {
    z: Vec<f64>,
    grid_width: usize,
    grid_height: usize,
    x_values: Option<Vec<f64>>,
    y_values: Option<Vec<f64>>,
    title: Option<String>,
    colormap: Colormap,
    wireframe: bool,
    width: f32,
    height: f32,
    chart_size: ChartSize,
    x_log: bool,
    y_log: bool,
    z_min: Option<f64>,
    z_max: Option<f64>,
    x_label: Option<String>,
    y_label: Option<String>,
    z_label: Option<String>,
    /// External state for camera/interaction control
    external_state: Option<Rc<RefCell<Surface3DState>>>,
    design: Option<Arc<DesignSystem>>,
}

impl std::fmt::Debug for Surface3DChart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Surface3DChart")
            .field("grid_width", &self.grid_width)
            .field("grid_height", &self.grid_height)
            .field("colormap", &self.colormap)
            .field("title", &self.title)
            .field("wireframe", &self.wireframe)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl Surface3DChart {
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

    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set colormap.
    pub fn colormap(mut self, colormap: Colormap) -> Self {
        self.colormap = colormap;
        self
    }

    /// Enable wireframe mode.
    pub fn wireframe(mut self, wireframe: bool) -> Self {
        self.wireframe = wireframe;
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

    /// Set logarithmic X-axis.
    pub fn x_log(mut self, log: bool) -> Self {
        self.x_log = log;
        self
    }

    /// Set logarithmic Y-axis.
    pub fn y_log(mut self, log: bool) -> Self {
        self.y_log = log;
        self
    }

    /// Set Z-axis range manually.
    pub fn z_range(mut self, min: f64, max: f64) -> Self {
        self.z_min = Some(min);
        self.z_max = Some(max);
        self
    }

    /// Set X-axis label.
    pub fn x_label(mut self, label: impl Into<String>) -> Self {
        self.x_label = Some(label.into());
        self
    }

    /// Set Y-axis label.
    pub fn y_label(mut self, label: impl Into<String>) -> Self {
        self.y_label = Some(label.into());
        self
    }

    /// Set Z-axis label.
    pub fn z_label(mut self, label: impl Into<String>) -> Self {
        self.z_label = Some(label.into());
        self
    }

    /// Set external state for camera/interaction control.
    ///
    /// When external state is provided, mouse interaction handlers on the parent
    /// view can update this state to control camera rotation, zoom, and pan.
    pub fn with_state(mut self, state: Rc<RefCell<Surface3DState>>) -> Self {
        self.external_state = Some(state);
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
                if self.x_log {
                    validate_positive(v, "x")?;
                }
                v.clone()
            }
            None => {
                if self.x_log {
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
                if self.y_log {
                    validate_positive(v, "y")?;
                }
                v.clone()
            }
            None => {
                if self.y_log {
                    return Err(ChartError::InvalidData {
                        field: "y",
                        reason: "log scale requires explicit positive y values",
                    });
                }
                (0..self.grid_height).map(|i| i as f64).collect()
            }
        };

        // Reshape z into Vec<Vec<f64>>
        // z is row-major (y varies slowly, x varies quickly)
        let mut z_grid = Vec::with_capacity(self.grid_height);
        let mut z = self.z;
        for _ in 0..self.grid_height {
            let row: Vec<f64> = z.drain(..self.grid_width).collect();
            z_grid.push(row);
        }

        // Calculate plot area (reserve space for title if present)
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };
        let plot_height = layout_height - title_height;

        // Create SurfaceData
        let mut data = SurfaceData::from_grid(x_values, y_values, z_grid);

        // Apply configurations to data
        if let Some(label) = self.x_label {
            data = data.with_x_label(label);
        }
        if let Some(label) = self.y_label {
            data = data.with_y_label(label);
        }
        if let Some(label) = self.z_label {
            data = data.with_z_label(label);
        }
        data = data.with_log_x(self.x_log).with_log_y(self.y_log);
        if let (Some(min), Some(max)) = (self.z_min, self.z_max) {
            data = data.with_z_range(min, max);
        }

        // Create Surface3DConfig
        let config = Surface3DConfig::from_design(&design)
            .colormap(self.colormap)
            .wireframe(self.wireframe);

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

        // Add surface element with optional external state
        let element = Surface3DElement::new(data, config);
        let element = if let Some(state) = self.external_state {
            element.with_state(state)
        } else {
            element
        };

        container = container.child(
            div()
                .w(px(layout_width))
                .h(px(plot_height))
                .relative()
                .child(element),
        );

        Ok(container)
    }
}

/// Create a 3D surface chart from z data with grid dimensions.
///
/// Data is in row-major order: `z[row * width + col]` where row 0 is at the bottom.
///
/// # Example
///
/// ```rust,ignore
/// use gpui_px::surface3d;
/// use d3rs::surface3d::Colormap;
///
/// // 3x3 grid
/// let z = vec![
///     1.0, 2.0, 3.0,  // row 0 (bottom)
///     4.0, 5.0, 6.0,  // row 1
///     7.0, 8.0, 9.0,  // row 2 (top)
/// ];
///
/// let chart = surface3d(&z, 3, 3)
///     .title("My Surface")
///     .colormap(Colormap::Viridis)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn surface3d(z: &[f64], grid_width: usize, grid_height: usize) -> Surface3DChart {
    Surface3DChart {
        z: z.to_vec(),
        grid_width,
        grid_height,
        x_values: None,
        y_values: None,
        title: None,
        colormap: Colormap::Viridis,
        wireframe: false,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        chart_size: ChartSize::default(),
        x_log: false,
        y_log: false,
        z_min: None,
        z_max: None,
        x_label: None,
        y_label: None,
        z_label: None,
        external_state: None,
        design: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface3d_builds() {
        let z = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let result = surface3d(&z, 3, 3).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_surface3d_with_custom_axes() {
        let z = vec![1.0, 2.0, 3.0, 4.0];
        let x = vec![0.0, 1.0];
        let y = vec![0.0, 1.0];
        let result = surface3d(&z, 2, 2).x(&x).y(&y).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_surface3d_with_unicode_labels() {
        let z = vec![1.0, 2.0, 3.0, 4.0];
        let result = surface3d(&z, 2, 2)
            .title("Cafe\u{301} \u{00b1}3 dB")
            .x_label("Elevation (\u{00b0})")
            .y_label("\u{65e5}\u{672c}\u{8a9e}")
            .z_label("\u{3bc}Pa")
            .build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_surface3d_responsive_size_defaults_and_fixed_opt_in() {
        let z = vec![1.0, 2.0, 3.0, 4.0];

        crate::assert_default_chart_size(surface3d(&z, 2, 2).chart_size);
        crate::assert_fixed_chart_size(
            surface3d(&z, 2, 2).size(420.0, 320.0).chart_size,
            420.0,
            320.0,
        );
        crate::assert_fill_chart_size(
            surface3d(&z, 2, 2)
                .size(420.0, 320.0)
                .fill()
                .min_size(360.0, 260.0)
                .aspect_ratio(1.3)
                .chart_size,
            360.0,
            260.0,
            Some(1.3),
        );
    }
}
