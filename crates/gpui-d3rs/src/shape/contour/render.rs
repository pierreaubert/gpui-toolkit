use super::contour_band_element::ContourBandElement;
use super::contour_config::ContourConfig;
use super::contour_element::ContourElement;
use super::heatmap_data::HeatmapData;
use super::heatmap_element::HeatmapElement;
use crate::contour::Contour;
use crate::contour::ContourBand;
use crate::scale::Scale;
use std::sync::Arc;

#[cfg(all(feature = "vello-gpui", not(test)))]
use gpui::IntoElement;

/// Render contours using scales
///
/// # Example
///
/// ```rust,no_run
/// use d3rs::prelude::*;
/// use d3rs::shape::contour::{render_contour, ContourConfig};
/// use d3rs::contour::ContourGenerator;
///
/// let values = vec![0.0; 16]; // 4x4 grid
/// let generator = ContourGenerator::new(4, 4);
/// let contours = generator.contours(&values, &[0.5]);
///
/// let x_scale = LinearScale::new().domain(0.0, 4.0).range(0.0, 400.0);
/// let y_scale = LinearScale::new().domain(0.0, 4.0).range(0.0, 400.0);
///
/// let config = ContourConfig::new()
///     .stroke_width(2.0)
///     .fill(true)
///     .fill_opacity(0.3);
///
/// // render_contour(contours, &x_scale, &y_scale, &config)
/// ```
pub fn render_contour<XS, YS>(
    contours: impl Into<Arc<[Contour]>>,
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
) -> ContourElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    ContourElement::new(contours, x_scale.clone(), y_scale.clone()).config(config.clone())
}

/// Render filled contour bands using scales
pub fn render_contour_bands<XS, YS>(
    bands: impl Into<Arc<[ContourBand]>>,
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
) -> ContourBandElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    ContourBandElement::new(bands, x_scale.clone(), y_scale.clone()).config(config.clone())
}

/// Render a heatmap (2D grid of colored cells) using scales
pub fn render_heatmap<XS, YS>(
    data: HeatmapData,
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
) -> HeatmapElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    HeatmapElement::new(data, x_scale.clone(), y_scale.clone()).config(config.clone())
}

#[cfg(all(feature = "vello-gpui", not(test)))]
pub fn render_contour_vello<XS, YS>(
    contours: impl Into<Arc<[Contour]>>,
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
    backend: crate::vello2d::RasterBackend,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    let contours: Arc<[Contour]> = contours.into();
    let x_scale = x_scale.clone();
    let y_scale = y_scale.clone();
    let config = config.clone();
    let value_range = (
        contours
            .iter()
            .map(|c| c.value)
            .fold(f64::INFINITY, f64::min),
        contours
            .iter()
            .map(|c| c.value)
            .fold(f64::NEG_INFINITY, f64::max),
    );
    crate::vello2d::VelloChartElement::with_builder(move |width, height| {
        use crate::vello2d::kurbo::{BezPath, PathEl, Stroke};
        use crate::vello2d::peniko::{Brush, Color};
        let (x0, x1) = x_scale.range();
        let (y0, y1) = y_scale.range();
        let x_span = (x1 - x0).abs().max(f64::EPSILON);
        let y_span = (y1 - y0).abs().max(f64::EPSILON);
        let value_span = (value_range.1 - value_range.0).abs();
        let color_for = |value: f64, fallback: crate::color::D3Color, alpha: f32| {
            let t = if value_span < 1e-10 {
                0.5
            } else {
                (value - value_range.0) / value_span
            };
            let color = config
                .color_scale
                .as_ref()
                .map(|scale| scale(t))
                .unwrap_or(fallback);
            let rgba = color.to_rgba();
            Brush::Solid(Color::new([rgba.r, rgba.g, rgba.b, rgba.a * alpha]))
        };
        let mut scene = crate::vello2d::ChartScene::new();
        for contour in contours.iter() {
            let stroke_brush = color_for(contour.value, config.stroke_color, config.stroke_opacity);
            let fill_brush = color_for(contour.value, config.fill_color, config.fill_opacity);
            for ring in &contour.coordinates {
                if ring.points.len() < 2 {
                    continue;
                }
                let mut path = BezPath::new();
                for (index, point) in ring.points.iter().enumerate() {
                    let x = ((x_scale.scale(point.x) - x0.min(x1)) / x_span) * width as f64;
                    let y = ((y_scale.scale(point.y) - y0.min(y1)) / y_span) * height as f64;
                    let element = if index == 0 {
                        PathEl::MoveTo((x, y).into())
                    } else {
                        PathEl::LineTo((x, y).into())
                    };
                    path.push(element);
                }
                path.push(PathEl::ClosePath);
                if config.fill {
                    scene.fill_path(path.clone(), fill_brush.clone());
                }
                if config.stroke_width > 0.0 && config.stroke_opacity > 0.0 {
                    scene.stroke_path(
                        path,
                        Stroke::new(config.stroke_width as f64),
                        stroke_brush.clone(),
                    );
                }
            }
        }
        scene
    })
    .backend(backend)
    .absolute()
}

