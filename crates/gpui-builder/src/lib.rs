#![forbid(unsafe_code)]

//! Generic constraint-based layout solver.
//!
//! Platform-agnostic layout system that resolves a tree of `LayoutNode`
//! declarations into concrete pixel sizes. Supports:
//!
//! - **Hard constraints** (`Sizing::Fixed`) — headers, footers, toolbars
//! - **Soft constraints** (`Sizing::Fractional`, `Sizing::Flex`) — resizable panels
//! - **Priority-based collapse** — lowest-priority panels collapse first
//! - **Auto-axis switching** — horizontal/vertical based on aspect ratio
//! - **Display tiers** — panels report their active display mode based on size
//! - **User preferences** — ratio overrides and manual collapse state
//!
//! # Usage
//!
//! ```rust
//! use gpui_builder::{solve, LayoutNode, ContainerNode, SlotNode, Sizing, Axis, LayoutPreferences};
//!
//! let children = [
//!     LayoutNode::Slot(SlotNode {
//!         id: "header",
//!         sizing: Sizing::Fixed(40.0),
//!         priority: 1.0,
//!         collapsible: false,
//!         display_tiers: &[],
//!         collapse_label: None,
//!     }),
//!     LayoutNode::Slot(SlotNode {
//!         id: "content",
//!         sizing: Sizing::flex(200.0),
//!         priority: 1.0,
//!         collapsible: false,
//!         display_tiers: &[],
//!         collapse_label: None,
//!     }),
//! ];
//!
//! let root = LayoutNode::Container(ContainerNode {
//!     id: "root",
//!     axis: Axis::Vertical,
//!     auto_axis: None,
//!     sizing: Sizing::flex(0.0),
//!     children: &children,
//!     divider_size: 0.0,
//! });
//!
//! let solved = solve(&root, 1200.0, 800.0, &LayoutPreferences::default());
//! assert_eq!(solved.find("header").unwrap().height, 40.0);
//! assert_eq!(solved.find("content").unwrap().height, 760.0);
//! ```

pub mod accessibility;
pub mod benchmark_report;
pub mod compat;
#[macro_use]
mod macros;
pub mod inspector;
pub mod plugin_chassis;
pub mod snapshots;
pub mod solved;
pub mod solver;
pub mod state;
pub mod stories;
pub mod types;
pub mod util;
pub mod validation;
pub mod visual_regression;

// Re-exports for convenience
pub use accessibility::{
    AccessibilityMetadata, AccessibilityNode, AccessibilityRole, AccessibilityTree,
    accessibility_tree_from_solved,
};
pub use benchmark_report::{
    BENCHMARK_REPORT_SCHEMA_VERSION, BENCHMARK_REPORT_TYPE, BenchmarkCase, BenchmarkReport,
    benchmark_cases, benchmark_report,
};
pub use inspector::{
    ContainerInspection, DisplayTierInspection, LayoutInspection, LayoutInspectionKind,
    LayoutInspectionNode, SizingInspection, SlotInspection, SolvedInspection, SolvedInspectionNode,
    inspect_layout, inspect_solved,
};
pub use snapshots::{LayoutSnapshot, LayoutSnapshotMatrix, LayoutViewport, solve_snapshot_matrix};
pub use solved::{
    CollapsedSlot, LayoutDebugReport, LayoutDebugWarning, LayoutDebugWarningKind, NodeIndex,
    OwnedSolvedNode, SolvedNode, SolvedNodeData, SolvedNodeRef, SolvedTree, SolvedTreeMap,
};
pub use solver::{
    RetainedLayoutSolver, TextMeasureCache, solve, solve_tree, solve_tree_into,
    solve_tree_into_with_cache, solve_tree_with_cache, solve_with_cache,
};
pub use state::{
    LayoutAction, LayoutCollapsedState, LayoutPreferenceSnapshot, LayoutRatioOverride, LayoutState,
};
pub use stories::{
    LayoutScenario, LayoutStory, LayoutStoryCatalog, SolvedLayoutScenario, StoryCollapsedState,
    StoryRatioOverride,
};
pub use types::{
    Axis, ContainerNode, DisplayTier, LayoutNode, LayoutPreferences, Sizing, SlotNode,
};
pub use validation::{
    LayoutIssue, LayoutIssueKind, LayoutIssueSeverity, LayoutValidationReport, validate_layout,
};
pub use visual_regression::{
    VisualColorScheme, VisualRegressionCase, VisualRegressionCoverageReport,
    VisualRegressionFinding, VisualRegressionManifest,
};

// gpui-pretext re-exports for text-measured sizing
pub use gpui_pretext::{
    EngineProfile as TextEngineProfile, PrepareOptions as TextPrepareOptions, TextMeasure,
};

// Compat re-exports
pub use compat::{
    ColumnRole, GroupDirection, KnobSize, Orientation, PluginAdaptations, PluginColumnConstraint,
    PluginLayoutThresholds, PluginLayoutTree, plugin_adaptations,
};

// Plugin chassis re-exports
pub use plugin_chassis::{
    ChassisLayout, FooterSpec, HeaderSpec, KnobSlot, RowSpec, SectionSpec, SolvedChassis,
    SolvedSection,
};
