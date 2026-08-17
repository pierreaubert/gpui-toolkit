#!/usr/bin/env python3
"""Compare canonical MeshPlot artifacts from two renderer adapters.

The component-lab Metal manifest and the headless WGPU manifest use different
field names, but both describe a set of named captures.  This module keeps the
comparison contract independent of either capture producer.  PNG captures use
pixel thresholds; SVG captures use deterministic XML canonicalization and an
exact semantic comparison.  A case must never silently compare a PNG as SVG
or vice versa.

The PNG decoder accepts the features emitted by the repository's capture
tools: 8-bit, non-interlaced RGB or RGBA images.  Keeping the decoder here
avoids a runtime dependency on Pillow in release and restricted-shell QA jobs.
"""

from __future__ import annotations

import argparse
import json
import math
import struct
import sys
import xml.etree.ElementTree as ET
import zlib
from dataclasses import dataclass
from pathlib import Path, PureWindowsPath
from typing import Any


SCHEMA_VERSION = 1
REPORT_TYPE = "gpui-mesh-plot-cross-adapter-visual-diff"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"
MAX_PNG_BYTES = 128 * 1024 * 1024
MAX_SVG_BYTES = 16 * 1024 * 1024
ARTIFACT_KINDS = frozenset({"png", "svg"})


class VisualCompareError(RuntimeError):
    """Raised when a capture or comparison contract is invalid."""


@dataclass(frozen=True)
class CaptureArtifact:
    path: Path
    kind: str


def _read_object(path: Path, description: str) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise VisualCompareError(f"invalid {description} {path}: {error}") from error
    if not isinstance(value, dict):
        raise VisualCompareError(f"invalid {description} {path}: expected a JSON object")
    return value


def _resolve_artifact_path(manifest_path: Path, repo_root: Path, path_text: str) -> Path:
    image_path = Path(path_text)
    if (
        image_path.is_absolute()
        or PureWindowsPath(path_text).drive
        or not path_text
        or not image_path.parts
        or "\\" in path_text
        or ".." in image_path.parts
    ):
        raise VisualCompareError(f"capture contains an unsafe artifact path: {path_text!r}")

    if len(image_path.parts) == 1:
        image = manifest_path.parent / image_path
    else:
        image = repo_root / image_path
    try:
        image.resolve().relative_to(repo_root.resolve())
    except ValueError as error:
        raise VisualCompareError(
            f"capture artifact escapes the repository: {path_text!r}"
        ) from error
    return image


def _artifact_kind(case: dict[str, Any], path_text: str, case_id: str) -> str:
    """Resolve and validate the explicit artifact kind for one capture.

    Older PNG manifests predate the field, so a missing kind remains
    backwards-compatible for ``.png`` paths only.  SVG is deliberately
    explicit: this prevents a future producer from accidentally applying the
    PNG byte/pixel contract to a vector artifact.
    """

    declared = case.get("artifact_kind") or case.get("kind")
    suffix = Path(path_text).suffix.lower().lstrip(".")
    if declared is None and suffix == "png":
        declared = "png"
    if not isinstance(declared, str) or declared not in ARTIFACT_KINDS:
        raise VisualCompareError(
            f"capture case {case_id} must declare artifact_kind as png or svg"
        )
    if suffix != declared:
        raise VisualCompareError(
            f"capture case {case_id} artifact kind {declared!r} does not match "
            f"path extension {suffix!r}"
        )
    return declared


def _manifest_renderer(manifest: dict[str, Any]) -> str:
    renderer = manifest.get("renderer") or manifest.get("renderer_id")
    if not isinstance(renderer, str) or not renderer.strip():
        renderer = "unknown"
    return renderer


