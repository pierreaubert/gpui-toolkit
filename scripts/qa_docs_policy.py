#!/usr/bin/env python3
"""Fail QA when workspace documentation or vendored governance drifts."""

from __future__ import annotations

import datetime as dt
import pathlib
import re
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]


def check() -> list[str]:
    errors: list[str] = []
    cargo_text = (ROOT / "Cargo.toml").read_text()
    cargo = tomllib.loads(cargo_text)
    readme = (ROOT / "README.md").read_text()

    members = {
        member.rstrip("/")
        for member in cargo["workspace"]["members"]
        if "*" not in member and (ROOT / member / "Cargo.toml").is_file()
    }
    documented = {
        path.rstrip("/") for path in re.findall(r"\]\(\./(crates/[^)]+)\)", readme)
    }
    governed_members = {member for member in members if "/3rdparties/" not in member}
    missing = sorted(governed_members - documented)
    if missing:
        errors.append("README workspace inventory missing: " + ", ".join(missing))

    gpui_tag = re.search(
        r'^gpui\s*=.*?tag\s*=\s*"([^"]+)"', cargo_text, re.MULTILINE
    )
    if not gpui_tag or gpui_tag.group(1) not in readme:
        errors.append("README GPUI revision does not match Cargo.toml")

    manifest = (ROOT / "crates/gpui-toolkit/src/vendored_patches.rs").read_text()
    paths = re.findall(r'local_path:\s*"([^"]+)"', manifest)
    today = dt.date.today()
    for path in paths:
        directory = ROOT / path
        doc = directory / "VENDORING.md"
        if not directory.is_dir():
            errors.append(f"vendored path does not exist: {path}")
            continue
        if not doc.is_file():
            errors.append(f"vendored path lacks VENDORING.md: {path}")
            continue
        text = doc.read_text()
        match = re.search(r"Last reviewed:\s*(\d{4}-\d{2}-\d{2})", text)
        if not match:
            errors.append(f"vendoring review date missing: {path}")
        elif (today - dt.date.fromisoformat(match.group(1))).days > 90:
            errors.append(f"vendoring review overdue (>90 days): {path}")
        for heading in ("## Upstream", "## Why Vendored", "## Verification"):
            if heading not in text:
                errors.append(f"{path}/VENDORING.md lacks {heading}")

    return errors


if __name__ == "__main__":
    failures = check()
    if failures:
        raise SystemExit("\n".join(f"- {failure}" for failure in failures))
    print("documentation and vendored governance policy passed")
