//! macOS Audio Unit platform backend for GPUI.
//!
//! Embeds GPUI rendering inside AUv3 AudioUnit ViewControllers via Metal/wgpu.
//! Follows the same pattern as gpui-ios but adapted for macOS NSView embedding:
//! - Takes an external NSView (from AUViewController) instead of creating a UIWindow
//! - Renders via CAMetalLayer + wgpu (Metal backend)
//! - Frame rendering driven by Swift CVDisplayLink/timer → `gpui_au_request_frame()`
//! - Mouse/keyboard events forwarded from NSView → FFI → GPUI window

#![cfg(target_os = "macos")]
// FFI functions necessarily dereference raw pointers from C callers
#![allow(clippy::not_unsafe_ptr_arg_deref)]
// objc msg_send! macro generates cfg(cargo-clippy) checks
#![allow(unexpected_cfgs)]
// Callback RefCell<Option<Box<dyn FnMut(...)>>> patterns are inherent to platform trait impls
#![allow(clippy::type_complexity)]

pub use gpui;

mod dispatcher;
mod display;
pub mod ffi;
mod helpers;
mod keychain;
mod params;
mod platform;
mod safety_report;
mod text_system;
mod window;

pub(crate) use dispatcher::AuDispatcher;
pub(crate) use display::AuDisplay;
pub use params::{
    AU_STATE_MAGIC, AU_STATE_MAX_ENTRIES, AU_STATE_VERSION, AuFullState, AuParamError,
    AuParameter, AuParameterTree, AuStateError,
};
pub use platform::AuPlatform;
pub use safety_report::{
    AU_SAFETY_BOUNDARIES, AU_SAFETY_REPORT_SCHEMA_VERSION, AU_SAFETY_REPORT_TYPE, AuSafetyBoundary,
    AuSafetyReport, AuSafetyStatus, au_safety_report,
};
pub(crate) use text_system::AuTextSystem;
pub(crate) use window::AuWindow;
pub use window::{PENDING_VIEW, PendingViewInfo};
