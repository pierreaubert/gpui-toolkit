//! Ordered crate publish/dry-run plan for release QA.

/// Schema version for [`PublishPlan`].
pub const PUBLISH_PLAN_SCHEMA_VERSION: u32 = 1;

/// Stable report type identifier for [`PublishPlan`].
pub const PUBLISH_PLAN_REPORT_TYPE: &str = "gpui-toolkit-publish-plan";

/// Current status for one crate in the publish plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublishPlanStatus {
    /// The crate's dry-run passed in the current release report.
    DryRunPassed,
    /// The crate must wait for an earlier crate in the publish order.
    BlockedByPredecessor,
    /// The crate has not been dry-run yet.
    PendingDryRun,
    /// Included in the signed source-beta tag, but intentionally not uploaded to crates.io.
    SourceReleaseReady,
    /// Intentionally postponed to a later registry wave with a named prerequisite.
    Deferred,
    /// The crate is intentionally excluded from this public release.
    Excluded,
}

impl PublishPlanStatus {
    /// Stable status label for release reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DryRunPassed => "dry-run-passed",
            Self::BlockedByPredecessor => "blocked-by-predecessor",
            Self::PendingDryRun => "pending-dry-run",
            Self::SourceReleaseReady => "source-release-ready",
            Self::Deferred => "deferred",
            Self::Excluded => "excluded",
        }
    }

    /// Return whether this status is sufficient for an external release claim.
    pub const fn is_release_ready(self) -> bool {
        matches!(
            self,
            Self::DryRunPassed | Self::SourceReleaseReady | Self::Deferred | Self::Excluded
        )
    }
}

/// One crate in the ordered publish plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishPlanEntry {
    /// Cargo package name.
    pub crate_name: &'static str,
    /// 1-based order within its publish lane.
    pub order: u8,
    /// Intended publish lane.
    pub lane: &'static str,
    /// Publish dry-run command.
    pub command: &'static str,
    /// Current status from the release report.
    pub status: PublishPlanStatus,
    /// Why this crate appears at this point in the order.
    pub reason: &'static str,
    /// Evidence recorded for this report.
    pub evidence: &'static str,
    /// What must be true before external release.
    pub release_requirement: &'static str,
}

/// Versioned publish plan for release notes and CI artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishPlan {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [PublishPlanEntry],
}

impl PublishPlan {
    /// Return true only when every publish-plan entry is release-ready.
    pub fn all_release_ready(self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.status.is_release_ready())
    }

    /// Return entries that still block the publish lane.
    pub fn blocking_entries(self) -> impl Iterator<Item = &'static PublishPlanEntry> {
        self.entries
            .iter()
            .filter(|entry| !entry.status.is_release_ready())
    }

    /// Render the plan as Markdown for release notes.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI Toolkit Publish Plan\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Order | Crate | Lane | Status | Command | Evidence | Requirement |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | `{}` | {} | {} |\n",
                entry.order,
                entry.crate_name,
                entry.lane,
                entry.status.as_str(),
                entry.command,
                entry.evidence,
                entry.release_requirement
            ));
        }

        markdown
    }
}

