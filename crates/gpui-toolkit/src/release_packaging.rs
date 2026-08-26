//! Release packaging evidence for public, beta, internal, and patched crates.

/// Schema version for [`ReleasePackagingReport`].
pub const RELEASE_PACKAGING_SCHEMA_VERSION: u32 = 2;

/// Stable report type identifier for [`ReleasePackagingReport`].
pub const RELEASE_PACKAGING_REPORT_TYPE: &str = "gpui-toolkit-release-packaging";

/// Current packaging state for one crate or release lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleasePackagingStatus {
    /// A packaging command passed in the current release report.
    Passed,
    /// Packaging was attempted and is blocked by a known predecessor or registry state.
    Blocked,
    /// Packaging is intentionally postponed until an ordered registry wave.
    Deferred,
    /// Packaging still needs to be executed before release.
    Pending,
    /// The crate/lane is intentionally not published from this workspace.
    Excluded,
    /// The evidence requires an external platform/app-store/signing gate.
    ExternalGate,
}

impl ReleasePackagingStatus {
    /// Stable status label for generated reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Blocked => "blocked",
            Self::Deferred => "deferred",
            Self::Pending => "pending",
            Self::Excluded => "excluded",
            Self::ExternalGate => "external-gate",
        }
    }

    /// Whether this state is sufficient for the selected public crate release.
    pub const fn is_release_ready(self) -> bool {
        matches!(self, Self::Passed | Self::Excluded)
    }
}

/// One packaging evidence row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleasePackagingEntry {
    /// Stable row id.
    pub id: &'static str,
    /// Crate name or release lane.
    pub crate_or_lane: &'static str,
    /// Intended distribution lane.
    pub lane: &'static str,
    /// Packaging/publish command or manual action.
    pub command_or_action: &'static str,
    /// Current packaging evidence state.
    pub status: ReleasePackagingStatus,
    /// Evidence observed in this release report.
    pub evidence: &'static str,
    /// Requirement before an external release may claim this row.
    pub release_requirement: &'static str,
}

impl ReleasePackagingEntry {
    /// Whether this row still blocks packaging release claims.
    pub const fn is_release_blocking(self) -> bool {
        !self.status.is_release_ready()
    }
}

/// Versioned release packaging evidence report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleasePackagingReport {
    pub schema_version: u32,
    pub report_type: &'static str,
    pub reviewed_on: &'static str,
    pub entries: &'static [ReleasePackagingEntry],
}

impl ReleasePackagingReport {
    /// Return true only when all packaging evidence rows are release-ready.
    pub fn all_release_ready(self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.status.is_release_ready())
    }

    /// Return entries that still block packaging/release claims.
    pub fn blocking_entries(self) -> impl Iterator<Item = &'static ReleasePackagingEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.is_release_blocking())
    }

    /// Render the report as Markdown for release artifacts.
    pub fn to_markdown_table(self) -> String {
        let mut markdown = format!(
            "# GPUI Toolkit Release Packaging Evidence\n\n\
             - schema_version: {}\n\
             - report_type: `{}`\n\
             - reviewed_on: {}\n\n\
             | Row | Crate or lane | Lane | Status | Command or action | Evidence | Release requirement |\n\
             | --- | --- | --- | --- | --- | --- | --- |\n",
            self.schema_version, self.report_type, self.reviewed_on
        );

        for entry in self.entries {
            markdown.push_str(&format!(
                "| {} | {} | {} | {} | `{}` | {} | {} |\n",
                entry.id,
                entry.crate_or_lane,
                entry.lane,
                entry.status.as_str(),
                entry.command_or_action,
                entry.evidence,
                entry.release_requirement
            ));
        }

        markdown
    }
}

