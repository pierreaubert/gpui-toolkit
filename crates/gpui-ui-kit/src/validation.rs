//! Schema-style validation for component props and config.
//!
//! Components with cross-field invariants (ranges, step alignment, wizard step
//! counts) implement [`Validate`]. A passing config returns `Ok(())`; a
//! failing config returns every field failure so forms and release tooling can
//! report them together. Use [`Validate::validate_first`] for the single-error
//! `Result` shape.
//!
//! The error type follows the crate's existing error-handling conventions
//! (see `QrCodeError` and `NativeAccessibilityAdapterError`): a descriptive
//! value type with [`std::fmt::Display`] and [`std::error::Error`] impls.

use gpui::SharedString;

/// A single schema validation failure for one field of a component's
/// props or config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Props/config field that failed validation (e.g. `"min"`, `"steps"`).
    pub field: SharedString,
    /// Human-readable failure description.
    pub message: SharedString,
}

impl ValidationError {
    /// Create a validation failure for `field` with `message`.
    pub fn new(field: impl Into<SharedString>, message: impl Into<SharedString>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Schema validation contract for component props and config.
///
/// `Err` always carries at least one [`ValidationError`]; an empty error list
/// is treated as valid by the provided helpers.
pub trait Validate {
    /// Validate the full config, collecting every field failure.
    fn validate(&self) -> Result<(), Vec<ValidationError>>;

    /// Validate the config, returning only the first field failure.
    fn validate_first(&self) -> Result<(), ValidationError> {
        match self.validate() {
            Ok(()) => Ok(()),
            Err(errors) => errors.into_iter().next().map_or(Ok(()), Err),
        }
    }

    /// Return true when the config passes validation.
    fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{Validate, ValidationError};

    struct StubConfig {
        failures: Vec<ValidationError>,
    }

    impl Validate for StubConfig {
        fn validate(&self) -> Result<(), Vec<ValidationError>> {
            if self.failures.is_empty() {
                Ok(())
            } else {
                Err(self.failures.clone())
            }
        }
    }

    #[test]
    fn validation_error_displays_field_and_message() {
        let error = ValidationError::new("min", "min must be <= max");
        assert_eq!(error.field.as_ref(), "min");
        assert_eq!(format!("{error}"), "min: min must be <= max");

        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn validate_collects_every_failure_and_first_returns_single_error() {
        let config = StubConfig {
            failures: vec![
                ValidationError::new("min", "min must be <= max"),
                ValidationError::new("step", "step must be positive"),
            ],
        };

        let errors = config.validate().expect_err("stub config is invalid");
        assert_eq!(errors.len(), 2);
        assert!(!config.is_valid());

        let first = config
            .validate_first()
            .expect_err("first failure is an error");
        assert_eq!(first.field.as_ref(), "min");
    }

    #[test]
    fn valid_config_passes_both_result_shapes() {
        let config = StubConfig { failures: vec![] };

        assert!(config.validate().is_ok());
        assert!(config.validate_first().is_ok());
        assert!(config.is_valid());
    }
}
