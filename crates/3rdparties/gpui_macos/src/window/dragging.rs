use cocoa :: { base :: { id } } ;
use gpui :: { FileDropEvent } ;
use objc :: { runtime :: { Object , Sel } } ;
use super::consts::NSDragOperationCopy;
use super::consts::NSDragOperationNone;
use super::get::get_window_state;
use super::mac_window_state::drag_event_position;
use super::mac_window_state::send_file_drop_event;
use super::misc::external_paths_from_event;
use super::types::NSDragOperation;

pub(super) extern "C" fn dragging_entered(this: &Object, _: Sel, dragging_info: id) -> NSDragOperation {
    let window_state = unsafe { get_window_state(this) };
    let position = drag_event_position(&window_state, dragging_info);
    let paths = external_paths_from_event(dragging_info);
    if let Some(event) = paths.map(|paths| FileDropEvent::Entered { position, paths })
        && send_file_drop_event(window_state, event)
    {
        return NSDragOperationCopy;
    }
    NSDragOperationNone
}

pub(super) extern "C" fn dragging_updated(this: &Object, _: Sel, dragging_info: id) -> NSDragOperation {
    let window_state = unsafe { get_window_state(this) };
    let position = drag_event_position(&window_state, dragging_info);
    if send_file_drop_event(window_state, FileDropEvent::Pending { position }) {
        NSDragOperationCopy
    } else {
        NSDragOperationNone
    }
}

pub(super) extern "C" fn dragging_exited(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
    send_file_drop_event(window_state, FileDropEvent::Exited);
}

