//! Release stability metadata for the aggregate toolkit surface.

/// Feature group that makes a crate visible through `gpui-toolkit`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateFeature {
    /// Compatibility aggregate containing all product surfaces.
    Core,
    /// Default UI, design, layout, text, and keybinding surface.
    Ui,
    /// Audio-oriented controls layered on the UI surface.
    Audio,
    /// WGPU-accelerated D3 and Plotly Express-style charts.
    Charts,
    /// Theme management layered on the UI surface.
    Themes,
    /// Support, lab, scaffolding, profiling, or runtime tooling.
    Tooling,
    /// Target-specific platform integration.
    Platform,
    /// Apple mobile platform integration reached through `platform`.
    Ios,
}

impl AggregateFeature {
    /// Cargo feature name used by `gpui-toolkit`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Ui => "ui",
            Self::Audio => "audio",
            Self::Charts => "charts",
            Self::Themes => "themes",
            Self::Tooling => "tooling",
            Self::Platform => "platform",
            Self::Ios => "ios",
        }
    }
}

/// Current release-readiness level for a crate in the aggregate surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityLevel {
    /// Suitable for public release after normal crate gates pass.
    ReleaseCandidate,
    /// Usable, but should be advertised with beta limitations.
    Beta,
    /// Useful as release support tooling, but not promised as stable runtime API.
    SupportTooling,
    /// Experimental or target-specific and held behind extra QA.
    Experimental,
    /// Aggregate/internal surface that should not be published yet.
    InternalOnly,
}

impl StabilityLevel {
    /// Stable human-readable label for reports and release notes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReleaseCandidate => "release-candidate",
            Self::Beta => "beta",
            Self::SupportTooling => "support-tooling",
            Self::Experimental => "experimental",
            Self::InternalOnly => "internal-only",
        }
    }
}

/// Release decision associated with a crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishDecision {
    /// Publish as part of the public core once gates pass.
    PublicCoreAfterGates,
    /// Publish as beta only if release notes call out limitations.
    BetaAfterGates,
    /// Postpone crates.io publication until named registry predecessors exist.
    DeferredRegistry,
    /// Keep as support tooling unless the release explicitly promises stability.
    SupportToolingOnly,
    /// Hold from public release pending target-specific QA.
    HoldForPlatformQa,
    /// Do not publish this aggregate/internal crate yet.
    DoNotPublish,
}

impl PublishDecision {
    /// Stable human-readable label for reports and release notes.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PublicCoreAfterGates => "public-core-after-gates",
            Self::BetaAfterGates => "source-beta-after-gates",
            Self::DeferredRegistry => "deferred-registry",
            Self::SupportToolingOnly => "support-tooling-only",
            Self::HoldForPlatformQa => "hold-for-platform-qa",
            Self::DoNotPublish => "do-not-publish",
        }
    }
}

/// Per-crate stability note used by release QA and app authors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrateStability {
    /// Cargo package/crate name.
    pub crate_name: &'static str,
    /// Aggregate feature that exposes the crate.
    pub aggregate_feature: AggregateFeature,
    /// Current API/readiness level.
    pub stability: StabilityLevel,
    /// Release decision for this toolkit release.
    pub publish_decision: PublishDecision,
    /// Required gate before the decision can move forward.
    pub required_gate: &'static str,
    /// Short limitation or release-note summary.
    pub note: &'static str,
}

