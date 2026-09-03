use super::types::ConformanceFinding;
use serde::Serialize;

/// Summary used by CI and component docs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct DesignConformanceReport {
    pub findings: Vec<ConformanceFinding>,
}

impl DesignConformanceReport {
    pub fn passed(&self) -> bool {
        self.findings.is_empty()
    }

    /// Breaking-change gate: panics with a CI-readable summary on failure.
    ///
    /// # Panics
    ///
    /// Panics when the report contains any findings.
    pub fn assert_passed(&self, context: &str) {
        assert!(
            self.findings.is_empty(),
            "design conformance gate failed for {context}: {:?}",
            self.findings
        );
    }
}