const PUBLISH_PLAN_ENTRIES: &[PublishPlanEntry] = &[
    PublishPlanEntry {
        crate_name: "gpui-design",
        order: 1,
        lane: "public-core",
        command: "cargo publish --dry-run --locked -p gpui-design",
        status: PublishPlanStatus::DryRunPassed,
        reason: "Leaf public-core design crate used by builder and UI kit.",
        evidence: "Locked dry-run passed from the release branch on 2026-08-07; the optional `gpui` feature remains outside the registry package verification path.",
        release_requirement: "Re-run from the clean release commit immediately before the explicit publish action.",
    },
    PublishPlanEntry {
        crate_name: "gpui-profiler",
        order: 2,
        lane: "public-core",
        command: "cargo publish --dry-run --locked -p gpui-profiler",
        status: PublishPlanStatus::DryRunPassed,
        reason: "GPUI-free allocation profiling support and a dev-dependency predecessor for gpui-pretext.",
        evidence: "Locked dry-run passed from the release branch on 2026-08-07.",
        release_requirement: "Publish before attempting the deferred gpui-pretext registry wave.",
    },
    PublishPlanEntry {
        crate_name: "gpui-ui-kit-macros",
        order: 3,
        lane: "public-core",
        command: "cargo publish --dry-run --locked -p gpui-ui-kit-macros",
        status: PublishPlanStatus::DryRunPassed,
        reason: "Standalone proc-macro package with no unpublished runtime dependency.",
        evidence: "Locked dry-run passed from the release branch on 2026-08-07.",
        release_requirement: "Re-run from the clean release commit immediately before the explicit publish action.",
    },
    PublishPlanEntry {
        crate_name: "gpui-pretext",
        order: 4,
        lane: "deferred-registry",
        command: "cargo publish --dry-run --locked -p gpui-pretext",
        status: PublishPlanStatus::Deferred,
        reason: "Package verification resolves the allocation-contract dev-dependency from crates.io.",
        evidence: "2026-08-07 locked dry-run stopped because gpui-profiler is not yet present on crates.io.",
        release_requirement: "Publish gpui-profiler, then require a clean locked dry-run before including gpui-pretext in a later registry wave.",
    },
    PublishPlanEntry {
        crate_name: "gpui-builder",
        order: 5,
        lane: "deferred-registry",
        command: "cargo publish --dry-run --locked -p gpui-builder",
        status: PublishPlanStatus::Deferred,
        reason: "Package verification requires compatible gpui-design and gpui-pretext registry releases.",
        evidence: "Registry predecessors are not all available at the workspace-compatible versions.",
        release_requirement: "Publish and verify both predecessors, then require a clean locked dry-run.",
    },
    PublishPlanEntry {
        crate_name: "gpui-ui-kit",
        order: 6,
        lane: "source-beta",
        command: "just qa && just release-rc <version>",
        status: PublishPlanStatus::SourceReleaseReady,
        reason: "The UI kit depends on the vendored, unpublished GPUI runtime.",
        evidence: "Distributed through the tagged source/RC bundle with renderer-backed stories and snapshots; not presented as a crates.io package.",
        release_requirement: "Keep beta/API and platform limitations in the release notes.",
    },
    PublishPlanEntry {
        crate_name: "gpui-audio-kit",
        order: 7,
        lane: "source-beta",
        command: "just qa && just release-rc <version>",
        status: PublishPlanStatus::SourceReleaseReady,
        reason: "Audio controls depend on GPUI and gpui-ui-kit.",
        evidence: "Included in the tagged source beta with focused control, allocation, accessibility, and snapshot evidence.",
        release_requirement: "Do not upload to crates.io until the complete dependency chain is registry-resolvable.",
    },
    PublishPlanEntry {
        crate_name: "gpui-keybinding",
        order: 8,
        lane: "source-beta",
        command: "just qa && just release-rc <version>",
        status: PublishPlanStatus::SourceReleaseReady,
        reason: "The runtime keybinding integration depends on unpublished GPUI.",
        evidence: "Included in the tagged source beta with conflict and platform-policy tests.",
        release_requirement: "Do not claim registry availability.",
    },
    PublishPlanEntry {
        crate_name: "gpui-themes",
        order: 9,
        lane: "source-beta",
        command: "just qa && just release-rc <version>",
        status: PublishPlanStatus::SourceReleaseReady,
        reason: "Theme tooling depends on GPUI and gpui-ui-kit.",
        evidence: "Included in the tagged source beta with schema/version compatibility tests.",
        release_requirement: "Do not claim registry availability.",
    },
    PublishPlanEntry {
        crate_name: "gpui-d3rs",
        order: 10,
        lane: "source-beta",
        command: "just qa && just release-rc <version>",
        status: PublishPlanStatus::SourceReleaseReady,
        reason: "Default visualization features integrate with unpublished GPUI.",
        evidence: "Included in the tagged source beta with capability, performance, interaction, and rendered-example evidence.",
        release_requirement: "Keep fallible-API and platform/rendering limitations explicit.",
    },
    PublishPlanEntry {
        crate_name: "gpui-px",
        order: 11,
        lane: "source-beta",
        command: "just qa && just release-rc <version>",
        status: PublishPlanStatus::SourceReleaseReady,
        reason: "Depends on gpui-d3rs and unpublished GPUI integration.",
        evidence: "Included in the tagged source beta with chart capability, accessibility, interaction, export, and snapshot evidence.",
        release_requirement: "Keep chart limitations explicit and do not claim registry availability.",
    },
];

