//! Minimal macOS Keychain bridge for opaque Python credential references.
//!
//! Secrets are passed directly to Security.framework, never through a shell
//! command or returned over the Python session protocol.

use serde_json::{json, Value};

pub fn handle(arguments: &Value) -> Result<Value, String> {
    let operation = arguments.get("operation").and_then(Value::as_str).unwrap_or("store");
    let reference = arguments.get("reference").and_then(Value::as_str).ok_or("credential_store requires reference")?;
    if reference.trim().is_empty() { return Err("credential reference must not be empty".into()); }
    match operation {
        "store" => {
            let secret = arguments.get("secret").and_then(Value::as_str).ok_or("credential_store store requires secret")?;
            store(reference, secret)?;
            Ok(json!({"ok": true, "credential_ref": reference}))
        }
        "delete" => { delete(reference)?; Ok(json!({"ok": true, "credential_ref": reference})) }
        _ => Err("credential_store operation must be store or delete".into()),
    }
}

#[cfg(target_os = "macos")]
fn service_name() -> String {
    format!("gpui-toolkit/{}", std::env::var("GPUI_TOOLKIT_APP_ID").unwrap_or_else(|_| "gpui-python-runtime".into()))
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::{c_char, c_void};
    type Item = *mut c_void;
    const ITEM_NOT_FOUND: i32 = -25300;
    const DUPLICATE_ITEM: i32 = -25299;
    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        fn SecKeychainAddGenericPassword(keychain: *mut c_void, service_len: u32, service: *const c_char, account_len: u32, account: *const c_char, password_len: u32, password: *const c_void, item: *mut Item) -> i32;
        fn SecKeychainFindGenericPassword(keychain: *mut c_void, service_len: u32, service: *const c_char, account_len: u32, account: *const c_char, password_len: *mut u32, password: *mut *mut c_void, item: *mut Item) -> i32;
        fn SecKeychainItemModifyAttributesAndData(item: Item, attributes: *const c_void, password_len: u32, password: *const c_void) -> i32;
        fn SecKeychainItemDelete(item: Item) -> i32;
        fn CFRelease(value: *const c_void);
    }
    fn args<'a, 'b>(service: &'a str, reference: &'b str) -> Result<(&'a [u8], &'b [u8]), String> {
        if service.as_bytes().len() > u32::MAX as usize || reference.as_bytes().len() > u32::MAX as usize { return Err("credential reference too long".into()); }
        Ok((service.as_bytes(), reference.as_bytes()))
    }
    pub(super) fn store(service: &str, reference: &str, secret: &str) -> Result<(), String> {
        let (service, account) = args(service, reference)?;
        let secret = secret.as_bytes();
        let mut item: Item = std::ptr::null_mut();
        let status = unsafe { SecKeychainAddGenericPassword(std::ptr::null_mut(), service.len() as u32, service.as_ptr().cast(), account.len() as u32, account.as_ptr().cast(), secret.len() as u32, secret.as_ptr().cast(), &mut item) };
        if status == 0 { if !item.is_null() { unsafe { CFRelease(item) }; } return Ok(()); }
        if status != DUPLICATE_ITEM { return Err(format!("Keychain store failed ({status})")); }
        let status = unsafe { SecKeychainFindGenericPassword(std::ptr::null_mut(), service.len() as u32, service.as_ptr().cast(), account.len() as u32, account.as_ptr().cast(), std::ptr::null_mut(), std::ptr::null_mut(), &mut item) };
        if status != 0 { return Err(format!("Keychain lookup failed ({status})")); }
        let status = unsafe { SecKeychainItemModifyAttributesAndData(item, std::ptr::null(), secret.len() as u32, secret.as_ptr().cast()) };
        unsafe { CFRelease(item) };
        if status == 0 { Ok(()) } else { Err(format!("Keychain update failed ({status})")) }
    }
    pub(super) fn delete(service: &str, reference: &str) -> Result<(), String> {
        let (service, account) = args(service, reference)?;
        let mut item: Item = std::ptr::null_mut();
        let status = unsafe { SecKeychainFindGenericPassword(std::ptr::null_mut(), service.len() as u32, service.as_ptr().cast(), account.len() as u32, account.as_ptr().cast(), std::ptr::null_mut(), std::ptr::null_mut(), &mut item) };
        if status == ITEM_NOT_FOUND { return Ok(()); }
        if status != 0 { return Err(format!("Keychain lookup failed ({status})")); }
        let status = unsafe { SecKeychainItemDelete(item) };
        unsafe { CFRelease(item) };
        if status == 0 { Ok(()) } else { Err(format!("Keychain delete failed ({status})")) }
    }
}

#[cfg(target_os = "macos")]
fn store(reference: &str, secret: &str) -> Result<(), String> { macos::store(&service_name(), reference, secret) }
#[cfg(target_os = "macos")]
fn delete(reference: &str) -> Result<(), String> { macos::delete(&service_name(), reference) }
#[cfg(not(target_os = "macos"))]
fn store(_: &str, _: &str) -> Result<(), String> { Err("credential_store is currently available on macOS only".into()) }
#[cfg(not(target_os = "macos"))]
fn delete(_: &str) -> Result<(), String> { Err("credential_store is currently available on macOS only".into()) }

#[cfg(test)]
mod tests {
    use super::handle;
    use serde_json::json;

    #[test]
    fn rejects_invalid_requests_before_platform_access() {
        assert!(handle(&json!({"operation": "store", "reference": ""})).is_err());
        assert!(handle(&json!({"operation": "read", "reference": "token"})).is_err());
        assert!(handle(&json!({"operation": "store", "reference": "token"})).is_err());
    }
}
