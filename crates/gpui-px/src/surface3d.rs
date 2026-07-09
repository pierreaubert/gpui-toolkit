//! 3D Surface chart.

use crate::error::ChartError;
use crate::{
    ChartAccessibilitySummary, ChartSize, DEFAULT_HEIGHT, DEFAULT_TITLE_FONT_SIZE, DEFAULT_WIDTH,
    ScaleType, TITLE_AREA_HEIGHT, apply_chart_size, default_design, finite_range, format_range,
    resolved_chart_dimensions, validate_data_array, validate_dimensions, validate_grid_dimensions,
    validate_monotonic, validate_positive,
};
use d3rs::gpu3d::{Colormap, Surface3DConfig, Surface3DElement, Surface3DState, SurfaceData};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{IntoElement, div, hsla, px};
use gpui_design::DesignSystem;
use std::cell::RefCell;
use std::fmt::Write;
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
    /// Export this surface chart as deterministic SVG using the chart's configured size.
    ///
    /// The SVG export uses a stable projected mesh instead of GPU readback, making it suitable
    /// for CI release artifacts and documentation snapshots.
    pub fn to_svg(&self) -> Result<String, ChartError> {
        self.to_svg_with_options(crate::StaticSvgOptions::new(self.width, self.height))
    }

    /// Export this surface chart as deterministic SVG with explicit viewport options.
    pub fn to_svg_with_options(
        &self,
        options: crate::StaticSvgOptions,
    ) -> Result<String, ChartError> {
        validate_data_array(&self.z, "z")?;
        validate_grid_dimensions(&self.z, self.grid_width, self.grid_height)?;
        validate_dimensions(options.width, options.height)?;
        crate::static_export::validate_plot_area(options)?;

        let x_values = self.resolve_static_axis_values(
            self.x_values.as_deref(),
            self.grid_width,
            self.x_log,
            "x",
        )?;
        let y_values = self.resolve_static_axis_values(
            self.y_values.as_deref(),
            self.grid_height,
            self.y_log,
            "y",
        )?;
        let z_domain = match (self.z_min, self.z_max) {
            (Some(min), Some(max)) => {
                crate::validate_range(min, max, "z_range")?;
                (min, max)
            }
            _ => {
                let [min, max] = finite_range(self.z.iter()).unwrap_or([0.0, 1.0]);
                (min, max)
            }
        };

        let plot_left = options.margin_left;
        let plot_top = options.margin_top;
        let plot_right = options.width - options.margin_right;
        let plot_bottom = options.height - options.margin_bottom;
        if plot_right <= plot_left || plot_bottom <= plot_top {
            return Err(ChartError::InvalidDimension {
                field: "width",
                value: options.width,
            });
        }

        let layout = StaticSurfaceLayout {
            left: plot_left,
            top: plot_top,
            right: plot_right,
            bottom: plot_bottom,
        };
        let x_domain = (*x_values.first().unwrap(), *x_values.last().unwrap());
        let y_domain = (*y_values.first().unwrap(), *y_values.last().unwrap());

        let mut svg = crate::static_export::svg_header(options);
        crate::static_export::draw_title(&mut svg, self.title.as_deref(), options.width);
        draw_static_surface_axes(
            &mut svg,
            options,
            layout,
            x_domain,
            y_domain,
            z_domain,
            (
                self.x_label.as_deref(),
                self.y_label.as_deref(),
                self.z_label.as_deref(),
            ),
        );

        svg.push_str("<g class=\"gpui-px-surface3d\">\n");
        if self.grid_width > 1 && self.grid_height > 1 {
            for yi in 0..(self.grid_height - 1) {
                for xi in 0..(self.grid_width - 1) {
                    let corners = [(xi, yi), (xi + 1, yi), (xi + 1, yi + 1), (xi, yi + 1)];
                    let mut points = String::new();
                    let mut value_sum = 0.0;
                    for (corner_x, corner_y) in corners {
                        let z = self.z[corner_y * self.grid_width + corner_x];
                        value_sum += z;
                        let (x, y) = project_static_surface_point(
                            x_values[corner_x],
                            y_values[corner_y],
                            z,
                            x_domain,
                            y_domain,
                            z_domain,
                            layout,
                            self.x_log,
                            self.y_log,
                        );
                        let _ = write!(points, "{x:.2},{y:.2} ");
                    }
                    let normalized = normalize_static_surface_z(value_sum / 4.0, z_domain);
                    let color = static_surface_colormap_hex(self.colormap, normalized);
                    let _ = writeln!(
                        svg,
                        "<polygon points=\"{}\" fill=\"{}\" stroke=\"#ffffff\" stroke-width=\"0.5\"><title>{:.3}</title></polygon>",
                        points.trim_end(),
                        color,
                        value_sum / 4.0
                    );
                }
            }
        } else {
            for (index, z) in self.z.iter().copied().enumerate() {
                let xi = index % self.grid_width;
                let yi = index / self.grid_width;
                let (x, y) = project_static_surface_point(
                    x_values[xi],
                    y_values[yi],
                    z,
                    x_domain,
                    y_domain,
                    z_domain,
                    layout,
                    self.x_log,
                    self.y_log,
                );
                let color = static_surface_colormap_hex(
                    self.colormap,
                    normalize_static_surface_z(z, z_domain),
                );
                let _ = writeln!(
                    svg,
                    "<circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"3\" fill=\"{color}\"><title>{z:.3}</title></circle>"
                );
            }
        }
        svg.push_str("</g>\n");

        if self.wireframe {
            draw_static_surface_wireframe(
                &mut svg,
                layout,
                &x_values,
                &y_values,
                &self.z,
                self.grid_width,
                self.grid_height,
                x_domain,
                y_domain,
                z_domain,
                self.x_log,
                self.y_log,
            );
        }
        draw_static_surface_colorbar(&mut svg, layout, self.colormap, z_domain);
        svg.push_str("</svg>\n");
        Ok(svg)
    }

    /// Return structured accessibility metadata for this chart.
    pub fn accessibility_summary(&self) -> ChartAccessibilitySummary {
        let x_range = self
            .x_values
            .as_ref()
            .and_then(|values| finite_range(values.iter()))
            .or_else(|| (self.grid_width > 0).then_some([0.0, (self.grid_width - 1) as f64]));
        let y_range = self
            .y_values
            .as_ref()
            .and_then(|values| finite_range(values.iter()))
            .or_else(|| (self.grid_height > 0).then_some([0.0, (self.grid_height - 1) as f64]));
        let value_range = finite_range(self.z.iter());
        let title = self.title.clone();
        let name = title.as_deref().unwrap_or("3D surface chart");
        let z_display_range = match (self.z_min, self.z_max) {
            (Some(min), Some(max)) => format!(" Display Z range {min:.3} to {max:.3}."),
            _ => String::new(),
        };
        let wireframe = if self.wireframe {
            " Wireframe rendering is enabled."
        } else {
            ""
        };
        let description = format!(
            "{name}: 3D surface chart with a {} by {} grid and {} samples. {}, {}, {}. X scale {}, Y scale {}.{z_display_range}{wireframe}",
            self.grid_width,
            self.grid_height,
            self.z.len(),
            format_range("X", x_range),
            format_range("Y", y_range),
            format_range("Z", value_range),
            if self.x_log { "log" } else { "linear" },
            if self.y_log { "log" } else { "linear" }
        );

        ChartAccessibilitySummary {
            chart_type: "surface3d",
            title,
            series_count: 1,
            datum_count: self.z.len(),
            x_range,
            y_range,
            value_range,
            x_scale: Some(if self.x_log {
                ScaleType::Log
            } else {
                ScaleType::Linear
            }),
            y_scale: Some(if self.y_log {
                ScaleType::Log
            } else {
                ScaleType::Linear
            }),
            series_labels: vec!["Surface values".to_string()],
            description,
        }
    }

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

    fn resolve_static_axis_values(
        &self,
        values: Option<&[f64]>,
        expected_len: usize,
        is_log: bool,
        field: &'static str,
    ) -> Result<Vec<f64>, ChartError> {
        match values {
            Some(values) => {
                if values.len() != expected_len {
                    return Err(ChartError::DataLengthMismatch {
                        x_field: field,
                        y_field: if field == "x" {
                            "grid_width"
                        } else {
                            "grid_height"
                        },
                        x_len: values.len(),
                        y_len: expected_len,
                    });
                }
                validate_data_array(values, field)?;
                validate_monotonic(values, field)?;
                if is_log {
                    validate_positive(values, field)?;
                }
                Ok(values.to_vec())
            }
            None => {
                if is_log {
                    return Err(ChartError::InvalidData {
                        field,
                        reason: if field == "x" {
                            "log scale requires explicit positive x values"
                        } else {
                            "log scale requires explicit positive y values"
                        },
                    });
                }
                Ok((0..expected_len).map(|index| index as f64).collect())
            }
        }
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
        // instead of cloning; only allocate when falling back to defaults.
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
                if self.x_log {
                    validate_positive(&v, "x")?;
                }
                v
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
                if self.y_log {
                    validate_positive(&v, "y")?;
                }
                v
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

#[derive(Debug, Clone, Copy)]
struct StaticSurfaceLayout {
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}

impl StaticSurfaceLayout {
    fn width(self) -> f32 {
        self.right - self.left
    }

    fn height(self) -> f32 {
        self.bottom - self.top
    }
}

fn draw_static_surface_axes(
    svg: &mut String,
    options: crate::StaticSvgOptions,
    layout: StaticSurfaceLayout,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
    z_domain: (f64, f64),
    labels: (Option<&str>, Option<&str>, Option<&str>),
) {
    if !options.show_axes {
        return;
    }

    let base_z = z_domain.0;
    let origin = project_static_surface_point(
        x_domain.0, y_domain.0, base_z, x_domain, y_domain, z_domain, layout, false, false,
    );
    let x_end = project_static_surface_point(
        x_domain.1, y_domain.0, base_z, x_domain, y_domain, z_domain, layout, false, false,
    );
    let y_end = project_static_surface_point(
        x_domain.0, y_domain.1, base_z, x_domain, y_domain, z_domain, layout, false, false,
    );
    let z_end = project_static_surface_point(
        x_domain.0, y_domain.0, z_domain.1, x_domain, y_domain, z_domain, layout, false, false,
    );

    svg.push_str("<g class=\"gpui-px-surface3d-axes\" stroke=\"#555\" fill=\"#555\" font-family=\"system-ui, sans-serif\" font-size=\"10\">\n");
    for (start, end) in [(origin, x_end), (origin, y_end), (origin, z_end)] {
        let _ = writeln!(
            svg,
            "<line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke-width=\"1\"/>",
            start.0, start.1, end.0, end.1
        );
    }

    let x_label = labels.0.unwrap_or("x");
    let y_label = labels.1.unwrap_or("y");
    let z_label = labels.2.unwrap_or("z");
    for (label, end) in [(x_label, x_end), (y_label, y_end), (z_label, z_end)] {
        let _ = writeln!(
            svg,
            "<text x=\"{:.2}\" y=\"{:.2}\">{}</text>",
            end.0 + 6.0,
            end.1,
            crate::static_export::escape_xml(label)
        );
    }
    svg.push_str("</g>\n");
}

fn draw_static_surface_wireframe(
    svg: &mut String,
    layout: StaticSurfaceLayout,
    x_values: &[f64],
    y_values: &[f64],
    z_values: &[f64],
    grid_width: usize,
    grid_height: usize,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
    z_domain: (f64, f64),
    x_log: bool,
    y_log: bool,
) {
    svg.push_str("<g class=\"gpui-px-surface3d-wireframe\" fill=\"none\" stroke=\"#333\" stroke-width=\"0.75\" stroke-opacity=\"0.65\">\n");
    for yi in 0..grid_height {
        let mut points = String::new();
        for xi in 0..grid_width {
            let z = z_values[yi * grid_width + xi];
            let (x, y) = project_static_surface_point(
                x_values[xi],
                y_values[yi],
                z,
                x_domain,
                y_domain,
                z_domain,
                layout,
                x_log,
                y_log,
            );
            let _ = write!(points, "{x:.2},{y:.2} ");
        }
        let _ = writeln!(svg, "<polyline points=\"{}\"/>", points.trim_end());
    }
    for xi in 0..grid_width {
        let mut points = String::new();
        for yi in 0..grid_height {
            let z = z_values[yi * grid_width + xi];
            let (x, y) = project_static_surface_point(
                x_values[xi],
                y_values[yi],
                z,
                x_domain,
                y_domain,
                z_domain,
                layout,
                x_log,
                y_log,
            );
            let _ = write!(points, "{x:.2},{y:.2} ");
        }
        let _ = writeln!(svg, "<polyline points=\"{}\"/>", points.trim_end());
    }
    svg.push_str("</g>\n");
}

fn draw_static_surface_colorbar(
    svg: &mut String,
    layout: StaticSurfaceLayout,
    colormap: Colormap,
    z_domain: (f64, f64),
) {
    let bar_width = 10.0;
    let bar_height = layout.height().min(120.0);
    let x = layout.right - bar_width - 6.0;
    let y = layout.top + 8.0;
    svg.push_str("<g class=\"gpui-px-surface3d-colorbar\">\n");
    for step in 0..12 {
        let t0 = step as f32 / 12.0;
        let fill = static_surface_colormap_hex(colormap, 1.0 - t0);
        let rect_y = y + bar_height * t0;
        let rect_h = (bar_height / 12.0).ceil();
        let _ = writeln!(
            svg,
            "<rect x=\"{x:.2}\" y=\"{rect_y:.2}\" width=\"{bar_width:.2}\" height=\"{rect_h:.2}\" fill=\"{fill}\"/>"
        );
    }
    let _ = writeln!(
        svg,
        "<rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{bar_width:.2}\" height=\"{bar_height:.2}\" fill=\"none\" stroke=\"#555\" stroke-width=\"0.75\"/>"
    );
    let _ = writeln!(
        svg,
        "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"system-ui, sans-serif\" font-size=\"9\" fill=\"#555\">{:.3}</text>",
        x + 14.0,
        y + 8.0,
        z_domain.1
    );
    let _ = writeln!(
        svg,
        "<text x=\"{:.2}\" y=\"{:.2}\" font-family=\"system-ui, sans-serif\" font-size=\"9\" fill=\"#555\">{:.3}</text>",
        x + 14.0,
        y + bar_height,
        z_domain.0
    );
    svg.push_str("</g>\n");
}

fn project_static_surface_point(
    x: f64,
    y: f64,
    z: f64,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
    z_domain: (f64, f64),
    layout: StaticSurfaceLayout,
    x_log: bool,
    y_log: bool,
) -> (f32, f32) {
    let x = normalize_static_surface_axis(x, x_domain, x_log) * 2.0 - 1.0;
    let y = normalize_static_surface_axis(y, y_domain, y_log) * 2.0 - 1.0;
    let z = normalize_static_surface_z(z, z_domain);
    let center_x = (layout.left + layout.right) / 2.0;
    let origin_y = layout.top + layout.height() * 0.72;
    let scale_x = layout.width() * 0.30;
    let scale_y = layout.height() * 0.16;
    let scale_z = layout.height() * 0.32;
    (
        center_x + (x - y) * scale_x,
        origin_y + (x + y) * scale_y - z * scale_z,
    )
}

fn normalize_static_surface_axis(value: f64, domain: (f64, f64), is_log: bool) -> f32 {
    let (value, min, max) = if is_log {
        (value.log10(), domain.0.log10(), domain.1.log10())
    } else {
        (value, domain.0, domain.1)
    };
    if max == min {
        return 0.5;
    }
    ((value - min) / (max - min)).clamp(0.0, 1.0) as f32
}

fn normalize_static_surface_z(value: f64, domain: (f64, f64)) -> f32 {
    if domain.1 == domain.0 {
        return 0.5;
    }
    ((value - domain.0) / (domain.1 - domain.0)).clamp(0.0, 1.0) as f32
}

fn static_surface_colormap_hex(colormap: Colormap, t: f32) -> String {
    let (r, g, b) = colormap.color_at(t);
    format!(
        "#{:02x}{:02x}{:02x}",
        static_surface_channel_to_u8(r),
        static_surface_channel_to_u8(g),
        static_surface_channel_to_u8(b)
    )
}

fn static_surface_channel_to_u8(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
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

    #[test]
    fn surface3d_static_export_writes_projected_mesh_title_and_colorbar() {
        let z = vec![1.0, 2.0, 3.0, 4.0];
        let svg = surface3d(&z, 2, 2)
            .title("Surface")
            .x_label("Frequency")
            .y_label("Angle")
            .z_label("SPL")
            .wireframe(true)
            .to_svg_with_options(crate::StaticSvgOptions::new(420.0, 280.0))
            .expect("surface SVG export should succeed");

        assert!(svg.contains("<title>Surface</title>"));
        assert!(svg.contains("class=\"gpui-px-surface3d\""));
        assert!(svg.contains("class=\"gpui-px-surface3d-wireframe\""));
        assert!(svg.contains("class=\"gpui-px-surface3d-colorbar\""));
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("Frequency"));
        assert!(svg.contains("SPL"));
    }

    #[test]
    fn surface3d_static_export_supports_log_axes() {
        let z = vec![1.0, 2.0, 3.0, 4.0];
        let svg = surface3d(&z, 2, 2)
            .x(&[10.0, 1000.0])
            .y(&[1.0, 100.0])
            .x_log(true)
            .y_log(true)
            .to_svg()
            .expect("surface SVG export should support explicit log axes");

        assert!(svg.contains("gpui-px-surface3d"));
    }

    #[test]
    fn surface3d_static_export_preserves_log_axis_validation_errors() {
        let z = vec![1.0, 2.0, 3.0, 4.0];
        let result = surface3d(&z, 2, 2).x_log(true).to_svg();

        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "x",
                reason: "log scale requires explicit positive x values"
            })
        ));
    }

    #[test]
    fn surface3d_static_export_preserves_z_range_validation_errors() {
        let z = vec![1.0, 2.0, 3.0, 4.0];
        let result = surface3d(&z, 2, 2).z_range(4.0, 1.0).to_svg();

        assert!(matches!(
            result,
            Err(ChartError::InvalidData {
                field: "z_range",
                reason: "range min must be less than max"
            })
        ));
    }
}
