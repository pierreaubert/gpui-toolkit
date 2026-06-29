use super::BUTTON_FOCUS_HANDLES;
use gpui::{App, ElementId, FocusHandle};

pub(super) const MAX_BUTTON_FOCUS_HANDLES: usize = 1024;

pub(super) fn button_focus_handle(id: &ElementId, cx: &mut App) -> FocusHandle {
    BUTTON_FOCUS_HANDLES.with(|handles| {
        let mut handles = handles.borrow_mut();
        while handles.len() > MAX_BUTTON_FOCUS_HANDLES {
            if let Some(key) = handles.keys().next().cloned() {
                handles.remove(&key);
            }
        }
        handles
            .entry(id.clone())
            .or_insert_with(|| cx.focus_handle())
            .clone()
    })
}
