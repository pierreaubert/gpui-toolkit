"""Host-backed design-token import, export, and validation declarations."""
from __future__ import annotations
from dataclasses import dataclass
from enum import Enum
from typing import Any

class DesignTokenFormat(str, Enum):
    STYLE_DICTIONARY_JSON = "style_dictionary_json"

@dataclass(frozen=True)
class DesignTokenOperation:
    operation: str
    format: DesignTokenFormat = DesignTokenFormat.STYLE_DICTIONARY_JSON
    input: str | None = None
    render_markdown: bool = False
    def __post_init__(self) -> None:
        if self.operation not in {"import", "export", "validate"}:
            raise ValueError("operation must be import, export, or validate")
        if self.operation in {"import", "validate"} and self.input is None:
            raise ValueError(f"{self.operation} requires token input")
    def to_spec(self) -> dict[str, Any]:
        return {"operation": self.operation, "format": self.format.value, "input": self.input, "render_markdown": self.render_markdown}

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
