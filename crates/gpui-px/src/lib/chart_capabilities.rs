//! Chart capability metadata for release QA.

/// Schema version for [`ChartCapabilityReport`].
pub const CHART_CAPABILITY_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`ChartCapabilityReport`].
pub const CHART_CAPABILITY_REPORT_TYPE: &str = "gpui-px-chart-capabilities";

/// Current readiness for a chart capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartCapabilityStatus {
    /// Implemented and covered by focused tests.
    Implemented,
    /// The capability has a core implementation but needs integration/runtime QA.
    Partial,
    /// The capability is not implemented yet.
    Missing,
    /// The capability belongs to the host app or platform bridge.
    AppBridgeRequired,
}

impl ChartCapabilityStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::Partial => "partial",
            Self::Missing => "missing",
            Self::AppBridgeRequired => "app-bridge-required",
        }
    }

    /// Whether this status is enough for a production charting claim.
    pub const fn is_release_ready(self) -> bool {
        matches!(self, Self::Implemented)
    }
}

/// One charting capability tracked by release QA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartCapabilityEntry {
    /// Stable capability id.
    pub id: &'static str,
    /// Human-readable capability name.
    pub capability: &'static str,
    /// Chart families covered by this capability.
    pub chart_families: &'static str,
    /// Current readiness.
    pub status: ChartCapabilityStatus,
    /// Evidence recorded for release notes.
    pub evidence: &'static str,
    /// Requirement before claiming production-grade parity.
    pub release_requirement: &'static str,
}

/// Versioned chart capability report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChartCapabilityReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [ChartCapabilityEntry],
}

impl ChartCapabilityReport {
    /// Return true only when every capability is release-ready.
    pub fn all_release_ready(self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.status.is_release_ready())
    }

    /// Return entries that still block production-grade charting claims.
    pub fn blocking_entries(self) -> impl Iterator<Item = &'static ChartCapabilityEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.status.is_release_ready())
    }

    /// Render the report as Markdown for release artifacts.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI PX Chart Capabilities\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Capability | Chart families | Status | Evidence | Release requirement |\n\
             | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} |\n",
                entry.capability,
                entry.chart_families,
                entry.status.as_str(),
                entry.evidence,
                entry.release_requirement
            ));
        }

        markdown
    }
}

const CHART_CAPABILITY_ENTRIES: &[ChartCapabilityEntry] = &[
    ChartCapabilityEntry {
        id: "chart-builders",
        capability: "Plotly Express-style chart builders",
        chart_families: "scatter, line, bar, area, boxplot, heatmap, contour, isoline, pie/donut, treemap, optional surface3d",
        status: ChartCapabilityStatus::Implemented,
        evidence: "Public builder functions exist for the listed chart families, with focused unit coverage across the chart modules.",
        release_requirement: "Keep chart builder tests green before release.",
    },
    ChartCapabilityEntry {
        id: "accessibility-summaries",
        capability: "Non-rendering accessibility summaries",
        chart_families: "all public chart builders, including optional surface3d",
        status: ChartCapabilityStatus::Implemented,
        evidence: "ChartAccessibilitySummary covers chart type, title, series labels, datum counts, finite ranges, scale types, and descriptions.",
        release_requirement: "Keep accessibility summary tests green and feed summaries into app-level native accessibility bridges.",
    },
    ChartCapabilityEntry {
        id: "interaction-state",
        capability: "Interaction state helpers",
        chart_families: "host-rendered chart surfaces",
        status: ChartCapabilityStatus::Implemented,
        evidence: "ChartInteraction exposes tested renderer-free brush, zoom, wheel, pan, hover-domain, and keyboard state helpers, plus interaction_qa_report() for release evidence.",
        release_requirement: "Keep interaction and interaction_qa tests green; attach host-app keybinding and tooltip QA before claiming product-level UX parity.",
    },
    ChartCapabilityEntry {
        id: "native-legends",
        capability: "Native rendered legends",
        chart_families: "multi-series 1D charts and categorical charts",
        status: ChartCapabilityStatus::Implemented,
        evidence: "Line, scatter, and bar charts render native legends and expose ChartLegendSummary metadata with labels, colors, marker shapes, positions, hidden state, and secondary-axis state.",
        release_requirement: "Keep legend summary and native legend build tests green for line, scatter, and bar charts.",
    },
    ChartCapabilityEntry {
        id: "annotations",
        capability: "Annotations and callouts",
        chart_families: "line, scatter, bar",
        status: ChartCapabilityStatus::Implemented,
        evidence: "Line, scatter, and bar charts expose ChartAnnotation metadata plus annotation summaries for point, axis-value, and category targets.",
        release_requirement: "Keep annotation metadata tests green and document renderer-level callout drawing as a host/rendering follow-up.",
    },
    ChartCapabilityEntry {
        id: "static-export",
        capability: "Static image/vector export",
        chart_families: "line, scatter, bar, area, pie/donut, heatmap, boxplot, treemap, isoline, contour, optional surface3d",
        status: ChartCapabilityStatus::Implemented,
        evidence: "Line, scatter, bar, area, pie/donut, heatmap, boxplot, treemap, isoline, contour, and optional surface3d builders expose deterministic to_svg()/to_svg_with_options() vector export with validation and focused tests.",
        release_requirement: "Keep static_export tests green and document broader image/PDF export as a future chart-family expansion.",
    },
    ChartCapabilityEntry {
        id: "visual-regression",
        capability: "Visual regression baselines",
        chart_families: "all rendered chart families",
        status: ChartCapabilityStatus::Implemented,
        evidence: "chart_visual_regression_manifest() records chart story ids, dashboard/panel/mobile viewports, light/dark/high-contrast schemes, and stable baseline/actual/diff artifact paths for every public chart family plus optional surface3d.",
        release_requirement: "Keep chart visual-regression manifest tests green and execute the listed captures through component-lab/showcase visual QA before attaching release artifacts.",
    },
    ChartCapabilityEntry {
        id: "native-accessibility-bridge",
        capability: "Native screen-reader bridge consumption",
        chart_families: "all chart families",
        status: ChartCapabilityStatus::Implemented,
        evidence: "ChartAccessibilitySummary::to_accessibility_tree() and to_bridge_snapshot() convert chart summaries into gpui-ui-kit AccessibilityTree/AccessibilityBridgeSnapshot payloads with image roles, labels, descriptions, ranges, scales, and series labels for host/native adapters.",
        release_requirement: "Keep accessibility bridge tests green and attach product-level screen-reader QA before claiming OS-level chart accessibility parity.",
    },
];