const RELEASE_PACKAGING_ENTRIES: &[ReleasePackagingEntry] = &[
    ReleasePackagingEntry {
        id: "gpui-design-dry-run",
        crate_or_lane: "gpui-design",
        lane: "public-core",
        command_or_action: "cargo publish --dry-run --locked -p gpui-design",
        status: ReleasePackagingStatus::Passed,
        evidence: "Locked dry-run passed from the release branch on 2026-08-07.",
        release_requirement: "Re-run from the clean release commit immediately before publishing.",
    },
    ReleasePackagingEntry {
        id: "gpui-profiler-dry-run",
        crate_or_lane: "gpui-profiler",
        lane: "public-core",
        command_or_action: "cargo publish --dry-run --locked -p gpui-profiler",
        status: ReleasePackagingStatus::Passed,
        evidence: "Locked dry-run passed from the release branch on 2026-08-07.",
        release_requirement: "Re-run from the clean release commit and publish before any later gpui-pretext registry wave.",
    },
    ReleasePackagingEntry {
        id: "gpui-pretext-dry-run",
        crate_or_lane: "gpui-pretext",
        lane: "deferred-registry",
        command_or_action: "cargo publish --dry-run --locked -p gpui-pretext",
        status: ReleasePackagingStatus::Deferred,
        evidence: "Locked dry-run on 2026-08-07 could not resolve the unpublished gpui-profiler dev-dependency from crates.io; gpui-pretext is deferred from registry wave 1.",
        release_requirement: "Publish gpui-profiler, then require a clean locked dry-run before a later registry wave.",
    },
    ReleasePackagingEntry {
        id: "gpui-builder-dry-run",
        crate_or_lane: "gpui-builder",
        lane: "deferred-registry",
        command_or_action: "cargo publish --dry-run --locked -p gpui-builder",
        status: ReleasePackagingStatus::Deferred,
        evidence: "Deferred from registry wave 1 because compatible gpui-design and gpui-pretext predecessors are not both published.",
        release_requirement: "Publish predecessors, then require a clean locked dry-run before a later registry wave.",
    },
    ReleasePackagingEntry {
        id: "gpui-ui-kit-macros-dry-run",
        crate_or_lane: "gpui-ui-kit-macros",
        lane: "public-core",
        command_or_action: "cargo publish --dry-run --locked -p gpui-ui-kit-macros",
        status: ReleasePackagingStatus::Passed,
        evidence: "Locked dry-run passed from the release branch on 2026-08-07.",
        release_requirement: "Re-run from the clean release commit immediately before publishing.",
    },
    ReleasePackagingEntry {
        id: "gpui-dependent-source-beta",
        crate_or_lane: "gpui-ui-kit + gpui-audio-kit + gpui-keybinding + gpui-themes",
        lane: "source-beta",
        command_or_action: "include in signed source/RC archive; do not upload to crates.io",
        status: ReleasePackagingStatus::Excluded,
        evidence: "These packages depend on the vendored, unpublished GPUI runtime and are explicitly source-beta only.",
        release_requirement: "Attach source archive QA and retain the beta limitation in release notes.",
    },
    ReleasePackagingEntry {
        id: "beta-visualization-dry-runs",
        crate_or_lane: "gpui-d3rs + gpui-px",
        lane: "beta-visualization",
        command_or_action: "cargo publish --dry-run per beta lane if included",
        status: ReleasePackagingStatus::Excluded,
        evidence: "Visualization crates are included in the tagged source beta and explicitly excluded from crates.io wave 1 because their default surface depends on GPUI.",
        release_requirement: "Attach capability, performance, accessibility, and renderer snapshot evidence to the source release.",
    },
    ReleasePackagingEntry {
        id: "tooling-support-crates",
        crate_or_lane: "gpui-component-lab + gpui-design-tools + gpui-python-runtime",
        lane: "support-tooling",
        command_or_action: "cargo publish --dry-run only if intentionally included",
        status: ReleasePackagingStatus::Excluded,
        evidence: "Support tooling is included in the tagged source archive and excluded from registry wave 1; gpui-profiler is tracked separately as a public package.",
        release_requirement: "Do not claim registry/API stability for these support tools.",
    },
    ReleasePackagingEntry {
        id: "internal-aggregate-and-apps",
        crate_or_lane: "gpui-toolkit + mobile/showcase/app crates",
        lane: "internal-or-app",
        command_or_action: "keep publish = false / publish = []",
        status: ReleasePackagingStatus::Excluded,
        evidence: "Aggregate, mobile, scaffolder, showcase, and app crates are treated as internal, experimental, or publish-disabled in the inclusion matrix.",
        release_requirement: "Do not publish unless manifests and release gates are deliberately changed.",
    },
    ReleasePackagingEntry {
        id: "vendored-patches",
        crate_or_lane: "block + objc + zed-font-kit + Zed backend patches",
        lane: "patched-dependencies",
        command_or_action: "keep as root [patch] dependencies and document VENDORING.md",
        status: ReleasePackagingStatus::Excluded,
        evidence: "vendored_patch_manifest() records active/inactive patch status, upstream refs, retained changes, and verification gates.",
        release_requirement: "Do not publish from this workspace; upstream, replace, or keep patch metadata current.",
    },
    ReleasePackagingEntry {
        id: "platform-installers",
        crate_or_lane: "AU/iOS/Android/tvOS/Windows packaging",
        lane: "platform-delivery",
        command_or_action: "platform build/signing/install validation",
        status: ReleasePackagingStatus::ExternalGate,
        evidence: "Rust compile/report artifacts exist for several platform crates, but installer/app-store/signing/device packaging evidence is not recorded.",
        release_requirement: "Attach platform package/signing/device validation before claiming cross-platform delivery parity.",
    },
];

