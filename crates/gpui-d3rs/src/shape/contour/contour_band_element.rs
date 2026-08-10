use super::contour_config::ContourConfig;
use crate::color::D3Color;
use crate::contour::ContourBand;
use crate::scale::Scale;
use gpui::prelude::*;
use gpui::*;
use std::panic;
use std::sync::Arc;

/// A custom element for rendering filled contour bands
pub struct ContourBandElement<XS, YS> {
    /// The contour bands to render
    pub(super) bands: Arc<[ContourBand]>,
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
}

impl<XS, YS> ContourBandElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone,
    YS: Scale<f64, f64> + Clone,
{
    /// Create a new contour band element
    pub fn new(bands: impl Into<Arc<[ContourBand]>>, x_scale: XS, y_scale: YS) -> Self {
        let bands = bands.into();

        // Calculate value range from bands
        let value_range = if bands.is_empty() {
            (0.0, 1.0)
        } else {
            let min = bands.iter().map(|b| b.lower).fold(f64::INFINITY, f64::min);
            let max = bands
                .iter()
                .map(|b| b.upper)
                .fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        };

        Self {
            bands,
            x_scale,
            y_scale,
            config: ContourConfig::default(),
            value_range,
            height: px(400.0),
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
}

/// Pre-built paths for a single contour band ring.
#[derive(Clone)]
pub struct PreparedBandPath {
    fill: Path<Pixels>,
    stroke: Path<Pixels>,
    color: Rgba,
}

/// Build all paint-ready paths for a set of contour bands.
fn prepare_band_paths<XS, YS>(
    bands: &[ContourBand],
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
    value_range: (f64, f64),
    bounds: Bounds<Pixels>,
) -> Vec<PreparedBandPath>
where
    XS: Scale<f64, f64>,
    YS: Scale<f64, f64>,
{
    let origin_x: f32 = bounds.origin.x.into();
    let origin_y: f32 = bounds.origin.y.into();
    let width: f32 = bounds.size.width.into();
    let height: f32 = bounds.size.height.into();

    let (x_range_min, x_range_max) = x_scale.range();
    let (y_range_min, y_range_max) = y_scale.range();
    let x_range_span = (x_range_max - x_range_min).abs();
    let y_range_span = (y_range_max - y_range_min).abs();
    let x_range_lo = x_range_min.min(x_range_max);
    let y_range_lo = y_range_min.min(y_range_max);

    let normalize_value = |value: f64| -> f64 {
        let (min, max) = value_range;
        if (max - min).abs() < 1e-10 {
            0.5
        } else {
            (value - min) / (max - min)
        }
    };

    let get_fill_color = |mid_value: f64| -> D3Color {
        let t = normalize_value(mid_value);
        if let Some(ref scale) = config.color_scale {
            scale(t)
        } else {
            config.fill_color
        }
    };

    let mut prepared = Vec::with_capacity(bands.iter().map(|b| b.polygons.len()).sum());

    for band in bands.iter() {
        let fill_color = get_fill_color(band.mid_value());

        for ring in &band.polygons {
            if ring.points.len() < 3 {
                continue;
            }

            let screen_points: Vec<Point<Pixels>> = ring
                .points
                .iter()
                .map(|p| {
                    let x_scaled = x_scale.scale(p.x);
                    let y_scaled = y_scale.scale(p.y);
                    let x_norm = ((x_scaled - x_range_lo) / x_range_span) as f32;
                    let y_norm = ((y_scaled - y_range_lo) / y_range_span) as f32;
                    let screen_x = origin_x + x_norm * width;
                    let screen_y = origin_y + y_norm * height;
                    point(px(screen_x), px(screen_y))
                })
                .collect();

            let mut fill_rgba = fill_color.to_rgba();
            fill_rgba.a *= config.fill_opacity;

            let mut fill_builder = PathBuilder::fill();
            fill_builder.move_to(screen_points[0]);
            for pt in &screen_points[1..] {
                fill_builder.line_to(*pt);
            }

            let mut stroke_builder = PathBuilder::stroke(px(2.0));
            stroke_builder.move_to(screen_points[0]);
            for pt in &screen_points[1..] {
                stroke_builder.line_to(*pt);
            }
            stroke_builder.line_to(screen_points[0]);

            if let (Ok(fill), Ok(stroke)) = (fill_builder.build(), stroke_builder.build()) {
                prepared.push(PreparedBandPath {
                    fill,
                    stroke,
                    color: fill_rgba,
                });
            }
        }
    }

    prepared
}

impl<XS, YS> IntoElement for ContourBandElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<XS, YS> Element for ContourBandElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    type RequestLayoutState = ();
    type PrepaintState = Vec<PreparedBandPath>;

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
        prepare_band_paths(
            &self.bands,
            &self.x_scale,
            &self.y_scale,
            &self.config,
            self.value_range,
            bounds,
        )
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        for prepared in prepaint.drain(..) {
            window.paint_path(prepared.fill, prepared.color);
            window.paint_path(prepared.stroke, prepared.color);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contour::ContourRing;
    use crate::scale::LinearScale;
    use crate::shape::path::Point as D3Point;

    fn test_bounds() -> Bounds<Pixels> {
        Bounds::new(point(px(0.0), px(0.0)), size(px(100.0), px(100.0)))
    }

    fn test_band() -> ContourBand {
        ContourBand {
            lower: 0.0,
            upper: 1.0,
            polygons: vec![ContourRing::new(vec![
                D3Point::new(0.0, 0.0),
                D3Point::new(1.0, 0.0),
                D3Point::new(1.0, 1.0),
                D3Point::new(0.0, 1.0),
                D3Point::new(0.0, 0.0),
            ])],
        }
    }

    #[::core::prelude::v1::test]
    fn prepare_band_paths_builds_fill_and_stroke() {
        let band = test_band();
        let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 100.0);
        let y_scale = LinearScale::new().domain(0.0, 1.0).range(100.0, 0.0);
        let config = ContourConfig::new().fill_opacity(0.5);

        let prepared = prepare_band_paths(
            std::slice::from_ref(&band),
            &x_scale,
            &y_scale,
            &config,
            (0.0, 1.0),
            test_bounds(),
        );

        assert_eq!(prepared.len(), 1);
    }

    #[::core::prelude::v1::test]
    fn prepare_band_paths_skips_small_rings() {
        let band = ContourBand {
            lower: 0.0,
            upper: 1.0,
            polygons: vec![ContourRing::new(vec![
                D3Point::new(0.0, 0.0),
                D3Point::new(1.0, 0.0),
            ])],
        };
        let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 100.0);
        let y_scale = LinearScale::new().domain(0.0, 1.0).range(100.0, 0.0);
        let config = ContourConfig::new();

        let prepared = prepare_band_paths(
            std::slice::from_ref(&band),
            &x_scale,
            &y_scale,
            &config,
            (0.0, 1.0),
            test_bounds(),
        );

        assert!(prepared.is_empty());
    }
}
