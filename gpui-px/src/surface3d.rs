//! 3D Surface chart.

use crate::error::ChartError;
use crate::{
    DEFAULT_HEIGHT, DEFAULT_TITLE_FONT_SIZE, DEFAULT_WIDTH, TITLE_AREA_HEIGHT, validate_data_array,
    validate_dimensions, validate_grid_dimensions, validate_monotonic, validate_positive,
};
use d3rs::gpu3d::{Colormap, Surface3DConfig, Surface3DElement, Surface3DState, SurfaceData};
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::{IntoElement, div, hsla, px};
use std::cell::RefCell;
use std::rc::Rc;

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
    x_log: bool,
    y_log: bool,
    z_min: Option<f64>,
    z_max: Option<f64>,
    x_label: Option<String>,
    y_label: Option<String>,
    z_label: Option<String>,
    /// External state for camera/interaction control
    external_state: Option<Rc<RefCell<Surface3DState>>>,
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
                if self.x_log {
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
                if self.y_log {
                    validate_positive(v, "y")?;
                }
                v.clone()
            }
            None => (0..self.grid_height).map(|i| i as f64).collect(),
        };

        // Reshape z into Vec<Vec<f64>>
        // z is row-major (y varies slowly, x varies quickly)
        let mut z_grid = Vec::with_capacity(self.grid_height);
        for y_idx in 0..self.grid_height {
            let start = y_idx * self.grid_width;
            let end = start + self.grid_width;
            z_grid.push(self.z[start..end].to_vec());
        }

        // Calculate plot area (reserve space for title if present)
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };
        let plot_height = self.height - title_height;

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
        let config = Surface3DConfig::new()
            .colormap(self.colormap)
            .wireframe(self.wireframe);

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

        // Add surface element with optional external state
        let element = Surface3DElement::new(data, config);
        let element = if let Some(state) = self.external_state {
            element.with_state(state)
        } else {
            element
        };

        container = container.child(
            div()
                .w(px(self.width))
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
        x_log: false,
        y_log: false,
        z_min: None,
        z_max: None,
        x_label: None,
        y_label: None,
        z_label: None,
        external_state: None,
    }
}
