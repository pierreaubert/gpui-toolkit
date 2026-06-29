use super::spectrum_colors::SpectrumColors;
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::panic;
use std::sync::Arc;

/// GPU-accelerated spectrum analyzer element.
pub struct SpectrumElement {
    pub(super) magnitudes: Arc<[f32]>,
    pub(super) min_freq: f32,
    pub(super) max_freq: f32,
    pub(super) smoothing: f32,
    pub(super) previous_magnitudes: Option<Arc<[f32]>>,
    pub(super) colors: SpectrumColors,
    pub(super) height: Pixels,
    pub(super) bar_gap: Pixels,
    scratch_heights: RefCell<Vec<f32>>,
}

impl SpectrumElement {
    pub fn new(magnitudes: impl Into<Arc<[f32]>>) -> Self {
        Self {
            magnitudes: magnitudes.into(),
            min_freq: 20.0,
            max_freq: 20000.0,
            smoothing: 0.3,
            previous_magnitudes: None,
            colors: SpectrumColors::default(),
            height: px(120.0),
            bar_gap: px(1.0),
            scratch_heights: RefCell::new(Vec::new()),
        }
    }

    pub fn frequency_range(mut self, min: f32, max: f32) -> Self {
        self.min_freq = min;
        self.max_freq = max;
        self
    }

    pub fn smoothing(mut self, smoothing: f32) -> Self {
        self.smoothing = smoothing.clamp(0.0, 0.99);
        self
    }

    pub fn previous(mut self, previous: impl Into<Arc<[f32]>>) -> Self {
        self.previous_magnitudes = Some(previous.into());
        self
    }

    pub fn colors(mut self, colors: SpectrumColors) -> Self {
        self.colors = colors;
        self
    }

    pub fn height(mut self, height: Pixels) -> Self {
        self.height = height;
        self
    }

    pub fn bar_gap(mut self, gap: Pixels) -> Self {
        self.bar_gap = gap;
        self
    }

    pub(super) fn db_to_height(&self, db: f32) -> f32 {
        ((db + 100.0) / 103.0).clamp(0.0, 1.0)
    }
}

