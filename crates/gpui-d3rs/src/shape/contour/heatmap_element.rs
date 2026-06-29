use super::contour_config::ContourConfig;
use super::heatmap_data::HeatmapData;
use crate::color::D3Color;
use crate::scale::Scale;
use gpui::prelude::*;
use gpui::*;
use std::panic;

/// A batched rectangle for heatmap rendering.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HeatmapQuad {
    /// Normalized x origin (0.0 - 1.0)
    pub x: f32,
    /// Normalized y origin (0.0 - 1.0)
    pub y: f32,
    /// Normalized width (0.0 - 1.0)
    pub width: f32,
    /// Normalized height (0.0 - 1.0)
    pub height: f32,
    /// Fill color
    pub color: D3Color,
}

/// A custom element for rendering heatmaps as colored quads
/// This eliminates anti-aliasing gaps between cells
pub struct HeatmapElement<XS, YS> {
    /// The heatmap data
    pub(super) data: HeatmapData,
    /// X scale
    pub(super) x_scale: XS,
    /// Y scale
    pub(super) y_scale: YS,
    /// Configuration
    pub(super) config: ContourConfig,
    /// Value range for color normalization
    pub(super) value_range: (f64, f64),
    /// Element height
    pub(super) height: Pixels,
    /// Cached batched quads from the last prepaint.
    pub(super) cached_quads: Vec<HeatmapQuad>,
    /// Generation key for the cached quads.
    pub(super) cache_generation: u64,
}

impl<XS, YS> HeatmapElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone,
    YS: Scale<f64, f64> + Clone,
{
    /// Create a new heatmap element
    pub fn new(data: HeatmapData, x_scale: XS, y_scale: YS) -> Self {
        // Calculate value range from data
        let value_range = if data.values.is_empty() {
            (0.0, 1.0)
        } else {
            let min = data.values.iter().cloned().fold(f64::INFINITY, f64::min);
            let max = data
                .values
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        };

        Self {
            data,
            x_scale,
            y_scale,
            config: ContourConfig::default(),
            value_range,
            height: px(400.0),
            cached_quads: Vec::new(),
            cache_generation: u64::MAX,
        }
    }

    /// Set the configuration
    pub fn config(mut self, config: ContourConfig) -> Self {
        self.config = config;
        self
    }

    /// Set the value range for color normalization
    pub fn value_range(mut self, min: f64, max: f64) -> Self {
        self.value_range = (min, max);
        self
    }

    /// Set the element height
    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    /// Normalize a value to 0.0-1.0 range
    pub(super) fn normalize_value(&self, value: f64) -> f64 {
        let (min, max) = self.value_range;
        if (max - min).abs() < 1e-10 {
            0.5
        } else {
            (value - min) / (max - min)
        }
    }

    /// Get fill color for a value
    pub(super) fn get_fill_color(&self, value: f64) -> D3Color {
        let t = self.normalize_value(value);
        if let Some(ref scale) = self.config.color_scale {
            scale(t)
        } else {
            self.config.fill_color
        }
    }

    /// Compute a generation key for the batched quads cache.
    pub(super) fn compute_generation(&self, bounds: &Bounds<Pixels>) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();
        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();
        width.to_bits().hash(&mut hasher);
        height.to_bits().hash(&mut hasher);
        origin_x.to_bits().hash(&mut hasher);
        origin_y.to_bits().hash(&mut hasher);
        f32::from(self.height).to_bits().hash(&mut hasher);
        self.value_range.0.to_bits().hash(&mut hasher);
        self.value_range.1.to_bits().hash(&mut hasher);
        let (x_range_min, x_range_max) = self.x_scale.range();
        let (y_range_min, y_range_max) = self.y_scale.range();
        x_range_min.to_bits().hash(&mut hasher);
        x_range_max.to_bits().hash(&mut hasher);
        y_range_min.to_bits().hash(&mut hasher);
        y_range_max.to_bits().hash(&mut hasher);
        self.config.fill_opacity.to_bits().hash(&mut hasher);
        hash_color(&self.config.fill_color, &mut hasher);
        if let Some(ref _scale) = self.config.color_scale {
            // Custom color functions can't be hashed; only cache when no custom scale.
            1u8.hash(&mut hasher);
        } else {
            0u8.hash(&mut hasher);
        }
        for v in &self.data.values {
            v.to_bits().hash(&mut hasher);
        }
        for v in &self.data.x_values {
            v.to_bits().hash(&mut hasher);
        }
        for v in &self.data.y_values {
            v.to_bits().hash(&mut hasher);
        }
        // Scales are not hashable, but their range affects normalization. Bounds are hashed above.
        hasher.finish()
    }

    /// Build batched quads for the heatmap.
    ///
    /// Cells are grouped into horizontal runs of the same color. Each run becomes a single quad,
    /// reducing the number of draw calls vs. one quad per cell.
    pub(super) fn build_quads(&self, bounds: &Bounds<Pixels>) -> Vec<HeatmapQuad> {
        let origin_x: f32 = bounds.origin.x.into();
        let origin_y: f32 = bounds.origin.y.into();
        let width: f32 = bounds.size.width.into();
        let height: f32 = bounds.size.height.into();

        let (x_range_min, x_range_max) = self.x_scale.range();
        let (y_range_min, y_range_max) = self.y_scale.range();
        let x_range_span = (x_range_max - x_range_min).abs();
        let y_range_span = (y_range_max - y_range_min).abs();
        let x_range_lo = x_range_min.min(x_range_max);
        let y_range_lo = y_range_min.min(y_range_max);

        let mut quads = Vec::new();

        for yi in 0..self.data.height {
            let y0_data = self.data.y_values[yi];
            let y1_data = if yi + 1 < self.data.height {
                self.data.y_values[yi + 1]
            } else if yi > 0 {
                y0_data + (y0_data - self.data.y_values[yi - 1])
            } else {
                y0_data * 1.1
            };

            let y0_scaled = self.y_scale.scale(y0_data);
            let y1_scaled = self.y_scale.scale(y1_data);
            let y0_norm = ((y0_scaled - y_range_lo) / y_range_span) as f32;
            let y1_norm = ((y1_scaled - y_range_lo) / y_range_span) as f32;
            let screen_y0 = origin_y + y0_norm.min(y1_norm) * height;
            let screen_y1 = origin_y + y0_norm.max(y1_norm) * height;
            let cell_height = (screen_y1 - screen_y0).max(1.0) + 0.5;

            let mut xi = 0;
            while xi < self.data.width {
                let value = match self.data.get(xi, yi) {
                    Some(v) if v.is_finite() => v,
                    _ => {
                        xi += 1;
                        continue;
                    }
                };
                let color = self.get_fill_color(value);

                // Find the end of the horizontal run with the same color.
                let mut run_end = xi + 1;
                while run_end < self.data.width {
                    let next_value = match self.data.get(run_end, yi) {
                        Some(v) if v.is_finite() => v,
                        _ => break,
                    };
                    let next_color = self.get_fill_color(next_value);
                    if next_color != color {
                        break;
                    }
                    run_end += 1;
                }

                let x0_data = self.data.x_values[xi];
                let x1_data = if run_end < self.data.width {
                    self.data.x_values[run_end]
                } else if self.data.width > 0 {
                    let last = self.data.x_values[self.data.width - 1];
                    let prev = self.data.x_values[self.data.width - 2];
                    last + (last - prev)
                } else {
                    x0_data * 1.1
                };

                let x0_scaled = self.x_scale.scale(x0_data);
                let x1_scaled = self.x_scale.scale(x1_data);
                let x0_norm = ((x0_scaled - x_range_lo) / x_range_span) as f32;
                let x1_norm = ((x1_scaled - x_range_lo) / x_range_span) as f32;
                let screen_x0 = origin_x + x0_norm.min(x1_norm) * width;
                let screen_x1 = origin_x + x0_norm.max(x1_norm) * width;
                let cell_width = (screen_x1 - screen_x0).max(1.0) + 0.5;

                quads.push(HeatmapQuad {
                    x: screen_x0,
                    y: screen_y0,
                    width: cell_width,
                    height: cell_height,
                    color,
                });

                xi = run_end;
            }
        }

        quads
    }

    /// Prepare batched quads, caching them when the generation key is unchanged.
    pub(super) fn prepare_quads(&mut self, bounds: Bounds<Pixels>) {
        let generation = self.compute_generation(&bounds);
        if self.cache_generation == generation && !self.cached_quads.is_empty() {
            return;
        }
        self.cached_quads = self.build_quads(&bounds);
        self.cache_generation = generation;
    }

    /// Return the cached quads for inspection (tests).
    #[cfg(test)]
    pub(super) fn cached_quads(&self) -> &[HeatmapQuad] {
        &self.cached_quads
    }
}

