//! iOS platform backend for GPUI.
//!
//! Vendored from gpui-mobile (https://github.com/itsbalamurali/gpui-mobile)
//! and adapted to work with our pinned GPUI revision (dd9efd9).
//!
//! This crate provides the `IosPlatform` implementation of GPUI's `Platform`
//! trait, enabling GPUI apps to run on iOS with Metal rendering via gpui_wgpu.

#![allow(
    unexpected_cfgs,
    reason = "the objc crate macros emit legacy cargo-clippy cfg checks on Apple targets"
)]
#![allow(
    unused_imports,
    reason = "r2factor split modules keep shared imports in generated facade files"
)]
#![allow(
    hidden_glob_reexports,
    reason = "r2factor facades re-export platform modules whose public surface changes by target"
)]

pub use gpui;

pub mod accessibility;
pub mod hot_reload;
pub mod instrumentation;
pub mod momentum;
pub mod native;
pub mod pencil;
pub mod platform_view;
pub mod widget;

// ── System chrome styling ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusBarContentStyle {
    Light,
    #[default]
    Dark,
}

// ── Text input callback ──────────────────────────────────────────────────────

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

type TextInputCallbackFn = Box<dyn FnMut(&str)>;

pub static TEXT_INPUT_DIRTY: AtomicBool = AtomicBool::new(false);

thread_local! {
    static TEXT_INPUT_CALLBACK: RefCell<Option<TextInputCallbackFn>> = RefCell::new(None);
}

pub fn set_text_input_callback(callback: Option<TextInputCallbackFn>) {
    TEXT_INPUT_CALLBACK.with(|cb| {
        *cb.borrow_mut() = callback;
    });
}

pub fn dispatch_text_input(text: &str) -> bool {
    TEXT_INPUT_CALLBACK.with(|cb| {
        if let Some(callback) = cb.borrow_mut().as_mut() {
            callback(text);
            TEXT_INPUT_DIRTY.store(true, Ordering::Release);
            true
        } else {
            false
        }
    })
}

// ── Software keyboard control ────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyboardType {
    #[default]
    Default,
    EmailAddress,
    Phone,
    NumberPad,
    URL,
    Decimal,
}

pub fn show_keyboard() {
    show_keyboard_with_type(KeyboardType::Default);
}

pub fn show_keyboard_with_type(keyboard_type: KeyboardType) {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        if let Some(wrapper) = ios::ffi::IOS_WINDOW_LIST.get() {
            unsafe {
                let windows = &*wrapper.0.get();
                if let Some(&window) = windows.last() {
                    (*window).show_keyboard_with_type(keyboard_type);
                }
            }
        }
    }
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    {
        let _ = keyboard_type;
    }
}

pub fn hide_keyboard() {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        if let Some(wrapper) = ios::ffi::IOS_WINDOW_LIST.get() {
            unsafe {
                let windows = &*wrapper.0.get();
                if let Some(&window) = windows.last() {
                    (*window).hide_keyboard();
                }
            }
        }
    }
}

// ── Keyboard height ─────────────────────────────────────────────────────────

pub static KEYBOARD_HEIGHT_BITS: AtomicU32 = AtomicU32::new(0);

pub fn keyboard_height() -> f32 {
    f32::from_bits(KEYBOARD_HEIGHT_BITS.load(Ordering::Relaxed))
}

pub fn set_keyboard_height(height: f32) {
    let prev = f32::from_bits(KEYBOARD_HEIGHT_BITS.load(Ordering::Relaxed));
    if (prev - height).abs() > 0.5 {
        KEYBOARD_HEIGHT_BITS.store(height.to_bits(), Ordering::Release);
        TEXT_INPUT_DIRTY.store(true, Ordering::Release);
    }
}

// ── Safe area insets ─────────────────────────────────────────────────────────

pub fn safe_area_insets() -> (f32, f32, f32, f32) {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        if let Some(wrapper) = ios::ffi::IOS_WINDOW_LIST.get() {
            unsafe {
                let windows = &*wrapper.0.get();
                if let Some(&window) = windows.last() {
                    return (*window).safe_area_insets();
                }
            }
        }
    }
    (0.0, 0.0, 0.0, 0.0)
}

// ── Scene metrics ───────────────────────────────────────────────────────────

pub fn scene_metrics() -> Option<native::IosSceneMetrics> {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        if let Some(wrapper) = ios::ffi::IOS_WINDOW_LIST.get() {
            unsafe {
                let windows = &*wrapper.0.get();
                if let Some(&window) = windows.last() {
                    return (*window).scene_metrics();
                }
            }
        }
    }
    None
}

pub fn native_bridge_report() -> native::NativeBridgeReport {
    native::NativeBridgeReport::current()
}

pub fn begin_metal_capture(label: &str) -> bool {
    instrumentation::begin_metal_capture(label)
}

pub fn end_metal_capture() {
    instrumentation::end_metal_capture();
}

// ── iOS / tvOS platform module ───────────────────────────────────────────────
// tvOS shares the same UIKit + Metal foundation as iOS, so we reuse the
// platform layer. Input handling is cfg-gated inside the module for the
// focus-engine (tvOS) vs touch (iOS) split.

#[cfg(any(target_os = "ios", target_os = "tvos"))]
pub mod ios;

#[cfg(any(target_os = "ios", target_os = "tvos"))]
pub use ios::{IosPlatform, current_platform};
