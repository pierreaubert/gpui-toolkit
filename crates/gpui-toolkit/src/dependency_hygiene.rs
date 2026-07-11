//! Dependency hygiene policy metadata for release QA.

/// Schema version for [`DependencyHygieneReport`].
pub const DEPENDENCY_HYGIENE_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`DependencyHygieneReport`].
pub const DEPENDENCY_HYGIENE_REPORT_TYPE: &str = "gpui-toolkit-dependency-hygiene";

/// Current release triage state for a dependency advisory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyAdvisoryTriageStatus {
    /// The advisory blocks an external release until remediated or accepted.
    ReleaseBlocking,
    /// The advisory has an explicit, documented risk acceptance for this release scope.
    RiskAccepted,
    /// The advisory is a non-vulnerability warning that still needs tracking.
    WarningTracked,
}

impl DependencyAdvisoryTriageStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReleaseBlocking => "release-blocking",
            Self::RiskAccepted => "risk-accepted",
            Self::WarningTracked => "warning-tracked",
        }
    }

    /// Return whether this advisory triage row still blocks an external release.
    pub const fn is_release_blocking(self) -> bool {
        matches!(self, Self::ReleaseBlocking)
    }
}

/// Current status for a dependency hygiene check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyHygieneStatus {
    /// The policy/configuration exists and can be consumed by release tooling.
    Configured,
    /// The required tool is available in the checked local environment.
    ToolAvailable,
    /// The check passed only after applying documented advisory acceptance.
    AcceptedWithWarnings,
    /// The required tool is missing in the checked local environment.
    ToolMissing,
    /// The check was executed and found a release-blocking issue.
    Failed,
    /// The check must be executed before an external release can claim it.
    ReleaseRunPending,
    /// The check requires human review of generated build or release output.
    ManualReviewRequired,
}

impl DependencyHygieneStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Configured => "configured",
            Self::ToolAvailable => "tool-available",
            Self::AcceptedWithWarnings => "accepted-with-warnings",
            Self::ToolMissing => "tool-missing",
            Self::Failed => "failed",
            Self::ReleaseRunPending => "release-run-pending",
            Self::ManualReviewRequired => "manual-review-required",
        }
    }

    /// Return whether this status is sufficient for an external release claim.
    pub const fn is_release_ready(self) -> bool {
        matches!(
            self,
            Self::Configured | Self::ToolAvailable | Self::AcceptedWithWarnings
        )
    }
}

/// One dependency hygiene check or policy input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependencyHygieneCheck {
    /// Stable check id.
    pub id: &'static str,
    /// Command, config path, or report entry point.
    pub command: &'static str,
    /// Current status from the release report.
    pub status: DependencyHygieneStatus,
    /// Why this check exists.
    pub purpose: &'static str,
    /// Evidence recorded for this report.
    pub evidence: &'static str,
    /// What must be true before an external release can claim this check.
    pub release_requirement: &'static str,
}

/// One dependency advisory triage row from the latest local audit run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependencyAdvisoryTriage {
    /// Stable RustSec advisory id.
    pub advisory_id: &'static str,
    /// Affected crate name.
    pub crate_name: &'static str,
    /// Resolved versions in the current lockfile that triggered the advisory.
    pub affected_versions: &'static str,
    /// Current release triage state.
    pub status: DependencyAdvisoryTriageStatus,
    /// Current dependency path or subsystem responsible for the finding.
    pub affected_path: &'static str,
    /// Current project decision.
    pub current_decision: &'static str,
    /// Required action before an external release.
    pub required_action: &'static str,
}

/// Versioned dependency hygiene report for release notes and CI artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependencyHygieneReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub cargo_deny_policy_path: &'static str,
    pub checks: &'static [DependencyHygieneCheck],
    pub advisory_triage: &'static [DependencyAdvisoryTriage],
}

impl DependencyHygieneReport {
    /// Return true only when every dependency hygiene check is release-ready.
    pub fn all_release_ready(self) -> bool {
        self.checks
            .iter()
            .all(|check| check.status.is_release_ready())
    }

