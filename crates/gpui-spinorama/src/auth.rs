//! Secure storage for the Spinorama API key.
//!
//! Native builds keep the key in the OS credential store (macOS Keychain,
//! Windows Credential Manager, Linux Secret Service) via `keyring`.
//! The key is never written to a file, log, or `Debug` output by this crate.
//! Wasm builds have no OS credential store, so the key lives only in
//! [`SpinoramaApp`] memory for the session and is never persisted to
//! `localStorage`/`sessionStorage`.

// Rust guideline compliant 2026-02-21

/// Credential-store service name for the Spinorama API key.
#[cfg_attr(target_family = "wasm", allow(dead_code, reason = "wasm keeps the key in memory only"))]
pub const KEY_SERVICE: &str = "gpui-spinorama";
/// Credential-store account name for the Spinorama API key.
#[cfg_attr(target_family = "wasm", allow(dead_code, reason = "wasm keeps the key in memory only"))]
pub const KEY_ACCOUNT: &str = "spinorama-api-key";
/// Environment variable that overrides the stored key (CI / headless use).
pub const ENV_API_KEY: &str = "SPINORAMA_API_KEY";

/// A Spinorama API key whose `Debug` output never leaks the secret.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    /// Wraps a raw key, trimming surrounding whitespace.
    ///
    /// Returns `None` for an empty or whitespace-only key.
    pub fn new(raw: impl Into<String>) -> Option<Self> {
        let raw = raw.into();
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(Self(trimmed.to_string()))
        }
    }

    /// Borrows the key for an `Authorization: Bearer …` header.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ApiKey(<redacted>, {} chars)", self.0.chars().count())
    }
}

/// Loads the key from `SPINORAMA_API_KEY`, else the OS credential store.
///
/// Wasm builds only honour the in-memory key held by the app, so this
/// returns `None` there unless the environment override is present.
pub fn load_api_key() -> Option<ApiKey> {
    if let Ok(raw) = std::env::var(ENV_API_KEY)
        && let Some(key) = ApiKey::new(raw) {
            return Some(key);
        }
    #[cfg(not(target_family = "wasm"))]
    {
        let entry = keyring::Entry::new(KEY_SERVICE, KEY_ACCOUNT).ok()?;
        match entry.get_password() {
            Ok(raw) => ApiKey::new(raw),
            Err(_) => None,
        }
    }
    #[cfg(target_family = "wasm")]
    {
        None
    }
}

/// Saves the key to the OS credential store.
///
/// On wasm this is a no-op: the caller keeps the key in memory only.
///
/// # Errors
///
/// Returns an error when the OS credential store is unavailable.
pub fn save_api_key(key: &ApiKey) -> Result<(), String> {
    #[cfg(not(target_family = "wasm"))]
    {
        let entry = keyring::Entry::new(KEY_SERVICE, KEY_ACCOUNT)
            .map_err(|e| format!("credential store unavailable: {e}"))?;
        entry
            .set_password(key.as_str())
            .map_err(|e| format!("could not save API key: {e}"))
    }
    #[cfg(target_family = "wasm")]
    {
        let _ = key;
        Ok(())
    }
}

/// Removes the key from the OS credential store, if present.
///
/// # Errors
///
/// Returns an error when the credential store itself fails; a missing
/// entry is not an error.
pub fn delete_api_key() -> Result<(), String> {
    #[cfg(not(target_family = "wasm"))]
    {
        let entry = keyring::Entry::new(KEY_SERVICE, KEY_ACCOUNT)
            .map_err(|e| format!("credential store unavailable: {e}"))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("could not delete API key: {e}")),
        }
    }
    #[cfg(target_family = "wasm")]
    {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_key_debug_redacts_secret() {
        let key = ApiKey::new("super-secret-value").expect("non-empty key");
        let rendered = format!("{key:?}");
        assert!(rendered.contains("ApiKey"));
        assert!(!rendered.contains("super-secret-value"));
    }

    #[test]
    fn empty_keys_are_rejected() {
        assert!(ApiKey::new("").is_none());
        assert!(ApiKey::new("   ").is_none());
        assert_eq!(ApiKey::new("  abc  ").expect("key").as_str(), "abc");
    }
}
