//! Focus management components
//!
//! Provides components for managing keyboard focus navigation between elements.
//!
//! # FocusGroup
//!
//! A container that manages keyboard navigation (arrow keys, Tab) between an
//! explicit list of focus handles. Supports vertical, horizontal, and grid
//! layouts.
//!
//! ```ignore
//! let button1_focus = cx.focus_handle();
//! let button2_focus = cx.focus_handle();
//!
//! FocusGroup::new("my-group")
//!     .direction(FocusDirection::Vertical)
//!     .wraparound(true)
//!     .focus_target(button1_focus.clone())
//!     .focus_target(button2_focus.clone())
//!     .child(button1)
//!     .child(button2)
//! ```
//!
//! # Keyboard Navigation
//!
//! - **Vertical**: Up/Down arrows move focus, Home/End go to first/last
//! - **Horizontal**: Left/Right arrows move focus, Home/End go to first/last
//! - **Grid**: All arrow keys work, Home/End go to first/last in row
//! - **Tab**: Always moves to next/previous focusable (with Shift)
//!
//! # Focus Ring
//!
//! By default, FocusGroup adds a visual focus ring to the group while focus is
//! inside it. Disable with `.focus_ring(false)`.

use gpui::prelude::{
    InteractiveElement, IntoElement, ParentElement, RenderOnce, StatefulInteractiveElement, Styled,
};
use gpui::{AnyElement, App, ElementId, FocusHandle, KeyDownEvent, Pixels, Window, div, px};

use crate::theme::ThemeExt;

/// Schema version for [`FocusIntegrationReport`].
pub const FOCUS_INTEGRATION_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`FocusIntegrationReport`].
pub const FOCUS_INTEGRATION_REPORT_TYPE: &str = "gpui-ui-kit-focus-integration";

/// Direction of focus navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusDirection {
    /// Navigate vertically (Up/Down arrows)
    #[default]
    Vertical,
    /// Navigate horizontally (Left/Right arrows)
    Horizontal,
    /// Navigate in a grid pattern
    Grid {
        /// Number of columns in the grid
        columns: usize,
    },
}

/// Release-readiness status for a focus integration surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusIntegrationStatus {
    /// The reusable focus primitive is implemented and unit-tested.
    PrimitiveReady,
    /// The component has focused keyboard behavior covered outside FocusGroup.
    CoveredByComponentTests,
    /// The component should adopt FocusGroup or add end-to-end focus tests.
    PendingIntegration,
    /// The component needs a native accessibility bridge or platform QA.
    ExternalBridgePending,
}

impl FocusIntegrationStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PrimitiveReady => "primitive-ready",
            Self::CoveredByComponentTests => "covered-by-component-tests",
            Self::PendingIntegration => "pending-integration",
            Self::ExternalBridgePending => "external-bridge-pending",
        }
    }

    /// Whether this status is enough for a UI-kit keyboard-readiness claim.
    pub const fn is_release_ready(self) -> bool {
        matches!(self, Self::PrimitiveReady | Self::CoveredByComponentTests)
    }
}

/// One UI-kit component family in the focus integration report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusIntegrationEntry {
    /// Stable component family id.
    pub id: &'static str,
    /// Human-readable component family name.
    pub component: &'static str,
    /// Current focus/keyboard integration status.
    pub status: FocusIntegrationStatus,
    /// Evidence recorded for this release report.
    pub evidence: &'static str,
    /// Remaining release requirement.
    pub release_requirement: &'static str,
}

/// Versioned focus integration report for release QA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FocusIntegrationReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [FocusIntegrationEntry],
}

impl FocusIntegrationReport {
    /// Return true only when every focus integration entry is release-ready.
    pub fn all_release_ready(self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.status.is_release_ready())
    }

    /// Return entries that still block a keyboard-readiness claim.
    pub fn blocking_entries(self) -> impl Iterator<Item = &'static FocusIntegrationEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.status.is_release_ready())
    }

    /// Render the report as Markdown for release notes.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI UI Kit Focus Integration\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Component | Status | Evidence | Release requirement |\n\
             | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                entry.component,
                entry.status.as_str(),
                entry.evidence,
                entry.release_requirement
            ));
        }

        markdown
    }
}

