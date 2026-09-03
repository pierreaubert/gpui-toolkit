//! # GPUI Toolkit
//!
//! Convenience aggregate crate that re-exports all libraries in the
//! `gpui-toolkit` workspace so that a single dependency gives access to the
//! whole toolkit.
//!
//! This crate is intentionally unpublished while the workspace stabilizes
//! (`publish = false`; see `gpui-release-gates` docs for the publish story).
//! Its feature sets define the public boundary used by release QA:
//!
//! - `ui` (default): UI components, design, layout, text, and keybindings.
//! - `audio`, `charts`, and `themes`: opt-in product surfaces. Charts retain
//!   the WGPU-backed defaults of `gpui-d3rs` and `gpui-px`.
//! - `core`: compatibility aggregate containing `ui`, `audio`, `charts`, and
//!   `themes`.
//! - `tooling`: support crates for labs, design tools, scaffolding, profiling,
//!   mini-app shells, and Python scene/runtime work.
//! - `platform`: AU and Apple mobile platform integration crates that require
//!   target-specific QA before public release.
//! - `ios`: iOS integration as a target-specific opt-in.
//! - `all`: every aggregate feature.
//!
//! Release-QA metadata (`crate_stability_manifest`, `release_qa_matrix`, …)
//! lives in [`gpui_release_gates`](https://github.com/pierreaubert/gpui-toolkit)
//! and is re-exported here unchanged so existing `gpui_toolkit::…` paths keep
//! resolving. New code should import gates directly from `gpui-release-gates`
//! to avoid pulling the aggregate feature surface.
//!
//! ```rust
//! use gpui_toolkit::{
//!     crate_stability_manifest, dependency_hygiene_report, publish_plan,
//!     release_notes_artifact_report, release_notes_report, release_packaging_report,
//!     release_qa_matrix,
//!     platform_capability_matrix,
//!     vendored_patch_manifest
//! };
//!
//! assert!(!crate_stability_manifest().is_empty());
//! assert!(dependency_hygiene_report().all_release_ready());
//! assert!(publish_plan().all_release_ready());
//! assert!(!release_notes_report().all_release_ready());
//! assert!(!release_notes_artifact_report().blocking_artifacts().collect::<Vec<_>>().is_empty());
//! assert!(!release_packaging_report().all_release_ready());
//! assert!(!release_qa_matrix().all_passed());
//! assert!(!platform_capability_matrix().all_release_ready());
//! assert!(!vendored_patch_manifest().patches.is_empty());
//! ```

// Release gates live in their own dependency-free crate; re-export the full
// surface here so `gpui_toolkit::release_qa_matrix` and friends keep working.
pub use gpui_release_gates::{
    DEPENDENCY_HYGIENE_REPORT_TYPE, DEPENDENCY_HYGIENE_SCHEMA_VERSION, DependencyAdvisoryTriage,
    DependencyAdvisoryTriageStatus, DependencyHygieneCheck, DependencyHygieneReport,
    DependencyHygieneStatus, dependency_advisory_triage, dependency_hygiene_checks,
    dependency_hygiene_report,
};
pub use gpui_release_gates::{
    PUBLISH_PLAN_REPORT_TYPE, PUBLISH_PLAN_SCHEMA_VERSION, PublishPlan, PublishPlanEntry,
    PublishPlanStatus, publish_plan, publish_plan_entries,
};
pub use gpui_release_gates::{
    RELEASE_NOTES_ARTIFACT_REPORT_TYPE, RELEASE_NOTES_REPORT_TYPE, RELEASE_NOTES_SCHEMA_VERSION,
    ReleaseNotesArtifact, ReleaseNotesArtifactReport, ReleaseNotesArtifactStatus,
    ReleaseNotesEntry, ReleaseNotesReport, ReleaseNotesStatus, release_notes_artifact_report,
    release_notes_artifacts, release_notes_entries, release_notes_report,
};
pub use gpui_release_gates::{
    RELEASE_PACKAGING_REPORT_TYPE, RELEASE_PACKAGING_SCHEMA_VERSION, ReleasePackagingEntry,
    ReleasePackagingReport, ReleasePackagingStatus, release_packaging_entries,
    release_packaging_report,
};
pub use gpui_release_gates::{
    PLATFORM_CAPABILITY_MATRIX_REPORT_TYPE, PLATFORM_CAPABILITY_MATRIX_SCHEMA_VERSION,
    PlatformCapability, PlatformCapabilityMatrix, PlatformCapabilityStatus, PlatformEvidence,
    RELEASE_QA_MATRIX_REPORT_TYPE, RELEASE_QA_MATRIX_SCHEMA_VERSION, ReleaseQaGate,
    ReleaseQaMatrix, ReleaseQaStatus, platform_capabilities, platform_capability_matrix,
    release_qa_gates, release_qa_matrix,
};
pub use gpui_release_gates::{
    AggregateFeature, CRATE_STABILITY_MANIFEST, CrateStability, PublishDecision, StabilityLevel,
    crate_stability_manifest,
};
pub use gpui_release_gates::{
    VENDORED_PATCH_REPORT_TYPE, VENDORED_PATCH_SCHEMA_VERSION, VendoredPatch,
    VendoredPatchMaintenance, VendoredPatchManifest, VendoredPatchStatus, vendored_patch_manifest,
    vendored_patches,
};

#[cfg(feature = "platform")]
pub use gpui_au;
#[cfg(feature = "audio")]
pub use gpui_audio_kit;
#[cfg(feature = "ui")]
pub use gpui_builder;
#[cfg(feature = "tooling")]
pub use gpui_component_lab;
#[cfg(feature = "charts")]
pub extern crate d3rs as gpui_d3rs;
#[cfg(feature = "ui")]
pub use gpui_design;
#[cfg(feature = "tooling")]
pub use gpui_design_tools;

#[cfg(feature = "ios")]
pub use gpui_ios;

#[cfg(feature = "ui")]
pub use gpui_keybinding;
#[cfg(feature = "tooling")]
pub use gpui_miniapp;
#[cfg(feature = "ui")]
pub use gpui_pretext;
#[cfg(feature = "tooling")]
pub use gpui_profiler;
#[cfg(feature = "charts")]
pub use gpui_px;
#[cfg(feature = "tooling")]
pub use gpui_python_runtime;
#[cfg(feature = "tooling")]
pub use gpui_scaffolder;
#[cfg(feature = "themes")]
pub use gpui_themes;
#[cfg(feature = "ui")]
pub use gpui_ui_kit;
#[cfg(feature = "ui")]
pub use gpui_ui_kit_macros;
