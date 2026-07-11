use super::consts::ACCESSIBILITY_ELEMENT_CLASS_REGISTERED;
use super::consts::GPUI_WINDOW_IVAR;
use super::consts::METAL_VIEW_CLASS_REGISTERED;
use super::consts::STATUS_BAR_STYLE;
use super::consts::TEXT_INPUT_VIEW_CLASS_REGISTERED;
use super::consts::VC_CLASS_REGISTERED;
use super::consts::WINDOW_CLASS_REGISTERED;
#[cfg(target_os = "ios")]
use super::handle::handle_indirect_scroll;
#[cfg(any(target_os = "ios", target_os = "tvos"))]
use super::handle::handle_presses;
use super::handle::handle_touches;
use super::ios_window::IosWindow;
use super::misc::dispatch_accessibility_element_action;
#[cfg(target_os = "ios")]
use objc::runtime::Protocol;
use objc::{
    class,
    declare::ClassDecl,
    msg_send,
    runtime::{BOOL, Class, NO, Object, Sel, YES},
    sel, sel_impl,
};
#[cfg(target_os = "ios")]
use std::io::Write;

#[cfg(target_os = "ios")]
pub(super) fn input_diag_log(message: &str) {
    log::info!("GPUI iOS input diag: {message}");
    eprintln!("{message}");
    let path = std::env::temp_dir().join("gpui-ios-input-diag.log");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{message}");
    }
}

/// Register a UIWindow subclass so we can diagnose whether indirect scroll
/// input reaches UIKit before any view or gesture recognizer filtering.
#[cfg(target_os = "ios")]
pub(super) fn register_window_class() -> &'static Class {
    WINDOW_CLASS_REGISTERED.call_once(|| {
        let superclass = class!(UIWindow);
        let mut decl = ClassDecl::new("GPUIWindow", superclass).unwrap();

        extern "C" fn send_event(this: &Object, _sel: Sel, event: *mut Object) {
            unsafe {
                if !event.is_null() {
                    let event_type: i64 = msg_send![event, type];
                    let event_subtype: i64 = msg_send![event, subtype];
                    let modifiers: usize = msg_send![event, modifierFlags];
                    let buttons: isize = msg_send![event, buttonMask];
                    input_diag_log(&format!(
                        "window sendEvent type={event_type} subtype={event_subtype} modifiers=0x{modifiers:x} buttons=0x{buttons:x}"
                    ));
                }

                let superclass = class!(UIWindow);
                let _: () = msg_send![super(this, superclass), sendEvent: event];
            }
        }

        unsafe {
            decl.add_method(
                sel!(sendEvent:),
                send_event as extern "C" fn(&Object, Sel, *mut Object),
            );
        }

        decl.register();
    });

    class!(GPUIWindow)
}

/// Register a custom UIViewController subclass that allows overriding
/// `preferredStatusBarStyle` at runtime.
pub(super) fn register_view_controller_class() -> &'static Class {
    VC_CLASS_REGISTERED.call_once(|| {
        let superclass = class!(UIViewController);
        let mut decl = ClassDecl::new("GPUIViewController", superclass).unwrap();

        // Override preferredStatusBarStyle
        extern "C" fn preferred_status_bar_style(_this: &Object, _sel: Sel) -> isize {
            let style = STATUS_BAR_STYLE.load(std::sync::atomic::Ordering::Relaxed);
            if style == 1 {
                1 // UIStatusBarStyleLightContent
            } else {
                3 // UIStatusBarStyleDarkContent (iOS 13+)
            }
        }

        // Override viewDidLayoutSubviews — called by UIKit on rotation,
        // split-screen changes, and any other layout pass.
        extern "C" fn view_did_layout_subviews(this: &Object, _sel: Sel) {
            // Call super
            unsafe {
                let superclass = class!(UIViewController);
                let _: () = msg_send![super(this, superclass), viewDidLayoutSubviews];
            }

            // Notify all registered GPUI windows about the layout change.
            if let Some(wrapper) = super::super::ffi::IOS_WINDOW_LIST.get() {
                unsafe {
                    let windows = &*wrapper.0.get();
                    for &window_ptr in windows.iter() {
                        if !window_ptr.is_null() {
                            let window = &*window_ptr;
                            window.handle_layout_change();
                        }
                    }
                }
            }
        }

        unsafe {
            decl.add_method(
                sel!(preferredStatusBarStyle),
                preferred_status_bar_style as extern "C" fn(&Object, Sel) -> isize,
            );
            decl.add_method(
                sel!(viewDidLayoutSubviews),
                view_did_layout_subviews as extern "C" fn(&Object, Sel),
            );
        }

        decl.register();
    });

    class!(GPUIViewController)
}

