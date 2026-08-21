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

/// Boot the showcase as a full GPUI app. Shared by the native `main` and the
/// wasm `start` entry point.
pub fn run_showcase() {
    use gpui::AppContext as _;
    let config = gpui_miniapp::MiniAppConfig::new("UI Kit Showcase")
        .size(1200.0, 900.0)
        .scrollable(true)
        .with_theme(true)
        .with_i18n(true);
    #[cfg(target_family = "wasm")]
    let config = config.initial_theme(gpui_miniapp::web_initial_theme());
    gpui_miniapp::MiniApp::run(config, |cx| cx.new(Showcase::new));
}
