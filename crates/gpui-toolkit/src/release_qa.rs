//! Release QA matrix metadata for the aggregate toolkit.

/// Schema version for [`ReleaseQaMatrix`].
pub const RELEASE_QA_MATRIX_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`ReleaseQaMatrix`].
pub const RELEASE_QA_MATRIX_REPORT_TYPE: &str = "gpui-toolkit-release-qa-matrix";

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
        command: "cargo run --bin layout-showcase --features showcase",
        status: ReleaseQaStatus::ManualRequired,
        evidence: "Host compilation passed through workspace checks; runtime showcase walkthrough was not recorded.",
        release_requirement: "Record a macOS desktop launch, keyboard, resize, and visual smoke pass.",
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
        status: ReleaseQaStatus::Blocked,
        evidence: "Blocked locally by missing aarch64-linux-android-clang from the Android NDK.",
        release_requirement: "Install/configure the NDK toolchain and record target compile plus emulator/device smoke results.",
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
        command: "cargo check --target x86_64-pc-windows-msvc plus native smoke pass",
        status: ReleaseQaStatus::Pending,
        evidence: "Known hide/unhide panic stubs were fixed; no native Windows target check was recorded.",
        release_requirement: "Run Windows target compile and input/IME/accessibility smoke tests on Windows.",
    },
    ReleaseQaGate {
        id: "showcase-visual",
        area: "Showcase visual QA",
        command: "component-lab visual manifest capture and pixel diff",
        status: ReleaseQaStatus::Pending,
        evidence: "Component-lab and builder visual manifests exist, but screenshots/diffs were not captured.",
        release_requirement: "Attach generated screenshots/diffs or a visual-regression report for release artifacts.",
    },
    ReleaseQaGate {
        id: "publish-dry-runs",
        area: "Crate publishing",
        command: "gpui_toolkit::publish_plan() and cargo publish --dry-run per selected public crate",
        status: ReleaseQaStatus::Pending,
        evidence: "Ordered publish plan exists and gpui-design dry-run passed; gpui-builder and gpui-ui-kit are blocked until predecessor crates are available on crates.io.",
        release_requirement: "Run dry-runs in publish-plan order for selected crates and record pass/fail output.",
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
        status: ReleaseQaStatus::Partial,
        evidence: "dependency_hygiene_report() records an accepted cargo audit run with --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195, zero remaining vulnerabilities, warning-class unmaintained advisories, and resolved future-incompatibility debt; cargo-deny installation/execution is still missing.",
        release_requirement: "Attach the accepted audit output, keep the quick-xml acceptance reviewed until upstream dependencies update, install/run cargo-deny, and triage any deny findings before external release.",
    },
];

/// Return the current release QA matrix.
pub const fn release_qa_matrix() -> ReleaseQaMatrix {
    ReleaseQaMatrix {
        schema_version: RELEASE_QA_MATRIX_SCHEMA_VERSION,
        report_type: RELEASE_QA_MATRIX_REPORT_TYPE,
        reviewed_on: "2026-07-08",
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
        assert_eq!(matrix.reviewed_on, "2026-07-08");
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
        assert!(
            matrix
                .gates
                .iter()
                .any(|gate| gate.status == ReleaseQaStatus::Blocked)
        );
        assert!(
            matrix
                .gates
                .iter()
                .any(|gate| gate.status == ReleaseQaStatus::ManualRequired)
        );
        assert!(
            matrix
                .gates
                .iter()
                .any(|gate| gate.status == ReleaseQaStatus::Pending)
        );
    }

    #[test]
    fn release_qa_matrix_records_dependency_hygiene_partial_acceptance() {
        let gate = release_qa_gates()
            .iter()
            .find(|gate| gate.id == "dependency-hygiene")
            .expect("dependency-hygiene release QA gate");

        assert_eq!(gate.status, ReleaseQaStatus::Partial);
        assert!(gate.evidence.contains("--ignore RUSTSEC-2026-0194"));
        assert!(gate.evidence.contains("zero remaining vulnerabilities"));
        assert!(gate.evidence.contains("future-incompatibility"));
        assert!(gate.release_requirement.contains("install/run cargo-deny"));
    }

    #[test]
    fn release_qa_matrix_markdown_names_commands_and_blockers() {
        let markdown = release_qa_matrix().to_markdown_table();

        assert!(markdown.contains(RELEASE_QA_MATRIX_REPORT_TYPE));
        assert!(markdown.contains("cargo check --workspace --all-targets"));
        assert!(markdown.contains("aarch64-linux-android-clang"));
        assert!(markdown.contains("publish_plan"));
        assert!(markdown.contains("release_notes_report"));
        assert!(markdown.contains("dependency_hygiene_report"));
        assert!(markdown.contains("--ignore RUSTSEC-2026-0194"));
        assert!(markdown.contains("install/run cargo-deny"));
    }
}
