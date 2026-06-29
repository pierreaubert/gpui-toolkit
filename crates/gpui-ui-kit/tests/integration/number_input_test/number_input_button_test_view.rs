use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::number_input::NumberInput;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// View for testing +/- button clicks
pub(super) struct NumberInputButtonTestView {
    pub(super) value: Rc<RefCell<f64>>,
    pub(super) change_count: Arc<AtomicUsize>,
}

impl Render for NumberInputButtonTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            NumberInput::new("button-test-input")
                .value(current_value)
                .min(0.0)
                .max(100.0)
                .step(5.0)
                .width(150.0)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}
