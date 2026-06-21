use crate :: ns_string ;
use cocoa :: { appkit :: { NSScreen } , base :: { id } , foundation :: { NSDictionary , NSUInteger } } ;
use core_graphics :: display :: { CGDirectDisplayID } ;
use objc :: { msg_send , runtime :: { Object , Sel } , sel , sel_impl } ;
use super::get::get_window_state;

pub(super) extern "C" fn display_layer(this: &Object, _: Sel, _: id) {
    let window_state = unsafe { get_window_state(this) };
    let mut lock = window_state.lock();
    if let Some(mut callback) = lock.request_frame_callback.take() {
        lock.renderer.set_presents_with_transaction(true);
        lock.stop_display_link();
        drop(lock);
        callback(Default::default());

        let mut lock = window_state.lock();
        lock.request_frame_callback = Some(callback);
        lock.renderer.set_presents_with_transaction(false);
        lock.start_display_link();
    }
}

pub(super) unsafe fn display_id_for_screen(screen: id) -> CGDirectDisplayID {
    unsafe {
        let device_description = NSScreen::deviceDescription(screen);
        let screen_number_key: id = ns_string("NSScreenNumber");
        let screen_number = device_description.objectForKey_(screen_number_key);
        let screen_number: NSUInteger = msg_send![screen_number, unsignedIntegerValue];
        screen_number as CGDirectDisplayID
    }
}

