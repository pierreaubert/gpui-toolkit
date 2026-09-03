use super::spectrum_colors::SpectrumColors;
use gpui::prelude::*;
use gpui::*;
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;
use std::sync::Arc;

use d3rs::render2d::{Renderer2D, VelloBackend};

fn bar_x_bounds(total_width: f32, bar_count: usize, gap: f32, index: usize) -> (f32, f32) {
    let step_width = total_width / bar_count as f32;
    let half_gap = gap.clamp(0.0, step_width) * 0.5;
    (
        step_width * index as f32 + half_gap,
        step_width * (index + 1) as f32 - half_gap,
    )
}

/// GPU-accelerated spectrum analyzer element.
pub struct SpectrumElement {
    id: ElementId,
    source_location: &'static panic::Location<'static>,
    pub(super) magnitudes: Arc<[f32]>,
    pub(super) min_freq: f32,
    pub(super) max_freq: f32,
    pub(super) smoothing: f32,
    pub(super) previous_magnitudes: Option<Arc<[f32]>>,
    pub(super) colors: SpectrumColors,
    pub(super) height: Pixels,
    pub(super) bar_gap: Pixels,
    renderer_2d: Renderer2D,
    vello_backend: VelloBackend,
    #[cfg(feature = "vello")]
    painter: d3rs::vello2d::VelloScenePainter,
}

impl SpectrumElement {
    #[track_caller]
    pub fn new(magnitudes: impl Into<Arc<[f32]>>) -> Self {
        let source_location = panic::Location::caller();
        Self {
            id: ElementId::CodeLocation(*source_location),
            source_location,
            magnitudes: magnitudes.into(),
            min_freq: 20.0,
            max_freq: 20000.0,
            smoothing: 0.3,
            previous_magnitudes: None,
            colors: SpectrumColors::default(),
            height: px(120.0),
            bar_gap: px(1.0),
            renderer_2d: Renderer2D::default(),
            vello_backend: VelloBackend::default(),
            #[cfg(feature = "vello")]
            painter: d3rs::vello2d::VelloScenePainter::new(),
        }
    }

    /// Build an element from the latest frame published to a [`super::MeterFifo`].
    ///
    /// Snapshots into the caller-owned `scratch` buffer (keep it in view
    /// state and reuse it across frames) so the audio thread can publish
    /// without the UI thread allocating per frame or touching
    /// `Rc<RefCell<...>>` element state.
    pub fn new_shared(fifo: &super::MeterFifo, scratch: &mut Vec<f32>) -> Self {
        Self::new(fifo.snapshot_shared(scratch))
    }

    pub fn frequency_range(mut self, min: f32, max: f32) -> Self {
        self.min_freq = min;
        self.max_freq = max;
        self
    }

    /// Blend the current magnitudes with [`Self::previous`].
    ///
    /// For audio-rate updates, keep the previous magnitudes in an `Arc<[f32]>`
    /// (or update a [`super::MeterData`] in place) and reuse that buffer across
    /// frames; passing a freshly allocated vector each repaint defeats the
    /// spectrum's allocation-free smoothing path.
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

    /// Select the high-level 2D renderer. Vello is the default.
    pub fn renderer_2d(mut self, renderer: Renderer2D) -> Self {
        self.renderer_2d = renderer;
        self
    }

    /// Select the Vello WGPU/CPU backend.
    pub fn vello_backend(mut self, backend: VelloBackend) -> Self {
        self.vello_backend = backend;
        #[cfg(feature = "vello")]
        {
            self.painter.set_backend(backend);
        }
        self
    }

    pub(super) fn db_to_height(&self, db: f32) -> f32 {
        ((db + 100.0) / 103.0).clamp(0.0, 1.0)
    }

