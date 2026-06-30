//! Android platform backend for GPUI.
//!
//! This crate is initially ported from `itsbalamurali/gpui-mobile` and keeps
//! the Android backend separate from the existing iOS backend so we can test
//! GPUI on Android without perturbing the working iOS showcase.

pub use gpui;

pub mod momentum;
pub mod platform_view;

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "android")]
pub fn current_platform(headless: bool) -> std::rc::Rc<dyn gpui::Platform> {
    android::current_platform(headless)
}

#[cfg(not(target_os = "android"))]
pub fn current_platform(_headless: bool) -> std::rc::Rc<dyn gpui::Platform> {
    panic!("gpui-android can only create a platform on Android")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusBarContentStyle {
    Light,
    #[default]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SystemChromeStyle {
    pub status_bar_color: Option<u32>,
    pub status_bar_style: StatusBarContentStyle,
    pub navigation_bar_color: Option<u32>,
}

impl Default for SystemChromeStyle {
    fn default() -> Self {
        Self {
            status_bar_color: None,
            status_bar_style: StatusBarContentStyle::Dark,
            navigation_bar_color: None,
        }
    }
}

pub fn set_system_chrome(style: &SystemChromeStyle) {
    #[cfg(target_os = "android")]
    {
        android::jni::set_system_chrome(style);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = style;
    }
}

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
    #[cfg(target_os = "android")]
    {
        android::jni::show_keyboard_android(keyboard_type);
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = keyboard_type;
    }
}

pub fn hide_keyboard() {
    #[cfg(target_os = "android")]
    {
        android::jni::hide_keyboard_android();
    }
}

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

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

#[cfg(target_os = "android")]
#[allow(dead_code)]
mod packages {
    pub mod deeplink {
        pub fn notify_deep_link(_url: &str) {}
    }

    pub mod media_session {
        #[derive(Clone, Copy, Debug)]
        pub enum MediaAction {
            Play,
            Pause,
            Stop,
            Next,
            Previous,
        }

        pub fn notify_action(_action: MediaAction) {}
        pub fn notify_seek(_position_ms: u64) {}
    }
}
