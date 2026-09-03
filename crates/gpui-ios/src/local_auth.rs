//! LocalAuthentication bridge (Face ID / Touch ID / device passcode).
//!
//! Thin, host-testable wrapper over `LAContext`: apps authenticate with a
//! [`LocalAuthRequest`] and receive the outcome through a one-shot callback,
//! mirroring the [`crate::pencil`] dispatch conventions. Off device every
//! entry point reports [`LocalAuthError::Unsupported`] instead of failing.

use std::fmt;
#[cfg(any(target_os = "ios", target_os = "tvos"))]
use std::sync::Mutex;

/// Biometry available on the device, mirroring `LABiometryType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BiometryKind {
    #[default]
    Unknown,
    None,
    TouchId,
    FaceId,
    OpticId,
}

impl BiometryKind {
    #[cfg(any(test, target_os = "ios", target_os = "tvos"))]
    const fn from_raw(value: i64) -> Self {
        match value {
            0 => Self::None,
            1 => Self::TouchId,
            2 => Self::FaceId,
            3 => Self::OpticId,
            _ => Self::Unknown,
        }
    }
}

/// Authentication policy, mirroring `LAPolicy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocalAuthPolicy {
    /// Biometrics or device passcode fallback (`LAPolicyDeviceOwnerAuthentication`).
    #[default]
    DeviceOwnerAuthentication,
    /// Biometrics only, no passcode fallback
    /// (`LAPolicyDeviceOwnerAuthenticationWithBiometrics`).
    DeviceOwnerAuthenticationWithBiometrics,
}

impl LocalAuthPolicy {
    #[cfg(any(test, target_os = "ios", target_os = "tvos"))]
    const fn raw_value(self) -> i64 {
        match self {
            Self::DeviceOwnerAuthentication => 1,
            Self::DeviceOwnerAuthenticationWithBiometrics => 2,
        }
    }
}

/// Authentication failure modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalAuthError {
    /// Not running on iOS/tvOS.
    Unsupported,
    /// The reason string was empty, blank, oversized, or unrepresentable.
    InvalidReason,
    /// No enrolled biometry and no device passcode for the policy.
    NotAvailable,
    /// The user cancelled or the system interrupted authentication.
    UserCancelled,
    /// Authentication failed for another reason (carries detail).
    Failed(String),
}

impl fmt::Display for LocalAuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => write!(f, "local authentication requires iOS or tvOS"),
            Self::InvalidReason => write!(f, "local authentication reason must not be blank"),
            Self::NotAvailable => write!(f, "no enrolled biometry or device passcode"),
            Self::UserCancelled => write!(f, "local authentication was cancelled"),
            Self::Failed(detail) => write!(f, "local authentication failed: {detail}"),
        }
    }
}

impl std::error::Error for LocalAuthError {}

/// One authentication attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAuthRequest {
    /// User-visible explanation shown in the system prompt.
    pub reason: String,
    /// Whether a device-passcode fallback is allowed.
    pub policy: LocalAuthPolicy,
}

impl LocalAuthRequest {
    pub fn new(reason: impl Into<String>, policy: LocalAuthPolicy) -> Self {
        Self {
            reason: reason.into(),
            policy,
        }
    }

    /// Reject blank, oversized, or NUL-containing reasons before touching UIKit.
    pub fn validate(&self) -> Result<(), LocalAuthError> {
        if self.reason.trim().is_empty() || self.reason.len() > 1024 {
            return Err(LocalAuthError::InvalidReason);
        }
        if self.reason.contains('\0') {
            return Err(LocalAuthError::InvalidReason);
        }
        Ok(())
    }
}

type AuthCallback = Box<dyn FnOnce(Result<(), LocalAuthError>) + Send>;

#[cfg(any(target_os = "ios", target_os = "tvos"))]
struct PendingAuth {
    context: *mut std::ffi::c_void,
    callback: Option<AuthCallback>,
}

// SAFETY: only the main thread touches the stored context pointer, and the
// callback is consumed exactly once via `Option::take`.
#[cfg(any(target_os = "ios", target_os = "tvos"))]
unsafe impl Send for PendingAuth {}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
static PENDING_AUTH: Mutex<Option<PendingAuth>> = Mutex::new(None);

/// Which biometry the device offers. Returns [`BiometryKind::Unknown`] off device.
pub fn biometry_kind() -> BiometryKind {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        unsafe { biometry_kind_impl() }
    }
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    {
        BiometryKind::Unknown
    }
}