/// Return the current chart capability report.
pub const fn chart_capability_report() -> ChartCapabilityReport {
    ChartCapabilityReport {
        schema_version: CHART_CAPABILITY_SCHEMA_VERSION,
        report_type: CHART_CAPABILITY_REPORT_TYPE,
        reviewed_on: "2026-07-08",
        entries: CHART_CAPABILITY_ENTRIES,
    }
}

/// Return chart capability entries without allocating.
pub const fn chart_capability_entries() -> &'static [ChartCapabilityEntry] {
    CHART_CAPABILITY_ENTRIES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_capability_report_has_stable_contract() {
        let report = chart_capability_report();

        assert_eq!(report.schema_version, CHART_CAPABILITY_SCHEMA_VERSION);
        assert_eq!(report.report_type, CHART_CAPABILITY_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-08");
        assert!(!report.entries.is_empty());
        assert!(report.all_release_ready());
    }

    #[test]
    fn chart_capability_report_has_unique_ids() {
        let mut ids = std::collections::BTreeSet::new();

        for entry in chart_capability_entries() {
            assert!(
                ids.insert(entry.id),
                "duplicate chart capability {}",
                entry.id
            );
            assert!(!entry.capability.is_empty());
            assert!(!entry.chart_families.is_empty());
            assert!(!entry.status.as_str().is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn chart_capability_report_marks_accessibility_ready() {
        assert!(chart_capability_entries().iter().any(|entry| {
            entry.id == "accessibility-summaries"
                && entry.status == ChartCapabilityStatus::Implemented
        }));
    }

    #[test]
    fn chart_capability_report_names_plotly_style_blockers() {
        let blocking = chart_capability_report()
            .blocking_entries()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();

        assert!(!blocking.contains(&"interaction-state"));
        assert!(!blocking.contains(&"native-legends"));
        assert!(!blocking.contains(&"annotations"));
        assert!(!blocking.contains(&"static-export"));
        assert!(!blocking.contains(&"visual-regression"));
        assert!(!blocking.contains(&"native-accessibility-bridge"));
    }

    #[test]
    fn chart_capability_markdown_names_statuses() {
        let markdown = chart_capability_report().to_markdown_table();

        assert!(markdown.contains(CHART_CAPABILITY_REPORT_TYPE));
        assert!(markdown.contains("Non-rendering accessibility summaries"));
        assert!(markdown.contains("Static image/vector export"));
        assert!(markdown.contains("implemented"));
        assert!(markdown.contains("AccessibilityBridgeSnapshot"));
    }
}
