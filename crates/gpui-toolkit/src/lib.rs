//! # GPUI Toolkit
//!
//! Convenience aggregate crate that re-exports all libraries in the
//! `gpui-toolkit` workspace so that a single dependency gives access to the
//! whole toolkit.
//!
//! This crate is intentionally unpublished while the workspace stabilizes. Its
//! feature sets define the public boundary used by release QA:
//!
//! - `core` (default): UI, design, layout, text, keybinding, themes, audio UI,
//!   and visualization crates considered for the public toolkit surface.
//! - `tooling`: support crates for labs, design tools, scaffolding, profiling,
//!   mini-app shells, and Python scene/runtime work.
//! - `platform`: AU and Apple mobile platform integration crates that require
//!   target-specific QA before public release.
//! - `ios`: iOS integration as a target-specific opt-in.
//! - `all`: every aggregate feature.
//!
//! `crate_stability_manifest()` exposes the current per-crate stability notes
//! used by release QA and should be updated before changing publish scope.
//! `release_qa_matrix()` exposes the platform/release gate matrix used to keep
//! compile checks, manual device passes, publish dry-runs, and dependency
//! hygiene visible before an external release.
//! `dependency_hygiene_report()` exposes the dependency security policy,
//! required audit/deny commands, local tool availability, and remaining release
//! blockers for dependency checks.
//! `publish_plan()` exposes ordered crate dry-run/publish status so workspace
//! crates that depend on each other are not treated as independent gates.
//! `release_notes_report()` exposes crate-level release-note readiness,
//! including required stability, limitation, platform-support, and artifact
//! sections.
//! `release_notes_artifact_report()` records the stable release-note
//! attachment inventory, separating generated in-repo reports from publish,
//! platform, and manual QA gates.
//! `release_packaging_report()` exposes the packaging pass/fail evidence ledger
//! for public, beta, internal, patched, and platform delivery lanes.
//! `vendored_patch_manifest()` exposes the active/inactive vendored dependency
//! patch stack so upstream refs, local changes, and upgrade gates stay visible.
//!
//! ```rust
//! use gpui_toolkit::{
//!     crate_stability_manifest, dependency_hygiene_report, publish_plan,
//!     release_notes_artifact_report, release_notes_report, release_packaging_report,
//!     release_qa_matrix,
//!     vendored_patch_manifest, gpui_ui_kit, gpui_design, gpui_d3rs
//! };
//!
//! assert!(!crate_stability_manifest().is_empty());
//! assert!(!dependency_hygiene_report().all_release_ready());
//! assert!(!publish_plan().all_release_ready());
//! assert!(!release_notes_report().all_release_ready());
//! assert!(!release_notes_artifact_report().blocking_artifacts().collect::<Vec<_>>().is_empty());
//! assert!(!release_packaging_report().all_release_ready());
//! assert!(!release_qa_matrix().all_passed());
//! assert!(!vendored_patch_manifest().patches.is_empty());
//! ```

mod dependency_hygiene;
mod publish_plan;
mod release_notes;
mod release_packaging;
mod release_qa;
mod stability;
mod vendored_patches;

pub use dependency_hygiene::{
    DEPENDENCY_HYGIENE_REPORT_TYPE, DEPENDENCY_HYGIENE_SCHEMA_VERSION, DependencyAdvisoryTriage,
    DependencyAdvisoryTriageStatus, DependencyHygieneCheck, DependencyHygieneReport,
    DependencyHygieneStatus, dependency_advisory_triage, dependency_hygiene_checks,
    dependency_hygiene_report,
};
pub use publish_plan::{
    PUBLISH_PLAN_REPORT_TYPE, PUBLISH_PLAN_SCHEMA_VERSION, PublishPlan, PublishPlanEntry,
    PublishPlanStatus, publish_plan, publish_plan_entries,
};
pub use release_notes::{
    RELEASE_NOTES_ARTIFACT_REPORT_TYPE, RELEASE_NOTES_REPORT_TYPE, RELEASE_NOTES_SCHEMA_VERSION,
    ReleaseNotesArtifact, ReleaseNotesArtifactReport, ReleaseNotesArtifactStatus,
    ReleaseNotesEntry, ReleaseNotesReport, ReleaseNotesStatus, release_notes_artifact_report,
    release_notes_artifacts, release_notes_entries, release_notes_report,
};
pub use release_packaging::{
    RELEASE_PACKAGING_REPORT_TYPE, RELEASE_PACKAGING_SCHEMA_VERSION, ReleasePackagingEntry,
    ReleasePackagingReport, ReleasePackagingStatus, release_packaging_entries,
    release_packaging_report,
};
pub use release_qa::{
    RELEASE_QA_MATRIX_REPORT_TYPE, RELEASE_QA_MATRIX_SCHEMA_VERSION, ReleaseQaGate,
    ReleaseQaMatrix, ReleaseQaStatus, release_qa_gates, release_qa_matrix,
};
pub use stability::{
    AggregateFeature, CRATE_STABILITY_MANIFEST, CrateStability, PublishDecision, StabilityLevel,
    crate_stability_manifest,
};
pub use vendored_patches::{
    VENDORED_PATCH_REPORT_TYPE, VENDORED_PATCH_SCHEMA_VERSION, VendoredPatch,
    VendoredPatchManifest, VendoredPatchStatus, vendored_patch_manifest, vendored_patches,
};

#[cfg(feature = "platform")]
pub use gpui_au;
#[cfg(feature = "core")]
pub use gpui_audio_kit;
#[cfg(feature = "core")]
pub use gpui_builder;
#[cfg(feature = "tooling")]
pub use gpui_component_lab;
#[cfg(feature = "core")]
pub extern crate d3rs as gpui_d3rs;
#[cfg(feature = "core")]
pub use gpui_design;
#[cfg(feature = "tooling")]
pub use gpui_design_tools;

#[cfg(feature = "ios")]
pub use gpui_ios;

#[cfg(feature = "core")]
pub use gpui_keybinding;
#[cfg(feature = "tooling")]
pub use gpui_miniapp;
#[cfg(feature = "core")]
pub use gpui_pretext;
#[cfg(feature = "tooling")]
pub use gpui_profiler;
#[cfg(feature = "core")]
pub use gpui_px;
#[cfg(feature = "tooling")]
pub use gpui_python_runtime;
#[cfg(feature = "tooling")]
pub use gpui_scaffolder;
#[cfg(feature = "core")]
pub use gpui_themes;
#[cfg(feature = "core")]
pub use gpui_ui_kit;
#[cfg(feature = "core")]
pub use gpui_ui_kit_macros;