/// Per-crate stability manifest for the aggregate crate's dependencies.
pub const CRATE_STABILITY_MANIFEST: &[CrateStability] = &[
    CrateStability {
        crate_name: "gpui-audio-kit",
        aggregate_feature: AggregateFeature::Audio,
        stability: StabilityLevel::Beta,
        publish_decision: PublishDecision::BetaAfterGates,
        required_gate: "source RC bundle plus focused audio-control, accessibility, and snapshot evidence",
        note: "Source beta only: the crate depends on the unpublished GPUI runtime; native accessibility bridge consumption remains app-level work.",
    },
    CrateStability {
        crate_name: "gpui-builder",
        aggregate_feature: AggregateFeature::Ui,
        stability: StabilityLevel::ReleaseCandidate,
        publish_decision: PublishDecision::DeferredRegistry,
        required_gate: "publish compatible gpui-design and gpui-pretext predecessors, then pass a locked dry-run",
        note: "Registry publication is deferred; the tagged source includes layout diagnostics and benchmark evidence.",
    },
    CrateStability {
        crate_name: "gpui-d3rs",
        aggregate_feature: AggregateFeature::Charts,
        stability: StabilityLevel::Beta,
        publish_decision: PublishDecision::BetaAfterGates,
        required_gate: "source RC bundle, large-data sanity checks, NaN/empty-input review, and rendered examples",
        note: "Visualization algorithms are broad; checked chord and quadtree paths exist, with more fallible APIs still to add.",
    },
    CrateStability {
        crate_name: "gpui-design",
        aggregate_feature: AggregateFeature::Ui,
        stability: StabilityLevel::ReleaseCandidate,
        publish_decision: PublishDecision::PublicCoreAfterGates,
        required_gate: "cargo publish --dry-run plus generated design documentation/conformance artifact",
        note: "Design tokens and conformance reporting are stable enough for the public core.",
    },
    CrateStability {
        crate_name: "gpui-keybinding",
        aggregate_feature: AggregateFeature::Ui,
        stability: StabilityLevel::Beta,
        publish_decision: PublishDecision::BetaAfterGates,
        required_gate: "source RC bundle plus shortcut conflict examples and platform policy docs",
        note: "Source beta only because runtime integration depends on unpublished GPUI; conflict-resolution and platform shortcut policy are documented.",
    },
    CrateStability {
        crate_name: "gpui-pretext",
        aggregate_feature: AggregateFeature::Ui,
        stability: StabilityLevel::ReleaseCandidate,
        publish_decision: PublishDecision::DeferredRegistry,
        required_gate: "publish gpui-profiler, then pass a clean locked cargo publish dry-run",
        note: "GPUI-free and MSRV-tested, but its package verifier requires the unpublished gpui-profiler dev-dependency from crates.io.",
    },
    CrateStability {
        crate_name: "gpui-px",
        aggregate_feature: AggregateFeature::Charts,
        stability: StabilityLevel::Beta,
        publish_decision: PublishDecision::BetaAfterGates,
        required_gate: "source RC bundle, large-data sanity checks, visual examples, and chart accessibility notes",
        note: "High-level charting has accessibility summaries; richer interactions and export remain beta limitations.",
    },
    CrateStability {
        crate_name: "gpui-themes",
        aggregate_feature: AggregateFeature::Themes,
        stability: StabilityLevel::Beta,
        publish_decision: PublishDecision::BetaAfterGates,
        required_gate: "source RC bundle plus theme schema/version compatibility tests",
        note: "Source beta only because GPUI and UI-kit dependencies are unpublished; community theme schema policy is documented and tested.",
    },
    CrateStability {
        crate_name: "gpui-ui-kit",
        aggregate_feature: AggregateFeature::Ui,
        stability: StabilityLevel::Beta,
        publish_decision: PublishDecision::BetaAfterGates,
        required_gate: "source RC bundle plus keyboard, accessibility, and renderer-backed snapshot evidence",
        note: "Broad source-beta component surface that depends on unpublished GPUI; compound-widget accessibility limitations remain explicit.",
    },
    CrateStability {
        crate_name: "gpui-ui-kit-macros",
        aggregate_feature: AggregateFeature::Ui,
        stability: StabilityLevel::ReleaseCandidate,
        publish_decision: PublishDecision::PublicCoreAfterGates,
        required_gate: "cargo publish --dry-run plus proc-macro diagnostic smoke tests",
        note: "Compile-time helper crate for the UI kit.",
    },
    CrateStability {
        crate_name: "gpui-component-lab",
        aggregate_feature: AggregateFeature::Tooling,
        stability: StabilityLevel::SupportTooling,
        publish_decision: PublishDecision::SupportToolingOnly,
        required_gate: "CLI docs, visual-manifest contract, and dry-run only if intentionally included",
        note: "Useful release/design-review tooling; public API stability is not promised by default.",
    },
    CrateStability {
        crate_name: "gpui-design-tools",
        aggregate_feature: AggregateFeature::Tooling,
        stability: StabilityLevel::SupportTooling,
        publish_decision: PublishDecision::SupportToolingOnly,
        required_gate: "CLI docs, schema/version notes, and dry-run only if intentionally included",
        note: "Design-token validation report contract is stable, but this is still support tooling.",
    },
    CrateStability {
        crate_name: "gpui-miniapp",
        aggregate_feature: AggregateFeature::Tooling,
        stability: StabilityLevel::SupportTooling,
        publish_decision: PublishDecision::DoNotPublish,
        required_gate: "platform backend distribution story and explicit decision to publish",
        note: "Mini app shell is useful for examples, but currently publish-disabled.",
    },
    CrateStability {
        crate_name: "gpui-profiler",
        aggregate_feature: AggregateFeature::Tooling,
        stability: StabilityLevel::ReleaseCandidate,
        publish_decision: PublishDecision::PublicCoreAfterGates,
        required_gate: "locked dry-run, Rust 1.89 MSRV, allocator-conflict documentation, and feature/no-feature tests",
        note: "Small GPUI-free profiling package selected for crates.io wave 1; enabling its global allocator remains opt-in.",
    },
    CrateStability {
        crate_name: "gpui-python-runtime",
        aggregate_feature: AggregateFeature::Tooling,
        stability: StabilityLevel::SupportTooling,
        publish_decision: PublishDecision::SupportToolingOnly,
        required_gate: "schema/version notes and Python packaging decision",
        note: "Versioned Python app IR exists; packaging and large-scene behavior remain release considerations.",
    },
    CrateStability {
        crate_name: "gpui-scaffolder",
        aggregate_feature: AggregateFeature::Tooling,
        stability: StabilityLevel::SupportTooling,
        publish_decision: PublishDecision::DoNotPublish,
        required_gate: "Android/tvOS generated-project checks plus native Xcode/device gates",
        note: "Desktop and iOS simulator generated-project checks exist; mobile breadth is still incomplete.",
    },
    CrateStability {
        crate_name: "gpui-au",
        aggregate_feature: AggregateFeature::Platform,
        stability: StabilityLevel::Experimental,
        publish_decision: PublishDecision::HoldForPlatformQa,
        required_gate: "AUv3 host validation, text/window lifetime audit, and platform smoke test",
        note: "Audio Unit embedding is host-dependent and target-specific.",
    },
    CrateStability {
        crate_name: "gpui-ios",
        aggregate_feature: AggregateFeature::Ios,
        stability: StabilityLevel::Experimental,
        publish_decision: PublishDecision::HoldForPlatformQa,
        required_gate: "iOS simulator/device build plus VoiceOver, touch, keyboard, and rotation QA",
        note: "Mobile backend has native bridge risk and requires device validation.",
    },
    CrateStability {
        crate_name: "gpui-toolkit",
        aggregate_feature: AggregateFeature::Core,
        stability: StabilityLevel::InternalOnly,
        publish_decision: PublishDecision::DoNotPublish,
        required_gate: "constituent-crate dry-runs, stability notes, and explicit aggregate publish decision",
        note: "Aggregate crate remains publish-disabled while constituent crates stabilize.",
    },
];

