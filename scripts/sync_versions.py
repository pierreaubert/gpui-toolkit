#!/usr/bin/env python3
"""Synchronize the Rust workspace and Python package versions.

Cargo.toml is the canonical version source.  The other version declarations
are deliberately kept in sync because the Python wheel is built from the
workspace root while the runtime crate also has package metadata.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VERSION_RE = re.compile(
    r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)"
    r"(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$"
)

WORKSPACE_VERSION = re.compile(
    r"(?ms)(^\[workspace\.package\].*?^version\s*=\s*\")([^\"]+)(\")"
)
PROJECT_VERSION = re.compile(
    r"(?ms)(^\[project\].*?^version\s*=\s*\")([^\"]+)(\")"
)
PYTHON_VERSION = re.compile(r'(?m)^(__version__\s*=\s*")([^"]+)(")')
METADATA_VERSION = re.compile(r"(?m)^(Version:\s*)([^\r\n]+)(\r?$)")
LOCKFILE_VERSION = re.compile(
    r'(?ms)(^\[\[package\]\]\nname = "(?:gpui-python-runtime|gpui-toolkit)"\nversion = ")([^"]+)(")'
)

FILES = (
    ROOT / "Cargo.toml",
    ROOT / "pyproject.toml",
    ROOT / "crates/gpui-python-runtime/pyproject.toml",
    ROOT / "crates/gpui-python-runtime/python/gpui_toolkit/__init__.py",
)
GENERATED_METADATA = (
    ROOT / "crates/gpui-python-runtime/python/gpui_toolkit.egg-info/PKG-INFO"
)


def version_files() -> tuple[tuple[Path, re.Pattern[str]], ...]:
    """Return all version declarations, including generated metadata if present."""
    entries = (
        (FILES[0], WORKSPACE_VERSION),
        (FILES[1], PROJECT_VERSION),
        (FILES[2], PROJECT_VERSION),
        (FILES[3], PYTHON_VERSION),
    )
    lockfile = FILES[0].parent / "Cargo.lock"
    if lockfile.is_file():
        entries += ((lockfile, LOCKFILE_VERSION),)
    if GENERATED_METADATA.is_file():
        entries += ((GENERATED_METADATA, METADATA_VERSION),)
    return entries


def _replace_version(path: Path, pattern: re.Pattern[str], version: str) -> str:
    text = path.read_text(encoding="utf-8")
    updated, count = pattern.subn(rf"\g<1>{version}\g<3>", text)
    if count == 0:
        raise ValueError(f"could not find a version declaration in {path}")
    return updated


def _read_version(path: Path, pattern: re.Pattern[str]) -> str:
    matches = pattern.findall(path.read_text(encoding="utf-8"))
    if not matches:
        raise ValueError(f"could not find version declaration in {path}")
    versions = {match[1] for match in matches}
    if len(versions) != 1:
        raise ValueError(f"multiple version declarations disagree in {path}: {sorted(versions)}")
    return versions.pop()


def versions() -> dict[Path, str]:
    return {path: _read_version(path, pattern) for path, pattern in version_files()}


def validate_version(version: str) -> None:
    if VERSION_RE.fullmatch(version) is None:
        raise ValueError(f"invalid semantic version: {version!r}")


def check(expected: str | None = None, tag: str | None = None) -> str:
    current = versions()
    unique = set(current.values())
    if len(unique) != 1:
        details = ", ".join(f"{path.relative_to(ROOT)}={value}" for path, value in current.items())
        raise ValueError(f"version mismatch: {details}")

    version = next(iter(unique))
    validate_version(version)
    if expected is not None and version != expected:
        raise ValueError(f"expected version {expected}, found {version}")
    if tag is not None and tag.removeprefix("v") != version:
        raise ValueError(f"tag {tag!r} does not match version {version}")
    return version


def synchronize(version: str) -> None:
    validate_version(version)
    for path, pattern in version_files():
        path.write_text(_replace_version(path, pattern, version), encoding="utf-8")
    check(expected=version)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version", nargs="?", help="version to write to all package manifests")
    parser.add_argument("--check", action="store_true", help="verify all manifests are synchronized")
    parser.add_argument("--tag", help="also verify a release tag such as v0.9.15")
    args = parser.parse_args()

    if args.check and args.version is not None:
        parser.error("VERSION cannot be combined with --check")
    if not args.check and args.version is None:
        parser.error("provide VERSION or use --check")
    if args.tag is not None and not args.check:
        parser.error("--tag can only be used with --check")

    try:
        if args.check:
            version = check(tag=args.tag)
            print(f"versions synchronized at {version}")
        else:
            synchronize(args.version)
            print(f"synchronized versions at {args.version}")
    except (OSError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