/// Return the current release packaging evidence report.
pub const fn release_packaging_report() -> ReleasePackagingReport {
    ReleasePackagingReport {
        schema_version: RELEASE_PACKAGING_SCHEMA_VERSION,
        report_type: RELEASE_PACKAGING_REPORT_TYPE,
        reviewed_on: "2026-08-07",
        entries: RELEASE_PACKAGING_ENTRIES,
    }
}

/// Return packaging evidence rows without allocating.
pub const fn release_packaging_entries() -> &'static [ReleasePackagingEntry] {
    RELEASE_PACKAGING_ENTRIES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_packaging_report_has_stable_contract() {
        let report = release_packaging_report();

        assert_eq!(report.schema_version, RELEASE_PACKAGING_SCHEMA_VERSION);
        assert_eq!(report.report_type, RELEASE_PACKAGING_REPORT_TYPE);
        assert_eq!(report.reviewed_on, "2026-08-07");
        assert!(!report.entries.is_empty());
        assert!(!report.all_release_ready());

        for entry in report.entries {
            assert!(!entry.id.is_empty());
            assert!(!entry.crate_or_lane.is_empty());
            assert!(!entry.lane.is_empty());
            assert!(!entry.command_or_action.is_empty());
            assert!(!entry.status.as_str().is_empty());
            assert!(!entry.evidence.is_empty());
            assert!(!entry.release_requirement.is_empty());
        }
    }

    #[test]
    fn release_packaging_report_has_unique_rows() {
        let mut ids = std::collections::BTreeSet::new();

        for entry in release_packaging_entries() {
            assert!(ids.insert(entry.id), "duplicate packaging row {}", entry.id);
        }
    }

    #[test]
    fn release_packaging_report_records_selected_passes_and_explicit_exclusions() {
        let entries = release_packaging_entries();

        assert!(entries.iter().any(|entry| {
            entry.id == "gpui-design-dry-run" && entry.status == ReleasePackagingStatus::Passed
        }));
        assert!(entries.iter().any(|entry| {
            entry.id == "gpui-profiler-dry-run" && entry.status == ReleasePackagingStatus::Passed
        }));
        assert!(entries.iter().any(|entry| {
            entry.id == "gpui-pretext-dry-run" && entry.status == ReleasePackagingStatus::Deferred
        }));
        assert!(entries.iter().any(|entry| {
            entry.id == "internal-aggregate-and-apps"
                && entry.status == ReleasePackagingStatus::Excluded
        }));
        assert!(entries.iter().any(|entry| {
            entry.id == "platform-installers"
                && entry.status == ReleasePackagingStatus::ExternalGate
        }));
    }

    #[test]
    fn release_packaging_report_blocks_until_packaging_evidence_is_attached() {
        let blocking = release_packaging_report()
            .blocking_entries()
            .map(|entry| entry.id)
            .collect::<Vec<_>>();

        assert!(!blocking.contains(&"gpui-design-dry-run"));
        assert!(!blocking.contains(&"internal-aggregate-and-apps"));
        assert!(!blocking.contains(&"vendored-patches"));
        assert!(blocking.contains(&"gpui-pretext-dry-run"));
        assert!(blocking.contains(&"gpui-builder-dry-run"));
        assert!(blocking.contains(&"platform-installers"));
    }

    #[test]
    fn release_packaging_markdown_names_packaging_commands_and_lanes() {
        let markdown = release_packaging_report().to_markdown_table();

        assert!(markdown.contains(RELEASE_PACKAGING_REPORT_TYPE));
        assert!(markdown.contains("cargo publish --dry-run --locked -p gpui-design"));
        assert!(markdown.contains("gpui-profiler"));
        assert!(markdown.contains("support-tooling"));
        assert!(markdown.contains("platform-delivery"));
        assert!(markdown.contains("vendored_patch_manifest"));
    }
}