fn hash_color(color: &D3Color, hasher: &mut std::collections::hash_map::DefaultHasher) {
    use std::hash::Hash;
    color.r.to_bits().hash(hasher);
    color.g.to_bits().hash(hasher);
    color.b.to_bits().hash(hasher);
    color.a.to_bits().hash(hasher);
}

impl<XS, YS> IntoElement for HeatmapElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<XS, YS> Element for HeatmapElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        // Use the scale's range for width to ensure alignment with other chart elements
        let (x_range_min, x_range_max) = self.x_scale.range();
        let computed_width = px((x_range_max - x_range_min).abs() as f32);

        let layout_id = window.request_layout(
            Style {
                size: size(computed_width.into(), self.height.into()),
                min_size: size(px(100.0).into(), px(100.0).into()),
                ..Default::default()
            },
            [],
            cx,
        );
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        self.prepare_quads(bounds);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        // `prepaint` already prepares the quads; only fall back if it was
        // skipped (e.g. direct unit-test paint calls).
        if self.cache_generation == u64::MAX {
            self.prepare_quads(bounds);
        }

        for quad in &self.cached_quads {
            let mut fill_rgba = quad.color.to_rgba();
            fill_rgba.a *= self.config.fill_opacity;

            let cell_bounds = Bounds::new(
                point(px(quad.x), px(quad.y)),
                size(px(quad.width), px(quad.height)),
            );

            window.paint_quad(PaintQuad {
                bounds: cell_bounds,
                corner_radii: Corners::default(),
                background: fill_rgba.into(),
                border_widths: Edges::default(),
                border_color: gpui::transparent_black(),
                border_style: Default::default(),
            });
        }
    }
}
