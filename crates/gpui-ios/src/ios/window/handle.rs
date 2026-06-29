use super::super::events::*;
use super::consts::GPUI_WINDOW_IVAR;
use super::ios_window::IosWindow;
use super::misc::ns_string_to_string;
use super::register;
use objc::{
    msg_send,
    runtime::{Object, Sel},
    sel, sel_impl,
};
use std::ffi::c_void;

#[cfg(any(target_os = "ios", target_os = "tvos"))]
pub(super) fn handle_presses(
    view: &mut Object,
    presses: *mut Object,
    _event: *mut Object,
    is_down: bool,
) {
    unsafe {
        let window_ptr: *mut std::ffi::c_void = *view.get_ivar(GPUI_WINDOW_IVAR);
        if window_ptr.is_null() {
            return;
        }
        let window = &*(window_ptr as *const IosWindow);

        let all: *mut Object = msg_send![presses, allObjects];
        let count: usize = msg_send![all, count];

        for i in 0..count {
            let press: *mut Object = msg_send![all, objectAtIndex: i];
            let key: *mut Object = msg_send![press, key];
            if !key.is_null() {
                let key_code: usize = msg_send![key, keyCode];
                let modifier_flags: usize = msg_send![key, modifierFlags];
                let characters: *mut Object = msg_send![key, characters];
                window.handle_key_event_with_characters(
                    key_code as u32,
                    modifier_flags as u32,
                    ns_string_to_string(characters),
                    is_down,
                );
                continue;
            }

            #[cfg(target_os = "tvos")]
            {
                let press_type: i64 = (&*press).send_message(Sel::register("type"), ()).unwrap();
                window.handle_press(press_type, is_down);
            }
        }
    }
}

/// Handle touch events from the GPUIMetalView
pub(super) fn handle_touches(view: &mut Object, touches: *mut Object, event: *mut Object) {
    unsafe {
        // Get the window pointer from the view's ivar
        let window_ptr: *mut std::ffi::c_void = *view.get_ivar(GPUI_WINDOW_IVAR);
        if window_ptr.is_null() {
            log::warn!("GPUI iOS: Touch event but no window pointer set");
            return;
        }

        let window = &*(window_ptr as *const IosWindow);

        // Get all touches from the set
        let all_touches: *mut Object = msg_send![touches, allObjects];
        let count: usize = msg_send![all_touches, count];

        for i in 0..count {
            let touch: *mut Object = msg_send![all_touches, objectAtIndex: i];
            window.handle_touch(touch, event);
        }
    }
}
