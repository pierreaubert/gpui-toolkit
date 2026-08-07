"""Host-backed design-token import, export, and validation declarations."""
from __future__ import annotations
from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING, Any, Mapping

from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext

class DesignTokenFormat(str, Enum):
    STYLE_DICTIONARY_JSON = "style_dictionary_json"


class DesignTokenOperationKind(str, Enum):
    IMPORT = "import"
    EXPORT = "export"
    VALIDATE = "validate"
    HANDOFF = "handoff"

@dataclass(frozen=True)
class DesignTokenOperation:
    operation: DesignTokenOperationKind | str
    format: DesignTokenFormat = DesignTokenFormat.STYLE_DICTIONARY_JSON
    input: str | None = None
    render_markdown: bool = False
    def __post_init__(self) -> None:
        operation = DesignTokenOperationKind(self.operation)
        object.__setattr__(self, "operation", operation)
        if operation in {DesignTokenOperationKind.IMPORT, DesignTokenOperationKind.VALIDATE} and self.input is None:
            raise ValueError(f"{self.operation} requires token input")
        if operation in {DesignTokenOperationKind.EXPORT, DesignTokenOperationKind.HANDOFF} and self.input is not None:
            raise ValueError(f"{self.operation.value} does not accept token input")
    def to_spec(self) -> dict[str, Any]:
        return {"operation": self.operation.value, "format": self.format.value, "input": self.input, "render_markdown": self.render_markdown}
    def send(self, context: "SessionContext", request_id: str) -> None:
        """Run this operation in the native ``gpui-design-tools`` crate."""
        context.command(request_id, "design.tokens", **self.to_spec())

@dataclass(frozen=True)
class DesignTokenValidationReport:
    schema_version: int
    report_type: str
    passed: bool
    findings: tuple[str, ...]
    preset_count: int
    token_count: int
    conformance_markdown: str
    def __post_init__(self) -> None:
        if self.schema_version != 1 or self.preset_count < 0 or self.token_count < 0:
            raise ValueError("invalid native design-token report")
        if self.passed != (not self.findings):
            raise ValueError("passed must agree with validation findings")

    @classmethod
    def from_command(cls, result: CommandResult) -> "DesignTokenValidationReport":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"design-token command {result.status.value}")
        report = result.data.get("report")
        if not isinstance(report, Mapping):
            raise ValueError("native validation result does not contain a report")
        findings = report.get("findings", ())
        return cls(
            int(report.get("schema_version", 0)), str(report.get("report_type", "")),
            bool(report.get("passed")), tuple(str(item) for item in findings),
            int(report.get("preset_count", -1)), int(report.get("token_count", -1)),
            str(report.get("conformance_markdown", "")),
        )


@dataclass(frozen=True)
class ImportedDesignTokens:
    preset_count: int
    token_count: int
    raw: Mapping[str, Any]

    @classmethod
    def from_command(cls, result: CommandResult) -> "ImportedDesignTokens":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"design-token command {result.status.value}")
        raw = result.data.get("raw")
        if not isinstance(raw, Mapping):
            raise ValueError("native import result does not contain token data")
        return cls(int(result.data.get("preset_count", -1)), int(result.data.get("token_count", -1)), raw)


@dataclass(frozen=True)
class DesignToolingHandoffReport:
    schema_version: int
    report_type: str
    crate_name: str
    crate_version: str
    items: tuple[Mapping[str, Any], ...]

    @classmethod
    def from_command(cls, result: CommandResult) -> "DesignToolingHandoffReport":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"design-token command {result.status.value}")
        report = result.data.get("report")
        if not isinstance(report, Mapping) or not isinstance(report.get("items"), list):
            raise ValueError("native handoff result does not contain a report")
        return cls(
            int(report.get("schema_version", 0)), str(report.get("report_type", "")),
            str(report.get("crate_name", "")), str(report.get("crate_version", "")),
            tuple(item for item in report["items"] if isinstance(item, Mapping)),
        )
