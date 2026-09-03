//! Shared helpers for the gpui-au crate.

use objc::runtime::Object;
use std::ffi::CStr;

const DARK_AQUA_APPEARANCE_NAME: &[u8] = b"NSAppearanceNameDarkAqua";

/// Return whether an NSAppearance name is macOS Dark Aqua without creating a
/// comparison NSString on each lookup.
pub(crate) unsafe fn is_dark_aqua_appearance_name(name: *mut Object) -> bool {
    if name.is_null() {
        return false;
    }

    use objc::{msg_send, sel, sel_impl};
    let utf8: *const std::ffi::c_char = msg_send![name, UTF8String];
    if utf8.is_null() {
        return false;
    }

    // `UTF8String` returns a NUL-terminated pointer valid while `name` lives.
    is_dark_aqua_appearance_name_bytes(unsafe { CStr::from_ptr(utf8) }.to_bytes())
}

fn is_dark_aqua_appearance_name_bytes(name: &[u8]) -> bool {
    name == DARK_AQUA_APPEARANCE_NAME
}

/// Create an NSString from a Rust string using an explicit byte length.
///
/// This avoids the `stringWithUTF8String:` contract, which requires a
/// null-terminated C string and rejects interior NUL bytes.
pub(crate) unsafe fn ns_string_from_str(text: &str) -> *mut Object {
    use objc::{class, msg_send, sel, sel_impl};
    msg_send![
        class!(NSString),
        stringWithBytes: text.as_ptr() as *const std::ffi::c_void
        length: text.len()
        encoding: 4u64
    ]
}

/// Verbose diagnostic log for AU lifecycle progress and hot-path tracing.
///
/// AU extensions run inside the host's sandbox: unconditional `NSLog` traffic
/// (per-frame or per-instantiation chatter) shows up in the host console and
/// violates realtime/sandbox hygiene. Verbose logs are emitted in debug builds
/// or when the `verbose-logging` feature is enabled, and compiled out of
/// release builds otherwise. Genuine failures must keep using [`nslog`].
pub(crate) fn nslog_verbose(msg: &[u8]) {
    #[cfg(any(debug_assertions, feature = "verbose-logging"))]
    nslog(msg);
    #[cfg(not(any(debug_assertions, feature = "verbose-logging")))]
    let _ = msg;
}

/// Log via NSLog (always visible in Console.app, unlike Rust's log crate).
/// Accepts a byte slice with explicit length; the bytes are interpreted as UTF-8.
///
/// Reserved for genuine failures (null FFI arguments, renderer creation
/// errors). Progress/tracing logs must use [`nslog_verbose`] instead.
pub(crate) fn nslog(msg: &[u8]) {
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let ns_string: *mut Object = msg_send![
            class!(NSString),
            stringWithBytes: msg.as_ptr() as *const std::ffi::c_void
            length: msg.len()
            encoding: 4u64
        ];
        #[link(name = "Foundation", kind = "framework")]
        unsafe extern "C" {
            fn NSLog(format: *mut Object, ...);
        }
        let fmt: *mut Object = msg_send![class!(NSString), stringWithUTF8String: c"%@".as_ptr()];
        NSLog(fmt, ns_string);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_aqua_name_matches_exactly() {
        assert!(is_dark_aqua_appearance_name_bytes(
            b"NSAppearanceNameDarkAqua"
        ));
        assert!(!is_dark_aqua_appearance_name_bytes(b"NSAppearanceNameAqua"));
    }

    /// Compile-check that `nslog` accepts `&[u8]` (including byte-string literals).
    #[test]
    fn test_nslog_accepts_byte_slice() {
        // This test is primarily a compile-time check; we can't easily assert
        // NSLog output, but we ensure the signature works.
        nslog(b"test message without null terminator");
        nslog(b"");
    }

    /// `nslog_verbose` accepts the same inputs in every build config;
    /// in release without the feature it is a silent no-op.
    #[test]
    fn verbose_logging_helper_links() {
        nslog_verbose(b"verbose probe");
        nslog_verbose(b"");
    }
}
