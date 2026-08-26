"""Typed read-only aggregate release and platform QA report snapshots."""
from __future__ import annotations
from dataclasses import dataclass
from enum import Enum

class AggregateFeature(str, Enum):
    CORE="core"; UI="ui"; AUDIO="audio"; CHARTS="charts"; THEMES="themes"; TOOLING="tooling"; PLATFORM="platform"; IOS="ios"
class StabilityLevel(str, Enum):
    RELEASE_CANDIDATE="release-candidate"; BETA="beta"; SUPPORT_TOOLING="support-tooling"; EXPERIMENTAL="experimental"; INTERNAL_ONLY="internal-only"
class PublishDecision(str, Enum):
    PUBLIC_CORE_AFTER_GATES="public-core-after-gates"; BETA_AFTER_GATES="beta-after-gates"; SUPPORT_TOOLING_ONLY="support-tooling-only"; HOLD_FOR_PLATFORM_QA="hold-for-platform-qa"; DO_NOT_PUBLISH="do-not-publish"
class PlatformCapabilityStatus(str, Enum):
    SUPPORTED="supported"; PARTIAL="partial"; NOT_APPLICABLE="not-applicable"; UNVERIFIED="unverified"

@dataclass(frozen=True)
class CrateStability:
    crate_name: str
    aggregate_feature: AggregateFeature
    stability: StabilityLevel
    publish_decision: PublishDecision
    required_gate: str
    note: str
    def __post_init__(self) -> None:
        if not all((self.crate_name, self.required_gate, self.note)): raise ValueError("invalid crate stability entry")

@dataclass(frozen=True)
class PlatformEvidence:
    ci_compile: bool
    runtime_smoke: bool
    visual_diff: bool
    native_accessibility: bool
    performance: bool
    @property
    def complete(self) -> bool: return all(self.__dict__.values())

@dataclass(frozen=True)
class PlatformCapability:
    id: str
    platform: str
    tier: str
    pointer: PlatformCapabilityStatus
    touch: PlatformCapabilityStatus
    text_input: PlatformCapabilityStatus
    accessibility: PlatformCapabilityStatus
    evidence: PlatformEvidence
    blocker: str | None = None

@dataclass(frozen=True)
class PlatformCapabilityMatrix:
    schema_version: int
    report_type: str
    reviewed_on: str
    platforms: tuple[PlatformCapability, ...]
    def __post_init__(self) -> None:
        if self.schema_version != 1 or not self.report_type or not self.reviewed_on: raise ValueError("invalid platform capability matrix")
    @property
    def all_release_ready(self) -> bool:
        return all(item.blocker is None and item.evidence.complete for item in self.platforms)

@dataclass(frozen=True)
class ReleaseQaGate:
    id: str
    status: str
    command: str
    evidence: str
    blocker: str | None = None

@dataclass(frozen=True)
class ReleaseQaMatrix:
    schema_version: int
    report_type: str
    reviewed_on: str
    gates: tuple[ReleaseQaGate, ...]
    @property
    def all_passed(self) -> bool: return all(gate.status == "passed" and gate.blocker is None for gate in self.gates)

@dataclass(frozen=True)
class StabilityReport:
 schema_version: int
 report_type: str
 reviewed_on: str
 crates: tuple[CrateStability, ...]
 @property
 def all_release_ready(self) -> bool:
  return all(entry.publish_decision is not PublishDecision.DO_NOT_PUBLISH for entry in self.crates)

class DependencyHygieneStatus(str, Enum):
 CONFIGURED="configured"; TOOL_AVAILABLE="tool-available"; ACCEPTED_WITH_WARNINGS="accepted-with-warnings"; TOOL_MISSING="tool-missing"; FAILED="failed"; RELEASE_RUN_PENDING="release-run-pending"; MANUAL_REVIEW_REQUIRED="manual-review-required"
 @property
 def release_ready(self) -> bool: return self in {self.CONFIGURED, self.TOOL_AVAILABLE, self.ACCEPTED_WITH_WARNINGS}
class DependencyAdvisoryTriageStatus(str, Enum):
 RELEASE_BLOCKING="release-blocking"; RISK_ACCEPTED="risk-accepted"; WARNING_TRACKED="warning-tracked"
 @property
 def release_blocking(self) -> bool: return self is self.RELEASE_BLOCKING
@dataclass(frozen=True)
class DependencyHygieneCheck:
 id: str; command: str; status: DependencyHygieneStatus; purpose: str; evidence: str; release_requirement: str
@dataclass(frozen=True)
class DependencyAdvisoryTriage:
 advisory_id: str; crate_name: str; affected_versions: str; status: DependencyAdvisoryTriageStatus; affected_path: str; current_decision: str; required_action: str
@dataclass(frozen=True)
class DependencyHygieneReport:
 schema_version: int; report_type: str; reviewed_on: str; cargo_deny_policy_path: str
 checks: tuple[DependencyHygieneCheck, ...]; advisory_triage: tuple[DependencyAdvisoryTriage, ...]
 @property
 def all_release_ready(self) -> bool: return all(check.status.release_ready for check in self.checks)
 @property
 def blocking_checks(self) -> tuple[DependencyHygieneCheck, ...]: return tuple(check for check in self.checks if not check.status.release_ready)
 @property
 def blocking_advisories(self) -> tuple[DependencyAdvisoryTriage, ...]: return tuple(item for item in self.advisory_triage if item.status.release_blocking)

