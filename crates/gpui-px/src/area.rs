//! Area chart - Plotly Express style API.

use crate::error::ChartError;
use crate::{
    ChartSize, DEFAULT_COLOR, DEFAULT_HEIGHT, DEFAULT_PADDING_FRACTION, DEFAULT_TITLE_FONT_SIZE,
    DEFAULT_WIDTH, ScaleType, TITLE_AREA_HEIGHT, apply_chart_size, default_design, extent_padded,
    extent_padded_iter, resolved_chart_dimensions, validate_data_array, validate_data_length,
    validate_dimensions, validate_positive,
};
use d3rs::color::D3Color;
use d3rs::scale::{LinearScale, LogScale, Scale};
use d3rs::shape::{Area, Curve};
use d3rs::text::{GlyphTextConfig, render_glyph_text};
use gpui::prelude::*;
use gpui::{AnyElement, IntoElement, PathBuilder, Rgba, canvas, div, hsla, px};
use std::sync::Arc;
use gpui_design::DesignSystem;

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
    chart_size: ChartSize,
    x_scale_type: ScaleType,
    y_scale_type: ScaleType,
    design: Option<Arc<DesignSystem>>,
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

    /// Set baseline Y values (y0). Defaults to 0.0 if not specified.
    pub fn y0(mut self, y0: &[f64]) -> Self {
        self.y0 = Some(y0.to_vec());
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
        let plot_height = layout_height - title_height;

        // Calculate domains with padding
        let (x_min, x_max) = extent_padded(&self.x, DEFAULT_PADDING_FRACTION);

        // Calculate Y domain considering y and y0 without intermediate allocations.
        let y_data_min = self.y.iter().copied().fold(f64::INFINITY, f64::min);
        let (y_min, y_max) = if let Some(ref y0) = self.y0 {
            extent_padded_iter(
                self.y.iter().chain(y0.iter()).copied(),
                DEFAULT_PADDING_FRACTION,
            )
        } else if self.y_scale_type == ScaleType::Log {
            // For log scale, use data minimum as baseline instead of 0.0
            extent_padded(&self.y, DEFAULT_PADDING_FRACTION)
        } else {
            extent_padded_iter(
                self.y.iter().copied().chain(std::iter::once(0.0)),
                DEFAULT_PADDING_FRACTION,
            )
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
            None => {
                let y0 = if self.y_scale_type == ScaleType::Log {
                    y_data_min
                } else {
                    0.0
                };
                self.x
                    .iter()
                    .zip(self.y.iter())
                    .map(|(&x, &y1)| AreaDatum { x, y0, y1 })
                    .collect()
            }
        };

        let color = D3Color::from_hex(self.color);
        let fill_color = color.to_rgba();
        let opacity = self.opacity;
        let curve = self.curve;

        // Pre-build the area path points once per scale combination. This moves
        // the Area::generate + flatten work out of the paint closure.
        let build_area_points =
            |x_scale: Arc<dyn Scale<f64, f64>>, y_scale: Arc<dyn Scale<f64, f64>>| {
                let x_scale_for_area = x_scale.clone();
                let y_scale_for_area = y_scale.clone();
                let y_scale_for_y1 = y_scale.clone();
                let area = Area::new()
                    .x(move |d: &AreaDatum| x_scale_for_area.scale(d.x))
                    .y0(move |d: &AreaDatum| y_scale_for_area.scale(d.y0))
                    .y1(move |d: &AreaDatum| y_scale_for_y1.scale(d.y1))
                    .curve(curve);
                let path = area.generate(&data);
                path.flatten(0.5)
                    .into_iter()
                    .map(|p| gpui::point(px(p.x as f32), px(p.y as f32)))
                    .collect::<Vec<_>>()
            };

        let render_element = move |points: Vec<gpui::Point<gpui::Pixels>>| {
            canvas(
                move |bounds, _, _| (points.clone(), bounds),
                move |_, (points, bounds), window, _| {
                    if points.is_empty() {
                        return;
                    }

                    let origin_x: f32 = bounds.origin.x.into();
                    let origin_y: f32 = bounds.origin.y.into();

                    let mut path_builder = PathBuilder::fill();

                    let first = points[0];
                    path_builder.move_to(gpui::point(
                        px(origin_x + f32::from(first.x)),
                        px(origin_y + f32::from(first.y)),
                    ));

                    for p in points.iter().skip(1) {
                        path_builder.line_to(gpui::point(
                            px(origin_x + f32::from(p.x)),
                            px(origin_y + f32::from(p.y)),
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
                    .range(0.0, layout_width as f64);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height as f64, 0.0);
                render_element(build_area_points(Arc::new(x_scale), Arc::new(y_scale))).into_any_element()
            }
            (ScaleType::Log, ScaleType::Linear) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, layout_width as f64);
                let y_scale = LinearScale::new()
                    .domain(y_min, y_max)
                    .range(plot_height as f64, 0.0);
                render_element(build_area_points(Arc::new(x_scale), Arc::new(y_scale))).into_any_element()
            }
            (ScaleType::Linear, ScaleType::Log) => {
                let x_scale = LinearScale::new()
                    .domain(x_min, x_max)
                    .range(0.0, layout_width as f64);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height as f64, 0.0);
                render_element(build_area_points(Arc::new(x_scale), Arc::new(y_scale))).into_any_element()
            }
            (ScaleType::Log, ScaleType::Log) => {
                let x_scale = LogScale::new()
                    .domain(x_min.max(1e-10), x_max)
                    .range(0.0, layout_width as f64);
                let y_scale = LogScale::new()
                    .domain(y_min.max(1e-10), y_max)
                    .range(plot_height as f64, 0.0);
                render_element(build_area_points(Arc::new(x_scale), Arc::new(y_scale))).into_any_element()
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

        // Add plot area
        container = container.child(
            div()
                .w(px(layout_width))
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
        chart_size: ChartSize::default(),
        x_scale_type: ScaleType::Linear,
        y_scale_type: ScaleType::Linear,
        design: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_area_log_scale_without_y0_uses_data_min() {
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = area(&x, &y).y_scale(ScaleType::Log).build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_area_log_scale_with_y0_validates_positive() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![10.0, 20.0, 30.0];
        let y0 = vec![0.0, 0.0, 0.0];
        let result = area(&x, &y).y0(&y0).y_scale(ScaleType::Log).build();
        assert!(matches!(result, Err(ChartError::InvalidData { .. })));
    }

    #[test]
    fn test_area_responsive_size_defaults_and_fixed_opt_in() {
        let x = vec![1.0, 2.0, 3.0];
        let y = vec![2.0, 3.0, 4.0];

        crate::assert_default_chart_size(area(&x, &y).chart_size);
        crate::assert_fixed_chart_size(area(&x, &y).size(320.0, 180.0).chart_size, 320.0, 180.0);
        crate::assert_fill_chart_size(
            area(&x, &y)
                .size(320.0, 180.0)
                .fill()
                .min_size(240.0, 160.0)
                .aspect_ratio(1.5)
                .chart_size,
            240.0,
            160.0,
            Some(1.5),
        );
    }
}
