//! iOS platform backend for GPUI.
//!
//! Vendored from gpui-mobile (<https://github.com/itsbalamurali/gpui-mobile>)
//! and adapted to work with our pinned GPUI revision (dd9efd9).
//!
//! This crate provides the `IosPlatform` implementation of GPUI's `Platform`
//! trait, enabling GPUI apps to run on iOS with Metal rendering via `gpui_wgpu`.

pub use gpui;

pub mod accessibility;
pub mod haptics;
pub mod hot_reload;
pub mod instrumentation;
pub mod local_auth;
pub mod momentum;
pub mod native;
pub mod notifications;
pub mod pencil;
pub mod platform_view;
pub mod scene_state;
pub mod widget;

// ── System chrome styling ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusBarContentStyle {
    Light,
    #[default]
    Dark,
}

// ── Text input callback ──────────────────────────────────────────────────────

use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

type TextInputCallbackFn = Box<dyn FnMut(&str)>;
type KeyboardLayoutCallbackFn = Box<dyn FnMut()>;
type ThermalStateCallbackFn = Box<dyn FnMut()>;

pub static TEXT_INPUT_DIRTY: AtomicBool = AtomicBool::new(false);

thread_local! {
    static TEXT_INPUT_CALLBACK: RefCell<Option<TextInputCallbackFn>> = RefCell::new(None);
    static KEYBOARD_LAYOUT_CALLBACK: RefCell<Option<KeyboardLayoutCallbackFn>> = RefCell::new(None);
    static THERMAL_STATE_CALLBACK: RefCell<Option<ThermalStateCallbackFn>> = RefCell::new(None);
    static THERMAL_STATE_CALLBACK_GENERATION: Cell<u64> = const { Cell::new(0) };
    static TEXT_INPUT_CALLBACK_GENERATION: Cell<u64> = const { Cell::new(0) };
    static KEYBOARD_LAYOUT_CALLBACK_GENERATION: Cell<u64> = const { Cell::new(0) };
}

pub fn set_text_input_callback(callback: Option<TextInputCallbackFn>) {
    TEXT_INPUT_CALLBACK_GENERATION.with(|generation| {
        generation.set(generation.get().wrapping_add(1));
    });
    TEXT_INPUT_CALLBACK.with(|cb| {
        *cb.borrow_mut() = callback;
    });
}

pub fn dispatch_text_input(text: &str) -> bool {
    TEXT_INPUT_CALLBACK.with(|cb| {
        let generation = TEXT_INPUT_CALLBACK_GENERATION.with(Cell::get);
        let mut callback = cb.borrow_mut().take();
        if let Some(handler) = callback.as_mut() {
            handler(text);
            if TEXT_INPUT_CALLBACK_GENERATION.with(Cell::get) == generation {
                *cb.borrow_mut() = callback;
            }
            TEXT_INPUT_DIRTY.store(true, Ordering::Release);
            true
        } else {
            false
        }
    })
}

pub fn set_keyboard_layout_change_callback(callback: Option<KeyboardLayoutCallbackFn>) {
    KEYBOARD_LAYOUT_CALLBACK_GENERATION.with(|generation| {
        generation.set(generation.get().wrapping_add(1));
    });
    KEYBOARD_LAYOUT_CALLBACK.with(|cb| {
        *cb.borrow_mut() = callback;
    });
}

pub fn dispatch_keyboard_layout_change() -> bool {
    KEYBOARD_LAYOUT_CALLBACK.with(|cb| {
        let generation = KEYBOARD_LAYOUT_CALLBACK_GENERATION.with(Cell::get);
        let mut callback = cb.borrow_mut().take();
        if let Some(handler) = callback.as_mut() {
            handler();
            if KEYBOARD_LAYOUT_CALLBACK_GENERATION.with(Cell::get) == generation {
                *cb.borrow_mut() = callback;
            }
            TEXT_INPUT_DIRTY.store(true, Ordering::Release);
            true
        } else {
            false
        }
    })
}

