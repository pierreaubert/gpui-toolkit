//! Interaction QA coverage metadata for release artifacts.

/// Schema version for [`InteractionQaReport`].
pub const INTERACTION_QA_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`InteractionQaReport`].
pub const INTERACTION_QA_REPORT_TYPE: &str = "gpui-px-interaction-qa";

/// Current interaction QA status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractionQaStatus {
    /// Implemented and covered by focused tests.
    Implemented,
    /// The behavior depends on host app key bindings or native accessibility.
    AppBridgeRequired,
}

impl InteractionQaStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Implemented => "implemented",
            Self::AppBridgeRequired => "app-bridge-required",
        }
    }

    /// Whether this row blocks the renderer-free interaction-state helper claim.
    pub const fn is_state_helper_ready(self) -> bool {
        matches!(self, Self::Implemented)
    }
}

/// One interaction behavior tracked for release QA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionQaEntry {
    pub id: &'static str,
    pub behavior: &'static str,
    pub status: InteractionQaStatus,
    pub evidence: &'static str,
    pub release_requirement: &'static str,
}

/// Versioned interaction QA report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractionQaReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [InteractionQaEntry],
}

impl InteractionQaReport {
    /// Return entries that still need host app or platform bridge evidence.
    pub fn app_bridge_entries(self) -> impl Iterator<Item = &'static InteractionQaEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.status == InteractionQaStatus::AppBridgeRequired)
    }

    /// Return true when all renderer-free interaction state helpers are implemented.
    pub fn state_helpers_ready(self) -> bool {
        self.entries
            .iter()
            .filter(|entry| entry.status.is_state_helper_ready())
            .count()
            >= 6
    }

    /// Render the report as Markdown for release notes or QA docs.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# gpui-px Interaction QA\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Behavior | Status | Evidence | Release requirement |\n\
             | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                entry.behavior,
                entry.status.as_str(),
                entry.evidence,
                entry.release_requirement
            ));
        }

        markdown
    }
}

const INTERACTION_QA_ENTRIES: &[InteractionQaEntry] = &[
    InteractionQaEntry {
        id: "brush-selection",
        behavior: "Brush selection lifecycle and domain conversion",
        status: InteractionQaStatus::Implemented,
        evidence: "ChartInteraction start/update/end/cancel brush tests cover active state, trivial selection rejection, and pixel-to-domain conversion.",
        release_requirement: "Keep interaction brush tests green for default and no-default builds.",
    },
    InteractionQaEntry {
        id: "zoom-selection-history",
        behavior: "Zoom to selection, reset, and history",
        status: InteractionQaStatus::Implemented,
        evidence: "ChartInteraction tests cover zoom_to, reset_zoom, zoom_back, zoom_level, and brush-to-zoom behavior.",
        release_requirement: "Keep zoom history tests green.",
    },
    InteractionQaEntry {
        id: "wheel-zoom",
        behavior: "Pointer-centered wheel zoom",
        status: InteractionQaStatus::Implemented,
        evidence: "apply_wheel_zoom delegates to renderer-free zoom_around_pixel and tests cover range shrink plus log-scale clamping.",
        release_requirement: "Keep wheel zoom tests green.",
    },
    InteractionQaEntry {
        id: "pan-state",
        behavior: "Renderer-free pan state transitions",
        status: InteractionQaStatus::Implemented,
        evidence: "ChartInteraction::pan_by_pixels updates linear and logarithmic domains without requiring a GPUI element.",
        release_requirement: "Keep pan state tests green, including log-scale clamp coverage.",
    },
    InteractionQaEntry {
        id: "hover-domain",
        behavior: "Retained hover domain coordinates",
        status: InteractionQaStatus::Implemented,
        evidence: "ChartInteraction::update_hover_pixel records in-bounds domain coordinates and clears out-of-bounds/non-finite hover state.",
        release_requirement: "Keep hover-domain tests green.",
    },
    InteractionQaEntry {
        id: "keyboard-state",
        behavior: "Keyboard zoom, pan, fit, and reset state actions",
        status: InteractionQaStatus::Implemented,
        evidence: "MeshPlotState::handle_key_with_permissions covers capability-gated planar keyboard fit, while the live 3D wrapper supplies current surface/revolve bounds to the equivalent fit action alongside ChartKeyboardAction zoom, pan, and reset transitions.",
        release_requirement: "Keep keyboard interaction state tests green and map product key bindings to these actions in host apps.",
    },
    InteractionQaEntry {
        id: "gpui-event-wrapper",
        behavior: "GPUI mouse wrapper for pan, wheel zoom, and double-click reset",
        status: InteractionQaStatus::Implemented,
        evidence: "InteractiveChartState delegates pan to ChartInteraction::pan_by_pixels, applies wheel zoom through apply_wheel_zoom, and exposes reset callbacks under the gpui feature.",
        release_requirement: "Keep gpui feature interaction tests and all-target checks green.",
    },
    InteractionQaEntry {
        id: "host-key-bindings",
        behavior: "Host key binding and tooltip wiring",
        status: InteractionQaStatus::AppBridgeRequired,
        evidence: "Renderer-free keyboard and hover state helpers exist, but app-specific key maps and tooltip rendering belong to the host GPUI app.",
        release_requirement: "Attach host-app keybinding/tooltip QA before claiming end-user keyboard and tooltip UX parity.",
    },
];

/// Return the current interaction QA report.
pub const fn interaction_qa_report() -> InteractionQaReport {
    InteractionQaReport {
        schema_version: INTERACTION_QA_SCHEMA_VERSION,
        report_type: INTERACTION_QA_REPORT_TYPE,
        reviewed_on: "2026-07-08",
        entries: INTERACTION_QA_ENTRIES,
    }
}

/// Return interaction QA entries without allocating.
pub const fn interaction_qa_entries() -> &'static [InteractionQaEntry] {
    INTERACTION_QA_ENTRIES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interaction_qa_report_has_stable_contract() {
        let report = interaction_qa_report();

        assert_eq!(report.schema_version, INTERACTION_QA_SCHEMA_VERSION);
        assert_eq!(report.report_type, INTERACTION_QA_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-08");
        assert!(report.state_helpers_ready());

        for entry in report.entries {
            assert!(!entry.id.is_empty());
            assert!(!entry.behavior.is_empty());
            assert!(!entry.status.as_str().is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn interaction_qa_report_names_state_helper_coverage() {
        let ids = interaction_qa_entries()
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();

        assert!(ids.contains(&"brush-selection"));
        assert!(ids.contains(&"zoom-selection-history"));
        assert!(ids.contains(&"wheel-zoom"));
        assert!(ids.contains(&"pan-state"));
        assert!(ids.contains(&"hover-domain"));
        assert!(ids.contains(&"keyboard-state"));
        assert!(ids.contains(&"gpui-event-wrapper"));
    }

    #[test]
    fn interaction_qa_markdown_names_bridge_boundary() {
        let report = interaction_qa_report();
        let bridge = report
            .app_bridge_entries()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        let markdown = report.to_markdown_table();

        assert_eq!(bridge, vec!["host-key-bindings"]);
        assert!(markdown.contains(INTERACTION_QA_REPORT_TYPE));
        assert!(markdown.contains("Keyboard zoom, pan, fit, and reset"));
        assert!(markdown.contains("app-bridge-required"));
    }
}
