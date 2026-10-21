#!/usr/bin/env python3
"""Fail QA when workspace documentation or vendored governance drifts."""

from __future__ import annotations

import datetime as dt
import os
import pathlib
import re
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]

THIRD_PARTIES_GIT_URL = "https://github.com/pierreaubert/sotf-3rdparties"
THIRD_PARTIES_PROBE = "gpui/crates/gpui/VENDORED.md"


def _cargo_lock_third_parties_rev() -> str | None:
    lock = ROOT / "Cargo.lock"
    if not lock.is_file():
        return None
    prefix = f'source = "git+{THIRD_PARTIES_GIT_URL}'
    for line in lock.read_text().splitlines():
        stripped = line.strip()
        if stripped.startswith(prefix):
            rev = stripped.rsplit("#", 1)[-1].rstrip('"')
            if rev:
                return rev
    return None


def _third_parties_dir() -> tuple[pathlib.Path, bool] | None:
    """Locate the sotf-3rdparties tree; the flag marks cargo snapshots.

    Mirrors ``sotf_third_parties_dir()`` in gpui-release-gates: the
    ``SOTF_3RDPARTIES_DIR`` override wins, then a sibling consolidated
    checkout (monorepo layout), then the cargo git checkout at the
    Cargo.lock-pinned revision. Cargo checkouts are immutable snapshots of
    another repository, so their embedded ``Last reviewed`` dates cannot be
    refreshed from here; the vendored-patch manifest in this repository
    owns the review cadence for them.
    """
    override = os.environ.get("SOTF_3RDPARTIES_DIR")
    if override:
        return pathlib.Path(override), False
    sibling = ROOT / "../sotf-3rdparties"
    if (sibling / THIRD_PARTIES_PROBE).is_file():
        return sibling, False
    rev = _cargo_lock_third_parties_rev()
    if rev is None:
        return None
    homes: list[pathlib.Path] = []
    if cargo_home := os.environ.get("CARGO_HOME"):
        homes.append(pathlib.Path(cargo_home))
    if home := os.environ.get("HOME"):
        homes.append(pathlib.Path(home) / ".cargo")
    for home in homes:
        checkouts = home / "git/checkouts"
        if not checkouts.is_dir():
            continue
        for checkout in sorted(checkouts.glob("sotf-3rdparties-*")):
            for revision in sorted(checkout.iterdir()):
                if rev.startswith(revision.name) and (
                    revision / THIRD_PARTIES_PROBE
                ).is_file():
                    return revision, True
    return None


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
        "../sotf-3rdparties/gpui/crates/gpui_web/src/platform.rs",
        "../sotf-3rdparties/gpui/crates/gpui/src/svg_renderer.rs",
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

    third_parties = _third_parties_dir()
    gpui_dependency = cargo["workspace"]["dependencies"]["gpui"]
    gpui_revision = gpui_dependency.get("tag")
    snapshot_dir: pathlib.Path | None = None
    if gpui_path := gpui_dependency.get("path"):
        snapshot_dir = ROOT / gpui_path
    elif "git" in gpui_dependency and third_parties is not None:
        # Git-sourced snapshot: the resolved sotf-3rdparties tree carries
        # the VENDORED.md provenance record with the upstream base ref.
        snapshot_dir = third_parties[0] / "gpui/crates/gpui"
    if gpui_revision is None and snapshot_dir is not None:
        vendored = snapshot_dir / "VENDORED.md"
        if vendored.is_file():
            revision = re.search(
                r"^- Base ref:\s*(\S+)\s*$", vendored.read_text(), re.MULTILINE
            )
            if revision:
                gpui_revision = revision.group(1)

    if not gpui_revision or gpui_revision not in readme:
        errors.append("README GPUI revision does not match Cargo.toml")

    # The manifest migrated from gpui-toolkit to gpui-release-gates (Sep 2026);
    # accept either location so the policy tracks the code, not history.
    manifest_path = ROOT / "crates/gpui-release-gates/src/vendored_patches.rs"
    if not manifest_path.is_file():
        manifest_path = ROOT / "crates/gpui-toolkit/src/vendored_patches.rs"
    manifest = manifest_path.read_text()
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
    if third_parties is None:
        errors.append(
            "sotf-3rdparties checkout not found (set SOTF_3RDPARTIES_DIR "
            "or build the workspace once so cargo materializes it)"
        )
    else:
        base, is_snapshot = third_parties
        for path in paths:
            directory = base / path
            if not directory.is_dir():
                errors.append(f"vendored path does not exist: {path}")
                continue
            vendoring = directory / "VENDORING.md"
            vendored = directory / "VENDORED.md"
            if vendoring.is_file():
                # Hand-written governance doc: review-date and heading checks.
                # The overdue clock only applies to writable checkouts; for
                # cargo snapshots the manifest cadence above is authoritative.
                text = vendoring.read_text()
                match = re.search(r"Last reviewed:\s*(\d{4}-\d{2}-\d{2})", text)
                if not match:
                    errors.append(f"vendoring review date missing: {path}")
                elif not is_snapshot and (
                    today - dt.date.fromisoformat(match.group(1))
                ).days > 90:
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