/// Register a custom UIView subclass that uses CAMetalLayer as its backing layer.
/// This is required for Metal rendering on iOS.
pub(super) fn register_metal_view_class() -> &'static Class {
    METAL_VIEW_CLASS_REGISTERED.call_once(|| {
        let superclass = class!(UIView);
        let mut decl = ClassDecl::new("GPUIMetalView", superclass).unwrap();

        #[cfg(target_os = "ios")]
        if let Some(protocol) = Protocol::get("UIGestureRecognizerDelegate") {
            decl.add_protocol(protocol);
        }

        // Add ivar to store window pointer for touch handling
        decl.add_ivar::<*mut std::ffi::c_void>(GPUI_WINDOW_IVAR);

        // Override layerClass to return CAMetalLayer
        extern "C" fn layer_class(_self: &Class, _sel: Sel) -> *const Class {
            class!(CAMetalLayer) as *const Class
        }

        // Touch handling methods (iOS touch + tvOS Siri Remote touch surface)
        extern "C" fn touches_began(
            this: &mut Object,
            _sel: Sel,
            touches: *mut Object,
            event: *mut Object,
        ) {
            handle_touches(this, touches, event);
        }

        extern "C" fn touches_moved(
            this: &mut Object,
            _sel: Sel,
            touches: *mut Object,
            event: *mut Object,
        ) {
            handle_touches(this, touches, event);
        }

        extern "C" fn touches_ended(
            this: &mut Object,
            _sel: Sel,
            touches: *mut Object,
            event: *mut Object,
        ) {
            handle_touches(this, touches, event);
        }

        extern "C" fn touches_cancelled(
            this: &mut Object,
            _sel: Sel,
            touches: *mut Object,
            event: *mut Object,
        ) {
            handle_touches(this, touches, event);
        }

        #[cfg(target_os = "ios")]
        extern "C" fn handle_indirect_scroll_gesture(
            this: &mut Object,
            _sel: Sel,
            recognizer: *mut Object,
        ) {
            handle_indirect_scroll(this, recognizer);
        }

        #[cfg(target_os = "ios")]
        extern "C" fn gesture_should_receive_event(
            _this: &Object,
            _sel: Sel,
            _recognizer: *mut Object,
            event: *mut Object,
        ) -> BOOL {
            if event.is_null() {
                return NO;
            }

            unsafe {
                let event_type: i64 = msg_send![event, type];
                let event_subtype: i64 = msg_send![event, subtype];
                let modifiers: usize = msg_send![event, modifierFlags];
                let buttons: isize = msg_send![event, buttonMask];
                input_diag_log(&format!(
                    "indirect_scroll delegate event type={event_type} subtype={event_subtype} modifiers=0x{modifiers:x} buttons=0x{buttons:x}"
                ));
                YES
            }
        }

        #[cfg(target_os = "ios")]
        extern "C" fn gesture_should_receive_touch(
            _this: &Object,
            _sel: Sel,
            _recognizer: *mut Object,
            touch: *mut Object,
        ) -> BOOL {
            if touch.is_null() {
                return NO;
            }

            unsafe {
                let touch_type: i64 = msg_send![touch, type];
                input_diag_log(&format!(
                    "indirect_scroll delegate touch type={touch_type}"
                ));
                if touch_type == 0 { NO } else { YES }
            }
        }

        #[cfg(target_os = "ios")]
        extern "C" fn gesture_should_receive_press(
            _this: &Object,
            _sel: Sel,
            _recognizer: *mut Object,
            press: *mut Object,
        ) -> BOOL {
            if press.is_null() {
                return NO;
            }

            unsafe {
                let press_type: i64 = msg_send![press, type];
                input_diag_log(&format!(
                    "indirect_scroll delegate press type={press_type}"
                ));
                YES
            }
        }

        #[cfg(target_os = "ios")]
        extern "C" fn gesture_should_recognize_simultaneously(
            _this: &Object,
            _sel: Sel,
            _recognizer: *mut Object,
            _other: *mut Object,
        ) -> BOOL {
            YES
        }

        // iOS/tvOS press handling — hardware keyboards on iOS and Siri Remote
        // buttons on tvOS. Maps button presses to GPUI keyboard/mouse events.
        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        extern "C" fn presses_began(
            this: &mut Object,
            _sel: Sel,
            presses: *mut Object,
            event: *mut Object,
        ) {
            handle_presses(this, presses, event, true, "began");
        }

        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        extern "C" fn presses_changed(
            this: &mut Object,
            _sel: Sel,
            presses: *mut Object,
            event: *mut Object,
        ) {
            handle_presses(this, presses, event, true, "changed");
        }

        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        extern "C" fn presses_ended(
            this: &mut Object,
            _sel: Sel,
            presses: *mut Object,
            event: *mut Object,
        ) {
            handle_presses(this, presses, event, false, "ended");
        }

        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        extern "C" fn presses_cancelled(
            this: &mut Object,
            _sel: Sel,
            presses: *mut Object,
            event: *mut Object,
        ) {
            handle_presses(this, presses, event, false, "cancelled");
        }

        // iOS hardware keyboard events are delivered through the first
        // responder chain, so the render view must be eligible when no text
        // input is active.
        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        extern "C" fn can_become_first_responder(_this: &Object, _sel: Sel) -> BOOL {
            YES
        }

        // On tvOS the view must be focusable for UIPress events to arrive.
        #[cfg(target_os = "tvos")]
        extern "C" fn can_become_focused(_this: &Object, _sel: Sel) -> BOOL {
            YES
        }

        unsafe {
            // Add class method for layerClass
            decl.add_class_method(
                sel!(layerClass),
                layer_class as extern "C" fn(&Class, Sel) -> *const Class,
            );

            // Add touch handling instance methods
            decl.add_method(
                sel!(touchesBegan:withEvent:),
                touches_began as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
            );
            decl.add_method(
                sel!(touchesMoved:withEvent:),
                touches_moved as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
            );
            decl.add_method(
                sel!(touchesEnded:withEvent:),
                touches_ended as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
            );
            decl.add_method(
                sel!(touchesCancelled:withEvent:),
                touches_cancelled as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
            );

            #[cfg(target_os = "ios")]
            decl.add_method(
                sel!(handleIndirectScroll:),
                handle_indirect_scroll_gesture as extern "C" fn(&mut Object, Sel, *mut Object),
            );
            #[cfg(target_os = "ios")]
            decl.add_method(
                sel!(gestureRecognizer:shouldReceiveEvent:),
                gesture_should_receive_event
                    as extern "C" fn(&Object, Sel, *mut Object, *mut Object) -> BOOL,
            );
            #[cfg(target_os = "ios")]
            decl.add_method(
                sel!(gestureRecognizer:shouldReceiveTouch:),
                gesture_should_receive_touch
                    as extern "C" fn(&Object, Sel, *mut Object, *mut Object) -> BOOL,
            );
            #[cfg(target_os = "ios")]
            decl.add_method(
                sel!(gestureRecognizer:shouldReceivePress:),
                gesture_should_receive_press
                    as extern "C" fn(&Object, Sel, *mut Object, *mut Object) -> BOOL,
            );
            #[cfg(target_os = "ios")]
            decl.add_method(
                sel!(gestureRecognizer:shouldRecognizeSimultaneouslyWithGestureRecognizer:),
                gesture_should_recognize_simultaneously
                    as extern "C" fn(&Object, Sel, *mut Object, *mut Object) -> BOOL,
            );

            // iOS/tvOS: press handling for hardware keyboards and Siri Remote
            #[cfg(any(target_os = "ios", target_os = "tvos"))]
            {
                decl.add_method(
                    sel!(pressesBegan:withEvent:),
                    presses_began as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
                );
                decl.add_method(
                    sel!(pressesChanged:withEvent:),
                    presses_changed as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
                );
                decl.add_method(
                    sel!(pressesEnded:withEvent:),
                    presses_ended as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
                );
                decl.add_method(
                    sel!(pressesCancelled:withEvent:),
                    presses_cancelled as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
                );
                decl.add_method(
                    sel!(canBecomeFirstResponder),
                    can_become_first_responder as extern "C" fn(&Object, Sel) -> BOOL,
                );
                #[cfg(target_os = "tvos")]
                {
                    decl.add_method(
                        sel!(canBecomeFocused),
                        can_become_focused as extern "C" fn(&Object, Sel) -> BOOL,
                    );
                }
            }
        }

        decl.register();
    });

    class!(GPUIMetalView)
}

