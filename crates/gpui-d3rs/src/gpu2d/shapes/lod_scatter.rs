//! XY-style screen-bounded scatter rendering.
//!
//! A [`DensityPyramid`] is built once when the element is created. Painting a
//! dense trace then composes only a viewport-sized grid, instead of submitting
//! every original point to the GPU.

use super::misc::to_color4;
use crate::color::D3Color;
#[cfg(not(feature = "vello-gpui"))]
use crate::gpu2d::element::Chart2DElement;
#[cfg(not(feature = "vello-gpui"))]
use crate::gpu2d::primitives::Rect;
use crate::lod::{DensityPyramid, LodBounds};
use crate::scale::Scale;
use crate::shape::ScatterPoint;
use gpui::IntoElement;
#[cfg(not(feature = "vello-gpui"))]
use gpui::{Bounds, Pixels};
use std::sync::Arc;

/// Rendering policy for [`render_lod_scatter`].
#[derive(Debug, Clone)]
pub struct LodScatterConfig {
    /// Constant colour used for direct points and the density surface.
    pub color: D3Color,
    /// Maximum alpha at the densest cell.
    pub opacity: f32,
    /// Radius of exact, direct-rendered points.
    pub point_radius: f32,
    /// Number of visible points below which exact circles are rendered.
    pub direct_point_budget: usize,
    /// Edge dimension of the cached, power-of-two density base grid.
    pub pyramid_dimension: usize,
    /// Finest-level stretch allowed before `DensityPyramid::compose` declines
    /// a view. The current element uses the full scale domain; callers using
    /// the pyramid API directly can exact-bin a deeper visible window.
    pub max_upsample: usize,
    /// Viewport in normalized scale space. `LodBounds::new(0, 1, 0, 1)` shows
    /// the complete trace; narrower bounds compose a zoomed density view.
    pub viewport: LodBounds,
}

impl Default for LodScatterConfig {
    fn default() -> Self {
        Self {
            color: D3Color::from_hex(0x4f46e5),
            opacity: 0.88,
            point_radius: 3.0,
            direct_point_budget: 20_000,
            pyramid_dimension: 512,
            max_upsample: 2,
            viewport: LodBounds::new(0.0, 1.0, 0.0, 1.0).expect("unit viewport is valid"),
        }
    }
}

impl LodScatterConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: D3Color) -> Self {
        self.color = color;
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    pub fn direct_point_budget(mut self, budget: usize) -> Self {
        self.direct_point_budget = budget;
        self
    }

    pub fn pyramid_dimension(mut self, dimension: usize) -> Self {
        self.pyramid_dimension = dimension;
        self
    }

    pub fn viewport(mut self, viewport: LodBounds) -> Self {
        self.viewport = viewport;
        self
    }
}

const MAX_DENSITY_DIMENSION: usize = 1024;

/// Retained, normalized scatter data plus its derived density cache.
///
/// Construct this once for data that survives chart updates, then call
/// [`Self::render`] with changed visual policy or viewport values. Painting
/// never rebuilds the pyramid.
#[derive(Clone)]
pub struct LodScatter {
    normalized: Arc<[(f64, f64)]>,
    pyramid: Option<Arc<DensityPyramid>>,
}

impl LodScatter {
    /// Create a retained cache from scale-normalized coordinates.
    pub fn from_normalized(points: Vec<(f64, f64)>, pyramid_dimension: usize) -> Self {
        let normalized: Arc<[(f64, f64)]> = points
            .into_iter()
            .filter(|(x, y)| x.is_finite() && y.is_finite())
            .collect::<Vec<_>>()
            .into();
        let (x, y): (Vec<_>, Vec<_>) = normalized.iter().copied().unzip();
        let pyramid = DensityPyramid::build(
            &x,
            &y,
            LodBounds::new(0.0, 1.0, 0.0, 1.0).expect("unit bounds are valid"),
            pyramid_dimension,
        )
        .ok()
        .map(Arc::new);
        Self {
            normalized,
            pyramid,
        }
    }

