//! Stable machine-readable error codes.
//!
//! Every CLI failure maps to an [`ErrorCode`]. Agents and scripts branch on
//! the code, never on the human-readable message, which may change wording
//! at any time. Codes are append-only: once shipped, a code's meaning never
//! changes and a code is never removed; new failure modes get new codes.

// Rust guideline compliant 2026-02-21

use std::fmt::{Display, Formatter};

/// Machine-readable CLI failure code.
///
/// Codes serialize as their `SCREAMING_SNAKE` names and stay stable across
/// releases. See the module documentation for the append-only rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Fallback for failures without a more specific code.
    Unknown,
    /// An argument or option value is invalid or inconsistent.
    InvalidArgument,
    /// Writing an output file failed or was refused.
    WriteFailed,
    /// The component catalog could not be built.
    CatalogLoad,
    /// No component matched the requested name.
    UnknownComponent,
    /// No template matched the requested id.
    UnknownTemplate,
    /// Refused to overwrite an existing file.
    FileExists,
    /// No source file could be located for the request.
    NoSource,
    /// No theme preset matched the requested id.
    UnknownTheme,
    /// A layout expression failed to parse.
    LayoutParse,
    /// A layout expression is well-formed but invalid.
    LayoutInvalid,
}

impl ErrorCode {
    /// Canonical wire name of this code.
    ///
    /// The returned name is the exact string in JSON envelopes and the
    /// value agents match on.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_toolkit_cli::ErrorCode;
    ///
    /// assert_eq!(ErrorCode::InvalidArgument.as_str(), "ERR_INVALID_ARGUMENT");
    /// ```
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "ERR_UNKNOWN",
            Self::InvalidArgument => "ERR_INVALID_ARGUMENT",
            Self::WriteFailed => "ERR_WRITE_FAILED",
            Self::CatalogLoad => "ERR_CATALOG_LOAD",
            Self::UnknownComponent => "ERR_UNKNOWN_COMPONENT",
            Self::UnknownTemplate => "ERR_UNKNOWN_TEMPLATE",
            Self::FileExists => "ERR_FILE_EXISTS",
            Self::NoSource => "ERR_NO_SOURCE",
            Self::UnknownTheme => "ERR_UNKNOWN_THEME",
            Self::LayoutParse => "ERR_LAYOUT_PARSE",
            Self::LayoutInvalid => "ERR_LAYOUT_INVALID",
        }
    }
}

impl Display for ErrorCode {
    /// Renders the canonical wire name.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl serde::Serialize for ErrorCode {
    /// Serializes the code as its canonical wire name.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// Typed CLI failure with a stable code.
///
/// Carries the [`ErrorCode`] agents branch on, a human-readable message,
/// and optional follow-up suggestions (for example the closest matching
/// command name).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolkitError {
    /// Stable code identifying this failure.
    pub code: ErrorCode,
    /// Human-readable description; wording is not stable.
    pub message: String,
    /// Follow-up suggestions, such as similarly named commands.
    pub suggestions: Vec<Suggestion>,
}

/// Follow-up hint attached to a [`ToolkitError`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Suggestion {
    /// Suggested value, such as a command name.
    pub name: String,
    /// Why this value is suggested.
    pub reason: String,
}

impl Suggestion {
    /// Builds a suggestion from a name and a reason.
    ///
    /// Both inputs accept any string-like value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_toolkit_cli::Suggestion;
    ///
    /// let hint = Suggestion::new("init", "similar name");
    /// assert_eq!(hint.name, "init");
    /// ```
    pub fn new(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            reason: reason.into(),
        }
    }
}

impl ToolkitError {
    /// Builds an error with a code and a message.
    ///
    /// The message accepts any string-like value and is human-readable
    /// only; consumers must branch on [`ErrorCode`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_toolkit_cli::{ErrorCode, ToolkitError};
    ///
    /// let error = ToolkitError::new(ErrorCode::WriteFailed, "disk is full");
    /// assert_eq!(error.code, ErrorCode::WriteFailed);
    /// ```
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            suggestions: Vec::new(),
        }
    }

    /// Attaches follow-up suggestions to this error.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gpui_toolkit_cli::{ErrorCode, Suggestion, ToolkitError};
    ///
    /// let error = ToolkitError::new(ErrorCode::Unknown, "nope")
    ///     .with_suggestions([Suggestion::new("init", "similar name")]);
    /// assert_eq!(error.suggestions.len(), 1);
    /// ```
    pub fn with_suggestions(mut self, suggestions: impl IntoIterator<Item = Suggestion>) -> Self {
        self.suggestions = suggestions.into_iter().collect();
        self
    }
}

impl Display for ToolkitError {
    /// Renders `[CODE] message` for terminal output.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ToolkitError {}
