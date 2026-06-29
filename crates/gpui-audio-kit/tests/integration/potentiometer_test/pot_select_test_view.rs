use gpui::{Context, Window, div, prelude::*};
use gpui_audio_kit::audio::potentiometer::Potentiometer;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// View that tracks select events
pub(super) struct PotSelectTestView {
    pub(super) selected: Rc<RefCell<bool>>,
    pub(super) select_count: Arc<AtomicUsize>,
}

impl Render for PotSelectTestView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = *self.selected.borrow();
        let selected_rc = self.selected.clone();
        let select_count = self.select_count.clone();

        div().size_full().child(
            Potentiometer::new("select-test-pot")
                .value(50.0)
                .min(0.0)
                .max(100.0)
                .label("Selectable")
                .selected(is_selected)
                .on_select(move |_window, _cx| {
                    *selected_rc.borrow_mut() = true;
                    select_count.fetch_add(1, Ordering::SeqCst);
                }),
        )
    }
}
