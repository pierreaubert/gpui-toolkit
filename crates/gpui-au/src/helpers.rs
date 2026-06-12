//! Shared helpers for the gpui-au crate.

use objc::runtime::Object;

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

/// Log via NSLog (always visible in Console.app, unlike Rust's log crate).
/// Accepts a byte slice with explicit length; the bytes are interpreted as UTF-8.
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

    /// Compile-check that `nslog` accepts `&[u8]` (including byte-string literals).
    #[test]
    fn test_nslog_accepts_byte_slice() {
        // This test is primarily a compile-time check; we can't easily assert
        // NSLog output, but we ensure the signature works.
        nslog(b"test message without null terminator");
        nslog(b"");
    }
}