    fn update_scratch_heights(&self, scratch: &mut Vec<f32>) {
        scratch.clear();
        scratch.extend(
            self.magnitudes
                .iter()
                .enumerate()
                .map(|(index, &magnitude)| {
                    let smoothed_magnitude = if let Some(ref previous) = self.previous_magnitudes {
                        previous.get(index).map_or(magnitude, |previous| {
                            previous * self.smoothing + magnitude * (1.0 - self.smoothing)
                        })
                    } else {
                        magnitude
                    };
                    self.db_to_height(smoothed_magnitude)
                }),
        );
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
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static panic::Location<'static>> {
        Some(self.source_location)
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
        id: Option<&GlobalElementId>,
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
        let meter_height = bounds.size.height;

        // Elements are recreated on every view render. Retain the scratch
        // buffer under the element's GlobalElementId so the backing allocation
        // survives those reconstructions and GPUI releases it with the element.
        let scratch = if let Some(id) = id {
            window.with_element_state::<Rc<RefCell<Vec<f32>>>, _>(id, |state, _window| {
                let state = state.unwrap_or_else(|| Rc::new(RefCell::new(Vec::new())));
                (Rc::clone(&state), state)
            })
        } else {
            Rc::new(RefCell::new(Vec::new()))
        };
        let mut scratch = scratch.borrow_mut();
        self.update_scratch_heights(&mut scratch);

        #[cfg(feature = "vello")]
        if self.renderer_2d.is_vello() {
            use d3rs::vello2d::ChartScene;
            use d3rs::vello2d::kurbo::{BezPath, PathEl, Rect};
            use d3rs::vello2d::peniko::{Brush, Color};

            let width: f32 = bounds.size.width.into();
            let height: f32 = bounds.size.height.into();
            let yellow_threshold = self.db_to_height(-6.0);
            let red_threshold = self.db_to_height(-1.0);
            let mut scene = ChartScene::new();
            let color_brush =
                |color: Rgba| Brush::Solid(Color::new([color.r, color.g, color.b, color.a]));
            scene.fill_rect(
                Rect::new(0.0, 0.0, width as f64, height as f64),
                color_brush(self.colors.background),
            );

            let mut bands = [BezPath::new(), BezPath::new(), BezPath::new()];
            for (index, &ratio) in scratch.iter().enumerate() {
                let (x0, x1) = bar_x_bounds(width, bar_count, self.bar_gap.into(), index);
                let y = |value: f32| height - height * value;
                let segments = [
                    (0usize, ratio.min(yellow_threshold), self.colors.low),
                    (
                        1usize,
                        (ratio - yellow_threshold).clamp(0.0, red_threshold - yellow_threshold),
                        self.colors.mid,
                    ),
                    (2usize, (ratio - red_threshold).max(0.0), self.colors.high),
                ];
                for (band, amount, _) in segments {
                    if amount <= 0.0 {
                        continue;
                    }
                    let bottom = match band {
                        0 => 1.0,
                        1 => yellow_threshold,
                        _ => red_threshold,
                    };
                    let top = match band {
                        0 => amount,
                        1 => yellow_threshold + amount,
                        _ => red_threshold + amount,
                    };
                    bands[band].push(PathEl::MoveTo((x0 as f64, y(bottom) as f64).into()));
                    bands[band].push(PathEl::LineTo((x1 as f64, y(bottom) as f64).into()));
                    bands[band].push(PathEl::LineTo((x1 as f64, y(top) as f64).into()));
                    bands[band].push(PathEl::LineTo((x0 as f64, y(top) as f64).into()));
                    bands[band].push(PathEl::ClosePath);
                }
            }
            for (path, color) in
                bands
                    .into_iter()
                    .zip([self.colors.low, self.colors.mid, self.colors.high])
            {
                if !path.elements().is_empty() {
                    scene.fill_path(path, color_brush(color));
                }
            }
            self.painter.set_backend(self.vello_backend);
            self.painter.paint_retained(id, &scene, bounds, window);
            return;
        }

        let mut green_path = PathBuilder::fill();
        let mut yellow_path = PathBuilder::fill();
        let mut red_path = PathBuilder::fill();
        let width: f32 = bounds.size.width.into();
        let gap: f32 = self.bar_gap.into();

        for (index, &height_ratio) in scratch.iter().enumerate() {
            let (x0, x1) = bar_x_bounds(width, bar_count, gap, index);
            let x0 = bounds.origin.x + px(x0);
            let x1 = bounds.origin.x + px(x1);
            let green_height = height_ratio.min(yellow_threshold);
            let green_y = bounds.origin.y + meter_height - (meter_height * green_height);
            green_path.move_to(point(x0, bounds.origin.y + meter_height));
            green_path.line_to(point(x1, bounds.origin.y + meter_height));
            green_path.line_to(point(x1, green_y));
            green_path.line_to(point(x0, green_y));
            green_path.close();

            if height_ratio > yellow_threshold {
                let yellow_height =
                    (height_ratio - yellow_threshold).min(red_threshold - yellow_threshold);
                let yellow_top = yellow_threshold + yellow_height;
                let yellow_y = bounds.origin.y + meter_height - (meter_height * yellow_top);
                let yellow_bottom_y =
                    bounds.origin.y + meter_height - (meter_height * yellow_threshold);
                yellow_path.move_to(point(x0, yellow_bottom_y));
                yellow_path.line_to(point(x1, yellow_bottom_y));
                yellow_path.line_to(point(x1, yellow_y));
                yellow_path.line_to(point(x0, yellow_y));
                yellow_path.close();
            }

            if height_ratio > red_threshold {
                let red_height = height_ratio - red_threshold;
                let red_top = red_threshold + red_height;
                let red_y = bounds.origin.y + meter_height - (meter_height * red_top);
                let red_bottom_y = bounds.origin.y + meter_height - (meter_height * red_threshold);
                red_path.move_to(point(x0, red_bottom_y));
                red_path.line_to(point(x1, red_bottom_y));
                red_path.line_to(point(x1, red_y));
                red_path.line_to(point(x0, red_y));
                red_path.close();
            }
        }

        if let Ok(path) = green_path.build() {
            window.paint_path(path, self.colors.low);
        }

        if let Ok(path) = yellow_path.build() {
            window.paint_path(path, self.colors.mid);
        }

        if let Ok(path) = red_path.build() {
            window.paint_path(path, self.colors.high);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SpectrumElement, bar_x_bounds};
    use crate::spectrum::SpectrumColors;
    use d3rs::render2d::{Renderer2D, VelloBackend};
    use gpui::{Element, IntoElement, px};

    #[test]
    fn scratch_height_update_reuses_the_caller_buffer() {
        let element = SpectrumElement::new(vec![-30.0_f32, -60.0, -10.0]);
        let mut scratch = Vec::new();

        for _ in 0..3 {
            element.update_scratch_heights(&mut scratch);
            assert_eq!(scratch.len(), element.magnitudes.len());
        }

        assert!(scratch.capacity() >= 3);
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
    fn bar_gap_bounds_are_centered_and_never_invert() {
        assert_eq!(bar_x_bounds(60.0, 3, 2.0, 0), (1.0, 19.0));
        assert_eq!(bar_x_bounds(60.0, 3, 2.0, 1), (21.0, 39.0));
        assert_eq!(bar_x_bounds(60.0, 3, -2.0, 2), (40.0, 60.0));

        let (start, end) = bar_x_bounds(60.0, 3, 100.0, 1);
        assert_eq!(start, end);
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
        assert!(_same.id().is_some());
    }

    #[test]
    fn default_renderer_contract_is_vello_when_available() {
        let element = SpectrumElement::new(vec![0.0]);
        assert_eq!(element.renderer_2d, Renderer2D::default());
        assert_eq!(element.vello_backend, VelloBackend::default());
    }
}
