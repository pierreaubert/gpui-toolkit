"""Non-destructive host command declarations for ``gpui-scaffolder``."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import Path, PurePath
from typing import TYPE_CHECKING, Any

from .commands import CommandResult, CommandStatus

if TYPE_CHECKING:
    from .app import SessionContext

@dataclass(frozen=True)
class ScaffoldOptions:
    name: str
    output_dir: str
    force: bool = False
    dry_run: bool = True
    def __post_init__(self) -> None:
        path = PurePath(self.name)
        if not self.name.strip() or len(path.parts) != 1 or path.name in {".", ".."}:
            raise ValueError("app name must be a single directory name")
        if not self.output_dir.strip(): raise ValueError("output_dir is required")
    def to_spec(self) -> dict[str, Any]:
        return {"name": self.name, "output_dir": self.output_dir, "force": self.force, "dry_run": self.dry_run}
    def preview(self, context: "SessionContext", request_id: str) -> None:
        context.command(request_id, "scaffolder.preview", **self.to_spec())
    def write(self, context: "SessionContext", request_id: str) -> None:
        context.command(request_id, "scaffolder.write", **self.to_spec())

@dataclass(frozen=True)
class ScaffoldedApp:
    app_dir: str
    package_name: str
    title: str
    def __post_init__(self) -> None:
        if not all((self.app_dir, self.package_name, self.title)): raise ValueError("invalid scaffold result")

    @classmethod
    def from_command(cls, result: CommandResult) -> "ScaffoldedApp":
        if result.status is not CommandStatus.SUCCEEDED:
            raise RuntimeError(result.error or f"scaffold command {result.status.value}")
        return cls(str(result.data.get("app_dir", "")), str(result.data.get("package_name", "")), str(result.data.get("title", "")))

@dataclass(frozen=True)
class ScaffoldPreview:
    app: ScaffoldedApp
    files: tuple[Path, ...]

    @classmethod
    def from_command(cls, result: CommandResult) -> "ScaffoldPreview":
        app = ScaffoldedApp.from_command(result)
        files = result.data.get("files")
        if not isinstance(files, list):
            raise ValueError("native scaffold preview does not contain generated files")
        return cls(app, tuple(Path(str(path)) for path in files))