impl IntoElement for SpectrumElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for SpectrumElement {
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
        let layout_id = window.request_layout(
            Style {
                size: size(relative(1.0).into(), self.height.into()),
                min_size: size(px(100.0).into(), px(60.0).into()),
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
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
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
        let bar_count = self.magnitudes.len();
        if bar_count == 0 {
            return;
        }

        window.paint_quad(PaintQuad {
            bounds,
            corner_radii: Corners::all(px(4.0)),
            background: self.colors.background.into(),
            border_widths: Edges::default(),
            border_color: Hsla::transparent_black(),
            border_style: Default::default(),
        });

        let yellow_threshold = self.db_to_height(-6.0);
        let red_threshold = self.db_to_height(-1.0);
        let step_width = bounds.size.width / bar_count as f32;
        let meter_height = bounds.size.height;

        let mut scratch = self.scratch_heights.borrow_mut();
        scratch.clear();
        scratch.extend(self.magnitudes.iter().enumerate().map(|(i, &mag)| {
            let smoothed_mag = if let Some(ref prev) = self.previous_magnitudes {
                if i < prev.len() {
                    prev[i] * self.smoothing + mag * (1.0 - self.smoothing)
                } else {
                    mag
                }
            } else {
                mag
            };
            self.db_to_height(smoothed_mag)
        }));

        let mut green_path = PathBuilder::fill();
        green_path.move_to(point(bounds.origin.x, bounds.origin.y + meter_height));
        let mut yellow_path = PathBuilder::fill();
        let mut has_yellow = false;
        let mut red_path = PathBuilder::fill();
        let mut has_red = false;

        for (i, &height_ratio) in scratch.iter().enumerate() {
            let x = bounds.origin.x + step_width * i as f32;
            let green_height = height_ratio.min(yellow_threshold);
            let green_y = bounds.origin.y + meter_height - (meter_height * green_height);
            green_path.line_to(point(x, green_y));
            green_path.line_to(point(x + step_width, green_y));

            if height_ratio > yellow_threshold {
                if !has_yellow {
                    has_yellow = true;
                    yellow_path.move_to(point(
                        bounds.origin.x,
                        bounds.origin.y + meter_height - (meter_height * yellow_threshold),
                    ));
                }
                let yellow_height =
                    (height_ratio - yellow_threshold).min(red_threshold - yellow_threshold);
                let yellow_top = yellow_threshold + yellow_height;
                let yellow_y = bounds.origin.y + meter_height - (meter_height * yellow_top);
                yellow_path.line_to(point(x, yellow_y));
                yellow_path.line_to(point(x + step_width, yellow_y));
            } else if has_yellow {
                let yellow_bottom_y =
                    bounds.origin.y + meter_height - (meter_height * yellow_threshold);
                yellow_path.line_to(point(x, yellow_bottom_y));
                yellow_path.line_to(point(x + step_width, yellow_bottom_y));
            }

            if height_ratio > red_threshold {
                if !has_red {
                    has_red = true;
                    red_path.move_to(point(
                        bounds.origin.x,
                        bounds.origin.y + meter_height - (meter_height * red_threshold),
                    ));
                }
                let red_height = height_ratio - red_threshold;
                let red_top = red_threshold + red_height;
                let red_y = bounds.origin.y + meter_height - (meter_height * red_top);
                red_path.line_to(point(x, red_y));
                red_path.line_to(point(x + step_width, red_y));
            } else if has_red {
                let red_bottom_y = bounds.origin.y + meter_height - (meter_height * red_threshold);
                red_path.line_to(point(x, red_bottom_y));
                red_path.line_to(point(x + step_width, red_bottom_y));
            }
        }

        green_path.line_to(point(
            bounds.origin.x + bounds.size.width,
            bounds.origin.y + meter_height,
        ));
        green_path.line_to(point(bounds.origin.x, bounds.origin.y + meter_height));
        if let Ok(path) = green_path.build() {
            window.paint_path(path, self.colors.low);
        }

        if has_yellow {
            let yellow_bottom_y =
                bounds.origin.y + meter_height - (meter_height * yellow_threshold);
            yellow_path.line_to(point(bounds.origin.x + bounds.size.width, yellow_bottom_y));
            yellow_path.line_to(point(bounds.origin.x, yellow_bottom_y));
            if let Ok(path) = yellow_path.build() {
                window.paint_path(path, self.colors.mid);
            }
        }

        if has_red {
            let red_bottom_y = bounds.origin.y + meter_height - (meter_height * red_threshold);
            red_path.line_to(point(bounds.origin.x + bounds.size.width, red_bottom_y));
            red_path.line_to(point(bounds.origin.x, red_bottom_y));
            if let Ok(path) = red_path.build() {
                window.paint_path(path, self.colors.high);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SpectrumElement;
    use crate::spectrum::SpectrumColors;
    use gpui::{Element, IntoElement, px};

    #[test]
    fn scratch_buffer_is_reused_across_simulated_paints() {
        let element = SpectrumElement::new(vec![-30.0_f32, -60.0, -10.0]);
        assert!(element.scratch_heights.borrow().is_empty());

        for _ in 0..3 {
            let mut scratch = element.scratch_heights.borrow_mut();
            scratch.clear();
            scratch.extend(element.magnitudes.iter().enumerate().map(|(i, &mag)| {
                let smoothed = if let Some(ref prev) = element.previous_magnitudes {
                    if i < prev.len() {
                        prev[i] * element.smoothing + mag * (1.0 - element.smoothing)
                    } else {
                        mag
                    }
                } else {
                    mag
                };
                element.db_to_height(smoothed)
            }));
            assert_eq!(scratch.len(), element.magnitudes.len());
        }

        // Capacity should have been retained after clear/extend cycles.
        assert!(element.scratch_heights.borrow().capacity() >= 3);
    }

    #[test]
    fn builder_setters_chain() {
        let prev: Vec<f32> = vec![-40.0, -50.0];
        let element = SpectrumElement::new(vec![-30.0, -60.0])
            .frequency_range(20.0, 20_000.0)
            .smoothing(0.5)
            .previous(prev.clone())
            .colors(SpectrumColors::default())
            .height(px(100.0))
            .bar_gap(px(2.0));

        assert_eq!(element.min_freq, 20.0);
        assert_eq!(element.max_freq, 20_000.0);
        assert_eq!(element.smoothing, 0.5);
        assert!(element.previous_magnitudes.is_some());
        assert_eq!(element.height, px(100.0));
        assert_eq!(element.bar_gap, px(2.0));
    }

    #[test]
    fn smoothing_is_clamped_to_valid_range() {
        let low = SpectrumElement::new(vec![0.0]).smoothing(-0.5);
        assert_eq!(low.smoothing, 0.0);

        let high = SpectrumElement::new(vec![0.0]).smoothing(1.5);
        assert_eq!(high.smoothing, 0.99);
    }

    #[test]
    fn db_to_height_is_bounded() {
        let element = SpectrumElement::new(vec![]);
        assert_eq!(element.db_to_height(-100.0), 0.0);
        assert_eq!(element.db_to_height(3.0), 1.0);
        assert!(element.db_to_height(-50.0) > 0.0 && element.db_to_height(-50.0) < 1.0);
    }

    #[test]
    fn element_trait_methods_are_callable() {
        let element = SpectrumElement::new(vec![-30.0, -60.0]);
        let _same = element.into_element();
        assert!(_same.id().is_none());
    }
}