    /// Return checks that still block an external release claim.
    pub fn blocking_checks(self) -> impl Iterator<Item = &'static DependencyHygieneCheck> {
        self.checks
            .iter()
            .filter(|check| !check.status.is_release_ready())
    }

    /// Return advisory triage rows that still block an external release claim.
    pub fn blocking_advisories(self) -> impl Iterator<Item = &'static DependencyAdvisoryTriage> {
        self.advisory_triage
            .iter()
            .filter(|advisory| advisory.status.is_release_blocking())
    }

    /// Render the report as Markdown for release notes.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI Toolkit Dependency Hygiene\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\
             - cargo_deny_policy_path: `{}`\n\n\
             | Check | Status | Command or artifact | Purpose | Evidence | Release requirement |\n\
             | --- | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on, self.cargo_deny_policy_path
        );

        for check in self.checks {
            markdown.push_str(&format!(
                "| {} | {} | `{}` | {} | {} | {} |\n",
                check.id,
                check.status.as_str(),
                check.command,
                check.purpose,
                check.evidence,
                check.release_requirement
            ));
        }

        markdown.push_str(
            "\n| Advisory | Status | Crate | Versions | Affected path | Decision | Required action |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n",
        );
        for advisory in self.advisory_triage {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                advisory.advisory_id,
                advisory.status.as_str(),
                advisory.crate_name,
                advisory.affected_versions,
                advisory.affected_path,
                advisory.current_decision,
                advisory.required_action
            ));
        }

        markdown
    }
}

const DEPENDENCY_HYGIENE_CHECKS: &[DependencyHygieneCheck] = &[
    DependencyHygieneCheck {
        id: "cargo-audit-tool",
        command: "cargo audit --version",
        status: DependencyHygieneStatus::ToolAvailable,
        purpose: "Prove the RustSec advisory scanner is installed in the checked local environment.",
        evidence: "Local 2026-07-07 check reported cargo-audit-audit 0.22.0.",
        release_requirement: "Keep cargo-audit installed in the release runner and record the exact version used.",
    },
    DependencyHygieneCheck {
        id: "cargo-audit-release-run",
        command: "cargo audit --no-fetch --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195",
        status: DependencyHygieneStatus::AcceptedWithWarnings,
        purpose: "Scan the resolved lockfile for RustSec advisories and yanked crates.",
        evidence: "Local 2026-07-08 accepted-advisory run loaded 1159 advisories, ignored the two documented quick-xml RustSec ids, found zero remaining vulnerabilities, and reported 6 warning-class unmaintained advisories.",
        release_requirement: "Keep the explicit quick-xml acceptance reviewed for this release scope, attach the accepted audit output, and remove the ignores after upstream dependencies move to quick-xml >=0.41.0.",
    },
    DependencyHygieneCheck {
        id: "cargo-deny-policy",
        command: "deny.toml",
        status: DependencyHygieneStatus::Configured,
        purpose: "Define the advisory, license, duplicate-version, and source-origin policy used by cargo-deny.",
        evidence: "The repository root now contains deny.toml with explicit advisory, license, bans, and source sections.",
        release_requirement: "Keep the policy reviewed with every new dependency source or license class.",
    },
    DependencyHygieneCheck {
        id: "cargo-deny-tool",
        command: "cargo deny --version",
        status: DependencyHygieneStatus::ToolAvailable,
        purpose: "Prove the deny policy can be evaluated by the release runner.",
        evidence: "Local 2026-07-10 check installed cargo-deny 0.20.2.",
        release_requirement: "Keep cargo-deny installed in the release runner and record the exact version used.",
    },
    DependencyHygieneCheck {
        id: "cargo-deny-release-run",
        command: "cargo deny check advisories bans licenses sources",
        status: DependencyHygieneStatus::AcceptedWithWarnings,
        purpose: "Evaluate advisory, license, duplicate-version, and source-origin policy for the workspace graph.",
        evidence: "Local 2026-07-10 run passed all four checks after adding permissive licenses, accepting GPL-3.0-or-later for existing dependencies (autoeq, math-*, zlog/ztracing), and ignoring the tracked advisories.",
        release_requirement: "Re-run cargo-deny before every release and review any new license or advisory findings; revisit GPL acceptance if the public release set changes.",
    },
    DependencyHygieneCheck {
        id: "future-incompat-review",
        command: "cargo report future-incompatibilities --id <build-id>",
        status: DependencyHygieneStatus::Configured,
        purpose: "Catch compiler-reported dependency incompatibilities that are not RustSec advisories.",
        evidence: "The previous block 0.1.6 uninhabited-static report was resolved by the active vendored block patch; the latest toolkit all-features check emitted no future-incompatibility report.",
        release_requirement: "Review any future-incompatibility report emitted by the release build before tagging.",
    },
    DependencyHygieneCheck {
        id: "vendored-patch-triage",
        command: "gpui_toolkit::vendored_patch_manifest()",
        status: DependencyHygieneStatus::Configured,
        purpose: "Keep active and inactive vendored patch reasons, upstream bases, and verification gates visible.",
        evidence: "The vendored patch manifest records active patches for gpui_wgpu, gpui_windows, objc, and zed-font-kit.",
        release_requirement: "Re-run active patch verification and update retained-change notes before upgrading vendored crates.",
    },
];

