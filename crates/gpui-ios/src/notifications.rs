//! Local notification bridge (`UNUserNotificationCenter`).
//!
//! Host-testable scheduling of time-interval local notifications. Validation
//! runs everywhere; scheduling/cancellation touch UIKit only on iOS/tvOS and
//! report [`NotificationError::Unsupported`] elsewhere.
//!
//! Note: the containing app owns notification *authorization* (requested from
//! Swift via `UNUserNotificationCenter.requestAuthorization`). Scheduling
//! without authorization silently drops the notification — that is platform
//! behavior, not an error surfaced here.

/// Maximum delay accepted by [`schedule_local_notification`] (one year).
pub const MAX_NOTIFICATION_DELAY_SECONDS: f64 = 366.0 * 24.0 * 3_600.0;

/// One local notification to schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalNotificationRequest {
    /// Stable identifier for later cancellation.
    pub identifier: String,
    /// Bold title line (may be empty when `body` is set).
    pub title: String,
    /// Detail text (may be empty when `title` is set).
    pub body: String,
    /// Seconds from now until delivery; must be positive and finite.
    pub delay_seconds: f64,
    /// App-icon badge number; `None` leaves the badge untouched.
    pub badge: Option<u32>,
    /// Whether to play the default notification sound.
    pub sound: bool,
}

impl LocalNotificationRequest {
    pub fn validate(&self) -> Result<(), NotificationError> {
        if self.identifier.trim().is_empty() {
            return Err(NotificationError::InvalidRequest(
                "notification identifier must not be empty".to_string(),
            ));
        }
        if self.title.trim().is_empty() && self.body.trim().is_empty() {
            return Err(NotificationError::InvalidRequest(
                "notification title or body must not be empty".to_string(),
            ));
        }
        if !self.delay_seconds.is_finite()
            || self.delay_seconds <= 0.0
            || self.delay_seconds > MAX_NOTIFICATION_DELAY_SECONDS
        {
            return Err(NotificationError::InvalidRequest(format!(
                "notification delay must be within (0, {MAX_NOTIFICATION_DELAY_SECONDS}] seconds"
            )));
        }
        Ok(())
    }
}

/// Local notification failure modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationError {
    /// Not running on iOS/tvOS.
    Unsupported,
    /// The request failed [`LocalNotificationRequest::validate`].
    InvalidRequest(String),
    /// UIKit rejected the scheduling (carries detail).
    ScheduleFailed(String),
}

impl std::fmt::Display for NotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "local notifications require iOS or tvOS"),
            Self::InvalidRequest(detail) => write!(f, "invalid notification request: {detail}"),
            Self::ScheduleFailed(detail) => {
                write!(f, "scheduling local notification failed: {detail}")
            }
        }
    }
}

impl std::error::Error for NotificationError {}

/// Schedule `request` with `UNUserNotificationCenter`, returning the
/// identifier on success (usable with [`cancel_local_notification`]).
pub fn schedule_local_notification(
    request: &LocalNotificationRequest,
) -> Result<String, NotificationError> {
    request.validate()?;
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        unsafe { schedule_local_notification_impl(request) }
    }
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    {
        Err(NotificationError::Unsupported)
    }
}

/// Remove the pending notification with `identifier`. No-op off device.
pub fn cancel_local_notification(identifier: &str) {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        unsafe {
            cancel_local_notification_impl(identifier);
        }
    }
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    {
        let _ = identifier;
    }
}

