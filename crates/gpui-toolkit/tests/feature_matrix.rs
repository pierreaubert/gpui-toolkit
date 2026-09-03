//! Aggregate feature-matrix checks for the `gpui-toolkit` facade.
//!
//! The facade's only job is wiring features to crates, so these tests prove
//! the wiring compiles rather than trusting `Cargo.toml` by inspection:
//!
//! - [`stability_manifest_covers_every_aggregate_feature`] runs on every
//!   build (gates are a non-optional dep) and pins the stability manifest to
//!   the [`AggregateFeature`] enum.
//! - The `*_aggregate_resolves` tests only compile when their feature is
//!   enabled; they prove each feature actually re-exports its crates. Run the
//!   matrix explicitly, e.g.:
//!
//! ```sh
//! cargo test -p gpui-toolkit --features all
//! cargo check -p gpui-toolkit --no-default-features
//! cargo check -p gpui-toolkit --no-default-features --features core
//! cargo check -p gpui-toolkit --no-default-features --features tooling
//! ```

use gpui_toolkit::{AggregateFeature, crate_stability_manifest};

#[test]
fn stability_manifest_covers_every_aggregate_feature() {
    let manifest = crate_stability_manifest();
    assert!(!manifest.is_empty());
    for feature in [
        AggregateFeature::Core,
        AggregateFeature::Ui,
        AggregateFeature::Audio,
        AggregateFeature::Charts,
        AggregateFeature::Themes,
        AggregateFeature::Tooling,
        AggregateFeature::Platform,
        AggregateFeature::Ios,
    ] {
        assert!(
            manifest
                .iter()
                .any(|entry| entry.aggregate_feature == feature),
            "stability manifest has no entry for aggregate feature `{}`",
            feature.as_str()
        );
    }
}

#[test]
fn gates_reexport_through_facade() {
    // Back-compat: pre-split `gpui_toolkit::release_qa_matrix` paths resolve.
    assert!(!gpui_toolkit::release_qa_matrix().all_passed());
    assert!(!gpui_toolkit::crate_stability_manifest().is_empty());
    assert!(!gpui_toolkit::vendored_patch_manifest().patches.is_empty());
}

#[cfg(feature = "ui")]
#[test]
fn ui_aggregate_resolves() {
    use gpui_toolkit::gpui_builder as _;
    use gpui_toolkit::gpui_design as _;
    use gpui_toolkit::gpui_keybinding as _;
    use gpui_toolkit::gpui_pretext as _;
    use gpui_toolkit::gpui_ui_kit as _;
    use gpui_toolkit::gpui_ui_kit_macros as _;
}

#[cfg(feature = "audio")]
#[test]
fn audio_aggregate_resolves() {
    use gpui_toolkit::gpui_audio_kit as _;
}

#[cfg(feature = "charts")]
#[test]
fn charts_aggregate_resolves() {
    use gpui_toolkit::gpui_d3rs as _;
    use gpui_toolkit::gpui_px as _;
}

#[cfg(feature = "themes")]
#[test]
fn themes_aggregate_resolves() {
    use gpui_toolkit::gpui_themes as _;
}

#[cfg(feature = "tooling")]
#[test]
fn tooling_aggregate_resolves() {
    use gpui_toolkit::gpui_component_lab as _;
    use gpui_toolkit::gpui_design_tools as _;
    use gpui_toolkit::gpui_miniapp as _;
    use gpui_toolkit::gpui_profiler as _;
    use gpui_toolkit::gpui_python_runtime as _;
    use gpui_toolkit::gpui_scaffolder as _;
}

#[cfg(feature = "platform")]
#[test]
fn platform_aggregate_resolves() {
    use gpui_toolkit::gpui_au as _;
    use gpui_toolkit::gpui_ios as _;
}
