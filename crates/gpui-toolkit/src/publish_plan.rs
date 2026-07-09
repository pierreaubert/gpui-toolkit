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
            Self::Excluded => "excluded",
        }
    }

    /// Return whether this status is sufficient for an external release claim.
    pub const fn is_release_ready(self) -> bool {
        matches!(self, Self::DryRunPassed | Self::Excluded)
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
        command: "cargo publish --dry-run -p gpui-design --allow-dirty",
        status: PublishPlanStatus::DryRunPassed,
        reason: "Leaf public-core design crate used by builder and UI kit.",
        evidence: "Dry-run passed cleanly after optional GPUI integration re-exports were gated behind the gpui feature.",
        release_requirement: "Re-run immediately before publishing and remove --allow-dirty in a clean release worktree.",
    },
    PublishPlanEntry {
        crate_name: "gpui-pretext",
        order: 2,
        lane: "public-core",
        command: "cargo publish --dry-run -p gpui-pretext --allow-dirty",
        status: PublishPlanStatus::PendingDryRun,
        reason: "Leaf public-core text crate required before gpui-builder can resolve on crates.io.",
        evidence: "No dry-run was recorded in this release report.",
        release_requirement: "Run dry-run and publish before gpui-builder.",
    },
    PublishPlanEntry {
        crate_name: "gpui-builder",
        order: 3,
        lane: "public-core",
        command: "cargo publish --dry-run -p gpui-builder --allow-dirty",
        status: PublishPlanStatus::BlockedByPredecessor,
        reason: "Depends on gpui-design and gpui-pretext from crates.io during package verification.",
        evidence: "Dry-run failed because crates.io only has gpui-design 0.6.0, not the required 0.8 line.",
        release_requirement: "Re-run after gpui-design and gpui-pretext are available at compatible versions.",
    },
    PublishPlanEntry {
        crate_name: "gpui-ui-kit-macros",
        order: 4,
        lane: "public-core",
        command: "cargo publish --dry-run -p gpui-ui-kit-macros --allow-dirty",
        status: PublishPlanStatus::PendingDryRun,
        reason: "Proc-macro helper should be available before gpui-ui-kit.",
        evidence: "No dry-run was recorded in this release report.",
        release_requirement: "Run dry-run and publish before gpui-ui-kit.",
    },
    PublishPlanEntry {
        crate_name: "gpui-ui-kit",
        order: 5,
        lane: "public-core",
        command: "cargo publish --dry-run -p gpui-ui-kit --allow-dirty",
        status: PublishPlanStatus::BlockedByPredecessor,
        reason: "Depends on gpui-builder, gpui-design, and gpui-ui-kit-macros from crates.io during package verification.",
        evidence: "Dry-run failed because crates.io has no matching gpui-builder package yet.",
        release_requirement: "Re-run after gpui-builder, gpui-design, and gpui-ui-kit-macros are available at compatible versions.",
    },
    PublishPlanEntry {
        crate_name: "gpui-audio-kit",
        order: 6,
        lane: "public-core",
        command: "cargo publish --dry-run -p gpui-audio-kit --allow-dirty",
        status: PublishPlanStatus::PendingDryRun,
        reason: "Public-core audio controls should dry-run after the UI/design foundation is available.",
        evidence: "No dry-run was recorded in this release report.",
        release_requirement: "Run dry-run after UI/design dependencies are resolvable.",
    },
    PublishPlanEntry {
        crate_name: "gpui-keybinding",
        order: 7,
        lane: "public-core",
        command: "cargo publish --dry-run -p gpui-keybinding --allow-dirty",
        status: PublishPlanStatus::PendingDryRun,
        reason: "Public-core keybinding crate has no known in-lane predecessor but still needs packaging verification.",
        evidence: "No dry-run was recorded in this release report.",
        release_requirement: "Run dry-run before release.",
    },
    PublishPlanEntry {
        crate_name: "gpui-themes",
        order: 8,
        lane: "public-core",
        command: "cargo publish --dry-run -p gpui-themes --allow-dirty",
        status: PublishPlanStatus::PendingDryRun,
        reason: "Public-core theme crate needs packaging verification after schema docs landed.",
        evidence: "No dry-run was recorded in this release report.",
        release_requirement: "Run dry-run before release.",
    },
    PublishPlanEntry {
        crate_name: "gpui-d3rs",
        order: 9,
        lane: "beta-visualization",
        command: "cargo publish --dry-run -p gpui-d3rs --allow-dirty",
        status: PublishPlanStatus::PendingDryRun,
        reason: "Beta visualization crate should dry-run only if included in this release.",
        evidence: "No dry-run was recorded in this release report.",
        release_requirement: "Run dry-run if the beta visualization lane is included.",
    },
    PublishPlanEntry {
        crate_name: "gpui-px",
        order: 10,
        lane: "beta-visualization",
        command: "cargo publish --dry-run -p gpui-px --allow-dirty",
        status: PublishPlanStatus::PendingDryRun,
        reason: "Beta charting crate depends on visualization/foundation crates and should follow them.",
        evidence: "No dry-run was recorded in this release report.",
        release_requirement: "Run dry-run after required beta/public-core dependencies are resolvable if included.",
    },
];

/// Return the current publish plan.
pub const fn publish_plan() -> PublishPlan {
    PublishPlan {
        schema_version: PUBLISH_PLAN_SCHEMA_VERSION,
        report_type: PUBLISH_PLAN_REPORT_TYPE,
        reviewed_on: "2026-07-08",
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
        assert_eq!(plan.reviewed_on, "2026-07-08");
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
            "gpui-pretext",
            "gpui-builder",
            "gpui-ui-kit-macros",
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

        assert!(!plan.all_release_ready());
        assert!(!blocking.contains(&"gpui-design"));
        assert!(blocking.contains(&"gpui-builder"));
        assert!(blocking.contains(&"gpui-ui-kit"));
        assert!(blocking.contains(&"gpui-pretext"));
    }

    #[test]
    fn publish_plan_markdown_names_dry_run_statuses() {
        let markdown = publish_plan().to_markdown_table();

        assert!(markdown.contains(PUBLISH_PLAN_REPORT_TYPE));
        assert!(markdown.contains("gpui-design"));
        assert!(markdown.contains("dry-run-passed"));
        assert!(markdown.contains("blocked-by-predecessor"));
        assert!(markdown.contains("cargo publish --dry-run -p gpui-ui-kit"));
    }
}
