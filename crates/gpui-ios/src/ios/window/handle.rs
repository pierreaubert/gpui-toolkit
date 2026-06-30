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
    event: *mut Object,
    is_down: bool,
    phase_name: &'static str,
) {
    unsafe {
        let window_ptr: *mut std::ffi::c_void = *view.get_ivar(GPUI_WINDOW_IVAR);
        if window_ptr.is_null() {
            return;
        }
        let window = &*(window_ptr as *const IosWindow);

        let all: *mut Object = msg_send![presses, allObjects];
        let count: usize = msg_send![all, count];
        let event_type: i64 = msg_send![event, type];
        let event_subtype: i64 = msg_send![event, subtype];
        let event_modifiers: usize = msg_send![event, modifierFlags];
        let button_mask: isize = msg_send![event, buttonMask];
        eprintln!(
            "GPUI iOS diag: presses {phase_name} count={count} event_type={event_type} subtype={event_subtype} modifiers=0x{event_modifiers:x} button_mask=0x{button_mask:x}"
        );

        for i in 0..count {
            let press: *mut Object = msg_send![all, objectAtIndex: i];
            let press_phase: i64 = msg_send![press, phase];
            let press_type: i64 = msg_send![press, type];
            let force: f64 = msg_send![press, force];
            let key: *mut Object = msg_send![press, key];
            if !key.is_null() {
                let key_code: usize = msg_send![key, keyCode];
                let modifier_flags: usize = msg_send![key, modifierFlags];
                let characters: *mut Object = msg_send![key, characters];
                eprintln!(
                    "GPUI iOS diag: press[{i}] phase={press_phase} type={press_type} force={force:.3} key_code=0x{key_code:x} key_modifiers=0x{modifier_flags:x}"
                );
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
        #[cfg(target_os = "ios")]
        {
            let event_type: i64 = msg_send![event, type];
            let event_subtype: i64 = msg_send![event, subtype];
            register::input_diag_log(&format!(
                "view touches event_type={event_type} subtype={event_subtype} count={count}"
            ));
        }

        for i in 0..count {
            let touch: *mut Object = msg_send![all_touches, objectAtIndex: i];
            window.handle_touch(touch, event);
        }
    }
}

/// Handle indirect pointer scrolling from a trackpad or mouse wheel.
pub(super) fn handle_indirect_scroll(view: &mut Object, recognizer: *mut Object) {
    unsafe {
        let window_ptr: *mut std::ffi::c_void = *view.get_ivar(GPUI_WINDOW_IVAR);
        if window_ptr.is_null() {
            log::warn!("GPUI iOS: Indirect scroll event but no window pointer set");
            return;
        }

        let state: i64 = msg_send![recognizer, state];
        let touches: usize = msg_send![recognizer, numberOfTouches];
        let modifiers: usize = msg_send![recognizer, modifierFlags];
        let buttons: isize = msg_send![recognizer, buttonMask];
        register::input_diag_log(&format!(
            "indirect_scroll callback state={state} touches={touches} modifiers=0x{modifiers:x} buttons=0x{buttons:x}"
        ));

        let window = &*(window_ptr as *const IosWindow);
        window.handle_indirect_scroll(recognizer);
    }
}
