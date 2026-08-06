"""Non-destructive host command declarations for ``gpui-scaffolder``."""
from __future__ import annotations
from dataclasses import dataclass
from pathlib import PurePath
from typing import Any

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

@dataclass(frozen=True)
class ScaffoldedApp:
    app_dir: str
    package_name: str
    title: str
    def __post_init__(self) -> None:
        if not all((self.app_dir, self.package_name, self.title)): raise ValueError("invalid scaffold result")
