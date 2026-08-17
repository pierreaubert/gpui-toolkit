#!/usr/bin/env python3
"""Validation helpers for the retained MeshPlot WGPU visual contract."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path, PureWindowsPath
from typing import Any


SCHEMA_VERSION = 1
RENDERER = "wgpu-headless"
WIDTH = 256
HEIGHT = 192
CASE_IDS = ("mesh", "smooth", "cell", "wireframe", "isoline", "revolve")
CHECKSUM_RE = re.compile(r"^fnv1a64:[0-9a-f]{16}$")


class WgpuManifestError(RuntimeError):
    """Raised when a WGPU visual manifest is not release-safe."""


def _read_object(path: Path, description: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise WgpuManifestError(f"invalid {description} {path}: {error}") from error
    if not isinstance(value, dict):
        raise WgpuManifestError(f"invalid {description} {path}: expected a JSON object")
    return value


def _relative_image_path(manifest_path: Path, repo_root: Path, path_text: str) -> Path:
    image_path = Path(path_text)
    # Manifest paths are deliberately POSIX-relative. Rejecting backslashes
    # keeps validation consistent if a report is inspected on another host
    # where a Windows separator could otherwise change the path meaning.
    if (
        image_path.is_absolute()
        or PureWindowsPath(path_text).drive
        or not path_text
        or not image_path.parts
        or "\\" in path_text
        or ".." in image_path.parts
    ):
        raise WgpuManifestError(
            f"WGPU visual manifest contains an unsafe image path: {path_text!r}"
        )
    # Captures produced by the current example use a basename. Accept the
    # earlier repository-relative form as well so old development artifacts
    # fail only on their content, not on path layout.
    if len(image_path.parts) == 1:
        return manifest_path.parent / image_path
    return repo_root / image_path


def validate_manifest(
    path: Path,
    *,
    repo_root: Path,
    require_images: bool,
    allow_skipped: bool,
) -> dict[str, Any]:
    """Validate one actual or baseline manifest and return its JSON object."""

    manifest = _read_object(path, "WGPU visual manifest")
    if manifest.get("schema_version") != SCHEMA_VERSION:
        raise WgpuManifestError(f"WGPU visual manifest schema mismatch: {path}")
    if manifest.get("renderer") != RENDERER:
        raise WgpuManifestError(f"WGPU visual manifest renderer mismatch: {path}")

    status = manifest.get("status", "captured")
    if status == "skipped":
        if not allow_skipped:
            raise WgpuManifestError(f"WGPU visual manifest is skipped: {path}")
        if not isinstance(manifest.get("reason"), str) or not manifest["reason"].strip():
            raise WgpuManifestError("skipped WGPU visual manifest must include a reason")
        if manifest.get("cases") not in ([], None):
            raise WgpuManifestError("skipped WGPU visual manifest must not contain captures")
        return manifest
    if status != "captured":
        raise WgpuManifestError(f"unknown WGPU visual manifest status: {status!r}")

    if manifest.get("width") != WIDTH or manifest.get("height") != HEIGHT:
        raise WgpuManifestError(
            f"WGPU visual manifest must describe {WIDTH}x{HEIGHT} captures"
        )
    cases = manifest.get("cases")
    if not isinstance(cases, list) or len(cases) != len(CASE_IDS):
        raise WgpuManifestError(
            f"WGPU visual manifest must contain exactly {len(CASE_IDS)} cases"
        )

    expected_ids = set(CASE_IDS)
    seen_ids: set[str] = set()
    root = repo_root.resolve()
    for case in cases:
        if not isinstance(case, dict):
            raise WgpuManifestError("WGPU visual manifest contains a malformed case")
        case_id = case.get("id")
        if not isinstance(case_id, str) or case_id not in expected_ids or case_id in seen_ids:
            raise WgpuManifestError("WGPU visual manifest case IDs must be unique and canonical")
        seen_ids.add(case_id)
        if not isinstance(case.get("description"), str) or not case["description"].strip():
            raise WgpuManifestError(f"WGPU visual case {case_id} has no description")
        path_text = case.get("path")
        if not isinstance(path_text, str):
            raise WgpuManifestError(f"WGPU visual case {case_id} has no image path")
        image = _relative_image_path(path, repo_root, path_text)
        try:
            image.resolve().relative_to(root)
        except ValueError as error:
            raise WgpuManifestError(
                f"WGPU visual case {case_id} image escapes the repository"
            ) from error
        opaque_pixels = case.get("opaque_pixels")
        if (
            not isinstance(opaque_pixels, int)
            or isinstance(opaque_pixels, bool)
            or opaque_pixels < 0
            or opaque_pixels > WIDTH * HEIGHT
        ):
            raise WgpuManifestError(f"WGPU visual case {case_id} has invalid opaque_pixels")
        checksum = case.get("rgba_checksum")
        if not isinstance(checksum, str) or CHECKSUM_RE.fullmatch(checksum) is None:
            raise WgpuManifestError(f"WGPU visual case {case_id} has an invalid RGBA checksum")
        if require_images:
            if not image.is_file() or image.stat().st_size == 0:
                raise WgpuManifestError(f"missing WGPU visual capture: {image}")

    if seen_ids != expected_ids:
        raise WgpuManifestError("WGPU visual manifest case IDs do not match the canonical set")
    return manifest


def compare_manifests(actual: dict[str, Any], baseline: dict[str, Any]) -> None:
    """Compare release-significant WGPU fields between actual and baseline."""

    if actual.get("status", "captured") != "captured":
        raise WgpuManifestError("cannot compare a skipped WGPU visual capture")
    if baseline.get("status", "captured") != "captured":
        raise WgpuManifestError("WGPU visual baseline must contain captured cases")
    if actual.get("width") != baseline.get("width") or actual.get("height") != baseline.get("height"):
        raise WgpuManifestError("WGPU visual baseline dimensions do not match captures")

    actual_cases = {case["id"]: case for case in actual["cases"]}
    baseline_cases = {case["id"]: case for case in baseline["cases"]}
    if set(actual_cases) != set(baseline_cases):
        raise WgpuManifestError("WGPU visual baseline case IDs do not match current captures")
    for case_id in CASE_IDS:
        observed = actual_cases[case_id]
        expected = baseline_cases[case_id]
        for key in ("opaque_pixels", "rgba_checksum"):
            if observed.get(key) != expected.get(key):
                raise WgpuManifestError(
                    f"WGPU visual mismatch for {case_id}: {key} "
                    f"expected {expected.get(key)!r}, got {observed.get(key)!r}"
                )


def write_skip(path: Path, reason: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(
            {
                "schema_version": SCHEMA_VERSION,
                "renderer": RENDERER,
                "status": "skipped",
                "reason": reason,
                "cases": [],
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--actual", type=Path)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--required", action="store_true")
    parser.add_argument("--write-skip", type=Path)
    parser.add_argument("--reason", default="no usable WGPU adapter")
    args = parser.parse_args(argv)

    try:
        if args.write_skip is not None:
            write_skip(args.write_skip, args.reason)
            return 0
        if args.actual is None:
            parser.error("--actual is required unless --write-skip is used")
        actual = validate_manifest(
            args.actual,
            repo_root=args.repo_root,
            require_images=True,
            allow_skipped=not args.required,
        )
        if actual.get("status", "captured") == "skipped":
            if args.required:
                raise WgpuManifestError("WGPU visual lane is skipped but is required")
            print("WGPU visual captures skipped; no usable adapter was available")
            return 0
        if args.baseline is None or not args.baseline.is_file():
            if args.required:
                raise WgpuManifestError(
                    f"missing WGPU baseline {args.baseline}; promote from a clean release run"
                )
            print(f"WGPU visual captures produced; baseline not installed: {args.baseline}")
            return 0
        baseline = validate_manifest(
            args.baseline,
            repo_root=args.repo_root,
            require_images=False,
            allow_skipped=False,
        )
        compare_manifests(actual, baseline)
        print(f"WGPU MeshPlot visual baseline passed ({len(CASE_IDS)} cases)")
        return 0
    except WgpuManifestError as error:
        print(f"WGPU visual manifest failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
