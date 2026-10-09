#!/usr/bin/env python3
"""Reject std wall clocks in first-party crates shipped to wasm.

`std::time::Instant` and `std::time::SystemTime` panic on
wasm32-unknown-unknown ("time not implemented on this platform"). A panic in
a paint or frame path traps the frame and wedges the app with per-frame
borrow errors, so wasm-shipped crates must use `web_time` instead (backed by
the JS clock on wasm, std elsewhere).
"""

from __future__ import annotations

import pathlib
import re


ROOT = pathlib.Path(__file__).resolve().parents[1]

# Portable crates whose code ships in the wasm demos. Native platform crates
# (gpui-au, gpui-android, gpui-ios), native harnesses (gpui-component-lab),
# native tools (gpui-scaffolder, gpui-python-runtime), and proc-macro crates
# (which execute on the host) are out of scope: std clocks are correct there.
WASM_CRATES = (
    "crates/gpui-audio-kit/",
    "crates/gpui-builder/",
    "crates/gpui-d3rs/",
    "crates/gpui-design/",
    "crates/gpui-hello-web/",
    "crates/gpui-keybinding/",
    "crates/gpui-miniapp/",
    "crates/gpui-pretext/",
    "crates/gpui-px/",
    "crates/gpui-showcase/",
    "crates/gpui-themes/",
    "crates/gpui-toolkit/",
    "crates/gpui-ui-kit/",
)

# Native-only code inside otherwise wasm-shipped crates: mobile backends and
# host-side tests, benches, and examples (demo bins ship, so `bin/` stays in
# scope).
NATIVE_PATH_FRAGMENTS = (
    "/ios/",
    "/android/",
    "/tvos/",
    "/tests/",
    "/benches/",
    "/examples/",
)

CLOCK_NAMES = ("Instant", "SystemTime", "UNIX_EPOCH")

INLINE_CLOCK_USE = re.compile(r"\bstd::time::(Instant|SystemTime|UNIX_EPOCH)\b")
GROUPED_CLOCK_USE = re.compile(r"use\s+std::time::\s*\{([^}]*)\}", re.DOTALL)


def _in_scope(relative_path: str) -> bool:
    if not relative_path.startswith(WASM_CRATES):
        return False
    if relative_path.endswith("/tests.rs"):
        return False
    return not any(fragment in relative_path for fragment in NATIVE_PATH_FRAGMENTS)


def _strip_line_comments(text: str) -> str:
    # Documentation legitimately names the forbidden clocks; only code counts.
    return "\n".join(line.split("//", 1)[0] for line in text.split("\n"))


def check(root: pathlib.Path = ROOT) -> list[str]:
    """Return wasm-clock violations below ``root``."""
    violations: list[str] = []
    crates = root / "crates"
    if not crates.is_dir():
        return [f"{crates}: missing crates directory"]

    for source in sorted(crates.rglob("*.rs")):
        relative = source.relative_to(root).as_posix()
        if not _in_scope(relative):
            continue

        text = _strip_line_comments(source.read_text(encoding="utf-8"))
        seen: set[tuple[int, str]] = set()
        for match in INLINE_CLOCK_USE.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            seen.add((line, match.group(1)))
        for match in GROUPED_CLOCK_USE.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            for name in CLOCK_NAMES:
                if re.search(rf"\b{name}\b", match.group(1)):
                    seen.add((line, name))
        for line, name in sorted(seen):
            violations.append(
                f"{relative}:{line}: std::time::{name} panics on "
                f"wasm32-unknown-unknown; use web_time::{name} instead"
            )

    return violations


if __name__ == "__main__":
    errors = check()
    if errors:
        raise SystemExit("\n".join(errors))
    print("wasm clock QA passed: no std wall clocks in wasm-shipped crates")
