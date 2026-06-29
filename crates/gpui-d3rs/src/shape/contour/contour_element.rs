use super::contour_config::ContourConfig;
use super::contour_config::smooth_stroke_segment;
use super::misc::split_stroke_segments;
use crate::color::D3Color;
use crate::contour::Contour;
use crate::scale::Scale;
use gpui::prelude::*;
use gpui::*;
use std::panic;
use std::sync::Arc;

/// A custom element for rendering contours
pub struct ContourElement<XS, YS> {
    /// The contours to render
    pub(super) contours: Arc<[Contour]>,
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

impl<XS, YS> ContourElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone,
    YS: Scale<f64, f64> + Clone,
{
    /// Create a new contour element
    pub fn new(contours: impl Into<Arc<[Contour]>>, x_scale: XS, y_scale: YS) -> Self {
        let contours = contours.into();

        // Calculate value range from contours
        let value_range = if contours.is_empty() {
            (0.0, 1.0)
        } else {
            let min = contours
                .iter()
                .map(|c| c.value)
                .fold(f64::INFINITY, f64::min);
            let max = contours
                .iter()
                .map(|c| c.value)
                .fold(f64::NEG_INFINITY, f64::max);
            (min, max)
        };

        Self {
            contours,
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

/// Pre-built paths for a single contour ring.
#[derive(Clone)]
pub struct PreparedContourPath {
    fill: Option<(Path<Pixels>, Rgba)>,
    stroke: Option<(Path<Pixels>, Rgba)>,
}

/// Build all paint-ready paths for a set of contours.
///
/// This is called from `prepaint` so that coordinate transformation, jump
/// detection, smoothing, and `PathBuilder` construction happen once per frame
/// instead of inside the paint closure.
fn prepare_contour_paths<XS, YS>(
    contours: &[Contour],
    x_scale: &XS,
    y_scale: &YS,
    config: &ContourConfig,
    value_range: (f64, f64),
    bounds: Bounds<Pixels>,
) -> Vec<PreparedContourPath>
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

    let get_color = |value: f64, default: D3Color| -> D3Color {
        let t = normalize_value(value);
        if let Some(ref scale) = config.color_scale {
            scale(t)
        } else {
            default
        }
    };

    let mut prepared = Vec::with_capacity(contours.iter().map(|c| c.coordinates.len()).sum());

    for contour in contours.iter() {
        let stroke_color = get_color(contour.value, config.stroke_color);
        let fill_color = get_color(contour.value, config.fill_color);

        for ring in &contour.coordinates {
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

            let is_closed = if screen_points.len() >= 2 {
                let first = &screen_points[0];
                let last = &screen_points[screen_points.len() - 1];
                let dx: f32 = (first.x - last.x).into();
                let dy: f32 = (first.y - last.y).into();
                dx.abs() < 1.0 && dy.abs() < 1.0
            } else {
                false
            };

            let mut fill = None;
            if config.fill && screen_points.len() >= 3 && is_closed {
                let x_jump_threshold = width * 0.15;
                let y_jump_threshold = height * 0.15;
                let has_jump = screen_points.windows(2).any(|pair| {
                    let dx: f32 = (pair[1].x - pair[0].x).abs().into();
                    let dy: f32 = (pair[1].y - pair[0].y).abs().into();
                    dx > x_jump_threshold || dy > y_jump_threshold
                });

                if !has_jump {
                    let mut builder = PathBuilder::fill();
                    builder.move_to(screen_points[0]);
                    for pt in &screen_points[1..] {
                        builder.line_to(*pt);
                    }
                    if let Ok(path) = builder.build() {
                        let mut fill_rgba = fill_color.to_rgba();
                        fill_rgba.a *= config.fill_opacity;
                        fill = Some((path, fill_rgba));
                    }
                }
            }

            let mut stroke = None;
            if config.stroke_opacity > 0.0 && config.stroke_width > 0.0 {
                let x_jump_threshold = width * 0.15;
                let y_jump_threshold = height * 0.15;

                let points_to_draw = if screen_points.len() >= 2 {
                    let first = &screen_points[0];
                    let last = &screen_points[screen_points.len() - 1];
                    let dx: f32 = (first.x - last.x).abs().into();
                    let dy: f32 = (first.y - last.y).abs().into();
                    if dx < 2.0 && dy < 2.0 {
                        &screen_points[..screen_points.len() - 1]
                    } else {
                        &screen_points[..]
                    }
                } else {
                    &screen_points[..]
                };

                let segments =
                    split_stroke_segments(points_to_draw, x_jump_threshold, y_jump_threshold);
                let closes_single_segment = is_closed && segments.len() == 1;
                let mut builder = PathBuilder::stroke(px(config.stroke_width));
                let mut has_segments = false;

                for segment in segments {
                    let segment_is_closed = closes_single_segment && segment.len() >= 3;
                    let draw_points = smooth_stroke_segment(&segment, segment_is_closed, config);
                    if draw_points.len() < 2 {
                        continue;
                    }
                    builder.move_to(draw_points[0]);
                    for point in &draw_points[1..] {
                        builder.line_to(*point);
                    }
                    if segment_is_closed {
                        builder.line_to(draw_points[0]);
                    }
                    has_segments = true;
                }

                if has_segments && let Ok(path) = builder.build() {
                    let mut stroke_rgba = stroke_color.to_rgba();
                    stroke_rgba.a *= config.stroke_opacity;
                    stroke = Some((path, stroke_rgba));
                }
            }

            prepared.push(PreparedContourPath { fill, stroke });
        }
    }

    prepared
}

impl<XS, YS> IntoElement for ContourElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<XS, YS> Element for ContourElement<XS, YS>
where
    XS: Scale<f64, f64> + Clone + 'static,
    YS: Scale<f64, f64> + Clone + 'static,
{
    type RequestLayoutState = ();
    type PrepaintState = Vec<PreparedContourPath>;

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
        prepare_contour_paths(
            &self.contours,
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
            if let Some((path, color)) = prepared.fill {
                window.paint_path(path, color);
            }
            if let Some((path, color)) = prepared.stroke {
                window.paint_path(path, color);
            }
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

    fn test_contour() -> Contour {
        Contour {
            value: 0.5,
            coordinates: vec![ContourRing::new(vec![
                D3Point::new(0.0, 0.0),
                D3Point::new(1.0, 0.0),
                D3Point::new(1.0, 1.0),
                D3Point::new(0.0, 1.0),
                D3Point::new(0.0, 0.0),
            ])],
        }
    }

    #[test]
    fn prepare_contour_paths_builds_fill_and_stroke() {
        let contour = test_contour();
        let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 100.0);
        let y_scale = LinearScale::new().domain(0.0, 1.0).range(100.0, 0.0);
        let config = ContourConfig::new().fill(true).stroke_width(1.0);

        let prepared = prepare_contour_paths(
            std::slice::from_ref(&contour),
            &x_scale,
            &y_scale,
            &config,
            (0.0, 1.0),
            test_bounds(),
        );

        assert_eq!(prepared.len(), 1);
        assert!(prepared[0].fill.is_some(), "expected fill path");
        assert!(prepared[0].stroke.is_some(), "expected stroke path");
    }

    #[test]
    fn prepare_contour_paths_skips_small_rings() {
        let contour = Contour {
            value: 0.5,
            coordinates: vec![ContourRing::new(vec![
                D3Point::new(0.0, 0.0),
                D3Point::new(1.0, 0.0),
            ])],
        };
        let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 100.0);
        let y_scale = LinearScale::new().domain(0.0, 1.0).range(100.0, 0.0);
        let config = ContourConfig::new().fill(true).stroke_width(1.0);

        let prepared = prepare_contour_paths(
            std::slice::from_ref(&contour),
            &x_scale,
            &y_scale,
            &config,
            (0.0, 1.0),
            test_bounds(),
        );

        assert!(prepared.is_empty());
    }

    #[test]
    fn prepare_contour_paths_respects_fill_toggle() {
        let contour = test_contour();
        let x_scale = LinearScale::new().domain(0.0, 1.0).range(0.0, 100.0);
        let y_scale = LinearScale::new().domain(0.0, 1.0).range(100.0, 0.0);
        let config = ContourConfig::new().fill(false).stroke_width(1.0);

        let prepared = prepare_contour_paths(
            std::slice::from_ref(&contour),
            &x_scale,
            &y_scale,
            &config,
            (0.0, 1.0),
            test_bounds(),
        );

        assert_eq!(prepared.len(), 1);
        assert!(prepared[0].fill.is_none());
        assert!(prepared[0].stroke.is_some());
    }
}