def _manifest_cases(path: Path, repo_root: Path) -> tuple[str, dict[str, CaptureArtifact]]:
    manifest = _read_object(path, "visual capture manifest")
    status = manifest.get("status", "captured")
    if status != "captured" and manifest.get("passed") is not True:
        raise VisualCompareError(
            f"visual capture manifest is not a completed capture: {path}"
        )
    cases = manifest.get("cases")
    if not isinstance(cases, list) or not cases:
        raise VisualCompareError(f"visual capture manifest has no cases: {path}")

    result: dict[str, CaptureArtifact] = {}
    for case in cases:
        if not isinstance(case, dict):
            raise VisualCompareError(f"visual capture manifest has a malformed case: {path}")
        # WGPU uses id/path; component-lab uses capture_id/actual_path.  A
        # future adapter can use comparison_id to make the semantic mapping
        # explicit without changing either producer's native identifier.
        case_id = case.get("comparison_id") or case.get("id") or case.get("capture_id")
        path_text = case.get("path") or case.get("actual_path")
        if not isinstance(case_id, str) or not case_id.strip():
            raise VisualCompareError(f"visual capture case has no stable ID: {path}")
        if case_id in result:
            raise VisualCompareError(f"visual capture case IDs are not unique: {case_id}")
        if not isinstance(path_text, str):
            raise VisualCompareError(f"visual capture case {case_id} has no image path")
        kind = _artifact_kind(case, path_text, case_id)
        artifact = _resolve_artifact_path(path, repo_root, path_text)
        if not artifact.is_file() or artifact.stat().st_size == 0:
            raise VisualCompareError(f"missing visual capture for {case_id}: {artifact}")
        result[case_id] = CaptureArtifact(artifact, kind)
    return _manifest_renderer(manifest), result


def _paeth(left: int, above: int, upper_left: int) -> int:
    estimate = left + above - upper_left
    left_distance = abs(estimate - left)
    above_distance = abs(estimate - above)
    upper_left_distance = abs(estimate - upper_left)
    if left_distance <= above_distance and left_distance <= upper_left_distance:
        return left
    if above_distance <= upper_left_distance:
        return above
    return upper_left


def _decode_png(path: Path) -> tuple[int, int, bytes]:
    try:
        encoded = path.read_bytes()
    except OSError as error:
        raise VisualCompareError(f"cannot read PNG {path}: {error}") from error
    if len(encoded) > MAX_PNG_BYTES or not encoded.startswith(PNG_SIGNATURE):
        raise VisualCompareError(f"unsupported or oversized PNG: {path}")

    offset = len(PNG_SIGNATURE)
    width = height = bit_depth = color_type = interlace = None
    compressed = bytearray()
    saw_iend = False
    while offset < len(encoded):
        if len(encoded) - offset < 12:
            raise VisualCompareError(f"truncated PNG chunk in {path}")
        length = struct.unpack_from(">I", encoded, offset)[0]
        offset += 4
        chunk_type = encoded[offset : offset + 4]
        offset += 4
        end = offset + length
        if end + 4 > len(encoded):
            raise VisualCompareError(f"truncated PNG data in {path}")
        data = encoded[offset:end]
        offset = end
        expected_crc = struct.unpack_from(">I", encoded, offset)[0]
        offset += 4
        actual_crc = zlib.crc32(chunk_type)
        actual_crc = zlib.crc32(data, actual_crc) & 0xFFFFFFFF
        if actual_crc != expected_crc:
            raise VisualCompareError(f"PNG CRC mismatch in {path}")
        if chunk_type == b"IHDR":
            if len(data) != 13 or width is not None:
                raise VisualCompareError(f"invalid PNG header in {path}")
            width, height, bit_depth, color_type, compression, filtering, interlace = struct.unpack(
                ">IIBBBBB", data
            )
            if (
                width == 0
                or height == 0
                or bit_depth != 8
                or color_type not in (2, 6)
                or compression != 0
                or filtering != 0
                or interlace != 0
            ):
                raise VisualCompareError(
                    f"PNG must be 8-bit non-interlaced RGB/RGBA: {path}"
                )
        elif chunk_type == b"IDAT":
            compressed.extend(data)
        elif chunk_type == b"IEND":
            saw_iend = True
            break

    if width is None or height is None or color_type is None or not saw_iend:
        raise VisualCompareError(f"PNG is missing required chunks: {path}")
    channels = 4 if color_type == 6 else 3
    row_bytes = width * channels
    expected_size = (row_bytes + 1) * height
    try:
        decoded = zlib.decompress(compressed)
    except zlib.error as error:
        raise VisualCompareError(f"invalid PNG compression in {path}: {error}") from error
    if len(decoded) != expected_size:
        raise VisualCompareError(f"PNG scanline size mismatch in {path}")

    rows: list[bytearray] = []
    cursor = 0
    for row_index in range(height):
        filter_type = decoded[cursor]
        cursor += 1
        source = decoded[cursor : cursor + row_bytes]
        cursor += row_bytes
        row = bytearray(row_bytes)
        previous = rows[row_index - 1] if row_index else None
        for index, value in enumerate(source):
            left = row[index - channels] if index >= channels else 0
            above = previous[index] if previous is not None else 0
            upper_left = previous[index - channels] if previous is not None and index >= channels else 0
            if filter_type == 0:
                prediction = 0
            elif filter_type == 1:
                prediction = left
            elif filter_type == 2:
                prediction = above
            elif filter_type == 3:
                prediction = (left + above) // 2
            elif filter_type == 4:
                prediction = _paeth(left, above, upper_left)
            else:
                raise VisualCompareError(f"unsupported PNG filter {filter_type} in {path}")
            row[index] = (value + prediction) & 0xFF
        rows.append(row)

    if channels == 4:
        return width, height, b"".join(bytes(row) for row in rows)
    rgba = bytearray(width * height * 4)
    destination = 0
    for row in rows:
        for index in range(0, len(row), 3):
            rgba[destination : destination + 3] = row[index : index + 3]
            rgba[destination + 3] = 255
            destination += 4
    return width, height, bytes(rgba)


