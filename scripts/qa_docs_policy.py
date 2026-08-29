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

    governance_markers = {
        "CONTRIBUTING.md": ("# Contributing", "just qa", "CHANGELOG.md"),
        "SECURITY.md": ("# Security Policy", "Reporting a vulnerability", "private"),
        "SUPPORT.md": ("# Support Policy", "GitHub Issues", "SECURITY.md"),
        "CODE_OF_CONDUCT.md": ("# Code of Conduct", "harassment-free", "Report conduct"),
        "RELEASE.md": ("# Release Policy", "crates.io wave 1", "source beta", "MSRV"),
    }
    for relative, markers in governance_markers.items():
        path = ROOT / relative
        if not path.is_file():
            errors.append(f"required governance document missing: {relative}")
            continue
        text = path.read_text()
        for marker in markers:
            if marker not in text:
                errors.append(f"{relative} lacks required marker: {marker}")

    changelog = (ROOT / "CHANGELOG.md").read_text()
    if "## Unreleased" not in changelog:
        errors.append("CHANGELOG.md lacks an Unreleased section")

    asset_readme = ROOT / "crates/assets/README.md"
    asset_markers = (
        "shared asset bundle, not a Cargo crate",
        "include_bytes!",
        "crates/3rdparties/gpui_web/src/platform.rs",
        "crates/3rdparties/gpui/src/svg_renderer.rs",
        "Font License 1.1",
    )
    if not asset_readme.is_file():
        errors.append("crates/assets/README.md is missing")
    else:
        asset_text = asset_readme.read_text()
        for marker in asset_markers:
            if marker not in asset_text:
                errors.append(f"crates/assets/README.md lacks required marker: {marker}")

    public_msrv_crates = (
        "gpui-design",
        "gpui-pretext",
        "gpui-profiler",
        "gpui-ui-kit-macros",
    )
    for crate in public_msrv_crates:
        manifest_path = ROOT / "crates" / crate / "Cargo.toml"
        manifest = tomllib.loads(manifest_path.read_text())
        if manifest["package"].get("rust-version") != "1.89":
            errors.append(f"{crate} must declare rust-version = 1.89")

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

    gpui_dependency = cargo["workspace"]["dependencies"]["gpui"]
    gpui_revision = gpui_dependency.get("tag")
    if gpui_revision is None and (gpui_path := gpui_dependency.get("path")):
        vendored = ROOT / gpui_path / "VENDORED.md"
        if vendored.is_file():
            revision = re.search(
                r"^- Base ref:\s*(\S+)\s*$", vendored.read_text(), re.MULTILINE
            )
            if revision:
                gpui_revision = revision.group(1)

    if not gpui_revision or gpui_revision not in readme:
        errors.append("README GPUI revision does not match Cargo.toml")

    manifest = (ROOT / "crates/gpui-toolkit/src/vendored_patches.rs").read_text()
    paths = re.findall(r'local_path:\s*"([^"]+)"', manifest)
    today = dt.date.today()
    for patch in manifest.split("\n    VendoredPatch {")[1:]:
        name = re.search(r'name:\s*"([^"]+)"', patch)
        last_reviewed = re.search(r'last_reviewed:\s*"(\d{4}-\d{2}-\d{2})"', patch)
        cadence = re.search(r'review_cadence_days:\s*(\d+)', patch)
        if not (name and last_reviewed and cadence):
            errors.append("vendored patch maintenance metadata is incomplete")
            continue
        reviewed_on = dt.date.fromisoformat(last_reviewed.group(1))
        cadence_days = int(cadence.group(1))
        if (today - reviewed_on).days > cadence_days:
            errors.append(
                f"vendored patch review overdue: {name.group(1)} "
                f"({(today - reviewed_on).days} days > {cadence_days} days)"
            )
    for path in paths:
        directory = ROOT / path
        if not directory.is_dir():
            errors.append(f"vendored path does not exist: {path}")
            continue
        vendoring = directory / "VENDORING.md"
        vendored = directory / "VENDORED.md"
        if vendoring.is_file():
            # Hand-written governance doc: full review-date and heading checks.
            text = vendoring.read_text()
            match = re.search(r"Last reviewed:\s*(\d{4}-\d{2}-\d{2})", text)
            if not match:
                errors.append(f"vendoring review date missing: {path}")
            elif (today - dt.date.fromisoformat(match.group(1))).days > 90:
                errors.append(f"vendoring review overdue (>90 days): {path}")
            for heading in ("## Upstream", "## Why Vendored", "## Verification"):
                if heading not in text:
                    errors.append(f"{path}/VENDORING.md lacks {heading}")
        elif vendored.is_file():
            # Script-generated provenance record (import_gpui_upstream.py).
            text = vendored.read_text()
            for marker in ("- Upstream:", "- Base ref:"):
                if marker not in text:
                    errors.append(f"{path}/VENDORED.md lacks '{marker}' provenance")
        else:
            errors.append(f"vendored path lacks provenance doc (VENDORING.md or VENDORED.md): {path}")

    return errors


if __name__ == "__main__":
    failures = check()
    if failures:
        raise SystemExit("\n".join(f"- {failure}" for failure in failures))
    print("documentation and vendored governance policy passed")
