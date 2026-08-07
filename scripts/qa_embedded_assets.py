#!/usr/bin/env python3
"""Reject literal Rust include assets that a clean source archive cannot build."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path


INCLUDE_RE = re.compile(r'include_(?:str|bytes)!\(\s*"([^"\\]+)"\s*\)')


def tracked_files(root: Path) -> set[str]:
    completed = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    return {
        raw.decode("utf-8")
        for raw in completed.stdout.split(b"\0")
        if raw
    }


def scan(root: Path, tracked: set[str]) -> tuple[int, list[str]]:
    include_count = 0
    errors: list[str] = []
    for source in sorted((root / "crates").rglob("*.rs")):
        text = source.read_text(encoding="utf-8")
        source_relative = source.relative_to(root).as_posix()
        for match in INCLUDE_RE.finditer(text):
            include_count += 1
            literal = match.group(1)
            target = (source.parent / literal).resolve()
            try:
                relative = target.relative_to(root.resolve()).as_posix()
            except ValueError:
                errors.append(f"{source_relative}: include escapes repository: {literal}")
                continue
            if not target.is_file():
                errors.append(f"{source_relative}: included asset is missing: {relative}")
            elif relative not in tracked:
                errors.append(f"{source_relative}: included asset is not tracked by Git: {relative}")
    return include_count, errors


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    try:
        count, errors = scan(root, tracked_files(root))
    except (OSError, UnicodeDecodeError, subprocess.CalledProcessError) as error:
        print(f"embedded asset QA failed: {error}", file=sys.stderr)
        return 1
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(f"Embedded asset QA passed: {count} literal Rust includes are present and tracked")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