/// Return the current publish plan.
pub const fn publish_plan() -> PublishPlan {
    PublishPlan {
        schema_version: PUBLISH_PLAN_SCHEMA_VERSION,
        report_type: PUBLISH_PLAN_REPORT_TYPE,
        reviewed_on: "2026-08-07",
        entries: PUBLISH_PLAN_ENTRIES,
    }
}

/// Return publish-plan entries without allocating.
pub const fn publish_plan_entries() -> &'static [PublishPlanEntry] {
    PUBLISH_PLAN_ENTRIES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_plan_has_stable_contract() {
        let plan = publish_plan();

        assert_eq!(plan.schema_version, PUBLISH_PLAN_SCHEMA_VERSION);
        assert_eq!(plan.report_type, PUBLISH_PLAN_REPORT_TYPE);
        assert_eq!(plan.reviewed_on, "2026-08-07");
        assert!(!plan.entries.is_empty());

        for entry in plan.entries {
            assert!(!entry.crate_name.is_empty());
            assert!(entry.order > 0);
            assert!(!entry.lane.is_empty());
            assert!(!entry.command.is_empty());
            assert!(!entry.status.as_str().is_empty());
            assert!(!entry.reason.is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn publish_plan_has_unique_crate_names_and_order() {
        let entries = publish_plan_entries();
        for (index, entry) in entries.iter().enumerate() {
            assert!(
                !entries[..index]
                    .iter()
                    .any(|previous| previous.crate_name == entry.crate_name),
                "duplicate publish-plan crate {}",
                entry.crate_name
            );
            assert_eq!(usize::from(entry.order), index + 1);
        }
    }

    #[test]
    fn publish_plan_covers_release_lanes() {
        let crates = publish_plan_entries()
            .iter()
            .map(|entry| entry.crate_name)
            .collect::<Vec<_>>();

        for expected in [
            "gpui-design",
            "gpui-profiler",
            "gpui-ui-kit-macros",
            "gpui-pretext",
            "gpui-builder",
            "gpui-ui-kit",
            "gpui-audio-kit",
            "gpui-keybinding",
            "gpui-themes",
            "gpui-d3rs",
            "gpui-px",
        ] {
            assert!(crates.contains(&expected), "missing crate {expected}");
        }
    }

    #[test]
    fn publish_plan_blocks_release_until_ordered_dry_runs_pass() {
        let plan = publish_plan();
        let blocking = plan
            .blocking_entries()
            .map(|entry| entry.crate_name)
            .collect::<Vec<_>>();

        assert!(plan.all_release_ready());
        assert!(!blocking.contains(&"gpui-design"));
        assert!(!blocking.contains(&"gpui-profiler"));
        assert!(!blocking.contains(&"gpui-ui-kit-macros"));
        assert!(!blocking.contains(&"gpui-pretext"));
        assert!(!blocking.contains(&"gpui-builder"));
        assert!(!blocking.contains(&"gpui-ui-kit"));
    }

    #[test]
    fn publish_plan_markdown_names_dry_run_statuses() {
        let markdown = publish_plan().to_markdown_table();

        assert!(markdown.contains(PUBLISH_PLAN_REPORT_TYPE));
        assert!(markdown.contains("gpui-design"));
        assert!(markdown.contains("dry-run-passed"));
        assert!(markdown.contains("source-release-ready"));
        assert!(markdown.contains("deferred"));
        assert!(markdown.contains("cargo publish --dry-run --locked -p gpui-profiler"));
    }
}