/// Remove all pending notifications scheduled through this bridge.
/// No-op off device.
pub fn cancel_all_local_notifications() {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        unsafe {
            use objc::runtime::Object;
            use objc::{class, msg_send, sel, sel_impl};
            // SAFETY: `currentNotificationCenter` never returns null on device.
            let center: *mut Object =
                msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
            let _: () = msg_send![center, removeAllPendingNotificationRequests];
        }
    }
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
fn ns_string(value: &str) -> *mut objc::runtime::Object {
    use objc::{class, msg_send, sel, sel_impl};
    // Converted strings come from validated requests or fixed literals;
    // interior NUL bytes fall back to an empty string.
    let bytes = std::ffi::CString::new(value).unwrap_or_default();
    unsafe {
        // SAFETY: autoreleased NSString, valid for the synchronous calls below.
        msg_send![class!(NSString), stringWithUTF8String: bytes.as_ptr()]
    }
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
unsafe fn schedule_local_notification_impl(
    request: &LocalNotificationRequest,
) -> Result<String, NotificationError> {
    use objc::runtime::{NO, Object};
    use objc::{class, msg_send, sel, sel_impl};

    // SAFETY: all objects below are either autoreleased temporaries consumed
    // synchronously or released explicitly (`content`).
    let center: *mut Object =
        msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
    if center.is_null() {
        return Err(NotificationError::ScheduleFailed(
            "UNUserNotificationCenter unavailable".to_string(),
        ));
    }

    let content: *mut Object = msg_send![class!(UNMutableNotificationContent), new];
    if content.is_null() {
        return Err(NotificationError::ScheduleFailed(
            "could not create UNMutableNotificationContent".to_string(),
        ));
    }
    let _: () = msg_send![content, setTitle: ns_string(&request.title)];
    let _: () = msg_send![content, setBody: ns_string(&request.body)];
    if let Some(badge) = request.badge {
        let number: *mut Object = msg_send![class!(NSNumber), numberWithUnsignedInt: badge];
        let _: () = msg_send![content, setBadge: number];
    }
    if request.sound {
        let sound: *mut Object = msg_send![class!(UNNotificationSound), defaultSound];
        if !sound.is_null() {
            let _: () = msg_send![content, setSound: sound];
        }
    }

    let trigger: *mut Object = msg_send![
        class!(UNTimeIntervalNotificationTrigger),
        triggerWithTimeInterval: request.delay_seconds
        repeats: NO
    ];
    if trigger.is_null() {
        let _: () = msg_send![content, release];
        return Err(NotificationError::ScheduleFailed(
            "could not create UNTimeIntervalNotificationTrigger".to_string(),
        ));
    }
    let notification: *mut Object = msg_send![
        class!(UNNotificationRequest),
        requestWithIdentifier: ns_string(&request.identifier)
        content: content
        trigger: trigger
    ];
    let _: () = msg_send![content, release];
    if notification.is_null() {
        return Err(NotificationError::ScheduleFailed(
            "could not create UNNotificationRequest".to_string(),
        ));
    }
    // `withCompletionHandler:` is nullable; a null handler means fire-and-forget.
    let _: () = msg_send![
        center,
        addNotificationRequest: notification
        withCompletionHandler: std::ptr::null::<Object>()
    ];
    Ok(request.identifier.clone())
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
unsafe fn cancel_local_notification_impl(identifier: &str) {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    // SAFETY: autoreleased temporaries consumed synchronously.
    let center: *mut Object =
        msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
    if center.is_null() {
        return;
    }
    let id = ns_string(identifier);
    let ids: *mut Object = msg_send![class!(NSArray), arrayWithObject: id];
    let _: () = msg_send![center, removePendingNotificationRequestsWithIdentifiers: ids];
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_request() -> LocalNotificationRequest {
        LocalNotificationRequest {
            identifier: "take-a-break".to_string(),
            title: "Time to rest".to_string(),
            body: "You have practised for 30 minutes.".to_string(),
            delay_seconds: 60.0,
            badge: Some(1),
            sound: true,
        }
    }

    #[test]
    fn valid_request_passes_validation() {
        valid_request().validate().unwrap();
    }

    #[test]
    fn identifier_must_not_be_blank() {
        let mut request = valid_request();
        request.identifier = "  ".to_string();
        assert!(matches!(
            request.validate(),
            Err(NotificationError::InvalidRequest(_))
        ));
    }

    #[test]
    fn title_or_body_must_be_present() {
        let mut request = valid_request();
        request.title.clear();
        request.body.clear();
        assert!(matches!(
            request.validate(),
            Err(NotificationError::InvalidRequest(_))
        ));

        request.body = "Body only".to_string();
        request.validate().unwrap();
    }

    #[test]
    fn delay_must_be_a_positive_bounded_interval() {
        for delay in [
            0.0,
            -1.0,
            f64::NAN,
            f64::INFINITY,
            MAX_NOTIFICATION_DELAY_SECONDS + 1.0,
        ] {
            let mut request = valid_request();
            request.delay_seconds = delay;
            assert!(
                request.validate().is_err(),
                "delay {delay:?} should be rejected"
            );
        }
        let mut request = valid_request();
        request.delay_seconds = MAX_NOTIFICATION_DELAY_SECONDS;
        request.validate().unwrap();
    }

    #[test]
    fn off_device_scheduling_reports_unsupported() {
        #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
        {
            assert_eq!(
                schedule_local_notification(&valid_request()),
                Err(NotificationError::Unsupported)
            );
            let mut request = valid_request();
            request.identifier.clear();
            assert!(matches!(
                schedule_local_notification(&request),
                Err(NotificationError::InvalidRequest(_))
            ));
            // Cancellation is a silent no-op off device.
            cancel_local_notification("anything");
            cancel_all_local_notifications();
        }
    }
}
