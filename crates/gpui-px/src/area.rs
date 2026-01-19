//! Area chart - Plotly Express style API.

use crate::error::ChartError;
use crate::{
    DEFAULT_COLOR, DEFAULT_HEIGHT, DEFAULT_PADDING_FRACTION, DEFAULT_TITLE_FONT_SIZE,
    DEFAULT_WIDTH, ScaleType, TITLE_AREA_HEIGHT, extent_padded, validate_data_array,
    validate_data_length, validate_dimensions, validate_positive,
};
use d3rs::color::D3Color;
use d3rs::scale::{LinearScale, LogScale, Scale};
use d3rs::shape::{Area, Curve};
use d3rs::text::{VectorFontConfig, render_vector_text};
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, PathBuilder, Rgba, canvas, div, hsla, px};
use std::sync::Arc;

/// Area chart builder.
#[derive(Clone)]
pub struct AreaChart {
    x: Vec<f64>,
    y: Vec<f64>,
    y0: Option<Vec<f64>>,
    title: Option<String>,
    color: u32,
    opacity: f32,
    curve: Curve,
    width: f32,
    height: f32,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
}

impl AreaChart {
    /// Set chart title (rendered at top of chart).
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set fill color as 24-bit RGB hex value (format: 0xRRGGBB).
    pub fn color(mut self, hex: u32) -> Self {
        self.color = hex;
        self
    }

