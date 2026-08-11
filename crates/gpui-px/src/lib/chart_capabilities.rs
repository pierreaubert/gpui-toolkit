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
    /// Exact component-lab story ids governed by this capability.
    pub story_ids: &'static [&'static str],
    /// Stable test/report contracts proving the capability status.
    pub test_contracts: &'static [&'static str],
    /// Current readiness.
    pub status: ChartCapabilityStatus,
    /// Evidence recorded for release notes.
    pub evidence: &'static str,
    /// Requirement before claiming production-grade parity.
    pub release_requirement: &'static str,
}

pub const PUBLIC_CHART_STORY_IDS: &[&str] = &[
    "px.line",
    "px.bar",
    "px.scatter",
    "px.area",
    "px.heatmap",
    "px.contour",
    "px.isoline",
    "px.pie",
    "px.donut",
    "px.boxplot",
    "px.treemap",
    "px.surface3d",
    "px.mesh_plot",
];

/// Every component-lab story that exercises a MeshPlot product contract.
///
/// These are intentionally separate from [`PUBLIC_CHART_STORY_IDS`]: only the
/// parent builder is a public chart-family story, while the named variants
/// make release evidence for render modes, views, and picking auditable.
pub const MESH_PLOT_RELEASE_STORY_IDS: &[&str] = &[
    "px.mesh_plot",
    "px.mesh_plot.mesh_only",
    "px.mesh_plot.smooth_fill",
    "px.mesh_plot.flat_fill",
    "px.mesh_plot.filled_contours",
    "px.mesh_plot.isolines",
    "px.mesh_plot.combined",
    "px.mesh_plot.axisymmetric_section",
    "px.mesh_plot.revolve",
    "px.mesh_plot.surface3d",
    "px.mesh_plot.picking",
];

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
        chart_families: "scatter, line, bar, area, boxplot, heatmap, contour, isoline, pie/donut, treemap, optional surface3d, mesh_plot",
        story_ids: PUBLIC_CHART_STORY_IDS,
        test_contracts: &[
            "component-lab:px-story-conformance",
            "chart builder module tests",
        ],
        status: ChartCapabilityStatus::Implemented,
        evidence: "Public builder functions exist for the listed chart families, with focused unit coverage across the chart modules.",
        release_requirement: "Keep chart builder tests green before release.",
    },
    ChartCapabilityEntry {
        id: "accessibility-summaries",
        capability: "Non-rendering accessibility summaries",
        chart_families: "all public chart builders, including optional surface3d and mesh_plot",
        story_ids: PUBLIC_CHART_STORY_IDS,
        test_contracts: &["accessibility summary tests", "accessibility bridge tests"],
        status: ChartCapabilityStatus::Implemented,
        evidence: "ChartAccessibilitySummary covers chart type, title, series labels, datum counts, finite ranges, scale types, and descriptions.",
        release_requirement: "Keep accessibility summary tests green and feed summaries into app-level native accessibility bridges.",
    },
    ChartCapabilityEntry {
        id: "interaction-state",
        capability: "Interaction state helpers",
        chart_families: "host-rendered chart surfaces",
        story_ids: PUBLIC_CHART_STORY_IDS,
        test_contracts: &["interaction::tests", "interaction_qa_report"],
        status: ChartCapabilityStatus::Partial,
        evidence: "ChartInteraction exposes tested renderer-free brush, zoom, wheel, pan, hover-domain, and keyboard state helpers, plus interaction_qa_report() for release evidence.",
        release_requirement: "Keep interaction and interaction_qa tests green; attach host-app keyboard, pointer, touch, focus, and tooltip QA before claiming product-level UX parity.",
    },
    ChartCapabilityEntry {
        id: "native-legends",
        capability: "Native rendered legends",
        chart_families: "multi-series 1D charts and categorical charts",
        story_ids: &["px.line", "px.scatter", "px.bar"],
        test_contracts: &["legend summary tests", "native legend build tests"],
        status: ChartCapabilityStatus::Implemented,
        evidence: "Line, scatter, and bar charts render native legends and expose ChartLegendSummary metadata with labels, colors, marker shapes, positions, hidden state, and secondary-axis state.",
        release_requirement: "Keep legend summary and native legend build tests green for line, scatter, and bar charts.",
    },
    ChartCapabilityEntry {
        id: "annotations",
        capability: "Annotations and callouts",
        chart_families: "line, scatter, bar",
        story_ids: &["px.line", "px.scatter", "px.bar"],
        test_contracts: &["annotation metadata tests"],
        status: ChartCapabilityStatus::Implemented,
        evidence: "Line, scatter, and bar charts expose ChartAnnotation metadata plus annotation summaries for point, axis-value, and category targets.",
        release_requirement: "Keep annotation metadata tests green and document renderer-level callout drawing as a host/rendering follow-up.",
    },
    ChartCapabilityEntry {
        id: "static-export",
        capability: "Static image/vector export",
        chart_families: "line, scatter, bar, area, pie/donut, heatmap, boxplot, treemap, isoline, contour, optional surface3d, mesh_plot",
        story_ids: PUBLIC_CHART_STORY_IDS,
        test_contracts: &["static_export tests"],
        status: ChartCapabilityStatus::Implemented,
        evidence: "Line, scatter, bar, area, pie/donut, heatmap, boxplot, treemap, isoline, contour, optional surface3d, and mesh_plot builders expose deterministic vector export with validation and focused tests.",
        release_requirement: "Keep static_export tests green and document broader image/PDF export as a future chart-family expansion.",
    },
    ChartCapabilityEntry {
        id: "streaming-preparation",
        capability: "Allocation-free same-length streaming preparation",
        chart_families: "line, scatter",
        story_ids: &["px.line", "px.scatter"],
        test_contracts: &[
            "allocation_contracts::warmed_line_and_scatter_stream_preparation_is_allocation_free",
            "streaming_cache_tests",
        ],
        status: ChartCapabilityStatus::Implemented,
        evidence: "Shared primary arrays can be replaced without copying; uniquely owned mapped point slices are updated in place and 1,000 alternating 10,000-point preparations allocate zero bytes after warm-up.",
        release_requirement: "Keep streaming cache ownership, held-frame preservation, validation, and allocation-contract tests green.",
    },
    ChartCapabilityEntry {
        id: "visual-regression",
        capability: "Visual regression baselines",
        chart_families: "all rendered chart families",
        story_ids: PUBLIC_CHART_STORY_IDS,
        test_contracts: &[
            "chart_visual_regression_manifest tests",
            "component-lab:px-story-conformance",
        ],
        status: ChartCapabilityStatus::Partial,
        evidence: "chart_visual_regression_manifest() records chart story ids, dashboard/panel/mobile viewports, light/dark/high-contrast schemes, and stable baseline/actual/diff artifact paths for every public chart family plus optional surface3d and mesh_plot.",
        release_requirement: "Keep chart visual-regression manifest tests green and execute the listed captures through component-lab/showcase visual QA before attaching release artifacts.",
    },
    ChartCapabilityEntry {
        id: "native-accessibility-bridge",
        capability: "Native screen-reader bridge consumption",
        chart_families: "all chart families",
        story_ids: PUBLIC_CHART_STORY_IDS,
        test_contracts: &[
            "accessibility bridge snapshot tests",
            "platform capability matrix",
        ],
        status: ChartCapabilityStatus::AppBridgeRequired,
        evidence: "ChartAccessibilitySummary::to_accessibility_tree() and to_bridge_snapshot() convert chart summaries into gpui-ui-kit AccessibilityTree/AccessibilityBridgeSnapshot payloads with image roles, labels, descriptions, ranges, scales, and series labels for host/native adapters.",
        release_requirement: "Keep accessibility bridge tests green and attach product-level screen-reader QA before claiming OS-level chart accessibility parity.",
    },
    ChartCapabilityEntry {
        id: "mesh-plot",
        capability: "Unstructured triangle mesh plots",
        chart_families: "mesh_plot",
        story_ids: MESH_PLOT_RELEASE_STORY_IDS,
        test_contracts: &[
            "mesh_plot builder and validation tests",
            "mesh_plot SVG and accessibility tests",
            "component-lab:px.mesh_plot",
        ],
        status: ChartCapabilityStatus::Partial,
        evidence: "TriangleMesh validation, scalar fields, contour/isoline preparation, deterministic SVG/PNG export, live native accessibility registration, Python frame retention, split resource-backed native decoding, all 11 component-lab MeshPlot stories, reviewed Metal parent-story baseline/diff artifacts, and verified Python host selection/callback artifacts are present; reference-machine allocation evidence and clean all-variant baselines remain open.",
        release_requirement: "Complete reference-machine allocation/performance evidence and promote clean-release visual baselines for every MeshPlot variant before promoting to implemented.",
    },
    ChartCapabilityEntry {
        id: "mesh-plot-rendering",
        capability: "Mesh plot rendering",
        chart_families: "mesh_plot (2D and 3D)",
        story_ids: MESH_PLOT_RELEASE_STORY_IDS,
        test_contracts: &[
            "mesh_3d_cache_tests",
            "gpui-d3rs offscreen renderer smoke tests",
            "component-lab:px.mesh_plot",
        ],
        status: ChartCapabilityStatus::Partial,
        evidence: "Retained 2D/3D geometry upload, Metal/wgpu custom draws, and an offscreen fallback are implemented. Native Metal tests cover depth, clipping, wireframe, large-revolve preparation, and current-camera Surface3d/revolve export; all 11 stories have a local 3x3 capture matrix, while only the parent story has reviewed versioned Metal baselines.",
        release_requirement: "Promote clean all-variant baselines and verify adapter-backed runtime rendering on each supported lane.",
    },
    ChartCapabilityEntry {
        id: "mesh-plot-fields",
        capability: "Mesh plot vertex and cell scalar fields",
        chart_families: "mesh_plot",
        story_ids: MESH_PLOT_RELEASE_STORY_IDS,
        test_contracts: &[
            "mesh_plot builder and validation tests",
            "mesh_3d_cache_tests",
        ],
        status: ChartCapabilityStatus::Partial,
        evidence: "Vertex and cell associations, validity masks, smooth/flat interpolation, NaN upload sentinels, and field-only retained updates are covered by focused Rust tests.",
        release_requirement: "Add reference visual evidence for both associations and masked-field behavior.",
    },
    ChartCapabilityEntry {
        id: "mesh-plot-contours",
        capability: "Mesh plot filled contours and isolines",
        chart_families: "mesh_plot",
        story_ids: MESH_PLOT_RELEASE_STORY_IDS,
        test_contracts: &[
            "mesh_contour_golden",
            "mesh_compute_diff_tests",
            "mesh_plot SVG tests",
        ],
        status: ChartCapabilityStatus::Partial,
        evidence: "Unstructured marching-triangle bands, isolines, GPU/CPU tie-break parity, and deterministic SVG output are implemented and focused-tested.",
        release_requirement: "Attach renderer-backed contour/isoline captures and readback diff evidence.",
    },
    ChartCapabilityEntry {
        id: "mesh-plot-axisymmetric",
        capability: "Mesh plot axisymmetric section and revolve",
        chart_families: "mesh_plot",
        story_ids: MESH_PLOT_RELEASE_STORY_IDS,
        test_contracts: &[
            "revolve tests",
            "component-lab:px.mesh_plot.axisymmetric_section",
            "component-lab:px.mesh_plot.revolve",
        ],
        status: ChartCapabilityStatus::Partial,
        evidence: "Axisymmetric r-z section and retained revolve/source mapping are implemented with radius and sweep validation; dedicated section/revolve component-lab stories and serialized native Metal depth/current-camera-export coverage are present.",
        release_requirement: "Promote clean visual baselines and verify revolved rendering/picking on every reference GPU lane.",
    },
    ChartCapabilityEntry {
        id: "mesh-plot-retained-updates",
        capability: "Mesh plot retained field-only updates",
        chart_families: "mesh_plot",
        story_ids: MESH_PLOT_RELEASE_STORY_IDS,
        test_contracts: &["mesh_plot_allocation_contracts", "mesh_3d_cache_tests"],
        status: ChartCapabilityStatus::Partial,
        evidence: "Geometry/field revisions, cache invalidation, field-only state preservation, and warmed allocation contracts are present.",
        release_requirement: "Attach reference-machine allocation and bounded-memory results for the release benchmark sizes.",
    },
    ChartCapabilityEntry {
        id: "mesh-plot-interaction",
        capability: "Mesh plot picking and native navigation",
        chart_families: "mesh_plot",
        story_ids: MESH_PLOT_RELEASE_STORY_IDS,
        test_contracts: &[
            "mesh_plot picking tests",
            "mesh_selection_payload test",
            "component-lab:px.mesh_plot.picking",
        ],
        status: ChartCapabilityStatus::Partial,
        evidence: "2D/3D picking, stable IDs, retained camera/viewport state, typed host selection payload construction, and end-to-end native Metal/Python-host pointer QA artifacts are present.",
        release_requirement: "Attach reference-lane interaction evidence before claiming cross-adapter product parity.",
    },
    ChartCapabilityEntry {
        id: "mesh-plot-python",
        capability: "Mesh plot Python and resource parity",
        chart_families: "mesh_plot",
        story_ids: MESH_PLOT_RELEASE_STORY_IDS,
        test_contracts: &[
            "test_meshplot.py",
            "test_meshplot_protocol.py",
            "Python mesh protocol tests",
        ],
        status: ChartCapabilityStatus::Partial,
        evidence: "Inline and retained binary geometry, fields, masks, IDs, revisioned patches, resource eviction, selection event schemas, and a real resource-backed Python host selection/callback trace are covered by protocol, unit, and native-host tests.",
        release_requirement: "Run Python-authored selection and patch flows through the native host session.",
    },
    ChartCapabilityEntry {
        id: "mesh-plot-export-accessibility",
        capability: "Mesh plot export and accessibility",
        chart_families: "mesh_plot",
        story_ids: MESH_PLOT_RELEASE_STORY_IDS,
        test_contracts: &[
            "mesh_plot SVG and accessibility tests",
            "mesh_plot_accessibility",
            "accessibility bridge tests",
        ],
        status: ChartCapabilityStatus::Partial,
        evidence: "Deterministic viewport-aware SVG/PNG export, structured accessibility summaries, live native image-role metadata, AccessibilityTree registration, and serialized native current-camera export coverage are implemented; product-level screen-reader and clean reference visual-export QA remain open.",
        release_requirement: "Attach clean exported-artifact comparisons and product screen-reader QA before claiming full parity.",
    },
];

