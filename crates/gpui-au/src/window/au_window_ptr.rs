use super::au_window::AuWindow;

/// Wrapper for a raw pointer to make it Send+Sync for Mutex storage.
/// The pointer is only accessed from the main thread; see the `Send`/`Sync`
/// impls below for the justification.
pub(super) struct AuWindowPtr(pub(super) *const AuWindow);

impl AuWindowPtr {
    #[cfg(not(test))]
    pub(super) fn assert_main_thread() {
        use objc::{
            class, msg_send,
            runtime::{BOOL, YES},
            sel, sel_impl,
        };
        // SAFETY: `NSThread` and its `isMainThread` class method always
        // exist; the message takes no arguments and returns a `BOOL`.
        unsafe {
            let is_main: BOOL = msg_send![class!(NSThread), isMainThread];
            assert!(
                is_main == YES,
                "AuWindowPtr must only be used on the main thread"
            );
        }
    }

    #[cfg(test)]
    pub(super) fn assert_main_thread() {}
}

// SAFETY: the pointer is only valid on the main thread; every non-test
// build asserts that invariant before registration, unregistration, or
// dereference.
unsafe impl Send for AuWindowPtr {}

// SAFETY: same as `Send` above.
unsafe impl Sync for AuWindowPtr {}

/// Global window pointer, used by `gpui_au_request_frame` to find the window.
/// Single-instance: only one AU GPUI window per process (each AU appex is its own process).
/// Uses Mutex instead of `OnceLock` to support view destruction and re-creation (common in DAWs).
pub(super) static AU_WINDOW: std::sync::Mutex<Option<AuWindowPtr>> = std::sync::Mutex::new(None);

/// Unregister the current AU window (called during destroy).
pub(crate) fn unregister_au_window() {
    AuWindowPtr::assert_main_thread();
    if let Ok(mut guard) = AU_WINDOW.lock() {
        *guard = None;
    }
}
