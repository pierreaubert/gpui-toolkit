//! Prefilled missing-capability reports.
//!
//! [`gap_report`] validates an area and a title, then renders a
//! ready-to-file issue body pointing at the owning tracker, so the
//! friction between hitting a gap (a missing component, template, or
//! token) and reporting it drops to one command.

// Rust guideline compliant 2026-02-21

use crate::error_codes::{ErrorCode, ToolkitError};
use serde::Serialize;

/// Discriminator for gap-report payloads in JSON envelopes.
pub const GAP_REPORT_TYPE: &str = "gap-report";

/// Valid report areas.
pub const GAP_AREAS: &[&str] = &["component", "template", "theme", "layout", "cli", "other"];

/// Prefilled issue payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GapReport {
    /// Report area, one of [`GAP_AREAS`].
    pub area: String,
    /// Issue title.
    pub title: String,
    /// Prefilled markdown body.
    pub body: String,
    /// Tracker URL receiving the issue.
    pub issues_url: String,
}

/// Builds a prefilled report for an area and a title.
///
/// # Errors
///
/// Returns [`ErrorCode::InvalidArgument`] for areas outside
/// [`GAP_AREAS`] or for blank titles.
///
/// # Examples
///
/// ```rust
/// use gpui_toolkit_cli::gap_report;
///
/// let report = gap_report("component", "Add DateRangePicker").unwrap();
/// assert_eq!(report.area, "component");
/// ```
pub fn gap_report(area: &str, title: &str) -> Result<GapReport, ToolkitError> {
    if !GAP_AREAS.contains(&area) {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            format!("area '{area}' must be one of: {}", GAP_AREAS.join(", ")),
        ));
    }
    if title.trim().is_empty() {
        return Err(ToolkitError::new(
            ErrorCode::InvalidArgument,
            "title must not be blank",
        ));
    }
    let issues_url = format!("{}/issues", env!("CARGO_PKG_REPOSITORY"));
    let body = format!(
        "## Gap\n\n**Area:** {area}\n**Title:** {title}\n**Toolkit:** {}\n\n\
        Describe the missing capability: what you tried, what you expected, \
        and what you used instead.\n",
        env!("CARGO_PKG_VERSION"),
    );
    Ok(GapReport {
        area: area.to_owned(),
        title: title.trim().to_owned(),
        body,
        issues_url,
    })
}

/// Renders a report as human-readable text.
pub fn render_gap_text(report: &GapReport, dense: bool) -> String {
    if dense {
        return format!("{} {} {}\n", report.area, report.title, report.issues_url);
    }
    format!("{}\nFile at: {}\n", report.body, report.issues_url)
}