pub fn set_thermal_state_change_callback(callback: Option<ThermalStateCallbackFn>) {
    THERMAL_STATE_CALLBACK_GENERATION.with(|generation| {
        generation.set(generation.get().wrapping_add(1));
    });
    THERMAL_STATE_CALLBACK.with(|cb| {
        *cb.borrow_mut() = callback;
    });
}

pub fn dispatch_thermal_state_change() -> bool {
    THERMAL_STATE_CALLBACK.with(|cb| {
        let generation = THERMAL_STATE_CALLBACK_GENERATION.with(Cell::get);
        let mut callback = cb.borrow_mut().take();
        if let Some(handler) = callback.as_mut() {
            handler();
            if THERMAL_STATE_CALLBACK_GENERATION.with(Cell::get) == generation {
                *cb.borrow_mut() = callback;
            }
            TEXT_INPUT_DIRTY.store(true, Ordering::Release);
            true
        } else {
            false
        }
    })
}

/// Swift template for hosting GPUI under the `UIScene` life cycle, which
/// iOS/tvOS 27 require (create-only from Xcode 27, general in the
/// following release). Adopting `UIScene` is the host app's decision —
/// nothing here changes existing app-delegate integrations.
///
/// Declare the `gpui_ios_*` C entry points in your bridging header, keep
/// calling `gpui_ios_set_app_callback` + `run_app()` exactly as in the
/// app-delegate flow, and let the scene delegate drive launching,
/// lifecycle forwarding, and the display link from here instead.
pub const IOS_SCENE_LIFECYCLE_TEMPLATE: &str = r#"
import SwiftUI
import UIKit

final class GPUIAppDelegate: NSObject, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        configurationForConnecting connectingSceneSession: UISceneSession,
        options: UIScene.ConnectionOptions
    ) -> UISceneConfiguration {
        let configuration = UISceneConfiguration(
            name: nil, sessionRole: connectingSceneSession.role)
        configuration.delegateClass = GPUIAppSceneDelegate.self
        return configuration
    }
}

final class GPUIAppSceneDelegate: NSObject, UIWindowSceneDelegate {
    var window: UIWindow?
    private var displayLink: CADisplayLink?

    func scene(
        _ scene: UIScene,
        willConnectTo session: UISceneSession,
        options connectionOptions: UIScene.ConnectionOptions
    ) {
        displayLink = CADisplayLink(
            target: self, selector: #selector(pumpFrame))
        displayLink?.add(to: .main, forMode: .common)
        // The first scene to connect claims the GPUI window; this
        // also finishes launching (later scenes are ignored).
        gpui_ios_scene_will_connect(
            UnsafeMutableRawPointer(Unmanaged.passUnretained(scene).toOpaque()))
    }

    func sceneDidDisconnect(_ scene: UIScene) {
        gpui_ios_scene_did_disconnect(
            UnsafeMutableRawPointer(Unmanaged.passUnretained(scene).toOpaque()))
    }

    @objc private func pumpFrame() {
        gpui_ios_request_current_frame()
    }

    func sceneWillEnterForeground(_ scene: UIScene) {
        gpui_ios_lifecycle_forwarding_will_enter_foreground(gpui_ios_get_window())
    }

    func sceneDidBecomeActive(_ scene: UIScene) {
        gpui_ios_lifecycle_forwarding_did_become_active(gpui_ios_get_window())
    }

    func sceneWillResignActive(_ scene: UIScene) {
        gpui_ios_lifecycle_forwarding_will_resign_active(gpui_ios_get_window())
    }

    func sceneDidEnterBackground(_ scene: UIScene) {
        gpui_ios_lifecycle_forwarding_did_enter_background(gpui_ios_get_window())
    }
}

final class GPUIRootViewController: UIViewController {
    override var preferredStatusBarStyle: UIStatusBarStyle { .lightContent }
}

struct GPUIAppRootView: UIViewControllerRepresentable {
    func makeUIViewController(context: Context) -> GPUIRootViewController {
        GPUIRootViewController()
    }