    /// Set fill opacity (0.0 - 1.0).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set curve interpolation type.
    pub fn curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }

    /// Set chart dimensions.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
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

    /// Set baseline Y values (y0). Defaults to 0.0 if not specified.
    pub fn y0(mut self, y0: &[f64]) -> Self {
        self.y0 = Some(y0.to_vec());
        self
    }

    /// Build and validate the chart, returning renderable element.
    pub fn build(self) -> Result<impl IntoElement, ChartError> {
        // Validate inputs
        validate_data_array(&self.x, "x")?;
        validate_data_array(&self.y, "y")?;
        validate_data_length(self.x.len(), self.y.len(), "x", "y")?;
        validate_dimensions(self.width, self.height)?;

        if let Some(ref y0) = self.y0 {
            validate_data_array(y0, "y0")?;
            validate_data_length(self.x.len(), y0.len(), "x", "y0")?;
        }

        // Validate positive values for log scales
        if self.x_scale_type == ScaleType::Log {
            validate_positive(&self.x, "x")?;
        }
        if self.y_scale_type == ScaleType::Log {
            validate_positive(&self.y, "y")?;
            if let Some(ref y0) = self.y0 {
                validate_positive(y0, "y0")?;
            }
        }

        // Calculate plot area (reserve space for title if present)
        let title_height = if self.title.is_some() {
            TITLE_AREA_HEIGHT
        } else {
            0.0
        };
        let plot_height = self.height - title_height;

        // Calculate domains with padding
        let (x_min, x_max) = extent_padded(&self.x, DEFAULT_PADDING_FRACTION);

        // Calculate Y domain considering y and y0
        let y_iter = self.y.iter();
        let (y_min, y_max) = if let Some(ref y0) = self.y0 {
            let all_y: Vec<f64> = y_iter.chain(y0.iter()).copied().collect();
            extent_padded(&all_y, DEFAULT_PADDING_FRACTION)
        } else {
            let mut all_y: Vec<f64> = y_iter.copied().collect();
            all_y.push(0.0); // Include baseline 0
            extent_padded(&all_y, DEFAULT_PADDING_FRACTION)
        };

        // Prepare data for rendering
        struct AreaDatum {
            x: f64,
            y0: f64,
            y1: f64,
        }

        let data: Vec<AreaDatum> = match &self.y0 {
            Some(y0) => self
                .x
                .iter()
                .zip(self.y.iter())
                .zip(y0.iter())
                .map(|((&x, &y1), &y0)| AreaDatum { x, y0, y1 })
                .collect(),
            None => self
                .x
                .iter()
                .zip(self.y.iter())
                .map(|(&x, &y1)| AreaDatum { x, y0: 0.0, y1 })
                .collect(),
        };

        let color = D3Color::from_hex(self.color);
        let fill_color = color.to_rgba();
        let opacity = self.opacity;
        let curve = self.curve;

        // Create render function
        let render_element = move |x_scale: Arc<dyn Scale<f64, f64>>,
                                   y_scale: Arc<dyn Scale<f64, f64>>| {
            let x_scale_prepaint = x_scale.clone();
            let y_scale_prepaint = y_scale.clone();

            canvas(
                move |bounds, _, _| (x_scale_prepaint.clone(), y_scale_prepaint.clone(), bounds),
                move |_, (x_scale, y_scale, bounds), window, _| {
                    let x_scale_x = x_scale.clone();
                    let y_scale_y0 = y_scale.clone();
                    let y_scale_y1 = y_scale.clone();

                    let area = Area::new()
                        .x(move |d: &AreaDatum| x_scale_x.scale(d.x))
                        .y0(move |d: &AreaDatum| y_scale_y0.scale(d.y0))
                        .y1(move |d: &AreaDatum| y_scale_y1.scale(d.y1))
                        .curve(curve);

                    let path = area.generate(&data);
                    let points = path.flatten(0.5);

                    let origin_x: f32 = bounds.origin.x.into();
                    let origin_y: f32 = bounds.origin.y.into();

                    if points.is_empty() {
                        return;
                    }

                    let mut path_builder = PathBuilder::fill();

                    let first = points[0];
                    path_builder.move_to(gpui::point(
                        px(origin_x + first.x as f32),
                        px(origin_y + first.y as f32),
                    ));

                    for p in points.iter().skip(1) {
                        path_builder.line_to(gpui::point(
                            px(origin_x + p.x as f32),
                            px(origin_y + p.y as f32),
                        ));
                    }

                    path_builder.close();

                    if let Ok(gpui_path) = path_builder.build() {
                        window.paint_path(
                            gpui_path,
                            Rgba {
                                r: fill_color.r,
                                g: fill_color.g,
                                b: fill_color.b,
                                a: fill_color.a * opacity,
                            },
                        );
                    }
                },
            )
        };

        // Build the element based on scale types
        let area_element: AnyElement = match (self.x_scale_type, self.y_scale_type) {
            (ScaleType::Linear, ScaleType::Linear) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, self.width as f64);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height as f64, 0.0);
                render_element(Arc::new(x_scale), Arc::new(y_scale)).into_any_element()
            }
            (ScaleType::Log, ScaleType::Linear) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, self.width as f64);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height as f64, 0.0);
                render_element(Arc::new(x_scale), Arc::new(y_scale)).into_any_element()
            }
            (ScaleType::Linear, ScaleType::Log) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, self.width as f64);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height as f64, 0.0);
                render_element(Arc::new(x_scale), Arc::new(y_scale)).into_any_element()
            }
            (ScaleType::Log, ScaleType::Log) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, self.width as f64);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height as f64, 0.0);
                render_element(Arc::new(x_scale), Arc::new(y_scale)).into_any_element()
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

        // Add plot area
        container = container.child(
            div()
                .w(px(self.width))
                .h(px(plot_height))
                .relative()
                .child(area_element),
        );

        Ok(container)
    }
}

/// Create an area chart from x and y data.
///
/// # Example
///
/// ```rust,ignore
/// use gpui_px::{area, CurveType};
///
/// let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let y = vec![2.0, 4.0, 3.0, 5.0, 4.5];
///
/// let chart = area(&x, &y)
///     .title("My Area Chart")
///     .color(0xff7f0e)
///     .curve(CurveType::MonotoneX)
///     .build()?;
/// # Ok::<(), gpui_px::ChartError>(())
/// ```
pub fn area(x: &[f64], y: &[f64]) -> AreaChart {
    AreaChart {
        x: x.to_vec(),
        y: y.to_vec(),
        y0: None,
        title: None,
        color: DEFAULT_COLOR,
        opacity: 0.6,
        curve: Curve::Linear,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        x_scale_type: ScaleType::Linear,
        y_scale_type: ScaleType::Linear,
    }
}
