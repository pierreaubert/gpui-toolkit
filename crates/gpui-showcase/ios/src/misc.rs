//! iOS showcase FFI helpers.

/// FFI panic guard. Wrap every `extern "C"` body in this so a Rust panic does
/// not unwind across the C ABI (which is UB under the workspace's
/// `panic = "unwind"` strategy).
pub(super) fn ffi_guard<F, R>(f: F) -> R
where
    F: FnOnce() -> R + std::panic::UnwindSafe,
    R: Default,
{
    match std::panic::catch_unwind(f) {
        Ok(r) => r,
        Err(_) => {
            log::error!("[iOS] FFI call panicked; returning default");
            R::default()
        }
    }
}