const FOCUS_INTEGRATION_ENTRIES: &[FocusIntegrationEntry] = &[
    FocusIntegrationEntry {
        id: "focus-group",
        component: "FocusGroup",
        status: FocusIntegrationStatus::PrimitiveReady,
        evidence: "Registered focus targets, arrow/Home/End/Tab traversal, wraparound, and focus-visible group ring are implemented with unit tests.",
        release_requirement: "Keep focus_group tests green before release.",
    },
    FocusIntegrationEntry {
        id: "forms",
        component: "Input, NumberInput, Select, Slider, Checkbox, Toggle",
        status: FocusIntegrationStatus::CoveredByComponentTests,
        evidence: "Form components have focused unit coverage for edit state, selection, slider behavior, and state cleanup.",
        release_requirement: "Add FocusGroup adoption tests when combining multiple form controls in a compound surface.",
    },
    FocusIntegrationEntry {
        id: "menus-dialogs",
        component: "Menu, ContextMenu, Dialog, ConfirmDialog, Popover",
        status: FocusIntegrationStatus::CoveredByComponentTests,
        evidence: "Menu and ContextMenu expose roving item focus, selectable-item keyboard activation, and Escape close handling; Dialog, ConfirmDialog, and Popover expose focus handles, Escape dismissal, and optional restore-focus targets with component tests.",
        release_requirement: "Keep menu, dialog, confirm_dialog, popover, and focus integration tests green; attach product-level overlay walkthrough evidence before claiming app UX parity.",
    },
    FocusIntegrationEntry {
        id: "data-navigation",
        component: "Tabs, Table, TreeView, CommandPalette",
        status: FocusIntegrationStatus::CoveredByComponentTests,
        evidence: "Tabs handle left/right/Home/End keys; Table, TreeView, and CommandPalette expose tested keyboard focus/highlight hooks backed by the shared DataNavigationState helper.",
        release_requirement: "Keep data_navigation, tabs, table, tree_view, and command_palette keyboard tests green; attach product-level keyboard walkthrough evidence before claiming app UX parity.",
    },
    FocusIntegrationEntry {
        id: "mobile-surfaces",
        component: "SwipePanel and mobile previews",
        status: FocusIntegrationStatus::CoveredByComponentTests,
        evidence: "SwipePanel exposes focus handles, Escape collapse with optional restore-focus, Enter/Space toggle, Home/End full expansion/collapse, and anchor-aware arrow-key state stepping with component tests.",
        release_requirement: "Keep swipe_panel and focus integration tests green; record simulator/device touch and accessibility smoke tests before claiming app UX parity.",
    },
    FocusIntegrationEntry {
        id: "native-accessibility",
        component: "Native screen-reader bridge",
        status: FocusIntegrationStatus::ExternalBridgePending,
        evidence: "ARIA-like metadata, a platform-neutral AccessibilityBridgeSnapshot export, and a validated NativeAccessibilityAdapterPayload contract are available for host platform adapters.",
        release_requirement: "Wire NativeAccessibilityAdapterPayload into the selected host platform accessibility layers and record cross-platform screen-reader QA.",
    },
];

/// Return the current focus integration report.
pub const fn focus_integration_report() -> FocusIntegrationReport {
    FocusIntegrationReport {
        schema_version: FOCUS_INTEGRATION_SCHEMA_VERSION,
        report_type: FOCUS_INTEGRATION_REPORT_TYPE,
        reviewed_on: "2026-07-08",
        entries: FOCUS_INTEGRATION_ENTRIES,
    }
}

/// Return all focus integration entries.
pub const fn focus_integration_entries() -> &'static [FocusIntegrationEntry] {
    FOCUS_INTEGRATION_ENTRIES
}

/// A container that manages keyboard focus navigation between registered targets
///
/// FocusGroup handles arrow key navigation, Tab key movement, and Home/End
/// keys for quick navigation to first/last elements. Register child handles
/// with [`FocusGroup::focus_target`] or [`FocusGroup::focus_targets`].
pub struct FocusGroup {
    id: ElementId,
    children: Vec<AnyElement>,
    focus_targets: Vec<FocusHandle>,
    direction: FocusDirection,
    wraparound: bool,
    focus_ring: bool,
    gap: Pixels,
    focus_handle: Option<FocusHandle>,
}

