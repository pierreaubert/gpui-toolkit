#!/usr/bin/env python3
"""Create and verify iOS/tvOS simulator launch-and-pixel evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any


REPORT_TYPE = "gpui-apple-simulator-smoke"
SCHEMA_VERSION = 1
MIN_UNIQUE_COLORS = 16
PLATFORMS = {"ios", "tvos"}
REVISION_RE = re.compile(r"^[0-9a-f]{40}$")


class EvidenceError(ValueError):
    """Raised when simulator evidence does not satisfy the release contract."""


def _require(report: dict[str, Any], key: str, expected: Any) -> None:
    actual = report.get(key)
    if actual != expected:
        raise EvidenceError(f"{key} must be {expected!r}, got {actual!r}")


def validate_report(report: dict[str, Any], screenshot: Path) -> None:
    _require(report, "schema_version", SCHEMA_VERSION)
    _require(report, "report_type", REPORT_TYPE)
    platform = report.get("platform")
    if platform not in PLATFORMS:
        raise EvidenceError(f"platform must be one of {sorted(PLATFORMS)}, got {platform!r}")
    for key in ("device_name", "runtime", "device_udid", "bundle_id"):
        value = report.get(key)
        if not isinstance(value, str) or not value.strip():
            raise EvidenceError(f"{key} must be a non-empty string")
    _require(report, "app_installed", True)
    _require(report, "app_launched", True)
    _require(report, "pixel_capture", True)
    _require(report, "capture_transport", "apple-simctl")
    _require(report, "interaction_scope", ["build", "install", "launch", "pixel-capture"])
    launch_pid = report.get("launch_pid")
    if not isinstance(launch_pid, int) or launch_pid <= 0:
        raise EvidenceError(f"launch_pid must be a positive integer, got {launch_pid!r}")
    unique_colors = report.get("pixel_unique_colors")
    if not isinstance(unique_colors, int) or unique_colors < MIN_UNIQUE_COLORS:
        raise EvidenceError(
            f"pixel_unique_colors must be >= {MIN_UNIQUE_COLORS}, got {unique_colors!r}"
        )
    for key in ("pixel_width", "pixel_height"):
        value = report.get(key)
        if not isinstance(value, int) or value <= 0:
            raise EvidenceError(f"{key} must be a positive integer, got {value!r}")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION_RE.fullmatch(revision) is None:
        raise EvidenceError("source_revision must be a 40-character lowercase Git revision")
    if not isinstance(report.get("source_dirty"), bool):
        raise EvidenceError("source_dirty must be a boolean")
    toolchains = report.get("toolchains")
    if not isinstance(toolchains, dict):
        raise EvidenceError("toolchains must be an object")
    for key in ("xcode", "rustc"):
        value = toolchains.get(key)
        if not isinstance(value, str) or not value.strip():
            raise EvidenceError(f"toolchains.{key} must be a non-empty string")
    required_manual = report.get("manual_required")
    if not isinstance(required_manual, list) or not all(
        isinstance(item, str) and item for item in required_manual
    ):
        raise EvidenceError("manual_required must be a non-empty string list")
    if not screenshot.is_file() or screenshot.stat().st_size == 0:
        raise EvidenceError(f"pixel artifact is missing or empty: {screenshot}")
    _require(report, "pixel_artifact", screenshot.name)
    digest = hashlib.sha256(screenshot.read_bytes()).hexdigest()
    _require(report, "pixel_sha256", digest)


def create_report(args: argparse.Namespace) -> dict[str, Any]:
    screenshot: Path = args.screenshot
    if not screenshot.is_file() or screenshot.stat().st_size == 0:
        raise EvidenceError(f"pixel artifact is missing or empty: {screenshot}")
    manual_required = (
        ["touch-navigation", "text-input-ime", "VoiceOver", "rotation", "device-run"]
        if args.platform == "ios"
        else ["remote-focus-navigation", "VoiceOver", "device-run"]
    )
    report: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "report_type": REPORT_TYPE,
        "platform": args.platform,
        "device_name": args.device_name,
        "runtime": args.runtime,
        "device_udid": args.device_udid,
        "bundle_id": args.bundle_id,
        "app_installed": True,
        "app_launched": True,
        "launch_pid": args.launch_pid,
        "pixel_capture": True,
        "pixel_artifact": screenshot.name,
        "pixel_unique_colors": args.unique_colors,
        "pixel_width": args.pixel_width,
        "pixel_height": args.pixel_height,
        "pixel_sha256": hashlib.sha256(screenshot.read_bytes()).hexdigest(),
        "capture_transport": "apple-simctl",
        "interaction_scope": ["build", "install", "launch", "pixel-capture"],
        "source_revision": args.source_revision,
        "source_dirty": args.source_dirty,
        "toolchains": {"xcode": args.xcode, "rustc": args.rustc},
        "manual_required": manual_required,
    }
    validate_report(report, screenshot)
    return report


def write_report(report: dict[str, Any], artifact: Path) -> None:
    artifact.parent.mkdir(parents=True, exist_ok=True)
    temporary = artifact.with_suffix(f"{artifact.suffix}.tmp")
    temporary.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(artifact)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", required=True, type=Path)
    parser.add_argument("--screenshot", required=True, type=Path)
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--platform", choices=sorted(PLATFORMS))
    parser.add_argument("--device-name")
    parser.add_argument("--runtime")
    parser.add_argument("--device-udid")
    parser.add_argument("--bundle-id")
    parser.add_argument("--launch-pid", type=int)
    parser.add_argument("--unique-colors", type=int)
    parser.add_argument("--pixel-width", type=int)
    parser.add_argument("--pixel-height", type=int)
    parser.add_argument("--source-revision")
    parser.add_argument("--source-dirty", action=argparse.BooleanOptionalAction)
    parser.add_argument("--xcode")
    parser.add_argument("--rustc")
    args = parser.parse_args()

    if args.verify:
        report = json.loads(args.artifact.read_text(encoding="utf-8"))
        validate_report(report, args.screenshot)
        return 0

    required = (
        "platform",
        "device_name",
        "runtime",
        "device_udid",
        "bundle_id",
        "launch_pid",
        "unique_colors",
        "pixel_width",
        "pixel_height",
        "source_revision",
        "source_dirty",
        "xcode",
        "rustc",
    )
    missing = [name for name in required if getattr(args, name) is None]
    if missing:
        parser.error(f"creation requires: {', '.join('--' + name.replace('_', '-') for name in missing)}")
    write_report(create_report(args), args.artifact)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (EvidenceError, json.JSONDecodeError, OSError) as error:
        raise SystemExit(f"Apple simulator evidence error: {error}") from error