/// Register a `UIAccessibilityElement` subclass that can route VoiceOver
/// actions back into GPUI's accessibility action callback.
pub(super) fn register_accessibility_element_class() -> &'static Class {
    ACCESSIBILITY_ELEMENT_CLASS_REGISTERED.call_once(|| {
        let superclass = class!(UIAccessibilityElement);
        let mut decl = ClassDecl::new("GPUIAccessibilityElement", superclass).unwrap();

        extern "C" fn accessibility_activate(this: &Object, _sel: Sel) -> BOOL {
            dispatch_accessibility_element_action(
                this,
                crate::accessibility::IosAccessibilityAction::Activate,
            ) as BOOL
        }

        extern "C" fn accessibility_increment(this: &Object, _sel: Sel) {
            let _ = dispatch_accessibility_element_action(
                this,
                crate::accessibility::IosAccessibilityAction::Increment,
            );
        }

        extern "C" fn accessibility_decrement(this: &Object, _sel: Sel) {
            let _ = dispatch_accessibility_element_action(
                this,
                crate::accessibility::IosAccessibilityAction::Decrement,
            );
        }

        extern "C" fn accessibility_perform_escape(this: &Object, _sel: Sel) -> BOOL {
            dispatch_accessibility_element_action(
                this,
                crate::accessibility::IosAccessibilityAction::Escape,
            ) as BOOL
        }

        extern "C" fn accessibility_perform_magic_tap(this: &Object, _sel: Sel) -> BOOL {
            dispatch_accessibility_element_action(
                this,
                crate::accessibility::IosAccessibilityAction::MagicTap,
            ) as BOOL
        }

        unsafe {
            decl.add_method(
                sel!(accessibilityActivate),
                accessibility_activate as extern "C" fn(&Object, Sel) -> BOOL,
            );
            decl.add_method(
                sel!(accessibilityIncrement),
                accessibility_increment as extern "C" fn(&Object, Sel),
            );
            decl.add_method(
                sel!(accessibilityDecrement),
                accessibility_decrement as extern "C" fn(&Object, Sel),
            );
            decl.add_method(
                sel!(accessibilityPerformEscape),
                accessibility_perform_escape as extern "C" fn(&Object, Sel) -> BOOL,
            );
            decl.add_method(
                sel!(accessibilityPerformMagicTap),
                accessibility_perform_magic_tap as extern "C" fn(&Object, Sel) -> BOOL,
            );
        }

        decl.register();
    });

    class!(GPUIAccessibilityElement)
}