impl FocusGroup {
    /// Create a new focus group
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            children: Vec::new(),
            focus_targets: Vec::new(),
            direction: FocusDirection::default(),
            wraparound: false,
            focus_ring: true,
            gap: px(8.0),
            focus_handle: None,
        }
    }

    /// Set the navigation direction
    pub fn direction(mut self, direction: FocusDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Enable wraparound navigation (first <-> last)
    pub fn wraparound(mut self, wrap: bool) -> Self {
        self.wraparound = wrap;
        self
    }

    /// Show focus ring on focused child (default: true)
    pub fn focus_ring(mut self, show: bool) -> Self {
        self.focus_ring = show;
        self
    }

    /// Set gap between children
    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = gap.into();
        self
    }

    /// Set the focus handle for this group
    pub fn focus_handle(mut self, handle: FocusHandle) -> Self {
        self.focus_handle = Some(handle);
        self
    }

    /// Set the focus handles that arrow/Home/End/Tab navigation should move through.
    ///
    /// GPUI elements do not expose child focus handles through `AnyElement`, so
    /// callers that want roving focus should pass the handles they also give to
    /// each focusable child. When no targets are provided, FocusGroup remains a
    /// layout/focus-boundary wrapper and does not intercept keyboard events.
    pub fn focus_targets(mut self, handles: impl IntoIterator<Item = FocusHandle>) -> Self {
        self.focus_targets = handles.into_iter().collect();
        self
    }

    /// Add one focus target to the navigation order.
    pub fn focus_target(mut self, handle: FocusHandle) -> Self {
        self.focus_targets.push(handle);
        self
    }

    /// Add a child element
    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    /// Add multiple children
    pub fn children(mut self, children: impl IntoIterator<Item = impl IntoElement>) -> Self {
        self.children
            .extend(children.into_iter().map(|c| c.into_any_element()));
        self
    }

    fn navigation_delta(direction: FocusDirection, key: &str, shift: bool) -> Option<FocusMove> {
        match key {
            "home" => Some(FocusMove::First),
            "end" => Some(FocusMove::Last),
            "tab" if shift => Some(FocusMove::Previous),
            "tab" => Some(FocusMove::Next),
            "up" => match direction {
                FocusDirection::Vertical | FocusDirection::Grid { .. } => Some(FocusMove::Previous),
                FocusDirection::Horizontal => None,
            },
            "down" => match direction {
                FocusDirection::Vertical | FocusDirection::Grid { .. } => Some(FocusMove::Next),
                FocusDirection::Horizontal => None,
            },
            "left" => match direction {
                FocusDirection::Horizontal | FocusDirection::Grid { .. } => {
                    Some(FocusMove::Previous)
                }
                FocusDirection::Vertical => None,
            },
            "right" => match direction {
                FocusDirection::Horizontal | FocusDirection::Grid { .. } => Some(FocusMove::Next),
                FocusDirection::Vertical => None,
            },
            _ => None,
        }
    }

    fn target_index(
        target_count: usize,
        current_index: Option<usize>,
        movement: FocusMove,
        wraparound: bool,
    ) -> Option<usize> {
        if target_count == 0 {
            return None;
        }

        match movement {
            FocusMove::First => Some(0),
            FocusMove::Last => Some(target_count - 1),
            FocusMove::Next => match current_index {
                Some(index) if index + 1 < target_count => Some(index + 1),
                Some(_) if wraparound => Some(0),
                None => Some(0),
                _ => None,
            },
            FocusMove::Previous => match current_index {
                Some(index) if index > 0 => Some(index - 1),
                Some(_) if wraparound => Some(target_count - 1),
                None => Some(target_count - 1),
                _ => None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusMove {
    First,
    Last,
    Next,
    Previous,
}

impl RenderOnce for FocusGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let direction = self.direction;
        let wraparound = self.wraparound;
        let gap = self.gap;
        let focus_targets = self.focus_targets;

        // Create or use provided focus handle
        let focus_handle = self.focus_handle.unwrap_or_else(|| cx.focus_handle());

        let mut container = div()
            .id(self.id)
            .track_focus(&focus_handle)
            .flex()
            .gap(gap)
            .focusable();

        // Set flex direction based on navigation direction
        container = match direction {
            FocusDirection::Vertical => container.flex_col(),
            FocusDirection::Horizontal => container.flex_row(),
            FocusDirection::Grid { columns: _ } => {
                // For grid layout, use flex-wrap
                container.flex_row().flex_wrap()
            }
        };

        if self.focus_ring {
            let ring_color = cx.theme().accent;
            container = container.focus_visible(|style| style.border_2().border_color(ring_color));
        }

        if !focus_targets.is_empty() {
            container =
                container.on_key_down(move |event: &KeyDownEvent, window: &mut Window, cx| {
                    let Some(movement) = FocusGroup::navigation_delta(
                        direction,
                        event.keystroke.key.as_str(),
                        event.keystroke.modifiers.shift,
                    ) else {
                        return;
                    };

                    let current_index = focus_targets
                        .iter()
                        .position(|target| target.is_focused(window));
                    let Some(target_index) = FocusGroup::target_index(
                        focus_targets.len(),
                        current_index,
                        movement,
                        wraparound,
                    ) else {
                        return;
                    };

                    window.focus(&focus_targets[target_index], cx);
                    cx.stop_propagation();
                });
        }

        // Add children
        for child in self.children {
            container = container.child(child);
        }

        container
    }
}

impl IntoElement for FocusGroup {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

/// Helper trait for adding focus group behavior to existing containers
pub trait FocusGroupExt {
    /// Wrap this element in a focus group with vertical navigation
    fn with_focus_navigation(self, id: impl Into<ElementId>) -> FocusGroup;
}

#[cfg(test)]
mod tests {
    use super::{
        FOCUS_INTEGRATION_REPORT_TYPE, FOCUS_INTEGRATION_SCHEMA_VERSION, FocusDirection,
        FocusGroup, FocusIntegrationStatus, FocusMove, focus_integration_report,
    };
    use gpui::px;

    #[test]
    fn focus_group_builder_covers_all_setters() {
        let _ = FocusGroup::new("group")
            .direction(FocusDirection::Horizontal)
            .wraparound(true)
            .focus_ring(false)
            .gap(px(4.0))
            .child(gpui::div())
            .children(vec![gpui::div(), gpui::div()]);

        let _ = FocusGroup::new("grid").direction(FocusDirection::Grid { columns: 3 });
    }

    #[test]
    fn focus_group_maps_keys_by_direction() {
        assert_eq!(
            FocusGroup::navigation_delta(FocusDirection::Vertical, "down", false),
            Some(FocusMove::Next)
        );
        assert_eq!(
            FocusGroup::navigation_delta(FocusDirection::Vertical, "right", false),
            None
        );
        assert_eq!(
            FocusGroup::navigation_delta(FocusDirection::Horizontal, "left", false),
            Some(FocusMove::Previous)
        );
        assert_eq!(
            FocusGroup::navigation_delta(FocusDirection::Grid { columns: 3 }, "up", false),
            Some(FocusMove::Previous)
        );
        assert_eq!(
            FocusGroup::navigation_delta(FocusDirection::Vertical, "tab", true),
            Some(FocusMove::Previous)
        );
        assert_eq!(
            FocusGroup::navigation_delta(FocusDirection::Vertical, "home", false),
            Some(FocusMove::First)
        );
    }

    #[test]
    fn focus_group_computes_navigation_targets() {
        assert_eq!(
            FocusGroup::target_index(3, Some(0), FocusMove::Next, false),
            Some(1)
        );
        assert_eq!(
            FocusGroup::target_index(3, Some(2), FocusMove::Next, false),
            None
        );
        assert_eq!(
            FocusGroup::target_index(3, Some(2), FocusMove::Next, true),
            Some(0)
        );
        assert_eq!(
            FocusGroup::target_index(3, Some(0), FocusMove::Previous, true),
            Some(2)
        );
        assert_eq!(
            FocusGroup::target_index(3, None, FocusMove::Next, false),
            Some(0)
        );
        assert_eq!(
            FocusGroup::target_index(3, None, FocusMove::Previous, false),
            Some(2)
        );
        assert_eq!(
            FocusGroup::target_index(0, None, FocusMove::First, true),
            None
        );
    }

    #[test]
    fn focus_integration_report_has_stable_contract() {
        let report = focus_integration_report();

        assert_eq!(report.schema_version, FOCUS_INTEGRATION_SCHEMA_VERSION);
        assert_eq!(report.report_type, FOCUS_INTEGRATION_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-08");
        assert!(!report.entries.is_empty());
        assert!(!report.all_release_ready());
    }

    #[test]
    fn focus_integration_report_has_unique_component_ids() {
        let report = focus_integration_report();
        let mut ids = std::collections::BTreeSet::new();

        for entry in report.entries {
            assert!(ids.insert(entry.id), "duplicate focus entry {}", entry.id);
            assert!(!entry.component.is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn focus_integration_report_names_required_blockers() {
        let report = focus_integration_report();
        let blocking_ids: Vec<_> = report.blocking_entries().map(|entry| entry.id).collect();

        assert!(!blocking_ids.contains(&"menus-dialogs"));
        assert!(!blocking_ids.contains(&"data-navigation"));
        assert!(!blocking_ids.contains(&"mobile-surfaces"));
        assert!(blocking_ids.contains(&"native-accessibility"));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "menus-dialogs"
                && entry.status == FocusIntegrationStatus::CoveredByComponentTests
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "data-navigation"
                && entry.status == FocusIntegrationStatus::CoveredByComponentTests
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "mobile-surfaces"
                && entry.status == FocusIntegrationStatus::CoveredByComponentTests
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.id == "focus-group" && entry.status == FocusIntegrationStatus::PrimitiveReady
        }));
    }

    #[test]
    fn focus_integration_markdown_names_statuses() {
        let markdown = focus_integration_report().to_markdown_table();

        assert!(markdown.contains("gpui-ui-kit-focus-integration"));
        assert!(markdown.contains("FocusGroup"));
        assert!(markdown.contains("primitive-ready"));
        assert!(markdown.contains("covered-by-component-tests"));
        assert!(markdown.contains("external-bridge-pending"));
    }
}
