//! Null-safe FFI boundary helpers (contract §§4, 13).
//!
//! Every host/guest boundary crossing is total: null pointers are
//! rejected or treated as queries, strings are validated, and state
//! transfer is two-phase (query size with a null buffer, then fill).

// Rust guideline compliant 2026-02-21

use std::ffi::{CStr, c_char};

/// Contains unwinding panics and returns a primitive ABI fallback.
///
/// Bindings must also validate raw memory contracts and poison affected editor
/// state. Abort-mode panics and process faults cannot be recovered. Standard
/// string payloads are dropped; unknown payloads are deliberately leaked because
/// their destructors could panic again across the ABI. No panic hook is changed.
/// Copy results cannot run a user destructor when choosing the fallback.
pub fn ffi_boundary<R: Copy>(fallback: R, callback: impl FnOnce() -> R) -> R {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(callback)) {
        Ok(value) => value,
        Err(payload) => {
            let Err(payload) = payload.downcast::<&'static str>() else {
                return fallback;
            };
            let Err(payload) = payload.downcast::<String>() else {
                return fallback;
            };
            std::mem::forget(payload);
            fallback
        }
    }
}

/// Copy a NUL-terminated C string; `None` for null or invalid UTF-8.
///
/// # Safety
///
/// `value` must be null or point to a valid NUL-terminated C string.
/// The bytes are copied out immediately.
pub unsafe fn optional_c_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }
    // SAFETY: non-null by the check above; the caller guarantees a valid
    // NUL-terminated string, and the bytes are copied out immediately.
    unsafe { CStr::from_ptr(value).to_str().ok().map(str::to_owned) }
}

/// Two-phase out-buffer fill; returns the total required size.
///
/// When `out` is null or `capacity` is 0, nothing is written and the
/// required size is returned. Otherwise up to `capacity` bytes are
/// written, `written` (when non-null) receives the count, and the return
/// value is the total required size — greater than `capacity` means the
/// caller's buffer was too small.
///
/// # Safety
///
/// `out` must be null or writable for `min(bytes.len(), capacity)`
/// bytes; `written` must be null or writable. Source and destination
/// must not overlap.
pub unsafe fn fill_out_buffer(
    bytes: &[u8],
    out: *mut u8,
    capacity: usize,
    written: *mut usize,
) -> usize {
    let required = bytes.len();
    if !written.is_null() {
        // SAFETY: `written` is null-checked; the caller guarantees a
        // writable out count.
        unsafe {
            *written = 0;
        }
    }
    if out.is_null() || capacity == 0 {
        return required;
    }
    let count = required.min(capacity);
    // SAFETY: `out` is non-null with room for `count <= capacity`
    // bytes; source and destination are disjoint by contract.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, count);
        if !written.is_null() {
            *written = count;
        }
    }
    required
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn boundary_preserves_success_and_contains_string_panics() {
        assert_eq!(ffi_boundary(0, || 42), 42);
        assert_eq!(ffi_boundary(0, || panic!("callback panic")), 0);
        assert_eq!(
            ffi_boundary(0, || std::panic::panic_any(String::from("owned panic"))),
            0
        );
    }

    #[test]
    fn boundary_contains_capture_destructor_panics() {
        struct Capture;
        impl Drop for Capture {
            fn drop(&mut self) {
                panic!("capture destructor");
            }
        }
        let capture = Capture;
        assert_eq!(
            ffi_boundary(0, move || {
                drop(capture);
                42
            }),
            0
        );
    }

    #[test]
    fn boundary_and_registry_poison_failed_callbacks() {
        let registry = crate::OwnedEditorRegistry::new();
        let handle = registry.insert(7).unwrap();
        ffi_boundary((), || {
            registry.with::<()>(handle, |_| panic!("editor callback"));
        });
        assert!(registry.is_empty());
        assert!(registry.with(handle, |v| *v).is_none());
    }

    #[test]
    #[cfg_attr(
        miri,
        ignore = "intentional unknown-payload quarantine; run separately with -Zmiri-ignore-leaks"
    )]
    fn boundary_never_runs_unknown_panic_payload_destructor() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        struct Payload(Arc<AtomicBool>);
        impl Drop for Payload {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Relaxed);
                panic!("payload destructor");
            }
        }
        let dropped = Arc::new(AtomicBool::new(false));
        let copy = dropped.clone();
        assert_eq!(
            ffi_boundary(0, move || std::panic::panic_any(Payload(copy))),
            0
        );
        assert!(!dropped.load(Ordering::Relaxed));
    }

    #[test]
    fn null_c_string_is_none() {
        // SAFETY: null is explicitly accepted.
        let value = unsafe { optional_c_string(std::ptr::null()) };
        assert!(value.is_none());
    }

    #[test]
    fn valid_c_string_is_copied() {
        let text = CString::new("cutoff").unwrap();
        // SAFETY: `text` is a live NUL-terminated string.
        let value = unsafe { optional_c_string(text.as_ptr()) };
        assert_eq!(value.as_deref(), Some("cutoff"));
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        let bytes: [u8; 3] = [0x66, 0x80, 0x00];
        // SAFETY: NUL-terminated buffer, live for the call.
        let value = unsafe { optional_c_string(bytes.as_ptr().cast::<c_char>()) };
        assert!(value.is_none());
    }

    #[test]
    fn out_buffer_query_reports_size_without_writing() {
        let bytes = vec![1u8, 2, 3, 4];
        let mut written = 0usize;
        // SAFETY: null buffer is an explicit size query.
        let required =
            unsafe { fill_out_buffer(&bytes, std::ptr::null_mut(), 0, &raw mut written) };
        assert_eq!(required, 4);
        assert_eq!(written, 0);
    }

    #[test]
    fn out_buffer_fill_copies_and_reports() {
        let bytes = vec![1u8, 2, 3, 4];
        let mut out = [0u8; 8];
        let mut written = 0usize;
        // SAFETY: `out` has room for the full payload.
        let required =
            unsafe { fill_out_buffer(&bytes, out.as_mut_ptr(), out.len(), &raw mut written) };
        assert_eq!(required, 4);
        assert_eq!(written, 4);
        assert_eq!(&out[..4], &[1u8, 2, 3, 4]);
    }

    #[test]
    fn out_buffer_truncates_to_capacity() {
        let bytes = vec![1u8, 2, 3, 4];
        let mut out = [0u8; 2];
        let mut written = 0usize;
        // SAFETY: `out` has room for `capacity` bytes.
        let required =
            unsafe { fill_out_buffer(&bytes, out.as_mut_ptr(), out.len(), &raw mut written) };
        assert_eq!(required, 4);
        assert_eq!(written, 2);
        assert_eq!(out, [1u8, 2]);
    }

    #[test]
    fn out_buffer_accepts_null_written() {
        let bytes = vec![7u8];
        let mut out = [0u8; 1];
        // SAFETY: `out` has room; null `written` is explicitly accepted.
        let required =
            unsafe { fill_out_buffer(&bytes, out.as_mut_ptr(), out.len(), std::ptr::null_mut()) };
        assert_eq!(required, 1);
        assert_eq!(out, [7u8]);
    }
}
