//! Android platform backend for GPUI.
//!
//! This crate is initially ported from `itsbalamurali/gpui-mobile` and keeps
//! the Android backend separate from the existing iOS backend so we can test
//! GPUI on Android without perturbing the working iOS showcase.

pub use gpui;

pub mod momentum;
pub mod platform_view;

#[cfg(any(target_os = "android", test))]
mod accessibility;
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

#[cfg(target_os = "android")]
use parking_lot::Mutex;
use std::cell::RefCell;
#[cfg(target_os = "android")]
use std::collections::VecDeque;
#[cfg(target_os = "android")]
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

type TextInputCallbackFn = Box<dyn FnMut(&str)>;

pub static TEXT_INPUT_DIRTY: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "android")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ImeEvent {
    Commit(String),
    SetComposing(String),
    FinishComposing,
    DeleteSurrounding { before: usize, after: usize },
}

#[cfg(target_os = "android")]
static IME_EVENTS: LazyLock<Mutex<VecDeque<ImeEvent>>> =
    LazyLock::new(|| Mutex::new(VecDeque::new()));

#[cfg(target_os = "android")]
pub(crate) fn enqueue_ime_event(event: ImeEvent) {
    IME_EVENTS.lock().push_back(event);
    TEXT_INPUT_DIRTY.store(true, Ordering::Release);
}

#[cfg(target_os = "android")]
pub(crate) fn drain_ime_events() -> Vec<ImeEvent> {
    let mut events = IME_EVENTS.lock();
    if events.is_empty() {
        return Vec::new();
    }
    events.drain(..).collect()
}

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

#[cfg(any(target_os = "android", test))]
pub(crate) fn credential_alias(service: &str, username: &str) -> String {
    // Stable FNV-1a: aliases must survive process restarts and app upgrades.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in service
        .as_bytes()
        .iter()
        .chain([0xff].iter())
        .chain(username.as_bytes())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("gpui_{hash:016x}")
}

#[cfg(test)]
mod credential_alias_tests {
    use super::credential_alias;

    #[test]
    fn credential_alias_is_stable_and_namespaces_service_and_username() {
        assert_eq!(
            credential_alias("https://example.com", "alice"),
            credential_alias("https://example.com", "alice")
        );
        assert_ne!(
            credential_alias("https://example.com", "alice"),
            credential_alias("https://example.com", "bob")
        );
        assert_ne!(
            credential_alias("https://example.com", "alice"),
            credential_alias("https://other.example", "alice")
        );
    }
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
