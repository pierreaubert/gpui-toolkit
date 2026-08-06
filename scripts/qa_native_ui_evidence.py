#!/usr/bin/env python3
"""Validate and annotate native GPUI smoke-test pixel evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


MIN_UNIQUE_COLORS = 16


class EvidenceError(ValueError):
    """Raised when native UI evidence does not satisfy the release contract."""


def _require(report: dict[str, Any], key: str, expected: Any) -> None:
    actual = report.get(key)
    if actual != expected:
        raise EvidenceError(f"{key} must be {expected!r}, got {actual!r}")


def validate_smoke_report(report: dict[str, Any], platform: str | None = None) -> None:
    _require(report, "schema_version", 3)
    _require(report, "report_type", "gpui-native-smoke")
    _require(report, "crate", "gpui-builder")
    _require(report, "window_opened", True)
    _require(report, "render_invoked", True)
    _require(report, "state_transition", "collapse-sidebar")
    _require(report, "state_transition_verified", True)
    _require(
        report,
        "interaction_scope",
        ["window-open", "render", "collapse-sidebar"],
    )
    if platform is not None:
        _require(report, "platform", platform)
    render_count = report.get("render_count")
    if not isinstance(render_count, int) or render_count < 2:
        raise EvidenceError(f"render_count must be an integer >= 2, got {render_count!r}")


def annotate_pixel_evidence(
    artifact: Path,
    screenshot: Path,
    unique_colors: int,
    capture_transport: str,
    platform: str | None = None,
) -> dict[str, Any]:
    if unique_colors < MIN_UNIQUE_COLORS:
        raise EvidenceError(
            "native UI screenshot is blank or near-uniform: "
            f"{unique_colors} colors (minimum {MIN_UNIQUE_COLORS})"
        )
    if not screenshot.is_file() or screenshot.stat().st_size == 0:
        raise EvidenceError(f"pixel artifact is missing or empty: {screenshot}")

    report = json.loads(artifact.read_text(encoding="utf-8"))
    validate_smoke_report(report, platform)
    report["pixel_capture"] = True
    report["pixel_artifact"] = screenshot.name
    report["pixel_unique_colors"] = unique_colors
    report["pixel_capture_transport"] = capture_transport

    temporary = artifact.with_suffix(f"{artifact.suffix}.tmp")
    temporary.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    temporary.replace(artifact)
    return report


def verify_pixel_evidence(
    artifact: Path,
    screenshot: Path,
    platform: str | None = None,
) -> dict[str, Any]:
    if not screenshot.is_file() or screenshot.stat().st_size == 0:
        raise EvidenceError(f"pixel artifact is missing or empty: {screenshot}")
    report = json.loads(artifact.read_text(encoding="utf-8"))
    validate_smoke_report(report, platform)
    _require(report, "pixel_capture", True)
    _require(report, "pixel_artifact", screenshot.name)
    unique_colors = report.get("pixel_unique_colors")
    if not isinstance(unique_colors, int) or unique_colors < MIN_UNIQUE_COLORS:
        raise EvidenceError(
            f"pixel_unique_colors must be an integer >= {MIN_UNIQUE_COLORS}, "
            f"got {unique_colors!r}"
        )
    transport = report.get("pixel_capture_transport")
    if not isinstance(transport, str) or not transport.strip():
        raise EvidenceError("pixel_capture_transport must be a non-empty string")
    return report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", required=True, type=Path)
    parser.add_argument("--screenshot", required=True, type=Path)
    parser.add_argument("--platform")
    parser.add_argument("--unique-colors", type=int)
    parser.add_argument("--capture-transport")
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()

    if args.verify:
        if args.unique_colors is not None or args.capture_transport is not None:
            parser.error("--verify cannot be combined with annotation options")
        verify_pixel_evidence(args.artifact, args.screenshot, args.platform)
        return 0

    if args.unique_colors is None or args.capture_transport is None:
        parser.error("annotation requires --unique-colors and --capture-transport")
    annotate_pixel_evidence(
        args.artifact,
        args.screenshot,
        args.unique_colors,
        args.capture_transport,
        args.platform,
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (EvidenceError, json.JSONDecodeError, OSError) as error:
        raise SystemExit(f"native UI evidence error: {error}") from error
