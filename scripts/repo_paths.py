#!/usr/bin/env python3
"""Repository path containment shared by the visual QA lanes.

The repo's ``target/`` directory may be a symlink to a shared build cache:
managed build wrappers relocate Cargo output out of the source tree while
keeping ``target/`` as a pointer. Captures beneath it are logically
in-repo, so containment checks accept the resolved ``target/`` directory
alongside the source tree itself. Manifest path tricks (absolute paths,
``..``) are still rejected by the callers before joining, and symlinks to
genuinely external locations keep failing.
"""

from __future__ import annotations

from pathlib import Path


def within_repo(root: Path, resolved: Path) -> bool:
    """Return True when the resolved path lives under root or root/target."""
    root_resolved = root.resolve()
    try:
        resolved.relative_to(root_resolved)
        return True
    except ValueError:
        pass
    try:
        resolved.relative_to((root / "target").resolve())
        return True
    except ValueError:
        return False


def display_path(path: Path, repo_root: Path) -> str:
    """Render a path repo-relative for reports, tolerating symlinked target/.

    Paths under the resolved ``target/`` directory render with the logical
    ``target/`` prefix; anything else must be under the source tree, exactly
    as before (a ValueError escapes otherwise).
    """
    resolved = path.resolve()
    root_resolved = repo_root.resolve()
    try:
        return resolved.relative_to(root_resolved).as_posix()
    except ValueError:
        pass
    return (
        Path("target") / resolved.relative_to((repo_root / "target").resolve())
    ).as_posix()