    func updateUIViewController(
        _ viewController: GPUIRootViewController, context: Context
    ) {}
}

@main
struct GPUIHostApp: App {
    @UIApplicationDelegateAdaptor(GPUIAppDelegate.self)
    var appDelegate

    var body: some Scene {
        WindowGroup {
            GPUIAppRootView().ignoresSafeArea()
        }
    }
}
"#;

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

/// Resolve the window keyboard helpers act on: the app window list first,
/// then the current foreign-parent window (embedded editors never enter
/// the app list, so app-only lookup silently no-ops for them).
#[cfg(any(target_os = "ios", target_os = "tvos"))]
fn with_keyboard_window(action: impl FnOnce(&ios::IosWindow)) {
    if let Some(wrapper) = ios::ffi::IOS_WINDOW_LIST.get() {
        // SAFETY: registered windows are boxed for their GPUI lifetime and
        // unregistered before being dropped (see ffi::consts).
        let window = unsafe {
            let windows = &*wrapper.0.get();
            windows.last().map(|&window| &*window)
        };
        if let Some(window) = window {
            action(window);
            return;
        }
    }
    if let Some(window) = ios::ffi::current_foreign_window() {
        action(window);
    }
}

pub fn show_keyboard_with_type(keyboard_type: KeyboardType) {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        with_keyboard_window(|window| window.show_keyboard_with_type(keyboard_type));
    }
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    {
        let _ = keyboard_type;
    }
}

pub fn hide_keyboard() {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        with_keyboard_window(|window| window.hide_keyboard());
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
        dispatch_keyboard_layout_change();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn thermal_state_callback_dispatches_and_clears() {
        use std::sync::atomic::{AtomicU32, Ordering};
        static CALLS: AtomicU32 = AtomicU32::new(0);
        CALLS.store(0, Ordering::Relaxed);
        set_thermal_state_change_callback(Some(Box::new(|| {
            CALLS.fetch_add(1, Ordering::Relaxed);
        })));
        assert!(dispatch_thermal_state_change());
        assert_eq!(CALLS.load(Ordering::Relaxed), 1);
        set_thermal_state_change_callback(None);
        assert!(!dispatch_thermal_state_change());
    }

    #[test]
    fn keyboard_layout_callback_dispatches_on_significant_height_changes() {
        set_keyboard_height(0.0);
        set_keyboard_layout_change_callback(None);

        let calls = Rc::new(Cell::new(0));
        let observed_height = Rc::new(Cell::new(0.0));
        let calls_for_callback = calls.clone();
        let height_for_callback = observed_height.clone();
        set_keyboard_layout_change_callback(Some(Box::new(move || {
            calls_for_callback.set(calls_for_callback.get() + 1);
            height_for_callback.set(keyboard_height());
        })));

        set_keyboard_height(216.0);

        assert_eq!(calls.get(), 1);
        assert_eq!(observed_height.get(), 216.0);

        set_keyboard_height(216.25);

        assert_eq!(calls.get(), 1);

        set_keyboard_layout_change_callback(None);
        set_keyboard_height(0.0);
    }

    #[test]
    fn text_input_callback_can_unregister_itself() {
        let calls = Rc::new(Cell::new(0));
        let calls_for_callback = calls.clone();
        set_text_input_callback(Some(Box::new(move |_| {
            calls_for_callback.set(calls_for_callback.get() + 1);
            set_text_input_callback(None);
        })));

        assert!(dispatch_text_input("first"));
        assert!(!dispatch_text_input("second"));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn keyboard_layout_callback_can_unregister_itself() {
        let calls = Rc::new(Cell::new(0));
        let calls_for_callback = calls.clone();
        set_keyboard_layout_change_callback(Some(Box::new(move || {
            calls_for_callback.set(calls_for_callback.get() + 1);
            set_keyboard_layout_change_callback(None);
        })));

        assert!(dispatch_keyboard_layout_change());
        assert!(!dispatch_keyboard_layout_change());
        assert_eq!(calls.get(), 1);
    }
}
