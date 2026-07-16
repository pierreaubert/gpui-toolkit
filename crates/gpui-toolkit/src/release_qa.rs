//! Release QA matrix metadata for the aggregate toolkit.

/// Schema version for [`ReleaseQaMatrix`].
pub const RELEASE_QA_MATRIX_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`ReleaseQaMatrix`].
pub const RELEASE_QA_MATRIX_REPORT_TYPE: &str = "gpui-toolkit-release-qa-matrix";

/// Schema version for [`PlatformCapabilityMatrix`].
pub const PLATFORM_CAPABILITY_MATRIX_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`PlatformCapabilityMatrix`].
pub const PLATFORM_CAPABILITY_MATRIX_REPORT_TYPE: &str = "gpui-toolkit-platform-capability-matrix";

/// Whether a platform capability is implemented and supported by the toolkit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformCapabilityStatus {
    /// The capability is part of the supported platform contract.
    Supported,
    /// Some of the capability exists, but the public contract is incomplete.
    Partial,
    /// The capability is intentionally outside the platform contract.
    NotApplicable,
    /// Support has not been established and must not be inferred.
    Unverified,
}

impl PlatformCapabilityStatus {
    /// Stable label for machine-generated and Markdown reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Partial => "partial",
            Self::NotApplicable => "not-applicable",
            Self::Unverified => "unverified",
        }
    }
}

/// Executed evidence available for one platform contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformEvidence {
    /// Whether the platform is compiled by maintained CI.
    pub ci_compile: bool,
    /// Whether a runtime smoke test is recorded by maintained CI.
    pub runtime_smoke: bool,
    /// Whether renderer screenshots are captured and compared on-platform.
    pub visual_diff: bool,
    /// Whether native accessibility actions/tree behavior are exercised.
    pub native_accessibility: bool,
    /// Whether steady-state allocation or frame-time evidence is recorded.
    pub performance: bool,
}

/// One platform's declared capabilities and executed QA evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCapability {
    /// Stable platform identifier.
    pub id: &'static str,
    /// User-facing platform name.
    pub platform: &'static str,
    /// Platform maturity tier from `qa.md`.
    pub tier: &'static str,
    /// Pointer or remote input support.
    pub pointer: PlatformCapabilityStatus,
    /// Touch input support.
    pub touch: PlatformCapabilityStatus,
    /// Hardware keyboard and text/IME support.
    pub text_input: PlatformCapabilityStatus,
    /// Native accessibility bridge support.
    pub accessibility: PlatformCapabilityStatus,
    /// Executed evidence; capability declarations never imply these fields.
    pub evidence: PlatformEvidence,
    /// Honest blocker that prevents a release-ready claim, or `None`.
    pub blocker: Option<&'static str>,
}

/// Versioned platform capability and executed-evidence matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCapabilityMatrix {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub platforms: &'static [PlatformCapability],
}

impl PlatformCapabilityMatrix {
    /// True only when every applicable platform has all release evidence.
    pub fn all_release_ready(self) -> bool {
        self.platforms.iter().all(|platform| {
            let evidence = platform.evidence;
            platform.blocker.is_none()
                && evidence.ci_compile
                && evidence.runtime_smoke
                && evidence.visual_diff
                && evidence.native_accessibility
                && evidence.performance
        })
    }

    /// Render capability declarations and evidence without conflating them.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI Toolkit Platform Capability Matrix\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Platform | Tier | Pointer | Touch | Text/IME | Accessibility | CI compile | Runtime | Visual | A11y evidence | Perf | Blocker |\n\
             | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );
        for item in self.platforms {
            let yes_no = |value| if value { "yes" } else { "no" };
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
                item.platform,
                item.tier,
                item.pointer.as_str(),
                item.touch.as_str(),
                item.text_input.as_str(),
                item.accessibility.as_str(),
                yes_no(item.evidence.ci_compile),
                yes_no(item.evidence.runtime_smoke),
                yes_no(item.evidence.visual_diff),
                yes_no(item.evidence.native_accessibility),
                yes_no(item.evidence.performance),
                item.blocker.unwrap_or("none")
            ));
        }
        markdown
    }
}

/// Current status for a release QA gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseQaStatus {
    /// The named command/gate passed in the current release report.
    Passed,
    /// Some host-side validation passed, but a target/runtime gate remains.
    Partial,
    /// The gate could not run in the current environment.
    Blocked,
    /// The gate requires a manual platform/device/host pass.
    ManualRequired,
    /// The gate is still pending and should run before external release.
    Pending,
}