def _canonical_svg(path: Path) -> bytes:
    try:
        encoded = path.read_bytes()
    except OSError as error:
        raise VisualCompareError(f"cannot read SVG {path}: {error}") from error
    if len(encoded) > MAX_SVG_BYTES:
        raise VisualCompareError(f"unsupported or oversized SVG: {path}")
    try:
        text = encoded.decode("utf-8-sig")
        canonical = ET.canonicalize(
            xml_data=text,
            strip_text=True,
            rewrite_prefixes=True,
            with_comments=False,
        )
    except (UnicodeDecodeError, ET.ParseError, ValueError) as error:
        raise VisualCompareError(f"invalid SVG {path}: {error}") from error
    return canonical.encode("utf-8")


def _compare_images(
    left_path: Path,
    right_path: Path,
    *,
    max_channel_delta: int,
    max_changed_fraction: float,
) -> dict[str, Any]:
    left_width, left_height, left_pixels = _decode_png(left_path)
    right_width, right_height, right_pixels = _decode_png(right_path)
    if (left_width, left_height) != (right_width, right_height):
        return {
            "status": "Failed",
            "width": left_width,
            "height": left_height,
            "right_width": right_width,
            "right_height": right_height,
            "changed_pixels": None,
            "changed_fraction": None,
            "max_channel_delta": None,
            "mean_channel_delta": None,
            "error": "capture dimensions do not match",
        }

    changed_pixels = 0
    max_delta = 0
    delta_sum = 0
    for index in range(0, len(left_pixels), 4):
        deltas = [abs(left_pixels[index + channel] - right_pixels[index + channel]) for channel in range(4)]
        pixel_delta = max(deltas)
        max_delta = max(max_delta, pixel_delta)
        delta_sum += sum(deltas)
        if pixel_delta > max_channel_delta:
            changed_pixels += 1
    pixel_count = left_width * left_height
    changed_fraction = changed_pixels / pixel_count
    mean_delta = delta_sum / len(left_pixels)
    passed = changed_fraction <= max_changed_fraction
    return {
        "status": "Passed" if passed else "Failed",
        "width": left_width,
        "height": left_height,
        "changed_pixels": changed_pixels,
        "changed_fraction": changed_fraction,
        "max_channel_delta": max_delta,
        "mean_channel_delta": mean_delta,
    }