/// Whether `policy` can currently be evaluated (enrolled biometry/passcode).
/// Always `false` off device.
pub fn can_evaluate_policy(policy: LocalAuthPolicy) -> bool {
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        unsafe { can_evaluate_policy_impl(policy) }
    }
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    {
        let _ = policy;
        false
    }
}

/// Evaluate `request`, invoking `callback` exactly once with the outcome.
///
/// Validation failures and off-device calls invoke the callback
/// synchronously; on device the callback fires on an arbitrary
/// background queue when the system prompt resolves.
pub fn authenticate(request: LocalAuthRequest, callback: AuthCallback) {
    if let Err(error) = request.validate() {
        callback(Err(error));
        return;
    }
    #[cfg(any(target_os = "ios", target_os = "tvos"))]
    {
        unsafe {
            authenticate_impl(request, callback);
        }
    }
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    {
        callback(Err(LocalAuthError::Unsupported));
    }
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
unsafe fn new_context() -> *mut objc::runtime::Object {
    use objc::{class, msg_send, sel, sel_impl};
    msg_send![class!(LAContext), new]
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
unsafe fn release_context(context: *mut objc::runtime::Object) {
    use objc::{msg_send, sel, sel_impl};
    if !context.is_null() {
        let _: () = msg_send![context, release];
    }
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
unsafe fn biometry_kind_impl() -> BiometryKind {
    use objc::{msg_send, sel, sel_impl};
    let context = unsafe { new_context() };
    if context.is_null() {
        return BiometryKind::Unknown;
    }
    // SAFETY: freshly allocated `LAContext`; released below.
    let raw: i64 = msg_send![context, biometryType];
    unsafe {
        release_context(context);
    }
    BiometryKind::from_raw(raw)
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
unsafe fn can_evaluate_policy_impl(policy: LocalAuthPolicy) -> bool {
    use objc::runtime::{BOOL, NO};
    use objc::{msg_send, sel, sel_impl};
    let context = unsafe { new_context() };
    if context.is_null() {
        return false;
    }
    // SAFETY: error parameter is nullable; passing null skips diagnostics.
    let ok: BOOL = msg_send![
        context,
        canEvaluatePolicy: policy.raw_value()
        error: std::ptr::null::<*mut objc::runtime::Object>()
    ];
    unsafe {
        release_context(context);
    }
    ok != NO
}

#[cfg(any(target_os = "ios", target_os = "tvos"))]
unsafe fn authenticate_impl(request: LocalAuthRequest, callback: AuthCallback) {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    let context = unsafe { new_context() };
    if context.is_null() {
        callback(Err(LocalAuthError::Failed(
            "could not create LAContext".to_string(),
        )));
        return;
    }
    // `validate()` already rejected NUL bytes, so this cannot fail.
    let c_reason = std::ffi::CString::new(request.reason.as_str())
        .unwrap_or_else(|_| std::ffi::CString::new("authenticate").unwrap());
    // SAFETY: autoreleased NSString, valid for the synchronous call below.
    let ns_reason: *mut Object =
        msg_send![class!(NSString), stringWithUTF8String: c_reason.as_ptr()];
    if ns_reason.is_null() {
        unsafe {
            release_context(context);
        }
        callback(Err(LocalAuthError::InvalidReason));
        return;
    }

    let mut pending = PENDING_AUTH.lock().unwrap();
    if pending.is_some() {
        unsafe {
            release_context(context);
        }
        callback(Err(LocalAuthError::Failed(
            "another authentication is already in progress".to_string(),
        )));
        return;
    }
    *pending = Some(PendingAuth {
        context: context as *mut std::ffi::c_void,
        callback: Some(callback),
    });
    drop(pending);

    // The reply block runs at most once per evaluation; a second invocation
    // finds no pending state and is ignored.
    // `success` is the raw ObjC `BOOL` (signed char); Rust `bool` is not a
    // valid block parameter encoding, so compare against zero explicitly.
    let reply = block2::RcBlock::new(
        move |success: std::os::raw::c_schar, error: *mut std::ffi::c_void| {
            let pending = PENDING_AUTH.lock().unwrap().take();
            let Some(state) = pending else {
                return;
            };
            unsafe {
                release_context(state.context as *mut Object);
            }
            let result = if success != 0 {
                Ok(())
            } else {
                Err(map_la_error(error as *mut Object))
            };
            if let Some(callback) = state.callback {
                callback(result);
            }
        },
    );
    // SAFETY: `LAContext` copies the block and invokes it exactly once with
    // `(BOOL, NSError *)`; the context pointer is stashed above and released
    // in the reply, so it outlives the evaluation.
    let _: () = msg_send![
        context,
        evaluatePolicy: request.policy.raw_value()
        localizedReason: ns_reason
        reply: &*reply
    ];
}

/// Map `NSError.code` from `LAError.h` to [`LocalAuthError`].
#[cfg(any(target_os = "ios", target_os = "tvos"))]
fn map_la_error(error: *mut objc::runtime::Object) -> LocalAuthError {
    if error.is_null() {
        return LocalAuthError::Failed("unknown error".to_string());
    }
    let code: i64 = unsafe {
        use objc::{msg_send, sel, sel_impl};
        // SAFETY: non-null `NSError` supplied by the reply block.
        msg_send![error, code]
    };
    match code {
        -2 | -4 => LocalAuthError::UserCancelled,
        -7..=-5 => LocalAuthError::NotAvailable,
        -3 | -8 => LocalAuthError::Failed("fallback or lockout".to_string()),
        _ => LocalAuthError::Failed(format!("LAError {code}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
    use std::sync::{Arc, Mutex as StdMutex};

    #[test]
    fn policy_raw_values_match_lapolicy() {
        assert_eq!(LocalAuthPolicy::DeviceOwnerAuthentication.raw_value(), 1);
        assert_eq!(
            LocalAuthPolicy::DeviceOwnerAuthenticationWithBiometrics.raw_value(),
            2
        );
    }

    #[test]
    fn biometry_kinds_map_from_labometrytype() {
        assert_eq!(BiometryKind::from_raw(0), BiometryKind::None);
        assert_eq!(BiometryKind::from_raw(1), BiometryKind::TouchId);
        assert_eq!(BiometryKind::from_raw(2), BiometryKind::FaceId);
        assert_eq!(BiometryKind::from_raw(3), BiometryKind::OpticId);
        assert_eq!(BiometryKind::from_raw(99), BiometryKind::Unknown);
    }

    #[test]
    fn blank_and_nul_reasons_are_rejected() {
        for reason in ["", "   ", "\t\n ", "ok\0no", &"x".repeat(1025)] {
            assert_eq!(
                LocalAuthRequest::new(reason, LocalAuthPolicy::DeviceOwnerAuthentication)
                    .validate(),
                Err(LocalAuthError::InvalidReason),
                "reason {reason:?} should be rejected"
            );
        }
        LocalAuthRequest::new("Unlock notes", LocalAuthPolicy::DeviceOwnerAuthentication)
            .validate()
            .unwrap();
    }

    #[test]
    fn error_display_is_human_readable() {
        assert!(LocalAuthError::Unsupported.to_string().contains("iOS"));
        assert!(
            LocalAuthError::Failed("x".to_string())
                .to_string()
                .contains('x')
        );
    }

    #[test]
    fn off_device_auth_reports_unsupported() {
        #[cfg(not(any(target_os = "ios", target_os = "tvos")))]
        {
            assert_eq!(biometry_kind(), BiometryKind::Unknown);
            assert!(!can_evaluate_policy(
                LocalAuthPolicy::DeviceOwnerAuthenticationWithBiometrics
            ));

            let outcome: Arc<StdMutex<Option<Result<(), LocalAuthError>>>> =
                Arc::new(StdMutex::new(None));
            let slot = outcome.clone();
            authenticate(
                LocalAuthRequest::new("Unlock", LocalAuthPolicy::DeviceOwnerAuthentication),
                Box::new(move |result| *slot.lock().unwrap() = Some(result)),
            );
            assert_eq!(
                outcome.lock().unwrap().take(),
                Some(Err(LocalAuthError::Unsupported))
            );

            // Validation still runs first, even off device.
            let outcome: Arc<StdMutex<Option<Result<(), LocalAuthError>>>> =
                Arc::new(StdMutex::new(None));
            let slot = outcome.clone();
            authenticate(
                LocalAuthRequest::new("  ", LocalAuthPolicy::DeviceOwnerAuthentication),
                Box::new(move |result| *slot.lock().unwrap() = Some(result)),
            );
            assert_eq!(
                outcome.lock().unwrap().take(),
                Some(Err(LocalAuthError::InvalidReason))
            );
        }
    }
}
