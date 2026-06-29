use gpui::{Context, Window, div, prelude::*};
use gpui_ui_kit::number_input::NumberInput;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// View for testing click-to-edit
pub(super) struct NumberInputEditTestView {
    pub(super) value: Rc<RefCell<f64>>,
    pub(super) change_count: Arc<AtomicUsize>,
}

impl Render for NumberInputEditTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let current_value = *self.value.borrow();
        let value_rc = self.value.clone();
        let change_count = self.change_count.clone();

        div().size_full().child(
            NumberInput::new("edit-test-input")
                .value(current_value)
                .min(0.0)
                .max(1000.0)
                .step(1.0)
                .width(150.0)
                .on_change(move |new_val, _window, _cx| {
                    *value_rc.borrow_mut() = new_val;
                    change_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}
