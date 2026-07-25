use super::consts::IOS_WINDOW_LIST;
use super::consts::register_window;
use super::consts::unregister_window;
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
