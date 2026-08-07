//! Minimal macOS Keychain bridge for opaque Python credential references.
//!
//! Secrets are passed directly to Security.framework, never through a shell
//! command or returned over the Python session protocol.

use serde_json::{Value, json};

pub fn handle(arguments: &Value) -> Result<Value, String> {
    let operation = arguments
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or("store");
    let reference = arguments
        .get("reference")
        .and_then(Value::as_str)
        .ok_or("credential_store requires reference")?;
    if reference.trim().is_empty() {
        return Err("credential reference must not be empty".into());
    }
    match operation {
        "store" => {
            let secret = arguments
                .get("secret")
                .and_then(Value::as_str)
                .ok_or("credential_store store requires secret")?;
            store(reference, secret)?;
            Ok(json!({"ok": true, "credential_ref": reference}))
        }
        "delete" => {
            delete(reference)?;
            Ok(json!({"ok": true, "credential_ref": reference}))
        }
        _ => Err("credential_store operation must be store or delete".into()),
    }
}

#[cfg(target_os = "macos")]
fn service_name() -> String {
    format!(
        "gpui-toolkit/{}",
        std::env::var("GPUI_TOOLKIT_APP_ID").unwrap_or_else(|_| "gpui-python-runtime".into())
    )
}

#[cfg(target_os = "macos")]
fn store(reference: &str, secret: &str) -> Result<(), String> {
    security_framework::passwords::set_generic_password(
        &service_name(),
        reference,
        secret.as_bytes(),
    )
    .map_err(|error| format!("Keychain store failed ({})", error.code()))
}

#[cfg(target_os = "macos")]
fn delete(reference: &str) -> Result<(), String> {
    const ITEM_NOT_FOUND: i32 = -25300;

    match security_framework::passwords::delete_generic_password(&service_name(), reference) {
        Ok(()) => Ok(()),
        Err(error) if error.code() == ITEM_NOT_FOUND => Ok(()),
        Err(error) => Err(format!("Keychain delete failed ({})", error.code())),
    }
}
#[cfg(not(target_os = "macos"))]
fn store(_: &str, _: &str) -> Result<(), String> {
    Err("credential_store is currently available on macOS only".into())
}
#[cfg(not(target_os = "macos"))]
fn delete(_: &str) -> Result<(), String> {
    Err("credential_store is currently available on macOS only".into())
}

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
