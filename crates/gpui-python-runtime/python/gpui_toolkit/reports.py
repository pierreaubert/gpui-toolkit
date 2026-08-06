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
