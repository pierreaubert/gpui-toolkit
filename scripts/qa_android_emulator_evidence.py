#!/usr/bin/env python3
"""Create and verify Android emulator launch, touch, pixel, and accessibility evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path
from typing import Any


REPORT_TYPE = "gpui-android-emulator-smoke"
SCHEMA_VERSION = 1
MIN_UNIQUE_COLORS = 16
REVISION_RE = re.compile(r"^[0-9a-f]{40}$")


class EvidenceError(ValueError):
    """Raised when Android evidence does not satisfy the release contract."""


def _require(report: dict[str, Any], key: str, expected: Any) -> None:
    actual = report.get(key)
    if actual != expected:
        raise EvidenceError(f"{key} must be {expected!r}, got {actual!r}")


def _artifact_digest(path: Path) -> str:
    if not path.is_file() or path.stat().st_size == 0:
        raise EvidenceError(f"artifact is missing or empty: {path}")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def validate_report(
    report: dict[str, Any], before: Path, after: Path, accessibility: Path
) -> None:
    _require(report, "schema_version", SCHEMA_VERSION)
    _require(report, "report_type", REPORT_TYPE)
    for key in ("device_name", "serial", "abi", "package", "activity"):
        value = report.get(key)
        if not isinstance(value, str) or not value.strip():
            raise EvidenceError(f"{key} must be a non-empty string")
    for key in ("api_level", "launch_pid", "launch_time_ms"):
        value = report.get(key)
        if not isinstance(value, int) or value <= 0:
            raise EvidenceError(f"{key} must be a positive integer, got {value!r}")
    for key in (
        "app_installed",
        "app_launched",
        "touch_injected",
        "render_changed_after_touch",
        "accessibility_tree_exported",
    ):
        _require(report, key, True)
    _require(report, "capture_transport", "android-adb")
    _require(
        report,
        "interaction_scope",
        ["build", "install", "launch", "pixel-capture", "touch-navigation", "accessibility-tree"],
    )
    pixel = report.get("pixel")
    if not isinstance(pixel, dict):
        raise EvidenceError("pixel must be an object")
    for key in ("width", "height"):
        value = pixel.get(key)
        if not isinstance(value, int) or value <= 0:
            raise EvidenceError(f"pixel.{key} must be a positive integer")
    for key in ("before_unique_colors", "after_unique_colors"):
        value = pixel.get(key)
        if not isinstance(value, int) or value < MIN_UNIQUE_COLORS:
            raise EvidenceError(f"pixel.{key} must be >= {MIN_UNIQUE_COLORS}")
    _require(report, "before_artifact", before.name)
    _require(report, "after_artifact", after.name)
    _require(report, "accessibility_artifact", accessibility.name)
    _require(report, "before_sha256", _artifact_digest(before))
    _require(report, "after_sha256", _artifact_digest(after))
    _require(report, "accessibility_sha256", _artifact_digest(accessibility))
    if report["before_sha256"] == report["after_sha256"]:
        raise EvidenceError("touch navigation must change the rendered pixels")
    for key in ("accessibility_node_count", "accessible_named_node_count"):
        value = report.get(key)
        if not isinstance(value, int) or value <= 0:
            raise EvidenceError(f"{key} must be a positive integer")
    revision = report.get("source_revision")
    if not isinstance(revision, str) or REVISION_RE.fullmatch(revision) is None:
        raise EvidenceError("source_revision must be a 40-character lowercase Git revision")
    if not isinstance(report.get("source_dirty"), bool):
        raise EvidenceError("source_dirty must be a boolean")
    toolchains = report.get("toolchains")
    if not isinstance(toolchains, dict):
        raise EvidenceError("toolchains must be an object")
    for key in ("adb", "java", "rustc"):
        value = toolchains.get(key)
        if not isinstance(value, str) or not value.strip():
            raise EvidenceError(f"toolchains.{key} must be a non-empty string")
    manual = report.get("manual_required")
    if not isinstance(manual, list) or not all(isinstance(item, str) and item for item in manual):
        raise EvidenceError("manual_required must be a non-empty string list")


def create_report(args: argparse.Namespace) -> dict[str, Any]:
    before_digest = _artifact_digest(args.before)
    after_digest = _artifact_digest(args.after)
    accessibility_digest = _artifact_digest(args.accessibility)
    report: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "report_type": REPORT_TYPE,
        "device_name": args.device_name,
        "serial": args.serial,
        "api_level": args.api_level,
        "abi": args.abi,
        "package": args.package,
        "activity": args.activity,
        "app_installed": True,
        "app_launched": True,
        "launch_pid": args.launch_pid,
        "launch_time_ms": args.launch_time_ms,
        "touch_injected": True,
        "render_changed_after_touch": before_digest != after_digest,
        "accessibility_tree_exported": True,
        "accessibility_node_count": args.accessibility_node_count,
        "accessible_named_node_count": args.accessible_named_node_count,
        "before_artifact": args.before.name,
        "after_artifact": args.after.name,
        "accessibility_artifact": args.accessibility.name,
        "before_sha256": before_digest,
        "after_sha256": after_digest,
        "accessibility_sha256": accessibility_digest,
        "pixel": {
            "width": args.pixel_width,
            "height": args.pixel_height,
            "before_unique_colors": args.before_unique_colors,
            "after_unique_colors": args.after_unique_colors,
        },
        "capture_transport": "android-adb",
        "interaction_scope": [
            "build",
            "install",
            "launch",
            "pixel-capture",
            "touch-navigation",
            "accessibility-tree",
        ],
        "source_revision": args.source_revision,
        "source_dirty": args.source_dirty,
        "toolchains": {"adb": args.adb, "java": args.java, "rustc": args.rustc},
        "manual_required": [
            "TalkBack-navigation-and-actions",
            "text-input-ime",
            "rotation-and-lifecycle",
            "physical-device",
            "hardware-gpu",
        ],
    }
    validate_report(report, args.before, args.after, args.accessibility)
    return report


def write_report(report: dict[str, Any], artifact: Path) -> None:
    artifact.parent.mkdir(parents=True, exist_ok=True)
    temporary = artifact.with_suffix(f"{artifact.suffix}.tmp")
    temporary.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(artifact)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", required=True, type=Path)
    parser.add_argument("--before", required=True, type=Path)
    parser.add_argument("--after", required=True, type=Path)
    parser.add_argument("--accessibility", required=True, type=Path)
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--device-name")
    parser.add_argument("--serial")
    parser.add_argument("--api-level", type=int)
    parser.add_argument("--abi")
    parser.add_argument("--package")
    parser.add_argument("--activity")
    parser.add_argument("--launch-pid", type=int)
    parser.add_argument("--launch-time-ms", type=int)
    parser.add_argument("--accessibility-node-count", type=int)
    parser.add_argument("--accessible-named-node-count", type=int)
    parser.add_argument("--before-unique-colors", type=int)
    parser.add_argument("--after-unique-colors", type=int)
    parser.add_argument("--pixel-width", type=int)
    parser.add_argument("--pixel-height", type=int)
    parser.add_argument("--source-revision")
    parser.add_argument("--source-dirty", action=argparse.BooleanOptionalAction)
    parser.add_argument("--adb")
    parser.add_argument("--java")
    parser.add_argument("--rustc")
    args = parser.parse_args()
    if args.verify:
        validate_report(
            json.loads(args.artifact.read_text(encoding="utf-8")),
            args.before,
            args.after,
            args.accessibility,
        )
        return 0
    required = [
        name
        for name in vars(args)
        if name not in {"artifact", "before", "after", "accessibility", "verify"}
        and getattr(args, name) is None
    ]
    if required:
        parser.error(f"creation requires: {', '.join('--' + name.replace('_', '-') for name in required)}")
    write_report(create_report(args), args.artifact)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (EvidenceError, json.JSONDecodeError, OSError) as error:
        raise SystemExit(f"Android emulator evidence error: {error}") from error