impl ReleaseQaStatus {
    /// Stable label for release notes and generated reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Partial => "partial",
            Self::Blocked => "blocked",
            Self::ManualRequired => "manual-required",
            Self::Pending => "pending",
        }
    }
}

/// One platform or release gate in the QA matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseQaGate {
    /// Stable gate id.
    pub id: &'static str,
    /// Human-readable platform or release area.
    pub area: &'static str,
    /// Command, recipe, or manual action that proves the gate.
    pub command: &'static str,
    /// Current status from the release report.
    pub status: ReleaseQaStatus,
    /// Evidence recorded in the release report.
    pub evidence: &'static str,
    /// What must be true before an external release can claim this area.
    pub release_requirement: &'static str,
}

/// Versioned release QA matrix for documentation and CI artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseQaMatrix {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub gates: &'static [ReleaseQaGate],
}

impl ReleaseQaMatrix {
    /// Return true only when every release gate is fully passed.
    pub fn all_passed(self) -> bool {
        self.gates
            .iter()
            .all(|gate| gate.status == ReleaseQaStatus::Passed)
    }

    /// Render the matrix as Markdown for release notes.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI Toolkit Release QA Matrix\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Gate | Area | Status | Command or action | Evidence | Release requirement |\n\
             | --- | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for gate in self.gates {
            markdown.push_str(&format!(
                "| {} | {} | {} | `{}` | {} | {} |\n",
                gate.id,
                gate.area,
                gate.status.as_str(),
                gate.command,
                gate.evidence,
                gate.release_requirement
            ));
        }

        markdown
    }
}

const RELEASE_QA_GATES: &[ReleaseQaGate] = &[
    ReleaseQaGate {
        id: "workspace-all-targets",
        area: "Workspace compile",
        command: "cargo check --workspace --all-targets",
        status: ReleaseQaStatus::Passed,
        evidence: "Passed after the gpui-au test API drift fix and aggregate feature-boundary work.",
        release_requirement: "Keep this green immediately before tagging or publishing.",
    },
    ReleaseQaGate {
        id: "macos-desktop",
        area: "macOS desktop/runtime",
        command: "layout-showcase --smoke-test --smoke-artifact <path>",
        status: ReleaseQaStatus::Partial,
        evidence: "Maintained native CI opens a macOS window, applies a sidebar state transition, and requires a second render; shared GPUI pointer contracts cover selection, collapse, and drag.",
        release_requirement: "Add keyboard/resize automation and renderer pixel capture/diff.",
    },
    ReleaseQaGate {
        id: "au-host",
        area: "Audio Unit host embedding",
        command: "AUv3 host validation with gpui-au",
        status: ReleaseQaStatus::ManualRequired,
        evidence: "gpui-au test targets now compile, but AUv3 host runtime validation is not recorded.",
        release_requirement: "Run at least one AUv3 host smoke test and text/window lifetime check.",
    },
    ReleaseQaGate {
        id: "ios-simulator",
        area: "iOS simulator",
        command: "cargo check --lib --target aarch64-apple-ios-sim for a generated scaffold",
        status: ReleaseQaStatus::Partial,
        evidence: "Generated iOS simulator Rust project compile-check passed; app simulator launch/device behavior was not recorded.",
        release_requirement: "Add simulator launch plus touch, safe-area, rotation, keyboard, and VoiceOver smoke results.",
    },
    ReleaseQaGate {
        id: "android-target",
        area: "Android target",
        command: "cargo check -p gpui-android --target aarch64-linux-android --lib",
        status: ReleaseQaStatus::Partial,
        evidence: "Android NDK aarch64-linux-android35-clang is installed and the target compile passes; emulator or device runtime smoke test is not yet recorded.",
        release_requirement: "Record an Android emulator or device launch, touch, keyboard, and visual smoke pass.",
    },
    ReleaseQaGate {
        id: "tvos-simulator-device",
        area: "tvOS simulator/device",
        command: "just tvos-* recipes and Xcode simulator/device run",
        status: ReleaseQaStatus::Partial,
        evidence: "tvOS README recipes are documented and cargo check -p gpui-showcase-tvos --lib passed.",
        release_requirement: "Record simulator/device run, signing status, and focus/remote UX validation.",
    },
    ReleaseQaGate {
        id: "windows-native",
        area: "Windows native",
        command: "layout-showcase.exe --smoke-test --smoke-artifact <path>",
        status: ReleaseQaStatus::Partial,
        evidence: "Maintained native CI opens a Windows window, applies a sidebar state transition, and requires a second render; shared GPUI pointer contracts cover selection, collapse, and drag.",
        release_requirement: "Add input/IME/accessibility automation and renderer pixel capture/diff.",
    },
    ReleaseQaGate {
        id: "showcase-visual",
        area: "Showcase visual QA",
        command: "component-lab visual manifest capture and pixel diff",
        status: ReleaseQaStatus::Pending,
        evidence: "Linux CI captures and validates builder showcase pixels; component-lab and cross-renderer screenshot/diff coverage remains pending.",
        release_requirement: "Attach generated screenshots/diffs or a visual-regression report for release artifacts.",
    },
    ReleaseQaGate {
        id: "publish-dry-runs",
        area: "Crate publishing",
        command: "gpui_toolkit::publish_plan() and cargo publish --dry-run per selected public crate",
        status: ReleaseQaStatus::Blocked,
        evidence: "gpui-design, gpui-pretext, and gpui-ui-kit-macros dry-runs passed. All other selected public-core crates require the Zed GPUI crate, which is not published on crates.io, so their dry-runs cannot resolve.",
        release_requirement: "Decide how to obtain a published GPUI dependency (wait for upstream, vendor/publish a fork, or shrink the public release set to GPUI-free crates) and rerun dry-runs.",
    },
    ReleaseQaGate {
        id: "release-notes",
        area: "Release notes/changelog",
        command: "gpui_toolkit::release_notes_report()",
        status: ReleaseQaStatus::Pending,
        evidence: "Stable release-note readiness report exists with crate-level stability, platform-support, limitation, and artifact requirements.",
        release_requirement: "Resolve or explicitly accept release-note blockers before tagging or publishing.",
    },
    ReleaseQaGate {
        id: "dependency-hygiene",
        area: "Security/dependency hygiene",
        command: "gpui_toolkit::dependency_hygiene_report(), cargo audit, and cargo deny",
        status: ReleaseQaStatus::Passed,
        evidence: "dependency_hygiene_report() records accepted cargo-audit and cargo-deny runs; the 2026-07-12 cargo-deny run passed advisories, bans, licenses, and sources with the documented warning-class exceptions.",
        release_requirement: "Attach the accepted outputs, review every warning and advisory exception, and remove exceptions when upstream dependencies permit.",
    },
];