    pub fn len(&self) -> usize {
        self.normalized.len()
    }

    pub fn is_empty(&self) -> bool {
        self.normalized.is_empty()
    }

    /// Render with a configurable direct/density tier policy.
    #[cfg(not(feature = "vello-gpui"))]
    pub fn render(&self, config: &LodScatterConfig) -> Chart2DElement {
        render_cached_scatter(self.normalized.clone(), self.pyramid.clone(), config)
    }

    #[cfg(feature = "vello-gpui")]
    pub fn render(&self, config: &LodScatterConfig) -> crate::vello2d::VelloChartElement {
        render_cached_scatter_vello(self.normalized.clone(), self.pyramid.clone(), config)
    }
}

#[cfg(not(feature = "vello-gpui"))]
fn render_cached_scatter(
    normalized: Arc<[(f64, f64)]>,
    pyramid: Option<Arc<DensityPyramid>>,
    config: &LodScatterConfig,
) -> Chart2DElement {
    let direct = normalized.len() <= config.direct_point_budget || pyramid.is_none();
    let color = to_color4(&config.color, config.opacity);
    let point_radius = config.point_radius.max(0.0);
    let viewport = config.viewport;
    let max_upsample = config.max_upsample.max(1);

    Chart2DElement::new(move |renderer, bounds: Bounds<Pixels>| {
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        if direct {
            for &(x, y) in normalized.iter() {
                if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) {
                    renderer.draw_circle(
                        x as f32 * width,
                        (1.0 - y as f32) * height,
                        point_radius,
                        color,
                    );
                }
            }
            return;
        }

        let grid_width = width.ceil().clamp(1.0, MAX_DENSITY_DIMENSION as f32) as usize;
        let grid_height = height.ceil().clamp(1.0, MAX_DENSITY_DIMENSION as f32) as usize;
        let Some(grid) = pyramid
            .as_ref()
            .and_then(|pyramid| pyramid.compose(viewport, grid_width, grid_height, max_upsample))
        else {
            // The view out-resolves the finest aggregate level. Drill into
            // canonical points in this window rather than showing a blank or
            // blurred density image; the retained cache keeps this scan local
            // to the visible data-space window.
            let x_span = viewport.x1 - viewport.x0;
            let y_span = viewport.y1 - viewport.y0;
            for &(x, y) in normalized.iter() {
                if x < viewport.x0 || x > viewport.x1 || y < viewport.y0 || y > viewport.y1 {
                    continue;
                }
                renderer.draw_circle(
                    ((x - viewport.x0) / x_span) as f32 * width,
                    (1.0 - (y - viewport.y0) as f32 / y_span as f32) * height,
                    point_radius.max(1.5),
                    color,
                );
            }
            return;
        };
        let maximum = grid.values.iter().copied().fold(0.0_f32, f32::max);
        if maximum <= 0.0 {
            return;
        }
        let denominator = (1.0 + maximum).ln();
        let cell_width = width / grid.width as f32;
        let cell_height = height / grid.height as f32;
        for (index, count) in grid.values.iter().copied().enumerate() {
            if count <= 0.0 {
                continue;
            }
            let column = index % grid.width;
            let row = grid.height - index / grid.width - 1;
            let alpha = color[3] * ((1.0 + count).ln() / denominator);
            renderer.draw_rect(
                Rect::new(
                    column as f32 * cell_width,
                    row as f32 * cell_height,
                    cell_width.ceil(),
                    cell_height.ceil(),
                ),
                [color[0], color[1], color[2], alpha],
                0.0,
            );
        }
    })
    .transparent()
    .absolute()
}

