//! Unified interactive-component behavior matrix for release QA.

/// Schema version for [`ComponentBehaviorReport`].
pub const COMPONENT_BEHAVIOR_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`ComponentBehaviorReport`].
pub const COMPONENT_BEHAVIOR_REPORT_TYPE: &str = "gpui-ui-kit-component-behavior-matrix";

/// Evidence status for one behavior dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BehaviorStatus {
    /// Implemented and exercised by focused component tests.
    Tested,
    /// Implemented and covered by component-lab conformance metadata/tests.
    ConformanceTested,
    /// The reusable contract exists, but native runtime QA remains.
    PlatformQaPending,
    /// The behavior does not apply to this component family.
    NotApplicable,
}

impl BehaviorStatus {
    /// Stable label used by generated reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tested => "tested",
            Self::ConformanceTested => "conformance-tested",
            Self::PlatformQaPending => "platform-qa-pending",
            Self::NotApplicable => "not-applicable",
        }
    }

    /// Whether this dimension is fully proved without an external platform run.
    pub const fn is_release_ready(self) -> bool {
        !matches!(self, Self::PlatformQaPending)
    }
}

/// Behavior evidence for one public interactive component family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentBehaviorEntry {
    pub id: &'static str,
    pub components: &'static str,
    pub pointer: BehaviorStatus,
    pub keyboard: BehaviorStatus,
    pub touch: BehaviorStatus,
    pub focus: BehaviorStatus,
    pub disabled: BehaviorStatus,
    pub accessibility: BehaviorStatus,
    pub responsive: BehaviorStatus,
    pub reduced_motion: BehaviorStatus,
    pub high_contrast: BehaviorStatus,
    pub evidence: &'static str,
    pub release_requirement: &'static str,
}

impl ComponentBehaviorEntry {
    /// Return true when every applicable behavior dimension is release-ready.
    pub const fn all_release_ready(self) -> bool {
        self.pointer.is_release_ready()
            && self.keyboard.is_release_ready()
            && self.touch.is_release_ready()
            && self.focus.is_release_ready()
            && self.disabled.is_release_ready()
            && self.accessibility.is_release_ready()
            && self.responsive.is_release_ready()
            && self.reduced_motion.is_release_ready()
            && self.high_contrast.is_release_ready()
    }
}

/// Versioned UI-kit behavior matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentBehaviorReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [ComponentBehaviorEntry],
}

impl ComponentBehaviorReport {
    pub fn all_release_ready(self) -> bool {
        self.entries.iter().all(|entry| entry.all_release_ready())
    }

    pub fn blocking_entries(self) -> impl Iterator<Item = &'static ComponentBehaviorEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.all_release_ready())
    }

    /// Render an attachable Markdown release artifact.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI UI Kit Component Behavior Matrix\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Family | Pointer | Keyboard | Touch | Focus | Disabled | Accessibility | Responsive | Reduced motion | High contrast |\n\
             | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );
        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                entry.components,
                entry.pointer.as_str(),
                entry.keyboard.as_str(),
                entry.touch.as_str(),
                entry.focus.as_str(),
                entry.disabled.as_str(),
                entry.accessibility.as_str(),
                entry.responsive.as_str(),
                entry.reduced_motion.as_str(),
                entry.high_contrast.as_str(),
            ));
        }
        markdown
    }
}

const TESTED: BehaviorStatus = BehaviorStatus::Tested;
const CONFORMANCE: BehaviorStatus = BehaviorStatus::ConformanceTested;
const PLATFORM: BehaviorStatus = BehaviorStatus::PlatformQaPending;
const NA: BehaviorStatus = BehaviorStatus::NotApplicable;