/// Return the current chart capability report.
pub const fn chart_capability_report() -> ChartCapabilityReport {
    ChartCapabilityReport {
        schema_version: CHART_CAPABILITY_SCHEMA_VERSION,
        report_type: CHART_CAPABILITY_REPORT_TYPE,
        reviewed_on: "2026-08-10",
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
        assert_eq!(report.reviewed_on, "2026-08-10");
        assert!(!report.entries.is_empty());
        assert!(!report.all_release_ready());
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
            assert!(!entry.story_ids.is_empty());
            assert!(!entry.test_contracts.is_empty());
            assert!(!entry.status.as_str().is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn every_public_chart_story_has_builder_capability_ownership() {
        let builders = chart_capability_entries()
            .iter()
            .find(|entry| entry.id == "chart-builders")
            .unwrap();
        assert_eq!(builders.story_ids, PUBLIC_CHART_STORY_IDS);
        let unique = builders
            .story_ids
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(unique.len(), builders.story_ids.len());
    }

    #[test]
    fn chart_capability_report_marks_accessibility_ready() {
        assert!(chart_capability_entries().iter().any(|entry| {
            entry.id == "accessibility-summaries"
                && entry.status == ChartCapabilityStatus::Implemented
        }));
    }

    #[test]
    fn chart_capability_report_records_streaming_heap_contract() {
        let streaming = chart_capability_entries()
            .iter()
            .find(|entry| entry.id == "streaming-preparation")
            .unwrap();
        assert_eq!(streaming.status, ChartCapabilityStatus::Implemented);
        assert_eq!(streaming.story_ids, &["px.line", "px.scatter"]);
        assert!(streaming.evidence.contains("10,000-point"));
    }

    #[test]
    fn chart_capability_report_names_plotly_style_blockers() {
        let blocking = chart_capability_report()
            .blocking_entries()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();

        assert!(blocking.contains(&"interaction-state"));
        assert!(!blocking.contains(&"native-legends"));
        assert!(!blocking.contains(&"annotations"));
        assert!(!blocking.contains(&"static-export"));
        assert!(blocking.contains(&"visual-regression"));
        assert!(blocking.contains(&"native-accessibility-bridge"));
    }

    #[test]
    fn mesh_plot_capability_report_has_separate_product_contracts() {
        let ids = chart_capability_entries()
            .iter()
            .filter(|entry| entry.id.starts_with("mesh-plot"))
            .map(|entry| entry.id)
            .collect::<std::collections::BTreeSet<_>>();

        for id in [
            "mesh-plot",
            "mesh-plot-rendering",
            "mesh-plot-fields",
            "mesh-plot-contours",
            "mesh-plot-axisymmetric",
            "mesh-plot-retained-updates",
            "mesh-plot-interaction",
            "mesh-plot-python",
            "mesh-plot-export-accessibility",
        ] {
            assert!(ids.contains(id), "missing mesh plot capability {id}");
        }
    }

    #[test]
    fn mesh_plot_capabilities_name_every_release_story_variant() {
        let expected = MESH_PLOT_RELEASE_STORY_IDS
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(expected.len(), 11);

        for entry in chart_capability_entries()
            .iter()
            .filter(|entry| entry.id.starts_with("mesh-plot"))
        {
            assert_eq!(
                entry
                    .story_ids
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>(),
                expected,
                "{} must retain all release-story provenance",
                entry.id
            );
            assert_eq!(entry.status, ChartCapabilityStatus::Partial);
        }
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