/// Register a custom UIView subclass that implements UIKeyInput protocol.
///
/// iOS requires the first-responder view to conform to `UIKeyInput` in order
/// for the software keyboard to actually route typed characters back to the
/// app.  Without this, `becomeFirstResponder` silently fails and no keyboard
/// appears.
///
/// The three required methods:
/// - `hasText` → always returns YES (simplifies things; no harm)
/// - `insertText:` → forwards the text to `IosWindow::handle_text_input`
/// - `deleteBackward` → dispatches a backspace via `crate::dispatch_text_input`
pub(super) fn register_text_input_view_class() -> &'static Class {
    TEXT_INPUT_VIEW_CLASS_REGISTERED.call_once(|| {
        let superclass = class!(UIView);
        let mut decl = ClassDecl::new("GPUITextInputView", superclass).unwrap();

        // Declare protocol conformance so iOS knows this view can receive
        // keyboard text input.
        if let Some(protocol) = objc::runtime::Protocol::get("UIKeyInput") {
            decl.add_protocol(protocol);
        }

        // Store the IosWindow pointer so callbacks can reach the Rust window.
        decl.add_ivar::<*mut std::ffi::c_void>(GPUI_WINDOW_IVAR);

        // UITextInputTraits property storage — UIView doesn't provide these,
        // but iOS reads them from the first responder to configure the keyboard.
        decl.add_ivar::<isize>("_keyboardType"); // UIKeyboardType
        decl.add_ivar::<isize>("_autocorrectionType"); // UITextAutocorrectionType
        decl.add_ivar::<isize>("_autocapitalizationType"); // UITextAutocapitalizationType

        // --- UIKeyInput protocol methods ---

        // BOOL hasText
        extern "C" fn has_text(_this: &Object, _sel: Sel) -> BOOL {
            YES
        }

        // void insertText:(NSString *)text
        extern "C" fn insert_text(this: &Object, _sel: Sel, text: *mut Object) {
            unsafe {
                let window_ptr: *mut std::ffi::c_void = *this.get_ivar(GPUI_WINDOW_IVAR);
                if window_ptr.is_null() || text.is_null() {
                    return;
                }
                let window = &*(window_ptr as *const IosWindow);
                window.handle_text_input(text);
            }
        }

        // void deleteBackward
        extern "C" fn delete_backward(this: &Object, _sel: Sel) {
            unsafe {
                let window_ptr: *mut std::ffi::c_void = *this.get_ivar(GPUI_WINDOW_IVAR);
                if window_ptr.is_null() {
                    return;
                }
                let window = &*(window_ptr as *const IosWindow);
                window.handle_delete_backward();
            }
        }

        // canBecomeFirstResponder must return YES
        extern "C" fn can_become_first_responder(_this: &Object, _sel: Sel) -> BOOL {
            YES
        }

        // Hardware keyboard press handling for iOS simulator/devices.
        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        extern "C" fn presses_began(
            this: &mut Object,
            _sel: Sel,
            presses: *mut Object,
            event: *mut Object,
        ) {
            handle_presses(this, presses, event, true, "began");
        }

        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        extern "C" fn presses_changed(
            this: &mut Object,
            _sel: Sel,
            presses: *mut Object,
            event: *mut Object,
        ) {
            handle_presses(this, presses, event, true, "changed");
        }

        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        extern "C" fn presses_ended(
            this: &mut Object,
            _sel: Sel,
            presses: *mut Object,
            event: *mut Object,
        ) {
            handle_presses(this, presses, event, false, "ended");
        }

        #[cfg(any(target_os = "ios", target_os = "tvos"))]
        extern "C" fn presses_cancelled(
            this: &mut Object,
            _sel: Sel,
            presses: *mut Object,
            event: *mut Object,
        ) {
            handle_presses(this, presses, event, false, "cancelled");
        }

        // --- UITextInputTraits property accessors ---
        extern "C" fn get_keyboard_type(this: &Object, _sel: Sel) -> isize {
            unsafe { *this.get_ivar::<isize>("_keyboardType") }
        }
        extern "C" fn set_keyboard_type(this: &mut Object, _sel: Sel, val: isize) {
            unsafe {
                this.set_ivar::<isize>("_keyboardType", val);
            }
        }
        extern "C" fn get_autocorrection_type(this: &Object, _sel: Sel) -> isize {
            unsafe { *this.get_ivar::<isize>("_autocorrectionType") }
        }
        extern "C" fn set_autocorrection_type(this: &mut Object, _sel: Sel, val: isize) {
            unsafe {
                this.set_ivar::<isize>("_autocorrectionType", val);
            }
        }
        extern "C" fn get_autocapitalization_type(this: &Object, _sel: Sel) -> isize {
            unsafe { *this.get_ivar::<isize>("_autocapitalizationType") }
        }
        extern "C" fn set_autocapitalization_type(this: &mut Object, _sel: Sel, val: isize) {
            unsafe {
                this.set_ivar::<isize>("_autocapitalizationType", val);
            }
        }

        unsafe {
            decl.add_method(
                sel!(hasText),
                has_text as extern "C" fn(&Object, Sel) -> BOOL,
            );
            decl.add_method(
                sel!(insertText:),
                insert_text as extern "C" fn(&Object, Sel, *mut Object),
            );
            decl.add_method(
                sel!(deleteBackward),
                delete_backward as extern "C" fn(&Object, Sel),
            );
            decl.add_method(
                sel!(canBecomeFirstResponder),
                can_become_first_responder as extern "C" fn(&Object, Sel) -> BOOL,
            );
            #[cfg(any(target_os = "ios", target_os = "tvos"))]
            {
                decl.add_method(
                    sel!(pressesBegan:withEvent:),
                    presses_began as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
                );
                decl.add_method(
                    sel!(pressesChanged:withEvent:),
                    presses_changed as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
                );
                decl.add_method(
                    sel!(pressesEnded:withEvent:),
                    presses_ended as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
                );
                decl.add_method(
                    sel!(pressesCancelled:withEvent:),
                    presses_cancelled as extern "C" fn(&mut Object, Sel, *mut Object, *mut Object),
                );
            }

            // UITextInputTraits property methods
            decl.add_method(
                sel!(keyboardType),
                get_keyboard_type as extern "C" fn(&Object, Sel) -> isize,
            );
            decl.add_method(
                sel!(setKeyboardType:),
                set_keyboard_type as extern "C" fn(&mut Object, Sel, isize),
            );
            decl.add_method(
                sel!(autocorrectionType),
                get_autocorrection_type as extern "C" fn(&Object, Sel) -> isize,
            );
            decl.add_method(
                sel!(setAutocorrectionType:),
                set_autocorrection_type as extern "C" fn(&mut Object, Sel, isize),
            );
            decl.add_method(
                sel!(autocapitalizationType),
                get_autocapitalization_type as extern "C" fn(&Object, Sel) -> isize,
            );
            decl.add_method(
                sel!(setAutocapitalizationType:),
                set_autocapitalization_type as extern "C" fn(&mut Object, Sel, isize),
            );
        }

        decl.register();
    });

    class!(GPUITextInputView)
}