const COMPONENT_BEHAVIOR_ENTRIES: &[ComponentBehaviorEntry] = &[
    ComponentBehaviorEntry {
        id: "actions",
        components: "Button, IconButton, ButtonSet, Link, Toolbar",
        pointer: TESTED,
        keyboard: TESTED,
        touch: CONFORMANCE,
        focus: TESTED,
        disabled: TESTED,
        accessibility: TESTED,
        responsive: CONFORMANCE,
        reduced_motion: CONFORMANCE,
        high_contrast: CONFORMANCE,
        evidence: "Component tests cover activation/focus/disabled semantics; component-lab covers touch targets, viewports, reduced motion, and high-contrast themes.",
        release_requirement: "Keep action component and component-lab conformance tests green.",
    },
    ComponentBehaviorEntry {
        id: "forms",
        components: "Input, NumberInput, Select, Slider, Checkbox, Toggle, ColorPickerView, SearchBar",
        pointer: TESTED,
        keyboard: TESTED,
        touch: CONFORMANCE,
        focus: TESTED,
        disabled: TESTED,
        accessibility: TESTED,
        responsive: CONFORMANCE,
        reduced_motion: CONFORMANCE,
        high_contrast: CONFORMANCE,
        evidence: "Form unit/integration tests cover editing, selection, stepping, disabled state, focus and ARIA metadata; component-lab covers responsive/touch/theme matrices.",
        release_requirement: "Add native IME and screen-reader evidence per platform before product release.",
    },
    ComponentBehaviorEntry {
        id: "overlays",
        components: "Dialog, ConfirmDialog, Popover, ContextMenu, Menu, MenuBar, Tooltip",
        pointer: TESTED,
        keyboard: TESTED,
        touch: CONFORMANCE,
        focus: TESTED,
        disabled: NA,
        accessibility: TESTED,
        responsive: CONFORMANCE,
        reduced_motion: CONFORMANCE,
        high_contrast: CONFORMANCE,
        evidence: "Focused tests cover Escape, activation, roving focus and focus restoration; visual/conformance matrices cover layout, motion and contrast.",
        release_requirement: "Attach platform overlay positioning and screen-reader walkthrough evidence.",
    },
    ComponentBehaviorEntry {
        id: "navigation",
        components: "Tabs, Accordion, Breadcrumbs, Sidebar, Wizard, StepIndicator, FocusGroup",
        pointer: TESTED,
        keyboard: TESTED,
        touch: CONFORMANCE,
        focus: TESTED,
        disabled: NA,
        accessibility: TESTED,
        responsive: CONFORMANCE,
        reduced_motion: CONFORMANCE,
        high_contrast: CONFORMANCE,
        evidence: "Navigation and FocusGroup tests cover arrow/Home/End/Tab behavior and state; component-lab supplies viewport/touch/theme conformance.",
        release_requirement: "Keep composite navigation focus order stable and attach mobile walkthrough evidence.",
    },
    ComponentBehaviorEntry {
        id: "data-navigation",
        components: "Table, TreeView, CommandPalette, DragList",
        pointer: TESTED,
        keyboard: TESTED,
        touch: CONFORMANCE,
        focus: TESTED,
        disabled: NA,
        accessibility: TESTED,
        responsive: CONFORMANCE,
        reduced_motion: CONFORMANCE,
        high_contrast: CONFORMANCE,
        evidence: "Shared DataNavigationState and component tests cover keyboard/highlight behavior; component-lab covers rendered bounds and theme/motion matrices.",
        release_requirement: "Add large-data frame/allocation budgets and native accessibility traversal evidence.",
    },
    ComponentBehaviorEntry {
        id: "workflow",
        components: "WorkflowCanvas, WorkflowNode, Port, PaneDivider, SplitPane",
        pointer: TESTED,
        keyboard: PLATFORM,
        touch: PLATFORM,
        focus: PLATFORM,
        disabled: NA,
        accessibility: PLATFORM,
        responsive: CONFORMANCE,
        reduced_motion: CONFORMANCE,
        high_contrast: CONFORMANCE,
        evidence: "Graph/history/hit-test and divider pointer behavior are tested; complete keyboard/touch/native accessibility traversal remains platform-level work.",
        release_requirement: "Implement and record keyboard/touch canvas manipulation and native accessible graph navigation.",
    },
    ComponentBehaviorEntry {
        id: "mobile-gestures",
        components: "SwipePanel, ContextPreview, PullToRefreshState, WaveformScrubber",
        pointer: TESTED,
        keyboard: TESTED,
        touch: PLATFORM,
        focus: TESTED,
        disabled: NA,
        accessibility: PLATFORM,
        responsive: CONFORMANCE,
        reduced_motion: CONFORMANCE,
        high_contrast: CONFORMANCE,
        evidence: "State and keyboard equivalents are component-tested; physical gesture and native accessibility behavior requires simulator/device evidence.",
        release_requirement: "Attach iOS and Android gesture, reduced-motion and screen-reader smoke results.",
    },
];

pub const fn component_behavior_report() -> ComponentBehaviorReport {
    ComponentBehaviorReport {
        schema_version: COMPONENT_BEHAVIOR_SCHEMA_VERSION,
        report_type: COMPONENT_BEHAVIOR_REPORT_TYPE,
        reviewed_on: "2026-07-12",
        entries: COMPONENT_BEHAVIOR_ENTRIES,
    }
}

pub const fn component_behavior_entries() -> &'static [ComponentBehaviorEntry] {
    COMPONENT_BEHAVIOR_ENTRIES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavior_matrix_has_stable_unique_complete_rows() {
        let report = component_behavior_report();
        assert_eq!(report.schema_version, COMPONENT_BEHAVIOR_SCHEMA_VERSION);
        assert_eq!(report.report_type, COMPONENT_BEHAVIOR_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-12");

        let mut ids = std::collections::BTreeSet::new();
        for entry in report.entries {
            assert!(
                ids.insert(entry.id),
                "duplicate behavior family {}",
                entry.id
            );
            assert!(!entry.components.is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn behavior_matrix_is_honest_about_platform_blockers() {
        let report = component_behavior_report();
        assert!(!report.all_release_ready());
        let blockers = report
            .blocking_entries()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        assert_eq!(blockers, ["workflow", "mobile-gestures"]);
    }

    #[test]
    fn behavior_matrix_covers_required_interactive_families() {
        let ids = component_behavior_entries()
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();
        for required in [
            "actions",
            "forms",
            "overlays",
            "navigation",
            "data-navigation",
            "workflow",
            "mobile-gestures",
        ] {
            assert!(
                ids.contains(&required),
                "missing behavior family {required}"
            );
        }
    }

    #[test]
    fn behavior_matrix_markdown_names_every_dimension() {
        let markdown = component_behavior_report().to_markdown_table();
        for heading in [
            "Pointer",
            "Keyboard",
            "Touch",
            "Focus",
            "Disabled",
            "Accessibility",
            "Responsive",
            "Reduced motion",
            "High contrast",
        ] {
            assert!(markdown.contains(heading), "missing heading {heading}");
        }
        assert!(markdown.contains("platform-qa-pending"));
    }
}
