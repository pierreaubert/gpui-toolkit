//! GPUI Showcase — a comprehensive demonstration of gpui-ui-kit components.
pub mod release_artifacts;
pub mod showcase;
pub use release_artifacts::{
    SHOWCASE_RELEASE_ARTIFACT_REPORT_TYPE, SHOWCASE_RELEASE_ARTIFACT_SCHEMA_VERSION,
    SHOWCASE_RELEASE_ARTIFACTS, SHOWCASE_VIEWPORTS, SHOWCASE_VISUAL_CAPTURE_REPORT_TYPE,
    SHOWCASE_VISUAL_CAPTURE_SCHEMA_VERSION, ShowcaseReleaseArtifact, ShowcaseReleaseArtifactReport,
    ShowcaseReleaseArtifactStatus, ShowcaseStoryInventory, ShowcaseVisualCapture,
    ShowcaseVisualCaptureManifest, ShowcaseVisualCaptureViewport, showcase_release_artifact_report,
    showcase_visual_capture_manifest,
};
pub use showcase::Showcase;