const DEPENDENCY_ADVISORY_TRIAGE: &[DependencyAdvisoryTriage] = &[
    DependencyAdvisoryTriage {
        advisory_id: "RUSTSEC-2026-0194",
        crate_name: "quick-xml",
        affected_versions: "0.30.0, 0.39.4",
        status: DependencyAdvisoryTriageStatus::RiskAccepted,
        affected_path: "xcb/zed-scap platform capture path and Linux zbus_xml/wayland-scanner accessibility/display paths",
        current_decision: "Accepted for the current internal toolkit snapshot because the affected paths are upstream Zed Linux platform dependencies and registry-compatible updates do not move xcb, wayland-scanner, or zbus_xml to quick-xml >=0.41.0.",
        required_action: "Carry the explicit cargo-audit --ignore entry only with release-manager approval; remove it once upstream dependencies move every resolved quick-xml copy to >=0.41.0.",
    },
    DependencyAdvisoryTriage {
        advisory_id: "RUSTSEC-2026-0195",
        crate_name: "quick-xml",
        affected_versions: "0.30.0, 0.39.4",
        status: DependencyAdvisoryTriageStatus::RiskAccepted,
        affected_path: "xcb/zed-scap platform capture path and Linux zbus_xml/wayland-scanner accessibility/display paths",
        current_decision: "Accepted for the current internal toolkit snapshot; memory-exhaustion parser risk remains documented in transitive upstream Linux platform dependencies.",
        required_action: "Carry the explicit cargo-audit --ignore entry only with release-manager approval; remove it once upstream dependencies move every resolved quick-xml copy to >=0.41.0.",
    },
    DependencyAdvisoryTriage {
        advisory_id: "RUSTSEC-2025-0141 / RUSTSEC-2024-0384 / RUSTSEC-2024-0436 / RUSTSEC-2026-0173 / RUSTSEC-2026-0192",
        crate_name: "bincode, instant, paste, proc-macro-error2, ttf-parser",
        affected_versions: "see cargo audit --no-fetch output",
        status: DependencyAdvisoryTriageStatus::WarningTracked,
        affected_path: "autoeq, gpui_windows, image/QR/camera, stacksafe, font rendering transitive paths",
        current_decision: "Tracked as unmaintained-warning debt, not the current vulnerability blocker.",
        required_action: "Review replacements during dependency upgrade work and keep warnings visible in release notes.",
    },
];

/// Return the current dependency hygiene report.
pub const fn dependency_hygiene_report() -> DependencyHygieneReport {
    DependencyHygieneReport {
        schema_version: DEPENDENCY_HYGIENE_SCHEMA_VERSION,
        report_type: DEPENDENCY_HYGIENE_REPORT_TYPE,
        reviewed_on: "2026-07-10",
        cargo_deny_policy_path: "deny.toml",
        checks: DEPENDENCY_HYGIENE_CHECKS,
        advisory_triage: DEPENDENCY_ADVISORY_TRIAGE,
    }
}

/// Return dependency hygiene checks without allocating.
pub const fn dependency_hygiene_checks() -> &'static [DependencyHygieneCheck] {
    DEPENDENCY_HYGIENE_CHECKS
}