def _compare_svgs(left_path: Path, right_path: Path) -> dict[str, Any]:
    left = _canonical_svg(left_path)
    right = _canonical_svg(right_path)
    equal = left == right
    return {
        "status": "Passed" if equal else "Failed",
        "canonical_equal": equal,
        "left_canonical_bytes": len(left),
        "right_canonical_bytes": len(right),
        "error": None if equal else "canonical SVG artifacts differ",
    }


def compare_manifests(
    left_path: Path,
    right_path: Path,
    *,
    repo_root: Path,
    max_channel_delta: int = 0,
    max_changed_fraction: float = 0.0,
) -> dict[str, Any]:
    """Compare two capture manifests and return a deterministic JSON report."""

    if max_channel_delta < 0:
        raise VisualCompareError("max channel delta must be non-negative")
    if not 0.0 <= max_changed_fraction <= 1.0 or not math.isfinite(max_changed_fraction):
        raise VisualCompareError("max changed fraction must be between 0 and 1")

    left_renderer, left_cases = _manifest_cases(left_path, repo_root)
    right_renderer, right_cases = _manifest_cases(right_path, repo_root)
    if set(left_cases) != set(right_cases):
        missing_left = sorted(set(right_cases) - set(left_cases))
        missing_right = sorted(set(left_cases) - set(right_cases))
        raise VisualCompareError(
            "capture case IDs do not match: "
            f"missing_left={missing_left}, missing_right={missing_right}"
        )

    cases: list[dict[str, Any]] = []
    for case_id in sorted(left_cases):
        left_artifact = left_cases[case_id]
        right_artifact = right_cases[case_id]
        if left_artifact.kind != right_artifact.kind:
            raise VisualCompareError(
                f"capture case {case_id} artifact kinds do not match: "
                f"{left_artifact.kind} vs {right_artifact.kind}"
            )
        if left_artifact.kind == "png":
            result = _compare_images(
                left_artifact.path,
                right_artifact.path,
                max_channel_delta=max_channel_delta,
                max_changed_fraction=max_changed_fraction,
            )
        else:
            result = _compare_svgs(left_artifact.path, right_artifact.path)
        cases.append(
            {
                "id": case_id,
                "artifact_kind": left_artifact.kind,
                "left_path": left_artifact.path
                .resolve()
                .relative_to(repo_root.resolve())
                .as_posix(),
                "right_path": right_artifact.path
                .resolve()
                .relative_to(repo_root.resolve())
                .as_posix(),
                **result,
            }
        )

    failed_count = sum(case["status"] != "Passed" for case in cases)
    kinds = {case["artifact_kind"] for case in cases}
    return {
        "schema_version": SCHEMA_VERSION,
        "report_type": REPORT_TYPE,
        "artifact_kind": next(iter(kinds)) if len(kinds) == 1 else "mixed",
        "passed": failed_count == 0,
        "left_renderer": left_renderer,
        "right_renderer": right_renderer,
        "max_channel_delta": max_channel_delta,
        "max_changed_fraction": max_changed_fraction,
        "compared_count": len(cases),
        "failed_count": failed_count,
        "cases": cases,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--left", type=Path, required=True)
    parser.add_argument("--right", type=Path, required=True)
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--output-report", type=Path)
    parser.add_argument("--max-channel-delta", type=int, default=0)
    parser.add_argument("--max-changed-fraction", type=float, default=0.0)
    args = parser.parse_args(argv)

    try:
        report = compare_manifests(
            args.left,
            args.right,
            repo_root=args.repo_root,
            max_channel_delta=args.max_channel_delta,
            max_changed_fraction=args.max_changed_fraction,
        )
        if args.output_report is not None:
            args.output_report.parent.mkdir(parents=True, exist_ok=True)
            args.output_report.write_text(
                json.dumps(report, indent=2, sort_keys=True) + "\n",
                encoding="utf-8",
            )
        print(
            f"MeshPlot cross-adapter visual comparison "
            f"{'passed' if report['passed'] else 'failed'} "
            f"({report['compared_count']} cases, {report['failed_count']} failed)"
        )
        return 0 if report["passed"] else 1
    except VisualCompareError as error:
        print(f"MeshPlot cross-adapter visual comparison failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
