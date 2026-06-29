use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::potentiometer::Potentiometer;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// View that tracks value changes
pub(super) struct PotValueChangeTestView {
    pub(super) value: Rc<RefCell<f64>>,
    pub(super) change_count: Arc<AtomicUsize>,
}

impl Render for PotValueChangeTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            Potentiometer::new("change-test-pot")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .unit("%")
                .label("Test")
                .on_change(move |new_value, _window, _cx| {
                    *value_rc.borrow_mut() = new_value;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}
