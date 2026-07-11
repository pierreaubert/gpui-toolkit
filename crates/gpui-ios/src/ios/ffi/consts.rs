use super::app_callback_cell::AppCallbackCell;
use super::asset_source_cell::AssetSourceCell;
use super::ios_app_state::IosAppState;
use super::window_list_wrapper::WindowListWrapper;
use std::backtrace::Backtrace;
use std::panic::{self};
use std::sync::{Once, OnceLock, atomic::AtomicBool};

pub(super) static IOS_APP_STATE: OnceLock<IosAppState> = OnceLock::new();

pub(super) static REQUEST_FRAME_DISABLED: AtomicBool = AtomicBool::new(false);

pub(super) static IOS_PANIC_HOOK: Once = Once::new();

pub(crate) static IOS_WINDOW_LIST: OnceLock<WindowListWrapper> = OnceLock::new();

pub(crate) fn register_window(window: *const super::super::window::IosWindow) {
    if let Some(wrapper) = IOS_WINDOW_LIST.get() {
        unsafe {
            (*wrapper.0.get()).push(window);
            log::info!("GPUI iOS: Registered window {:p}", window);
        }
    }
}

pub(crate) fn unregister_window(window: *const super::super::window::IosWindow) {
    if let Some(wrapper) = IOS_WINDOW_LIST.get() {
        unsafe {
            let windows = &mut *wrapper.0.get();
            windows.retain(|&w| w != window);
            log::info!("GPUI iOS: Unregistered window {:p}", window);
        }
    }
}

pub(crate) fn current_window() -> *const super::super::window::IosWindow {
    if let Some(wrapper) = IOS_WINDOW_LIST.get() {
        unsafe {
            let windows = &*wrapper.0.get();
            if let Some(&window) = windows.last() {
                return window;
            }
        }
    }
    std::ptr::null()
}

pub(crate) fn registered_window(
    window: *const super::super::window::IosWindow,
) -> Option<&'static super::super::window::IosWindow> {
    if window.is_null() {
        return None;
    }

    if let Some(wrapper) = IOS_WINDOW_LIST.get() {
        unsafe {
            let windows = &*wrapper.0.get();
            if windows.iter().any(|&registered| registered == window) {
                return Some(&*window);
            }
        }
    }

    None
}

pub(super) static APP_CALLBACK: OnceLock<AppCallbackCell> = OnceLock::new();

pub(super) static ASSET_SOURCE: OnceLock<AssetSourceCell> = OnceLock::new();

pub(super) fn install_ios_panic_hook() {
    IOS_PANIC_HOOK.call_once(|| {
        let previous_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            log::error!(
                "GPUI iOS: Rust panic: {info}\n{}",
                Backtrace::force_capture()
            );
            previous_hook(info);
        }));
    });
}
