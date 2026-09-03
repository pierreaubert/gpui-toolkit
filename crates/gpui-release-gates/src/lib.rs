//! # GPUI Release Gates
//!
//! Release-QA metadata for the `gpui-toolkit` workspace: per-crate stability
//! notes, the platform/release gate matrix, publish planning, release-note
//! readiness, packaging evidence, dependency hygiene, and the vendored-patch
//! manifest. This crate was split out of `gpui-toolkit` (facade) so desktop
//! builds do not pay for QA metadata they never execute, and so the gates
//! have a dependency-free home with a stable import path.
//!
//! This crate is intentionally unpublished (`publish = false`) while the
//! workspace stabilizes: the reports below assert that several gates are
//! still blocking (platform installers, publish dry-runs, device passes), so
//! publishing the gates alone would imply a release readiness that does not
//! exist. Revisit once `release_qa_matrix().all_passed()` and
//! `publish_plan().all_release_ready()` hold; the `gpui_wgpu` git dependency
//! in platform crates must also be resolved before any publish.
//!
//! `crate_stability_manifest()` exposes the current per-crate stability notes
//! used by release QA and should be updated before changing publish scope.
//! `release_qa_matrix()` exposes the platform/release gate matrix used to keep
//! compile checks, manual device passes, publish dry-runs, and dependency
//! hygiene visible before an external release.
//! `platform_capability_matrix()` separately records declared platform
//! capabilities and executed evidence, preventing compile support or shared
//! component tests from being mistaken for runtime, visual, accessibility, or
//! performance qualification.
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
//! use gpui_release_gates::{
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
    PLATFORM_CAPABILITY_MATRIX_REPORT_TYPE, PLATFORM_CAPABILITY_MATRIX_SCHEMA_VERSION,
    PlatformCapability, PlatformCapabilityMatrix, PlatformCapabilityStatus, PlatformEvidence,
    RELEASE_QA_MATRIX_REPORT_TYPE, RELEASE_QA_MATRIX_SCHEMA_VERSION, ReleaseQaGate,
    ReleaseQaMatrix, ReleaseQaStatus, platform_capabilities, platform_capability_matrix,
    release_qa_gates, release_qa_matrix,
};
pub use stability::{
    AggregateFeature, CRATE_STABILITY_MANIFEST, CrateStability, PublishDecision, StabilityLevel,
    crate_stability_manifest,
};
pub use vendored_patches::{
    VENDORED_PATCH_REPORT_TYPE, VENDORED_PATCH_SCHEMA_VERSION, VendoredPatch,
    VendoredPatchMaintenance, VendoredPatchManifest, VendoredPatchStatus, vendored_patch_manifest,
    vendored_patches,
};
