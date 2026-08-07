#!/usr/bin/env python3
"""Reject first-party unsafe Rust outside explicit native FFI boundaries."""

from __future__ import annotations

import pathlib
import re


ROOT = pathlib.Path(__file__).resolve().parents[1]

# These crates translate between Rust and native platform ABIs. Their
# callbacks, raw handles, and Objective-C/JNI state share one FFI ownership
# contract, so the policy treats each backend crate as a boundary.
FFI_BOUNDARY_DIRS = (
    "crates/gpui-android/",
    "crates/gpui-au/",
    "crates/gpui-ios/",
    "crates/gpui-showcase/android/",
    "crates/gpui-showcase/ios/",
    "crates/gpui-showcase/tvos/",
)

# This safe crate contains FFI attributes only as generated Rust source text.
# Its crate root also uses `forbid(unsafe_code)`, so executable scaffolder code
# cannot use this text-level exemption.
GENERATED_FFI_TEMPLATE_FILES = {
    "crates/gpui-scaffolder/src/lib.rs",
}

VENDORED_DIRS = (
    "crates/3rdparties/",
)

# Match Rust unsafe constructs, including Rust 2024 unsafe attributes, while
# allowing documentation to discuss the word "unsafe".
UNSAFE_CONSTRUCT = re.compile(
    r"\bunsafe\s*(?:\(|\{|fn\b|impl\b|trait\b|extern\b)",
    re.MULTILINE,
)


def _is_allowed(relative_path: str) -> bool:
    return (
        relative_path in GENERATED_FFI_TEMPLATE_FILES
        or relative_path.startswith(FFI_BOUNDARY_DIRS)
        or relative_path.startswith(VENDORED_DIRS)
    )


def check(root: pathlib.Path = ROOT) -> list[str]:
    """Return unsafe-policy violations below ``root``."""
    violations: list[str] = []
    crates = root / "crates"
    if not crates.is_dir():
        return [f"{crates}: missing crates directory"]

    for source in sorted(crates.rglob("*.rs")):
        relative = source.relative_to(root).as_posix()
        if _is_allowed(relative):
            continue

        text = source.read_text(encoding="utf-8")
        for match in UNSAFE_CONSTRUCT.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            violations.append(
                f"{relative}:{line}: unsafe Rust is allowed only at an "
                "explicit FFI boundary"
            )

    return violations


if __name__ == "__main__":
    errors = check()
    if errors:
        raise SystemExit("\n".join(errors))
