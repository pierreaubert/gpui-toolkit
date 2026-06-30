use std::panic::{AssertUnwindSafe, catch_unwind};

pub(crate) fn ffi_guard(function: impl FnOnce()) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(function)) {
        if let Some(message) = payload.downcast_ref::<&'static str>() {
            log::error!("panic crossing tvOS FFI boundary: {message}");
        } else if let Some(message) = payload.downcast_ref::<String>() {
            log::error!("panic crossing tvOS FFI boundary: {message}");
        } else {
            log::error!("panic crossing tvOS FFI boundary");
        }
    }
}
