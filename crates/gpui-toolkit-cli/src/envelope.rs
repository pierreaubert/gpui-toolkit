//! Typed JSON envelopes for CLI output.
//!
//! Every `--json` response is either a success [`Envelope`] or an
//! [`ErrorEnvelope`]. Both carry [`API_VERSION`] so consumers can detect
//! wire changes, and a `type` discriminator that names the payload shape
//! (for example `manifest` or `init.run`). The same shapes back the
//! library API, so spawning the binary and calling the crate agree.

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, Suggestion, ToolkitError};
use serde::Serialize;

/// Current envelope wire version.
///
/// Bumped only for breaking changes to the envelope shapes themselves;
/// new commands, fields, and error codes are additive and keep this at 1.
pub const API_VERSION: u32 = 1;

/// Typed success envelope wrapping a command payload.
///
/// The `kind` discriminator (serialized as `type`) names the payload
/// shape; `meta` carries optional envelope-level annotations and is
/// omitted when empty.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope<T: Serialize> {
    /// Wire version, always [`API_VERSION`].
    pub api_version: u32,
    /// Payload discriminator, such as `manifest`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Command payload.
    pub data: T,
    /// Optional envelope-level annotations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

/// Typed error envelope for CLI failures.
///
/// Mirrors [`Envelope`] but replaces the payload with a stable
/// [`ErrorCode`], a human-readable message, and optional suggestions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEnvelope {
    /// Wire version, always [`API_VERSION`].
    pub api_version: u32,
    /// Human-readable description; wording is not stable.
    pub error: String,
    /// Stable code identifying this failure.
    pub code: ErrorCode,
    /// Follow-up suggestions, omitted when empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<Suggestion>>,
}

/// Builds a success envelope for a payload.
///
/// The kind names the payload shape (for example `manifest`); data is
/// any serializable command result.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::{API_VERSION, success_envelope};
///
/// let envelope = success_envelope("manifest", vec!["init"]);
/// assert_eq!(envelope.api_version, API_VERSION);
/// assert_eq!(envelope.kind, "manifest");
/// ```
pub fn success_envelope<T: Serialize>(kind: impl Into<String>, data: T) -> Envelope<T> {
    Envelope {
        api_version: API_VERSION,
        kind: kind.into(),
        data,
        meta: None,
    }
}

/// Builds an error envelope from a typed failure.
///
/// Suggestions are omitted from the JSON when the error carries none.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::{ErrorCode, ToolkitError, error_envelope};
///
/// let error = ToolkitError::new(ErrorCode::Unknown, "nope");
/// let envelope = error_envelope(&error);
/// assert_eq!(envelope.code, ErrorCode::Unknown);
/// ```
pub fn error_envelope(error: &ToolkitError) -> ErrorEnvelope {
    ErrorEnvelope {
        api_version: API_VERSION,
        error: error.message.clone(),
        code: error.code,
        suggestions: (!error.suggestions.is_empty()).then(|| error.suggestions.clone()),
    }
}

/// Renders a value as pretty-printed JSON.
///
/// Output targets terminals and agent transcripts; parsing code must
/// accept any JSON whitespace.
///
/// # Errors
///
/// Returns a serialization error when the value cannot serialize, which
/// cannot happen for the crate's own envelope types.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::{render_json, success_envelope};
///
/// let text = render_json(&success_envelope("ping", true)).unwrap();
/// assert!(text.contains("\"type\": \"ping\""));
/// ```
pub fn render_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

/// Renders a value as single-line JSON.
///
/// Compact twin of [`render_json`] for `--dense` output; agents parse
/// either form.
///
/// # Errors
///
/// Returns a serialization error when the value cannot serialize, which
/// cannot happen for the crate's own envelope types.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::{render_json_compact, success_envelope};
///
/// let text = render_json_compact(&success_envelope("ping", true)).unwrap();
/// assert!(!text.contains('\n'));
/// ```
pub fn render_json_compact<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}
