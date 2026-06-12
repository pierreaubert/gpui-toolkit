"""Application declarations for Python-authored GPUI apps."""

from __future__ import annotations

import json
import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Sequence


def _spec(value: Any) -> Any:
    if hasattr(value, "to_spec"):
        return value.to_spec()
    return value


@dataclass(frozen=True)
class Section:
    id: str
    label: str
    content: Any

    def to_spec(self) -> dict[str, Any]:
        return {"id": self.id, "label": self.label, "content": _spec(self.content)}


@dataclass(frozen=True)
class App:
    title: str = "GPUI Python App"
    sections: Sequence[Section] = field(default_factory=list)
    width: float = 1240.0
    height: float = 820.0
    sidebar_title: str = "Python UI"
    sidebar_subtitle: str = "Python declarations, Rust renderers"

    def to_spec(self) -> dict[str, Any]:
        if not self.sections:
            raise ValueError("App requires at least one section")
        return {
            "title": self.title,
            "width": float(self.width),
            "height": float(self.height),
            "sidebar_title": self.sidebar_title,
            "sidebar_subtitle": self.sidebar_subtitle,
            "sections": [section.to_spec() for section in self.sections],
        }

    def run(self) -> None:
        if os.environ.get("GPUI_TOOLKIT_DUMP_IR") == "1":
            print(json.dumps(self.to_spec()))
            return

        script = Path(sys.argv[0]).resolve()
        repo_root = _find_repo_root(script)
        command = [
            "cargo",
            "run",
            "-p",
            "gpui-python-runtime",
            "--features",
            "showcase",
            "--bin",
            "gpui-python-showcase",
            "--",
            str(script),
        ]
        raise SystemExit(subprocess.call(command, cwd=repo_root))


def section(id: str, label: str, content: Any) -> Section:
    return Section(id=id, label=label, content=content)


def _find_repo_root(start: Path) -> Path:
    for candidate in [start, *start.parents]:
        if (candidate / "Cargo.toml").exists() and (candidate / "crates").is_dir():
            return candidate
    return Path.cwd()
