//! Haptic feedback bridge (`UIFeedbackGenerator`).
//!
//! Thin, host-testable wrapper over iOS/tvOS haptics: impact taps,
//! selection ticks, and notification outcomes. On other platforms every
//! entry point reports unavailability instead of failing.

/// Impact intensity, mirroring `UIImpactFeedbackGenerator.FeedbackStyle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HapticImpactStyle {
    #[default]
    Medium,
    Light,
    Heavy,
    Soft,
    Rigid,
}

impl HapticImpactStyle {
    /// Raw `UIImpactFeedbackStyle` value (`Light = 0` … `Rigid = 4`).
    pub const fn raw_value(self) -> i64 {
        match self {
            Self::Light => 0,
            Self::Medium => 1,
            Self::Heavy => 2,
            Self::Soft => 3,
            Self::Rigid => 4,
        }
    }
}

/// Notification outcome, mirroring `UINotificationFeedbackGenerator.FeedbackType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HapticNotificationKind {
    #[default]
    Success,
    Warning,
    Error,
}

impl HapticNotificationKind {
    /// Raw `UINotificationFeedbackType` value (`Success = 0` … `Error = 2`).
    pub const fn raw_value(self) -> i64 {
        match self {
            Self::Success => 0,
            Self::Warning => 1,
            Self::Error => 2,
        }
    }
}

/// One haptic event to play.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HapticFeedback {
    Impact(HapticImpactStyle),
    Selection,
    Notification(HapticNotificationKind),
}

/// Whether the current platform can play haptics (iOS/tvOS only).
pub fn is_haptics_available() -> bool {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        true
    }
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    {
        false
    }
}

/// Play `feedback`, returning `true` when it was dispatched to UIKit.
///
/// Never fails: on platforms without haptics this is a no-op returning
/// `false`, so callers can gate follow-up work on the return value.
pub fn trigger_haptic(feedback: HapticFeedback) -> bool {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        unsafe {
            trigger_haptic_impl(feedback);
        }
        true
    }
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    {
        let _ = feedback;
        false
    }
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
unsafe fn trigger_haptic_impl(feedback: HapticFeedback) {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    match feedback {
        HapticFeedback::Impact(style) => {
            // SAFETY: `UIImpactFeedbackGenerator` exists on all supported
            // iOS/tvOS versions; the generator is released after firing.
            let generator: *mut Object = msg_send![class!(UIImpactFeedbackGenerator), alloc];
            let generator: *mut Object = msg_send![generator, initWithStyle: style.raw_value()];
            if generator.is_null() {
                return;
            }
            let _: () = msg_send![generator, prepare];
            let _: () = msg_send![generator, impactOccurred];
            let _: () = msg_send![generator, release];
        }
        HapticFeedback::Selection => {
            // SAFETY: same lifetime contract as above.
            let generator: *mut Object = msg_send![class!(UISelectionFeedbackGenerator), alloc];
            let generator: *mut Object = msg_send![generator, init];
            if generator.is_null() {
                return;
            }
            let _: () = msg_send![generator, prepare];
            let _: () = msg_send![generator, selectionChanged];
            let _: () = msg_send![generator, release];
        }
        HapticFeedback::Notification(kind) => {
            // SAFETY: same lifetime contract as above.
            let generator: *mut Object = msg_send![class!(UINotificationFeedbackGenerator), alloc];
            let generator: *mut Object = msg_send![generator, init];
            if generator.is_null() {
                return;
            }
            let _: () = msg_send![generator, prepare];
            let _: () = msg_send![generator, notificationOccurred: kind.raw_value()];
            let _: () = msg_send![generator, release];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impact_styles_map_to_ui_feedback_constants() {
        assert_eq!(HapticImpactStyle::Light.raw_value(), 0);
        assert_eq!(HapticImpactStyle::Medium.raw_value(), 1);
        assert_eq!(HapticImpactStyle::Heavy.raw_value(), 2);
        assert_eq!(HapticImpactStyle::Soft.raw_value(), 3);
        assert_eq!(HapticImpactStyle::Rigid.raw_value(), 4);
    }

    #[test]
    fn notification_kinds_map_to_ui_feedback_constants() {
        assert_eq!(HapticNotificationKind::Success.raw_value(), 0);
        assert_eq!(HapticNotificationKind::Warning.raw_value(), 1);
        assert_eq!(HapticNotificationKind::Error.raw_value(), 2);
    }

    #[test]
    fn trigger_is_a_checked_no_op_off_device() {
        #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
        {
            assert!(!is_haptics_available());
            assert!(!trigger_haptic(HapticFeedback::Selection));
            assert!(!trigger_haptic(HapticFeedback::Impact(
                HapticImpactStyle::Heavy
            )));
            assert!(!trigger_haptic(HapticFeedback::Notification(
                HapticNotificationKind::Error
            )));
        }
    }
}
