//! iOS / tvOS platform implementation for GPUI.
//!
//! Both iOS and tvOS use UIKit + Metal, so this module serves both platforms.
//! The main difference is input handling: iOS uses multi-touch gestures while
//! tvOS uses the Siri Remote's focus engine and button presses.
//! Shared technologies:
//! - Grand Central Dispatch (GCD) for threading
//! - CoreText for text rendering
//! - Metal for GPU rendering
//! - CoreFoundation for many utilities

mod dispatcher;
mod display;
mod events;
pub mod ffi;
mod keychain;
mod platform;
mod text_input;
mod text_system;
mod window;

pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub use platform::*;
pub(crate) use text_system::IosTextSystem;
pub(crate) use window::*;

pub(crate) unsafe fn ns_string_from_str(text: &str) -> *mut objc::runtime::Object {
    use objc::{class, msg_send, sel, sel_impl};
    msg_send![
        class!(NSString),
        stringWithBytes: text.as_ptr() as *const std::ffi::c_void
        length: text.len()
        encoding: 4u64
    ]
}

/// Returns the platform implementation for iOS.
pub fn current_platform(_headless: bool) -> std::rc::Rc<dyn gpui::Platform> {
    std::rc::Rc::new(IosPlatform::new())
}
