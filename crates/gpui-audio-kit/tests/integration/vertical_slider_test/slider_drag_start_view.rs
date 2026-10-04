use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::vertical_slider::VerticalSlider;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// View that tracks drag start events
pub(super) struct SliderDragStartView {
    pub(super) started: Arc<AtomicBool>,
    pub(super) y: Rc<RefCell<Option<f32>>>,
    pub(super) value: Rc<RefCell<Option<f64>>>,
}

impl Render for SliderDragStartView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let started = self.started.clone();
        let y_cell = self.y.clone();
        let value_cell = self.value.clone();

        div().size_full().child(
            VerticalSlider::new("drag-start-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Drag Test")
                .on_drag_start(move |y, value, _window, _cx| {
                    started.store(true, Ordering::SeqCst);
                    *y_cell.borrow_mut() = Some(y);
                    *value_cell.borrow_mut() = Some(value);
                }),
        )
    }
}
