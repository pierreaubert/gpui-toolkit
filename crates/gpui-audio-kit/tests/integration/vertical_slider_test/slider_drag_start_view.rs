use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::vertical_slider::VerticalSlider;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// View that tracks drag start events
pub(super) struct SliderDragStartView {
    pub(super) drag_started: Arc<AtomicBool>,
    pub(super) drag_y: Rc<RefCell<Option<f32>>>,
    pub(super) drag_value: Rc<RefCell<Option<f64>>>,
}

impl Render for SliderDragStartView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let drag_started = self.drag_started.clone();
        let drag_y = self.drag_y.clone();
        let drag_value = self.drag_value.clone();

        div().size_full().child(
            VerticalSlider::new("drag-start-slider")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Drag Test")
                .on_drag_start(move |y, value, _window, _cx| {
                    drag_started.store(true, Ordering::SeqCst);
                    *drag_y.borrow_mut() = Some(y);
                    *drag_value.borrow_mut() = Some(value);
                }),
        )
    }
}