/// Return dependency advisory triage rows without allocating.
pub const fn dependency_advisory_triage() -> &'static [DependencyAdvisoryTriage] {
    DEPENDENCY_ADVISORY_TRIAGE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_hygiene_report_has_stable_contract() {
        let report = dependency_hygiene_report();

        assert_eq!(report.schema_version, DEPENDENCY_HYGIENE_SCHEMA_VERSION);
        assert_eq!(report.report_type, DEPENDENCY_HYGIENE_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-07-10");
        assert_eq!(report.cargo_deny_policy_path, "deny.toml");
        assert!(!report.checks.is_empty());
        assert!(!report.advisory_triage.is_empty());

        for check in report.checks {
            assert!(!check.id.is_empty());
            assert!(!check.command.is_empty());
            assert!(!check.status.as_str().is_empty());
            assert!(!check.purpose.is_empty());
            assert!(!check.evidence.is_empty());
            assert!(!check.release_requirement.is_empty());
        }
    }

    #[test]
    fn dependency_hygiene_report_has_unique_check_ids() {
        let checks = dependency_hygiene_checks();
        for (index, check) in checks.iter().enumerate() {
            assert!(
                !checks[..index]
                    .iter()
                    .any(|previous| previous.id == check.id),
                "duplicate dependency hygiene check id {}",
                check.id
            );
        }
    }

    #[test]
    fn dependency_hygiene_report_has_unique_advisory_triage_ids() {
        let advisories = dependency_advisory_triage();
        for (index, advisory) in advisories.iter().enumerate() {
            assert!(
                !advisories[..index]
                    .iter()
                    .any(|previous| previous.advisory_id == advisory.advisory_id),
                "duplicate dependency advisory triage id {}",
                advisory.advisory_id
            );
            assert!(!advisory.crate_name.is_empty());
            assert!(!advisory.affected_versions.is_empty());
            assert!(!advisory.status.as_str().is_empty());
            assert!(!advisory.affected_path.is_empty());
            assert!(!advisory.current_decision.is_empty());
            assert!(!advisory.required_action.is_empty());
        }
    }

    #[test]
    fn dependency_hygiene_report_covers_required_release_gates() {
        let ids = dependency_hygiene_checks()
            .iter()
            .map(|check| check.id)
            .collect::<Vec<_>>();

        for expected in [
            "cargo-audit-tool",
            "cargo-audit-release-run",
            "cargo-deny-policy",
            "cargo-deny-tool",
            "cargo-deny-release-run",
            "future-incompat-review",
            "vendored-patch-triage",
        ] {
            assert!(ids.contains(&expected), "missing check {expected}");
        }
    }

    #[test]
    fn dependency_hygiene_report_is_release_ready_after_tools_run() {
        let report = dependency_hygiene_report();
        let blocking = report
            .blocking_checks()
            .map(|check| check.id)
            .collect::<Vec<_>>();

        assert!(report.all_release_ready());
        assert!(blocking.is_empty());
        assert!(!blocking.contains(&"cargo-audit-release-run"));
        assert!(!blocking.contains(&"cargo-deny-tool"));
        assert!(!blocking.contains(&"cargo-deny-release-run"));
        assert!(!blocking.contains(&"future-incompat-review"));
    }

    #[test]
    fn dependency_hygiene_report_records_quick_xml_risk_acceptance() {
        let report = dependency_hygiene_report();
        let blocking = report
            .blocking_advisories()
            .map(|advisory| advisory.advisory_id)
            .collect::<Vec<_>>();

        assert!(!blocking.contains(&"RUSTSEC-2026-0194"));
        assert!(!blocking.contains(&"RUSTSEC-2026-0195"));
        assert!(dependency_advisory_triage().iter().any(|advisory| {
            advisory.advisory_id == "RUSTSEC-2026-0194"
                && advisory.status == DependencyAdvisoryTriageStatus::RiskAccepted
        }));
        assert!(dependency_advisory_triage().iter().any(|advisory| {
            advisory.advisory_id == "RUSTSEC-2026-0195"
                && advisory.status == DependencyAdvisoryTriageStatus::RiskAccepted
        }));
        assert!(
            dependency_advisory_triage()
                .iter()
                .any(|advisory| advisory.status == DependencyAdvisoryTriageStatus::WarningTracked)
        );
    }

    #[test]
    fn dependency_hygiene_markdown_names_policy_and_blockers() {
        let markdown = dependency_hygiene_report().to_markdown_table();

        assert!(markdown.contains(DEPENDENCY_HYGIENE_REPORT_TYPE));
        assert!(markdown.contains("deny.toml"));
        assert!(markdown.contains("cargo audit"));
        assert!(markdown.contains("cargo deny check advisories bans licenses sources"));
        assert!(markdown.contains("RUSTSEC-2026-0194"));
        assert!(markdown.contains("RUSTSEC-2026-0195"));
        assert!(markdown.contains("risk-accepted"));
        assert!(markdown.contains("--ignore RUSTSEC-2026-0194"));
        assert!(markdown.contains("quick-xml"));
        assert!(markdown.contains(">=0.41.0"));
        assert!(markdown.contains("vendored_patch_manifest"));
    }
}