#[cfg(all(feature = "vello-gpui", not(test)))]
pub fn render_contour_bands_vello<XS, YS>(
    bands: impl Into<Arc<[ContourBand]>>,
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
    backend: crate::vello2d::RasterBackend,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    let bands: Arc<[ContourBand]> = bands.into();
    let x_scale = x_scale.clone();
    let y_scale = y_scale.clone();
    let config = config.clone();
    let value_range = (
        bands.iter().map(|b| b.lower).fold(f64::INFINITY, f64::min),
        bands
            .iter()
            .map(|b| b.upper)
            .fold(f64::NEG_INFINITY, f64::max),
    );
    crate::vello2d::VelloChartElement::with_builder(move |width, height| {
        use crate::vello2d::kurbo::{BezPath, PathEl, Stroke};
        use crate::vello2d::peniko::{Brush, Color};
        let (x0, x1) = x_scale.range();
        let (y0, y1) = y_scale.range();
        let x_span = (x1 - x0).abs().max(f64::EPSILON);
        let y_span = (y1 - y0).abs().max(f64::EPSILON);
        let value_span = (value_range.1 - value_range.0).abs();
        let mut scene = crate::vello2d::ChartScene::new();
        for band in bands.iter() {
            let t = if value_span < 1e-10 {
                0.5
            } else {
                (band.mid_value() - value_range.0) / value_span
            };
            let color = config
                .color_scale
                .as_ref()
                .map(|scale| scale(t))
                .unwrap_or(config.fill_color);
            let rgba = color.to_rgba();
            for ring in &band.polygons {
                if ring.points.len() < 3 {
                    continue;
                }
                let mut path = BezPath::new();
                for (index, point) in ring.points.iter().enumerate() {
                    let x = ((x_scale.scale(point.x) - x0.min(x1)) / x_span) * width as f64;
                    let y = ((y_scale.scale(point.y) - y0.min(y1)) / y_span) * height as f64;
                    path.push(if index == 0 {
                        PathEl::MoveTo((x, y).into())
                    } else {
                        PathEl::LineTo((x, y).into())
                    });
                }
                path.push(PathEl::ClosePath);
                scene.fill_path(
                    path.clone(),
                    Brush::Solid(Color::new([
                        rgba.r,
                        rgba.g,
                        rgba.b,
                        rgba.a * config.fill_opacity,
                    ])),
                );
                if config.stroke_width > 0.0 && config.stroke_opacity > 0.0 {
                    scene.stroke_path(
                        path,
                        Stroke::new(config.stroke_width as f64),
                        Brush::Solid(Color::new([
                            rgba.r,
                            rgba.g,
                            rgba.b,
                            rgba.a * config.stroke_opacity,
                        ])),
                    );
                }
            }
        }
        scene
    })
    .backend(backend)
    .absolute()
}

#[cfg(all(feature = "vello-gpui", not(test)))]
pub fn render_heatmap_vello<XS, YS>(
    data: HeatmapData,
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
    backend: crate::vello2d::RasterBackend,
) -> impl IntoElement
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    let x_scale = x_scale.clone();
    let y_scale = y_scale.clone();
    let config = config.clone();
    let value_range = (
        data.values.iter().copied().fold(f64::INFINITY, f64::min),
        data.values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max),
    );
    crate::vello2d::VelloChartElement::with_builder(move |width, height| {
        use crate::vello2d::kurbo::Rect;
        use crate::vello2d::peniko::{Brush, Color};
        let (x0, x1) = x_scale.range();
        let (y0, y1) = y_scale.range();
        let x_span = (x1 - x0).abs().max(f64::EPSILON);
        let y_span = (y1 - y0).abs().max(f64::EPSILON);
        let value_span = (value_range.1 - value_range.0).abs();
        let mut scene = crate::vello2d::ChartScene::new();
        for row in 0..data.height {
            for column in 0..data.width {
                let Some(value) = data.get(column, row) else {
                    continue;
                };
                if !value.is_finite() {
                    continue;
                }
                let x_data = data.x_values[column];
                let x_next = data
                    .x_values
                    .get(column + 1)
                    .copied()
                    .unwrap_or(x_data + 1.0);
                let y_data = data.y_values[row];
                let y_next = data.y_values.get(row + 1).copied().unwrap_or(y_data + 1.0);
                let xa = ((x_scale.scale(x_data) - x0.min(x1)) / x_span) * width as f64;
                let xb = ((x_scale.scale(x_next) - x0.min(x1)) / x_span) * width as f64;
                let ya = ((y_scale.scale(y_data) - y0.min(y1)) / y_span) * height as f64;
                let yb = ((y_scale.scale(y_next) - y0.min(y1)) / y_span) * height as f64;
                let t = if value_span < 1e-10 {
                    0.5
                } else {
                    (value - value_range.0) / value_span
                };
                let color = config
                    .color_scale
                    .as_ref()
                    .map(|scale| scale(t))
                    .unwrap_or(config.fill_color);
                let rgba = color.to_rgba();
                scene.fill_rect(
                    Rect::new(xa.min(xb), ya.min(yb), xa.max(xb), ya.max(yb)),
                    Brush::Solid(Color::new([
                        rgba.r,
                        rgba.g,
                        rgba.b,
                        rgba.a * config.fill_opacity,
                    ])),
                );
            }
        }
        scene
    })
    .backend(backend)
    .absolute()
}