#[cfg(feature = "vello-gpui")]
fn render_cached_scatter_vello(
    normalized: Arc<[(f64, f64)]>,
    pyramid: Option<Arc<DensityPyramid>>,
    config: &LodScatterConfig,
) -> crate::vello2d::VelloChartElement {
    use crate::vello2d::kurbo::Rect;
    use crate::vello2d::peniko::{Brush, Color};

    let direct = normalized.len() <= config.direct_point_budget || pyramid.is_none();
    let color = to_color4(&config.color, config.opacity);
    let point_radius = config.point_radius.max(0.0);
    let viewport = config.viewport;
    let max_upsample = config.max_upsample.max(1);

    crate::vello2d::VelloChartElement::with_builder(move |width, height| {
        let mut scene = crate::vello2d::ChartScene::new();
        let brush = |rgba: [f32; 4]| Brush::Solid(Color::new(rgba));
        if direct {
            for &(x, y) in normalized.iter() {
                if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) {
                    scene.fill_circle(
                        x * width as f64,
                        (1.0 - y) * height as f64,
                        point_radius as f64,
                        brush(color),
                    );
                }
            }
            return scene;
        }

        let grid_width = width.ceil().clamp(1.0, MAX_DENSITY_DIMENSION as f32) as usize;
        let grid_height = height.ceil().clamp(1.0, MAX_DENSITY_DIMENSION as f32) as usize;
        let Some(grid) = pyramid
            .as_ref()
            .and_then(|pyramid| pyramid.compose(viewport, grid_width, grid_height, max_upsample))
        else {
            let x_span = viewport.x1 - viewport.x0;
            let y_span = viewport.y1 - viewport.y0;
            for &(x, y) in normalized.iter() {
                if x < viewport.x0 || x > viewport.x1 || y < viewport.y0 || y > viewport.y1 {
                    continue;
                }
                scene.fill_circle(
                    (x - viewport.x0) / x_span * width as f64,
                    (1.0 - (y - viewport.y0) / y_span) * height as f64,
                    point_radius.max(1.5) as f64,
                    brush(color),
                );
            }
            return scene;
        };
        let maximum = grid.values.iter().copied().fold(0.0_f32, f32::max);
        if maximum <= 0.0 {
            return scene;
        }
        let denominator = (1.0 + maximum).ln();
        let cell_width = width / grid.width as f32;
        let cell_height = height / grid.height as f32;
        for (index, count) in grid.values.iter().copied().enumerate() {
            if count <= 0.0 {
                continue;
            }
            let column = index % grid.width;
            let row = grid.height - index / grid.width - 1;
            let alpha = color[3] * ((1.0 + count).ln() / denominator);
            let x = column as f32 * cell_width;
            let y = row as f32 * cell_height;
            scene.fill_rect(
                Rect::new(
                    x as f64,
                    y as f64,
                    (x + cell_width.ceil()) as f64,
                    (y + cell_height.ceil()) as f64,
                ),
                brush([color[0], color[1], color[2], alpha]),
            );
        }
        scene
    })
    .absolute()
}

/// Render a scatter trace using exact circles for small datasets and a cached
/// density pyramid for large datasets.
///
/// The scales are applied once at construction, so logarithmic and other
/// continuous scales receive visually uniform density cells. The returned
/// element is deliberately stateless: construct a new element when changing
/// scales or viewport, while the public [`DensityPyramid`] API is available
/// for retained pan/zoom controllers.
pub fn render_lod_scatter<XS, YS>(
    x_scale: &XS,
    y_scale: &YS,
    data: &[ScatterPoint],
    config: &LodScatterConfig,
) -> impl IntoElement
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let (x_min, x_max) = x_scale.range();
    let (y_min, y_max) = y_scale.range();
    let x_span = x_max - x_min;
    let y_span = y_max - y_min;
    let mut normalized = Vec::with_capacity(data.len());
    for point in data {
        let x = if x_span == 0.0 {
            0.5
        } else {
            (x_scale.scale(point.x) - x_min) / x_span
        };
        let y = if y_span == 0.0 {
            0.5
        } else {
            (y_scale.scale(point.y) - y_min) / y_span
        };
        if x.is_finite() && y.is_finite() {
            normalized.push((x, y));
        }
    }

    LodScatter::from_normalized(normalized, config.pyramid_dimension).render(config)
}