const PLATFORM_CAPABILITIES: &[PlatformCapability] = &[
    PlatformCapability {
        id: "linux-desktop",
        platform: "Linux desktop",
        tier: "A",
        pointer: PlatformCapabilityStatus::Supported,
        touch: PlatformCapabilityStatus::Partial,
        text_input: PlatformCapabilityStatus::Supported,
        accessibility: PlatformCapabilityStatus::Partial,
        evidence: PlatformEvidence {
            ci_compile: true,
            runtime_smoke: true,
            visual_diff: false,
            native_accessibility: false,
            performance: false,
        },
        blocker: Some(
            "Renderer screenshot, native accessibility, and host-qualified performance evidence remain pending.",
        ),
    },
    PlatformCapability {
        id: "macos-desktop",
        platform: "macOS desktop",
        tier: "A",
        pointer: PlatformCapabilityStatus::Supported,
        touch: PlatformCapabilityStatus::NotApplicable,
        text_input: PlatformCapabilityStatus::Supported,
        accessibility: PlatformCapabilityStatus::Partial,
        evidence: PlatformEvidence {
            ci_compile: true,
            runtime_smoke: true,
            visual_diff: false,
            native_accessibility: false,
            performance: true,
        },
        blocker: Some("Renderer screenshot and native accessibility CI evidence remain pending."),
    },
    PlatformCapability {
        id: "windows-desktop",
        platform: "Windows desktop",
        tier: "A",
        pointer: PlatformCapabilityStatus::Supported,
        touch: PlatformCapabilityStatus::Partial,
        text_input: PlatformCapabilityStatus::Partial,
        accessibility: PlatformCapabilityStatus::Partial,
        evidence: PlatformEvidence {
            ci_compile: true,
            runtime_smoke: true,
            visual_diff: false,
            native_accessibility: false,
            performance: false,
        },
        blocker: Some(
            "IME, accessibility, renderer screenshot, and performance evidence remain pending.",
        ),
    },
    PlatformCapability {
        id: "ios",
        platform: "iOS",
        tier: "B",
        pointer: PlatformCapabilityStatus::Partial,
        touch: PlatformCapabilityStatus::Supported,
        text_input: PlatformCapabilityStatus::Partial,
        accessibility: PlatformCapabilityStatus::Partial,
        evidence: PlatformEvidence {
            ci_compile: false,
            runtime_smoke: false,
            visual_diff: false,
            native_accessibility: false,
            performance: false,
        },
        blocker: Some("Simulator/device compile, launch, VoiceOver, visual, and performance jobs remain external."),
    },
    PlatformCapability {
        id: "android",
        platform: "Android",
        tier: "B",
        pointer: PlatformCapabilityStatus::Partial,
        touch: PlatformCapabilityStatus::Supported,
        text_input: PlatformCapabilityStatus::Partial,
        accessibility: PlatformCapabilityStatus::Unverified,
        evidence: PlatformEvidence {
            ci_compile: false,
            runtime_smoke: false,
            visual_diff: false,
            native_accessibility: false,
            performance: false,
        },
        blocker: Some("Maintained target CI, emulator lifecycle/IME/TalkBack, visual, and performance evidence are absent."),
    },
    PlatformCapability {
        id: "tvos",
        platform: "tvOS",
        tier: "B",
        pointer: PlatformCapabilityStatus::Partial,
        touch: PlatformCapabilityStatus::NotApplicable,
        text_input: PlatformCapabilityStatus::Partial,
        accessibility: PlatformCapabilityStatus::Unverified,
        evidence: PlatformEvidence {
            ci_compile: false,
            runtime_smoke: false,
            visual_diff: false,
            native_accessibility: false,
            performance: false,
        },
        blocker: Some("Nightly simulator/device build, remote focus, accessibility, visual, and performance evidence remain manual."),
    },
    PlatformCapability {
        id: "auv3-host",
        platform: "macOS AUv3 host",
        tier: "B",
        pointer: PlatformCapabilityStatus::Supported,
        touch: PlatformCapabilityStatus::NotApplicable,
        text_input: PlatformCapabilityStatus::Partial,
        accessibility: PlatformCapabilityStatus::Partial,
        evidence: PlatformEvidence {
            ci_compile: true,
            runtime_smoke: false,
            visual_diff: false,
            native_accessibility: false,
            performance: false,
        },
        blocker: Some("No recorded DAW host attach/detach/resize, accessibility, visual, or frame-allocation matrix."),
    },
];

