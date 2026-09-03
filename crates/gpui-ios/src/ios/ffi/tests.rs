use super::consts::IOS_WINDOW_LIST;
use super::consts::register_window;
use super::consts::unregister_window;
use super::gpui_mod::{
    gpui_ios_attach_to_view, gpui_ios_detach_view, gpui_ios_did_become_active,
    gpui_ios_did_enter_background, gpui_ios_did_finish_launching, gpui_ios_get_window,
    gpui_ios_handle_key_event, gpui_ios_handle_text_input, gpui_ios_handle_touch,
    gpui_ios_refresh_accessibility, gpui_ios_register_platform_view_factory,
    gpui_ios_request_current_frame, gpui_ios_request_frame, gpui_ios_will_enter_foreground,
    gpui_ios_will_resign_active, gpui_ios_will_terminate,
};
use super::window_list_wrapper::WindowListWrapper;

#[test]
fn test_register_and_unregister_window() {
    let _ = IOS_WINDOW_LIST.set(WindowListWrapper(std::cell::UnsafeCell::new(Vec::new())));

    let dummy: *const crate::ios::window::IosWindow = 0x1234 as *const _;
    register_window(dummy);
    assert_eq!(unsafe { &*IOS_WINDOW_LIST.get().unwrap().0.get() }.len(), 1);

    unregister_window(dummy);
    assert!(unsafe { &*IOS_WINDOW_LIST.get().unwrap().0.get() }.is_empty());
}

#[test]
fn exported_host_entry_points_are_null_safe_before_view_attach() {
    assert!(gpui_ios_get_window().is_null());
    assert!(gpui_ios_attach_to_view(std::ptr::null_mut()).is_null());
    gpui_ios_detach_view(std::ptr::null_mut());
    gpui_ios_request_frame(std::ptr::null_mut());
    gpui_ios_request_current_frame();
    gpui_ios_handle_touch(
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null_mut(),
    );
    gpui_ios_handle_text_input(std::ptr::null_mut(), std::ptr::null_mut());
    gpui_ios_handle_key_event(std::ptr::null_mut(), 0, 0, false);
    gpui_ios_refresh_accessibility();
    gpui_ios_did_finish_launching(std::ptr::null_mut());
    gpui_ios_will_enter_foreground(std::ptr::null_mut());
    gpui_ios_did_become_active(std::ptr::null_mut());
    gpui_ios_will_resign_active(std::ptr::null_mut());
    gpui_ios_did_enter_background(std::ptr::null_mut());
    gpui_ios_will_terminate(std::ptr::null_mut());
    assert!(!unsafe {
        gpui_ios_register_platform_view_factory(std::ptr::null(), 0, None, None, None, None, None)
    });
}