class PublishPlanStatus(str, Enum):
 DRY_RUN_PASSED="dry-run-passed"; BLOCKED_BY_PREDECESSOR="blocked-by-predecessor"; PENDING_DRY_RUN="pending-dry-run"; EXCLUDED="excluded"
 @property
 def release_ready(self) -> bool: return self in {self.DRY_RUN_PASSED, self.EXCLUDED}
@dataclass(frozen=True)
class PublishPlanEntry:
 crate_name: str; order: int; lane: str; command: str; status: PublishPlanStatus; reason: str; evidence: str; release_requirement: str
@dataclass(frozen=True)
class PublishPlan:
 schema_version: int; report_type: str; reviewed_on: str; entries: tuple[PublishPlanEntry, ...]
 @property
 def all_release_ready(self) -> bool: return all(item.status.release_ready for item in self.entries)
 @property
 def blocking_entries(self) -> tuple[PublishPlanEntry, ...]: return tuple(item for item in self.entries if not item.status.release_ready)

class ReleaseNotesStatus(str, Enum):
 READY="ready"; PENDING_ARTIFACT="pending-artifact"; BLOCKED_BY_RELEASE_GATE="blocked-by-release-gate"; EXCLUDED="excluded"
 @property
 def release_ready(self) -> bool: return self in {self.READY, self.EXCLUDED}
class ReleaseNotesArtifactStatus(str, Enum):
 AVAILABLE="available"; PENDING_COMMAND="pending-command"; EXTERNAL_GATE="external-gate"; EXCLUDED="excluded"
 @property
 def blocking(self) -> bool: return self in {self.PENDING_COMMAND, self.EXTERNAL_GATE}
@dataclass(frozen=True)
class ReleaseNotesEntry:
 crate_name: str; lane: str; status: ReleaseNotesStatus; stability: str; platform_support: str; required_sections: str; evidence: str; release_requirement: str
@dataclass(frozen=True)
class ReleaseNotesArtifact:
 id: str; crate_name: str; artifact: str; source: str; status: ReleaseNotesArtifactStatus; evidence: str; release_requirement: str
@dataclass(frozen=True)
class ReleaseNotesReport:
 schema_version: int; report_type: str; reviewed_on: str; entries: tuple[ReleaseNotesEntry, ...]
 @property
 def all_release_ready(self) -> bool: return all(item.status.release_ready for item in self.entries)
@dataclass(frozen=True)
class ReleaseNotesArtifactReport:
 schema_version: int; report_type: str; reviewed_on: str; artifacts: tuple[ReleaseNotesArtifact, ...]
 @property
 def blocking_artifacts(self) -> tuple[ReleaseNotesArtifact, ...]: return tuple(item for item in self.artifacts if item.status.blocking)

class ReleasePackagingStatus(str, Enum):
 PASSED="passed"; BLOCKED="blocked"; DEFERRED="deferred"; PENDING="pending"; EXCLUDED="excluded"; EXTERNAL_GATE="external-gate"
 @property
 def release_ready(self) -> bool: return self in {self.PASSED, self.EXCLUDED}
@dataclass(frozen=True)
class ReleasePackagingEntry:
 id: str; crate_or_lane: str; lane: str; command_or_action: str; status: ReleasePackagingStatus; evidence: str; release_requirement: str
@dataclass(frozen=True)
class ReleasePackagingReport:
 schema_version: int; report_type: str; reviewed_on: str; entries: tuple[ReleasePackagingEntry, ...]
 @property
 def all_release_ready(self) -> bool: return all(item.status.release_ready for item in self.entries)
 @property
 def blocking_entries(self) -> tuple[ReleasePackagingEntry, ...]: return tuple(item for item in self.entries if not item.status.release_ready)

class VendoredPatchStatus(str, Enum): ACTIVE_PATCH="active-patch"; INACTIVE_SNAPSHOT="inactive-snapshot"
class VendoredPatchMaintenance(str, Enum): SCRIPT_VENDORED="script-vendored"; HAND_MAINTAINED="hand-maintained"
@dataclass(frozen=True)
class VendoredPatch:
 name: str; local_path: str; upstream: str; upstream_base: str; owner: str; last_reviewed: str; review_cadence_days: int; removal_condition: str; delta_evidence_command: str; local_ref: str; status: VendoredPatchStatus; maintenance: VendoredPatchMaintenance; reason: str; retained_changes: tuple[str, ...]; verification_gate: str; vendoring_doc: str
@dataclass(frozen=True)
class VendoredPatchManifest:
 schema_version: int; report_type: str; patches: tuple[VendoredPatch, ...]
 @property
 def active_patches(self) -> tuple[VendoredPatch, ...]: return tuple(item for item in self.patches if item.status is VendoredPatchStatus.ACTIVE_PATCH)
