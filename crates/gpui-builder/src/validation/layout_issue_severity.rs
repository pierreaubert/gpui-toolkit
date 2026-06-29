use std::fmt;

/// Severity for a layout validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutIssueSeverity {
    /// A declaration is invalid or ambiguous enough that solving it is risky.
    Error,
    /// A declaration is accepted by the solver but likely hurts UX, tooling, or accessibility.
    Warning,
}

impl fmt::Display for LayoutIssueSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "ERROR"),
            Self::Warning => write!(f, "WARNING"),
        }
    }
}