/// Return the versioned platform capability and evidence matrix.
pub const fn platform_capability_matrix() -> PlatformCapabilityMatrix {
    PlatformCapabilityMatrix {
        schema_version: PLATFORM_CAPABILITY_MATRIX_SCHEMA_VERSION,
        report_type: PLATFORM_CAPABILITY_MATRIX_REPORT_TYPE,
        reviewed_on: "2026-07-12",
        platforms: PLATFORM_CAPABILITIES,
    }
}

/// Return platform capability rows without allocating.
pub const fn platform_capabilities() -> &'static [PlatformCapability] {
    PLATFORM_CAPABILITIES
}

/// Return the current release QA matrix.
pub const fn release_qa_matrix() -> ReleaseQaMatrix {
    ReleaseQaMatrix {
        schema_version: RELEASE_QA_MATRIX_SCHEMA_VERSION,
        report_type: RELEASE_QA_MATRIX_REPORT_TYPE,
        reviewed_on: "2026-07-12",
        gates: RELEASE_QA_GATES,
    }
}

/// Return the release QA gates without allocating.
pub const fn release_qa_gates() -> &'static [ReleaseQaGate] {
    RELEASE_QA_GATES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_qa_matrix_has_stable_contract() {
        let matrix = release_qa_matrix();

        assert_eq!(matrix.schema_version, RELEASE_QA_MATRIX_SCHEMA_VERSION);
        assert_eq!(matrix.report_type, RELEASE_QA_MATRIX_REPORT_TYPE);
        assert_eq!(matrix.reviewed_on, "2026-07-12");
        assert!(!matrix.gates.is_empty());

        for gate in matrix.gates {
            assert!(!gate.id.is_empty());
            assert!(!gate.area.is_empty());
            assert!(!gate.command.is_empty());
            assert!(!gate.status.as_str().is_empty());
            assert!(!gate.evidence.is_empty());
            assert!(!gate.release_requirement.is_empty());
        }
    }

    #[test]
    fn platform_matrix_has_unique_complete_rows() {
        let matrix = platform_capability_matrix();
        assert_eq!(
            matrix.schema_version,
            PLATFORM_CAPABILITY_MATRIX_SCHEMA_VERSION
        );
        assert_eq!(matrix.report_type, PLATFORM_CAPABILITY_MATRIX_REPORT_TYPE);
        assert_eq!(matrix.reviewed_on, "2026-07-12");

        for (index, platform) in matrix.platforms.iter().enumerate() {
            assert!(!platform.id.is_empty());
            assert!(!platform.platform.is_empty());
            assert!(["A", "B", "C"].contains(&platform.tier));
            assert!(!matrix.platforms[..index]
                .iter()
                .any(|previous| previous.id == platform.id));
        }
    }

    #[test]
    fn platform_matrix_covers_every_advertised_runtime_without_inheriting_evidence() {
        let ids = platform_capabilities()
            .iter()
            .map(|platform| platform.id)
            .collect::<Vec<_>>();
        for expected in [
            "linux-desktop",
            "macos-desktop",
            "windows-desktop",
            "ios",
            "android",
            "tvos",
            "auv3-host",
        ] {
            assert!(ids.contains(&expected), "missing platform {expected}");
        }

        assert!(!platform_capability_matrix().all_release_ready());
        let ios = platform_capabilities()
            .iter()
            .find(|item| item.id == "ios")
            .unwrap();
        assert_eq!(ios.touch, PlatformCapabilityStatus::Supported);
        assert!(!ios.evidence.runtime_smoke);
        assert!(ios.blocker.is_some());
    }

    #[test]
    fn platform_matrix_markdown_separates_capability_from_evidence() {
        let markdown = platform_capability_matrix().to_markdown_table();
        assert!(markdown.contains(PLATFORM_CAPABILITY_MATRIX_REPORT_TYPE));
        assert!(markdown.contains("| Platform | Tier | Pointer | Touch | Text/IME |"));
        assert!(markdown.contains("| iOS | B | partial | supported | partial | partial | no |"));
        assert!(markdown.contains("TalkBack"));
        assert!(markdown.contains("DAW host attach/detach/resize"));
    }

    #[test]
    fn release_qa_matrix_has_unique_gate_ids() {
        let gates = release_qa_gates();
        for (index, gate) in gates.iter().enumerate() {
            assert!(
                !gates[..index].iter().any(|previous| previous.id == gate.id),
                "duplicate release QA gate id {}",
                gate.id
            );
        }
    }

    #[test]
    fn release_qa_matrix_covers_required_platforms_and_release_gates() {
        let ids = release_qa_gates()
            .iter()
            .map(|gate| gate.id)
            .collect::<Vec<_>>();

        for expected in [
            "workspace-all-targets",
            "macos-desktop",
            "au-host",
            "ios-simulator",
            "android-target",
            "tvos-simulator-device",
            "windows-native",
            "showcase-visual",
            "publish-dry-runs",
            "release-notes",
            "dependency-hygiene",
        ] {
            assert!(ids.contains(&expected), "missing gate {expected}");
        }
    }

    #[test]
    fn release_qa_matrix_is_not_all_passed_until_external_gates_run() {
        let matrix = release_qa_matrix();

        assert!(!matrix.all_passed());
        assert!(matrix
            .gates
            .iter()
            .any(|gate| gate.status == ReleaseQaStatus::Partial));
        assert!(matrix
            .gates
            .iter()
            .any(|gate| gate.status == ReleaseQaStatus::ManualRequired));
        assert!(matrix
            .gates
            .iter()
            .any(|gate| gate.status == ReleaseQaStatus::Pending));
    }

    #[test]
    fn release_qa_matrix_records_dependency_hygiene_acceptance() {
        let gate = release_qa_gates()
            .iter()
            .find(|gate| gate.id == "dependency-hygiene")
            .expect("dependency-hygiene release QA gate");

        assert_eq!(gate.status, ReleaseQaStatus::Passed);
        assert!(gate.evidence.contains("cargo-deny"));
        assert!(gate.evidence.contains("passed advisories"));
        assert!(gate.release_requirement.contains("review every warning"));
    }

    #[test]
    fn release_qa_matrix_markdown_names_commands_and_blockers() {
        let markdown = release_qa_matrix().to_markdown_table();

        assert!(markdown.contains(RELEASE_QA_MATRIX_REPORT_TYPE));
        assert!(markdown.contains("cargo check --workspace --all-targets"));
        assert!(
            markdown.contains("cargo check -p gpui-android --target aarch64-linux-android --lib")
        );
        assert!(markdown.contains("partial"));
        assert!(markdown.contains("publish_plan"));
        assert!(markdown.contains("release_notes_report"));
        assert!(markdown.contains("dependency_hygiene_report"));
        assert!(markdown.contains("passed advisories"));
        assert!(markdown.contains("review every warning"));
    }
}