/// Return the aggregate crate stability manifest.
pub const fn crate_stability_manifest() -> &'static [CrateStability] {
    CRATE_STABILITY_MANIFEST
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_has_stable_labels_for_every_entry() {
        for entry in crate_stability_manifest() {
            assert!(!entry.crate_name.is_empty());
            assert!(!entry.aggregate_feature.as_str().is_empty());
            assert!(!entry.stability.as_str().is_empty());
            assert!(!entry.publish_decision.as_str().is_empty());
            assert!(!entry.required_gate.is_empty());
            assert!(!entry.note.is_empty());
        }
    }

    #[test]
    fn manifest_has_no_duplicate_crate_names() {
        let manifest = crate_stability_manifest();
        for (index, entry) in manifest.iter().enumerate() {
            assert!(
                !manifest[..index]
                    .iter()
                    .any(|previous| previous.crate_name == entry.crate_name),
                "duplicate stability entry for {}",
                entry.crate_name
            );
        }
    }

    #[test]
    fn public_registry_decisions_match_wave_one() {
        let public_core: Vec<_> = crate_stability_manifest()
            .iter()
            .filter(|entry| entry.publish_decision == PublishDecision::PublicCoreAfterGates)
            .map(|entry| entry.crate_name)
            .collect();

        assert_eq!(
            public_core,
            ["gpui-design", "gpui-ui-kit-macros", "gpui-profiler"]
        );
    }

    #[test]
    fn aggregate_entry_remains_internal_only() {
        let aggregate = crate_stability_manifest()
            .iter()
            .find(|entry| entry.crate_name == "gpui-toolkit")
            .expect("gpui-toolkit stability entry");

        assert_eq!(aggregate.stability, StabilityLevel::InternalOnly);
        assert_eq!(aggregate.publish_decision, PublishDecision::DoNotPublish);
    }
}
