//! iOS Window implementation using UIWindow and UIViewController.
//!
//! iOS windows are fundamentally different from desktop windows:
//! - Always fullscreen (or split-screen on iPad)
//! - No title bar or window chrome
//! - Touch-based input
//! - Safe area insets for notch/home indicator
//!
//! The window is backed by a UIWindow containing a UIViewController
//! whose view hosts a CAMetalLayer. Rendering is performed by
//! `gpui_wgpu::WgpuRenderer` which drives wgpu over the Metal backend.


mod accessibility;
mod consts;
mod fallback_atlas;
mod handle;
mod ios_raw_handles;
mod ios_window;
mod misc;
mod register;
mod types;

pub(crate) use ios_window::*;
